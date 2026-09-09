"""``orchestration.service`` — U3 OrchestrationService (E3-S1..E3-S7).

The integration-orchestration shell over okc-core's authoritative, core-derived
checkpoint. All engine access goes through the U0 ``EngineWorker`` and the adapter
DTOs (ADR-0002 — no ``okc`` import here). RBAC is enforced in the router BEFORE any
method here runs (C-1). Reserving mutations respect the single active-mutation gate
(Q7) and the single-writer worker.
"""

from __future__ import annotations

import os
from collections.abc import Callable
from datetime import UTC, datetime

from ulid import ULID

from app.adapter import dto
from app.adapter.engine import OkcEngineImpl
from app.adapter.queue import EngineWorker
from app.config import AppConfig
from app.orchestration import projects as reg
from app.orchestration.models import (
    CompileResultView,
    CreateProjectRequest,
    FreezeResponse,
    JobAccepted,
    PreflightGateView,
    ProjectStatusView,
    ProjectView,
    ProviderProfileView,
    RouteBoundaryView,
    RunIntegrationRequest,
)
from app.serving.snapshots import SnapshotStore, compilation_receipt
from app.shared.authz import AdminPrincipal
from app.shared.error import EngineError, EngineErrorCategory, EngineErrorCode
from app.shared.jobs import JobProgress


def _now() -> str:
    return datetime.now(UTC).isoformat()


def _project_busy(project_id: str) -> EngineError:
    return EngineError(
        EngineErrorCode.PROJECT_BUSY,
        EngineErrorCategory.CONCURRENCY,
        f"project {project_id} has an integration job in progress",
    ).as_retryable()


