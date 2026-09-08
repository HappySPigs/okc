"""``serving.service`` — U5 ServingService facade (E5-S1..E5-S5).

Fans out over three concerns: the admin-gated publish/unpublish state flip
(``ServingStateStore``, no engine op), the machine read-only compiled-vault reads
(3-root filesystem + ``verify``/``explain`` via the non-reserving engine read path),
and the okc-mcp consumption contract. All engine access is through the U0
``EngineWorker`` + adapter DTOs (ADR-0002 — no ``okc`` import here).
"""

from __future__ import annotations

import os
from typing import Any

from sqlalchemy import text

from app.adapter.queue import EngineWorker
from app.config import AppConfig
from app.orchestration.projects import ProjectRegistry
from app.serving.models import (
    ContractEndpoint,
    ContractLocation,
    McpContractView,
    PublicationView,
    ServingFileListView,
    ServingProvenanceView,
    ServingVerifyView,
)
from app.serving.store import PublicationRow, ServingStateStore
from app.shared.error import EngineError, EngineErrorCategory, EngineErrorCode
from app.shared.state import StateDb

# The compiled merged vault is a Markdown-only 3-root layout (C-6). Only these
# roots are served; anything else (or a traversal) is 404.
SERVED_ROOTS: tuple[str, ...] = ("knowledge", "legacy", ".okc")


