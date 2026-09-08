"""``review.service`` — U4 ReviewService (E4-S1..E4-S6).

The Conflict/Critic review surface over okc-core's authoritative synthesis + CriticReport.
All engine access goes through the U0 ``EngineWorker`` and the adapter DTOs (ADR-0002 — no
``okc`` import here). RBAC is enforced in the router BEFORE any method here runs (C-1).

Every mutating decision is **record-then-act** (Q8): the ``DecisionGate`` runs first, then the
append-only audit row is written BEFORE the engine op, and (for the long regenerate op) the
resulting ``JobId`` is linked back. The only decisions representable are the three
``CuratorDecision`` variants — there is no winner-select path (C3).
"""

from __future__ import annotations

from collections.abc import Callable, Mapping
from typing import Any

from app.adapter import dto
from app.adapter.engine import OkcEngineImpl
from app.adapter.queue import EngineWorker
from app.orchestration.projects import ProjectRegistry
from app.review.gate import DecisionGate
from app.review.models import (
    ApproveClusterRequest,
    ApproveTaxonomyRequest,
    BlockingItemView,
    ClusterDetailView,
    ClusterSummaryView,
    DecisionReceipt,
    DecisionRecordView,
    RegenerateClusterRequest,
    ReviewGateView,
    TaxonomyReviewView,
    iter_clusters,
    parse_findings,
    summarize_cluster,
)
from app.shared.audit import (
    ApproveCluster,
    ApproveTaxonomy,
    AuditStore,
    HashBindings,
    NewDecisionRecord,
    RegenerateCluster,
)
from app.shared.authz import AdminPrincipal
from app.shared.error import EngineError, EngineErrorCategory, EngineErrorCode
from app.shared.jobs import JobProgress


def _opt_str(m: Mapping[str, Any], key: str) -> str | None:
    val = m.get(key)
    return str(val) if isinstance(val, str) and val else None


def _project_busy(project_id: str) -> EngineError:
    return EngineError(
        EngineErrorCode.PROJECT_BUSY,
        EngineErrorCategory.CONCURRENCY,
        f"project {project_id} has an operation in progress",
    ).as_retryable()