class OrchestrationService:
    def __init__(
        self, registry: reg.ProjectRegistry, engine: EngineWorker | None, config: AppConfig
    ) -> None:
        self._registry = registry
        self._engine = engine
        self._config = config
        self._snapshots = SnapshotStore(registry._db)

    # --- helpers ---

    def _require_engine(self) -> EngineWorker:
        if self._engine is None:
            raise EngineError.internal("engine worker unavailable")
        return self._engine

    def _resolve_in_root(self, path: str) -> str:
        """Return the absolute ``path`` iff it resolves inside projects_root."""
        cand = os.path.abspath(path)
        root = os.path.abspath(self._config.projects_root)
        try:
            if os.path.commonpath([cand, root]) != root:
                raise EngineError(
                    EngineErrorCode.PATH_UNSAFE, EngineErrorCategory.VALIDATION,
                    "path escapes the configured projects root",
                )
        except ValueError as exc:  # different drive / mixed roots
            raise EngineError(
                EngineErrorCode.PATH_UNSAFE, EngineErrorCategory.VALIDATION,
                "path escapes the configured projects root",
            ) from exc
        return cand

    def _guard_not_busy(self, project_id: str) -> None:
        if self._registry.has_active_reserving_job(project_id):
            raise _project_busy(project_id)

    def _to_project_view(self, row: reg.ProjectRow) -> ProjectView:
        return ProjectView(
            id=row.id, name=row.name, engine_root_abs_path=row.engine_root_abs_path,
            curator_id=row.curator_id, freeze_state=row.freeze_state, frozen_at=row.frozen_at,
            source_count=self._registry.source_count(row.id), created_at=row.created_at,
        )

    # --- E3-S1: project creation ---

    async def create_project(
        self, principal: AdminPrincipal, req: CreateProjectRequest
    ) -> ProjectView:
        engine = self._require_engine()
        project_id = f"proj_{ULID()}"
        if req.root is not None:
            # Core requires the directory to end in ``.okc-project``; it validates the
            # suffix and returns PROJECT_INVALID otherwise. We only enforce in-root here.
            engine_root = self._resolve_in_root(req.root)
        else:
            engine_root = os.path.abspath(
                os.path.join(self._config.projects_root, f"{project_id}.okc-project")
            )
        spec = dto.CreateProjectSpec(
            root_abs_path=engine_root, name=req.name, curator_id=principal.curator_label
        )
        ref = await engine.call(lambda e: e.create_project(spec))
        self._registry.create(
            project_id=project_id, name=req.name, engine_root=ref.root_abs_path,
            curator_id=principal.curator_label, created_by=principal.account_id,
        )
        return self._to_project_view(self._registry.get(project_id))

    def get_project(self, project_id: str) -> ProjectView:
        return self._to_project_view(self._registry.get(project_id))

    def list_projects(self) -> list[ProjectView]:
        return [self._to_project_view(r) for r in self._registry.list_rows()]

    # --- E3-S2: freeze (reproducible snapshot gate, Q1) ---

    def freeze(self, project_id: str) -> FreezeResponse:
        self._registry.get(project_id)  # 404 fast
        count = self._registry.source_count(project_id)
        if count < 1:
            raise EngineError.validation("cannot freeze a project with no registered sources")
        # The ≤10 cap is enforced by U2's SlotAccountant at add-source time; this is a
        # defensive read-side backstop (C-3).
        if count > 10:
            raise EngineError(
                EngineErrorCode.SOURCE_CAP_EXCEEDED, EngineErrorCategory.LIMIT,
                "project exceeds the 10-source cap (federation not implemented)",
            )
        fingerprint = self._registry.source_set_fingerprint(project_id)
        frozen_at = _now()
        self._registry.set_frozen(project_id, fingerprint, frozen_at)
        return FreezeResponse(
            project_id=project_id, freeze_state="frozen", source_set_fingerprint=fingerprint,
            frozen_at=frozen_at, source_count=count,
        )

    # --- E3-S3 / E3-S5: checkpoint projection + staleness ---

    async def status(self, project_id: str) -> ProjectStatusView:
        row = self._registry.get(project_id)
        engine = self._require_engine()
        status = await engine.read(lambda e: e.status(row.engine_root_abs_path))
        checkpoint = str(status.checkpoint)
        next_action, resolver = reg.CHECKPOINT_ACTIONS.get(
            checkpoint, ("await the current operation", "u3")
        )
        blocked = self._blocked_steps(checkpoint)
        return ProjectStatusView(
            project_id=project_id, checkpoint=checkpoint, next_action=next_action,
            resolver=resolver, progression=reg.PROGRESSION, blocked_steps=blocked,
            stale=self._is_stale(row), frozen=row.freeze_state == "frozen",
            source_count=self._registry.source_count(project_id),
            integration_plan_id=None,
        )

    def _blocked_steps(self, checkpoint: str) -> list[str]:
        if checkpoint not in reg.PROGRESSION:
            return []
        idx = reg.PROGRESSION.index(checkpoint)
        return reg.PROGRESSION[idx + 1 :]

    def _is_stale(self, row: reg.ProjectRow) -> bool:
        """Advisory staleness (Q6): unfrozen, or the live source-set fingerprint has
        drifted from the frozen one. The authoritative core signal (APPROVAL_STALE /
        checkpoint regression) always wins and surfaces as an error at action time."""
        if row.freeze_state != "frozen" or row.source_set_fingerprint is None:
            return True
        live = self._registry.source_set_fingerprint(row.id)
        return live != row.source_set_fingerprint

    def _require_current_freeze(self, row: reg.ProjectRow) -> None:
        if row.freeze_state != "frozen":
            raise EngineError.validation("project is not frozen; freeze the source set first")
        live = self._registry.source_set_fingerprint(row.id)
        if live != row.source_set_fingerprint:
            raise EngineError(
                EngineErrorCode.APPROVAL_STALE, EngineErrorCategory.APPROVAL,
                "sources changed since freeze; re-freeze before running or compiling",
            )

    # --- E3-S4: provider binding ---

    def list_providers(self) -> list[ProviderProfileView]:
        return [
            ProviderProfileView(name=p.name, kind=p.kind, endpoint=p.endpoint, model=p.model)
            for p in self._config.providers
        ]

    async def bind_provider(
        self, project_id: str, profile_name: str, role: str | None = None
    ) -> ProjectStatusView:
        row = self._registry.get(project_id)
        names = {p.name for p in self._config.providers}
        if profile_name not in names:
            raise EngineError.validation(f"unknown provider profile '{profile_name}'")
        # ``role=None`` sets the default profile for all roles; a role string binds
        # just that role (okc-core AiRole literals), enabling e.g. a dedicated
        # embedding model alongside a generative default.
        valid_roles = {"embedding", "organizer", "synthesis", "critic"}
        if role is not None and role not in valid_roles:
            raise EngineError.validation(
                f"unknown AI role '{role}' (expected one of {sorted(valid_roles)} or null)"
            )
        self._guard_not_busy(project_id)
        engine = self._require_engine()
        await engine.call(
            lambda e: e.set_ai_route(row.engine_root_abs_path, profile_name, role)
        )
        return await self.status(project_id)

    # --- E3-S4: preflight + remote disclosure gate (Q2 direct await, Q5 boundaries) ---

    async def preflight(self, project_id: str) -> PreflightGateView:
        row = self._registry.get(project_id)
        self._require_current_freeze(row)
        self._guard_not_busy(project_id)
        engine = self._require_engine()
        pf = await engine.call(lambda e: e.preflight(row.engine_root_abs_path))
        routes = self._parse_routes(pf.routes)
        return PreflightGateView(
            project_id=project_id, sensitive_findings=pf.sensitive_findings,
            routes=routes, requires_disclosure=any(r.boundary == "remote" for r in routes),
        )

    @staticmethod
    def _parse_routes(raw: object) -> list[RouteBoundaryView]:
        routes: list[RouteBoundaryView] = []
        if isinstance(raw, list):
            for item in raw:
                if isinstance(item, dict):
                    routes.append(
                        RouteBoundaryView(
                            role=str(item.get("role", "")),
                            profile_name=str(item.get("profile_name", "")),
                            boundary=str(item.get("boundary", "local")),
                        )
                    )
        return routes

    # --- E3-S3/S4/S7: integrate (long op -> JobId) ---

    async def integrate(self, project_id: str, req: RunIntegrationRequest) -> JobAccepted:
        row = self._registry.get(project_id)
        self._require_current_freeze(row)
        self._guard_not_busy(project_id)
        engine = self._require_engine()
        # Disclosure gate from real preflight boundaries (Q5).
        pf = await engine.call(lambda e: e.preflight(row.engine_root_abs_path))
        routes = self._parse_routes(pf.routes)
        remote = any(r.boundary == "remote" for r in routes)
        if remote and not (req.allow_remote_provider and req.remote_disclosure_confirmed):
            raise EngineError(
                EngineErrorCode.REMOTE_CONSENT_REQUIRED, EngineErrorCategory.CONSENT,
                "a remote provider route requires explicit disclosure and consent",
            )
        disclosure = dto.DisclosureCmd(
            allow_remote_provider=remote and req.allow_remote_provider,
            remote_disclosure_confirmed=remote and req.remote_disclosure_confirmed,
        )
        # Re-check just before enqueue (the preflight await released the worker).
        self._guard_not_busy(project_id)
        root = row.engine_root_abs_path

        def run(e: OkcEngineImpl, progress: Callable[[JobProgress], None]) -> None:
            e.integrate(root, disclosure, progress)

        job_id = engine.enqueue("integrate", project_id, None, run)
        return JobAccepted(job_id=job_id, project_id=project_id)

    # --- E3-S6: compile (ready-to-compile, no-clobber) ---

    async def compile(self, project_id: str, output_path: str | None) -> CompileResultView:
        row = self._registry.get(project_id)
        # (2) validate the explicit output path (cheap input validation) before state.
        if output_path is not None:
            output = self._resolve_in_root(output_path)
            if os.path.exists(output):
                raise EngineError(
                    EngineErrorCode.OUTPUT_EXISTS, EngineErrorCategory.PROJECT,
                    "output path already exists (no-clobber)",
                )
        else:
            output = os.path.abspath(
                os.path.join(self._config.projects_root, project_id, "compiled", str(ULID()))
            )
        self._guard_not_busy(project_id)
        self._require_current_freeze(row)
        engine = self._require_engine()
        status = await engine.read(lambda e: e.status(row.engine_root_abs_path))
        if str(status.checkpoint) not in {"ready_to_compile", "verified"}:
            raise EngineError(
                EngineErrorCode.APPROVAL_REQUIRED, EngineErrorCategory.APPROVAL,
                "project is not ready to compile (approvals incomplete)",
            )
        def compile_and_record(e: OkcEngineImpl) -> dto.CompileView:
            # Recheck inside the serialized writer: an upload may have queued
            # while the HTTP handler was awaiting the checkpoint read.
            self._require_current_freeze(self._registry.get(project_id))
            cv = e.compile(row.engine_root_abs_path, output)
            verified = e.verify(cv.path)
            if not verified.valid or not isinstance(verified.manifest, dict):
                raise EngineError(EngineErrorCode.VERIFICATION_FAILED, EngineErrorCategory.VERIFICATION,
                                  "compiled artifact failed verification")
            receipt = compilation_receipt(cv.path, verified.manifest,
                self._registry.source_set_fingerprint(project_id), row.engine_root_abs_path)
            receipt["decision_fingerprint"] = self._snapshots.decision_fingerprint(project_id)
            with self._registry._db.engine.connect() as conn:
                from sqlalchemy import text
                labels = conn.execute(text(
                    "SELECT source_id,owner_display_name,owner_kind,document_id FROM sources WHERE project_id=:pid"
                ), {"pid": project_id}).mappings().all()
                receipt["owner_labels"] = {r["source_id"]: {
                    "display_name": r["owner_display_name"], "owner_kind": r["owner_kind"],
                    "document_id": r["document_id"]} for r in labels}
            self._snapshots.record(project_id, receipt)
            return cv
        cv = await engine.call(compile_and_record)
        return CompileResultView(
            project_id=project_id, path=cv.path,
            integration_plan_id=cv.integration_plan_id, file_count=cv.file_count,
        )