class ServingService:
    def __init__(
        self,
        *,
        db: StateDb,
        engine: EngineWorker | None,
        config: AppConfig,
        store: ServingStateStore,
        projects: ProjectRegistry,
    ) -> None:
        self._db = db
        self._engine = engine
        self._config = config
        self._store = store
        self._projects = projects

    # --- helpers ---

    def _require_engine(self) -> EngineWorker:
        if self._engine is None:
            raise EngineError.internal("engine worker unavailable")
        return self._engine

    def _require_published(self, project_id: str) -> PublicationRow:
        """A machine read is allowed only for a published (live|stale) project."""
        row = self._store.get(project_id)
        if row is None or row.status == "offline":
            raise EngineError.not_found("no published serving for this project")
        return row

    def _effective_status(self, row: PublicationRow) -> tuple[str, bool]:
        """Derived staleness (E5-S5): a live publication whose bound corpus hash no
        longer matches the project's current source-set fingerprint is ``stale`` —
        still served, but labelled so consumers do not mistake it for fresh."""
        if row.status == "offline":
            return "offline", False
        current = self._projects.source_set_fingerprint(row.project_id)
        if row.bound_corpus_hash and current != row.bound_corpus_hash:
            return "stale", True
        return "live", False

    def _resolve_served_path(self, vault_root: str, rel_path: str) -> str:
        """Return the absolute path of ``rel_path`` iff it is an existing file under
        one of the three served roots. Any traversal / out-of-root / miss → 404."""
        if not rel_path or rel_path.startswith(("/", "\\")) or "\x00" in rel_path:
            raise EngineError.not_found("file not found")
        vault_real = os.path.realpath(vault_root)
        target = os.path.realpath(os.path.join(vault_root, rel_path))
        try:
            if os.path.commonpath([target, vault_real]) != vault_real:
                raise EngineError.not_found("file not found")
        except ValueError as exc:  # mixed drives / roots
            raise EngineError.not_found("file not found") from exc
        under_served_root = False
        for served in SERVED_ROOTS:
            root_real = os.path.realpath(os.path.join(vault_root, served))
            try:
                if os.path.commonpath([target, root_real]) == root_real:
                    under_served_root = True
                    break
            except ValueError:
                continue
        if not under_served_root or not os.path.isfile(target):
            raise EngineError.not_found("file not found")
        return target

    def _list_files(self, vault_root: str) -> list[str]:
        out: list[str] = []
        for served in SERVED_ROOTS:
            base = os.path.join(vault_root, served)
            if not os.path.isdir(base):
                continue
            for dirpath, _dirs, files in os.walk(base):
                for name in files:
                    rel = os.path.relpath(os.path.join(dirpath, name), vault_root)
                    out.append(rel.replace(os.sep, "/"))
        return sorted(out)

    def _owner_labels(self, project_id: str) -> dict[str, Any]:
        """Read-only owner labels from the U3-owned ``sources`` table (E5-S3)."""
        with self._db.engine.connect() as conn:
            rows = conn.execute(
                text(
                    "SELECT source_id, owner_display_name, owner_kind, document_id"
                    " FROM sources WHERE project_id=:pid"
                ),
                {"pid": project_id},
            ).mappings().all()
        return {
            r["source_id"]: {
                "owner_display_name": r["owner_display_name"],
                "owner_kind": r["owner_kind"],
                "document_id": r["document_id"],
            }
            for r in rows
        }

    @staticmethod
    def _media_type(path: str) -> str:
        lower = path.lower()
        if lower.endswith((".md", ".markdown")):
            return "text/markdown; charset=utf-8"
        if lower.endswith(".json"):
            return "application/json"
        return "text/plain; charset=utf-8"

    def _compiled_vault_path(self, project_id: str, manifest: Any) -> str | None:
        """Determine the compiled vault directory. Best-effort peek at the engine
        manifest for an output-path key; otherwise the newest dir under the U3
        compile convention ``{projects_root}/{project_id}/compiled/{run}``."""
        if isinstance(manifest, dict):
            for key in ("output_path", "compiled_vault_path", "vault_path", "path"):
                val = manifest.get(key)
                if isinstance(val, str) and os.path.isdir(val):
                    return os.path.abspath(val)
        base = os.path.join(self._config.projects_root, project_id, "compiled")
        if os.path.isdir(base):
            subdirs = [
                os.path.join(base, d)
                for d in os.listdir(base)
                if os.path.isdir(os.path.join(base, d))
            ]
            if subdirs:
                return os.path.abspath(max(subdirs, key=os.path.getmtime))
        return None

    @staticmethod
    def _manifest_identity(manifest: Any) -> tuple[str | None, str | None]:
        if isinstance(manifest, dict):
            plan = manifest.get("integration_plan_id")
            tax = manifest.get("taxonomy_hash")
            return (
                plan if isinstance(plan, str) else None,
                tax if isinstance(tax, str) else None,
            )
        return None, None

    def _publication_view(self, project_id: str) -> PublicationView:
        row = self._store.get(project_id)
        if row is None:
            self._projects.get(project_id)  # 404 if the project itself is unknown
            return PublicationView(project_id=project_id, status="offline")
        status, stale = self._effective_status(row)
        return PublicationView(
            project_id=project_id, status=status, stale=stale,
            compiled_vault_path=row.compiled_vault_path,
            bound_integration_plan_id=row.bound_integration_plan_id,
            bound_corpus_hash=row.bound_corpus_hash,
            bound_taxonomy_hash=row.bound_taxonomy_hash,
            published_at=row.published_at, published_by=row.published_by,
        )

    # --- E5-S1: admin publish / unpublish / status (pure state flip) ---

    async def publish(self, project_id: str, published_by: str | None) -> PublicationView:
        row = self._projects.get(project_id)  # 404 if unknown project
        engine = self._require_engine()
        status = await engine.read(lambda e: e.status(row.engine_root_abs_path))
        if str(status.checkpoint) != "verified":
            raise EngineError(
                EngineErrorCode.APPROVAL_REQUIRED, EngineErrorCategory.APPROVAL,
                "project is not Verified; compile the merged vault before publishing",
            )
        manifest: Any = None
        try:
            manifest = await engine.read(lambda e: e.manifest(row.engine_root_abs_path))
        except EngineError:
            manifest = None
        vault_path = self._compiled_vault_path(project_id, manifest)
        if vault_path is None:
            raise EngineError.not_found("no compiled vault found for this project; compile first")
        plan_id, tax_hash = self._manifest_identity(manifest)
        self._store.publish(
            project_id=project_id, compiled_vault_path=vault_path,
            bound_integration_plan_id=plan_id,
            bound_corpus_hash=self._projects.source_set_fingerprint(project_id),
            bound_taxonomy_hash=tax_hash, published_by=published_by,
        )
        return self._publication_view(project_id)

    def unpublish(self, project_id: str) -> PublicationView:
        self._projects.get(project_id)  # 404 if unknown project
        self._store.unpublish(project_id)
        return self._publication_view(project_id)

    def publication_status(self, project_id: str) -> PublicationView:
        return self._publication_view(project_id)

    # --- E5-S2: machine read-only file list / body ---

    def list_files(self, project_id: str) -> ServingFileListView:
        row = self._require_published(project_id)
        status, stale = self._effective_status(row)
        return ServingFileListView(
            project_id=project_id, status=status, stale=stale,
            bound_integration_plan_id=row.bound_integration_plan_id,
            files=self._list_files(row.compiled_vault_path),
        )

    def read_file(self, project_id: str, rel_path: str) -> tuple[bytes, str]:
        row = self._require_published(project_id)
        target = self._resolve_served_path(row.compiled_vault_path, rel_path)
        with open(target, "rb") as f:
            return f.read(), self._media_type(target)

    # --- E5-S3: verify / explain (non-reserving engine reads) ---

    async def verify(self, project_id: str) -> ServingVerifyView:
        row = self._require_published(project_id)
        engine = self._require_engine()
        status, stale = self._effective_status(row)
        vr = await engine.read(lambda e: e.verify(row.compiled_vault_path))
        return ServingVerifyView(
            project_id=project_id, status=status, stale=stale, valid=vr.valid,
            artifact_path=vr.artifact_path or row.compiled_vault_path, manifest=vr.manifest,
        )

    async def explain(self, project_id: str, rel_path: str) -> ServingProvenanceView:
        row = self._require_published(project_id)
        target = self._resolve_served_path(row.compiled_vault_path, rel_path)
        engine = self._require_engine()
        status, stale = self._effective_status(row)
        pv = await engine.read(lambda e: e.explain(row.compiled_vault_path, target))
        return ServingProvenanceView(
            project_id=project_id, status=status, stale=stale, file_path=rel_path,
            artifact_path=pv.artifact_path or row.compiled_vault_path, record=pv.record,
            owner_labels=self._owner_labels(project_id),
        )

    # --- E5-S4: okc-mcp consumption contract (location + format only) ---

    def contract(self, project_id: str, read_api_base: str) -> McpContractView:
        row = self._require_published(project_id)
        status, stale = self._effective_status(row)
        endpoints = [
            ContractEndpoint(
                method="GET", path=f"/api/serving/{project_id}/files",
                description="List relative Markdown paths under knowledge/, legacy/, .okc/.",
            ),
            ContractEndpoint(
                method="GET", path=f"/api/serving/{project_id}/file?path=...",
                description="Fetch a single file's raw body.",
            ),
            ContractEndpoint(
                method="GET", path=f"/api/serving/{project_id}/verify",
                description="Integrity/internal-consistency of the compiled vault.",
            ),
            ContractEndpoint(
                method="GET", path=f"/api/serving/{project_id}/explain?path=...",
                description="Per-file provenance record.",
            ),
            ContractEndpoint(
                method="GET", path=f"/api/serving/{project_id}/contract",
                description="This discovery document.",
            ),
        ]
        return McpContractView(
            project_id=project_id, status=status, stale=stale,
            location=ContractLocation(
                local_dir=row.compiled_vault_path, read_api_base=read_api_base
            ),
            bound_integration_plan_id=row.bound_integration_plan_id,
            bound_corpus_hash=row.bound_corpus_hash,
            bound_taxonomy_hash=row.bound_taxonomy_hash,
            endpoints=endpoints,
        )