class ReviewService:
    def __init__(
        self, registry: ProjectRegistry, engine: EngineWorker | None, audit: AuditStore
    ) -> None:
        self._registry = registry
        self._engine = engine
        self._audit = audit

    # --- helpers ---

    def _require_engine(self) -> EngineWorker:
        if self._engine is None:
            raise EngineError.internal("engine worker unavailable")
        return self._engine

    def _guard_not_busy(self, project_id: str) -> None:
        if self._registry.has_active_reserving_job(project_id):
            raise _project_busy(project_id)

    async def _read_cluster(
        self, engine: EngineWorker, root: str, cluster_id: str
    ) -> tuple[Mapping[str, Any], Mapping[str, Any]]:
        payload = await engine.read(lambda e: e.clusters(root))
        for cid, proposal, critic in iter_clusters(payload):
            if cid == cluster_id:
                return proposal, critic
        raise EngineError.not_found(f"cluster '{cluster_id}' not found")

    # --- E4-S1: taxonomy ---

    async def get_taxonomy(self, project_id: str) -> TaxonomyReviewView:
        row = self._registry.get(project_id)
        engine = self._require_engine()
        payload = await engine.read(lambda e: e.taxonomy(row.engine_root_abs_path))
        if isinstance(payload, Mapping):
            isv = payload.get("interop_schema_version")
            body = payload.get("payload", payload)
        else:
            isv, body = None, payload
        return TaxonomyReviewView(
            project_id=project_id,
            interop_schema_version=isv if isinstance(isv, int) else None,
            payload=body,
        )

    async def approve_taxonomy(
        self, principal: AdminPrincipal, project_id: str, req: ApproveTaxonomyRequest
    ) -> DecisionReceipt:
        row = self._registry.get(project_id)
        engine = self._require_engine()
        # Rationale is required iff the taxonomy was edited (E4-S1) — gate before core.
        if req.edited_clusters is not None and not (req.rationale and req.rationale.strip()):
            raise EngineError.validation("editing the taxonomy requires a rationale")
        self._guard_not_busy(project_id)
        # record-then-act: append the audit row BEFORE the engine op.
        audit_id = self._audit.append(
            NewDecisionRecord(
                project_id=project_id,
                account_id=principal.account_id,
                curator_id=principal.curator_label,
                decision=ApproveTaxonomy(
                    edited_clusters=req.edited_clusters, rationale=req.rationale
                ),
            )
        )
        cmd = dto.ApproveTaxonomyCmd(edited_clusters=req.edited_clusters, rationale=req.rationale)
        await engine.call(lambda e: e.approve_taxonomy(row.engine_root_abs_path, cmd))
        return DecisionReceipt(
            project_id=project_id, audit_id=audit_id, decision_kind="approve_taxonomy"
        )

    # --- E4-S2: cluster read ---

    async def list_clusters(self, project_id: str) -> list[ClusterSummaryView]:
        row = self._registry.get(project_id)
        engine = self._require_engine()
        payload = await engine.read(lambda e: e.clusters(row.engine_root_abs_path))
        return [summarize_cluster(cid, critic) for cid, _proposal, critic in iter_clusters(payload)]

    async def get_cluster(self, project_id: str, cluster_id: str) -> ClusterDetailView:
        row = self._registry.get(project_id)
        engine = self._require_engine()
        proposal, critic = await self._read_cluster(engine, row.engine_root_abs_path, cluster_id)
        findings = parse_findings(critic)
        return ClusterDetailView(
            cluster_id=cluster_id,
            proposal_hash=_opt_str(proposal, "proposal_hash") or _opt_str(critic, "proposal_hash"),
            critic_hash=_opt_str(critic, "critic_hash"),
            taxonomy_hash=_opt_str(proposal, "taxonomy_hash"),
            findings=findings,
            blocking=any(f.blocking for f in findings),
            proposal=dict(proposal) or None,
            contradictions=proposal.get("contradictions"),
        )

    # --- E4-S3: cluster approval (DecisionGate → record-then-act) ---

    async def approve_cluster(
        self,
        principal: AdminPrincipal,
        project_id: str,
        cluster_id: str,
        req: ApproveClusterRequest,
    ) -> DecisionReceipt:
        row = self._registry.get(project_id)
        engine = self._require_engine()
        # 1. body-only gate (no engine call yet): missing rationale → 400.
        DecisionGate.assert_rationales(req)
        # 2. fetch the cluster (read path) for the blocking gate + hash bindings.
        proposal, critic = await self._read_cluster(engine, row.engine_root_abs_path, cluster_id)
        findings = parse_findings(critic)
        # 3. blocking gate: Major/Critical waive or residual blocking → 422 (C-2).
        DecisionGate.assert_approvable(findings, req)
        self._guard_not_busy(project_id)
        # 4. record-then-act with the acted-on hash bindings.
        audit_id = self._audit.append(
            NewDecisionRecord(
                project_id=project_id,
                account_id=principal.account_id,
                curator_id=principal.curator_label,
                decision=ApproveCluster(
                    cluster_id=cluster_id,
                    omission_rationales=req.omission_rationales,
                    minor_waivers=req.minor_waivers,
                ),
                bindings=HashBindings(
                    proposal_hash=_opt_str(proposal, "proposal_hash")
                    or _opt_str(critic, "proposal_hash"),
                    critic_hash=_opt_str(critic, "critic_hash"),
                    taxonomy_hash=_opt_str(proposal, "taxonomy_hash"),
                ),
            )
        )
        cmd = dto.ApproveClusterCmd(
            cluster_id=cluster_id,
            omission_rationales=req.omission_rationales,
            minor_waivers=req.minor_waivers,
        )
        await engine.call(lambda e: e.approve_cluster(row.engine_root_abs_path, cmd))
        return DecisionReceipt(
            project_id=project_id, audit_id=audit_id, decision_kind="approve_cluster"
        )

    # --- E4-S4: regenerate (long AI op → JobId) ---

    async def regenerate_cluster(
        self,
        principal: AdminPrincipal,
        project_id: str,
        cluster_id: str,
        req: RegenerateClusterRequest,
    ) -> DecisionReceipt:
        row = self._registry.get(project_id)
        engine = self._require_engine()
        self._guard_not_busy(project_id)
        # record-then-act: append BEFORE enqueue, link the JobId after.
        audit_id = self._audit.append(
            NewDecisionRecord(
                project_id=project_id,
                account_id=principal.account_id,
                curator_id=principal.curator_label,
                decision=RegenerateCluster(cluster_id=cluster_id, feedback=req.feedback),
            )
        )
        cmd = dto.RegenerateClusterCmd(
            cluster_id=cluster_id,
            feedback=req.feedback,
            disclosure=dto.DisclosureCmd(
                allow_remote_provider=req.allow_remote_provider,
                remote_disclosure_confirmed=req.remote_disclosure_confirmed,
            ),
        )
        root = row.engine_root_abs_path

        def run(e: OkcEngineImpl, progress: Callable[[JobProgress], None]) -> None:
            e.regenerate_cluster(root, cmd, progress)

        job_id = engine.enqueue("regenerate_cluster", project_id, principal.account_id, run)
        self._audit.link_job(audit_id, job_id)
        return DecisionReceipt(
            project_id=project_id,
            audit_id=audit_id,
            decision_kind="regenerate_cluster",
            job_id=job_id,
        )

    # --- E4-S6: compile-eligibility scoreboard (read-only; never triggers compile) ---

    async def review_gate(self, project_id: str) -> ReviewGateView:
        row = self._registry.get(project_id)
        engine = self._require_engine()
        payload = await engine.read(lambda e: e.clusters(row.engine_root_abs_path))
        blocking_items: list[BlockingItemView] = []
        for cid, _proposal, critic in iter_clusters(payload):
            for f in parse_findings(critic):
                if f.blocking:
                    blocking_items.append(
                        BlockingItemView(
                            cluster_id=cid,
                            finding_id=f.finding_id,
                            severity=f.severity,
                            required_action="regenerate",
                        )
                    )
        if blocking_items:
            return ReviewGateView(
                project_id=project_id, state="Blocked", blocking_items=blocking_items
            )
        status = await engine.read(lambda e: e.status(row.engine_root_abs_path))
        checkpoint = str(status.checkpoint)
        state = "Ready" if checkpoint == "ready_to_compile" else "PendingApprovals"
        return ReviewGateView(project_id=project_id, state=state, checkpoint=checkpoint)

    # --- E4-S1/S3/S4: audit trail read ---

    def list_decisions(self, project_id: str) -> list[DecisionRecordView]:
        self._registry.get(project_id)  # 404 fast
        return [DecisionRecordView(**row) for row in self._audit.list_for_project(project_id)]
