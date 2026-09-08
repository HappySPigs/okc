"""``review.router`` — U4 registration + routes (E4-S1..E4-S6).

``register(app, state)`` builds the ReviewService and mounts the admin Conflict/Critic
review console under the existing ``/api/projects/{project_id}/...`` convention. Every route
is admin-gated (RBAC-before-core, C-1): the dependency runs before the handler body, so a
401/403 reaches zero engine calls. Regenerate returns a ``DecisionReceipt`` whose ``job_id``
is polled via the U0 mount ``GET /api/projects/{id}/jobs/{job_id}``.
"""

from __future__ import annotations

from typing import TYPE_CHECKING, cast

from fastapi import APIRouter, Depends, FastAPI

from app.orchestration.projects import ProjectRegistry
from app.review.models import (
    ApproveClusterRequest,
    ApproveTaxonomyRequest,
    ClusterDetailView,
    ClusterSummaryView,
    DecisionReceipt,
    DecisionRecordView,
    RegenerateClusterRequest,
    ReviewGateView,
    TaxonomyReviewView,
)
from app.review.service import ReviewService
from app.shared.authz import AdminPrincipal, AuthContext, Role, admin_context, require

if TYPE_CHECKING:  # avoid a runtime import of the app factory (no cycle)
    from app.main import AppState


def register(app: FastAPI, state: AppState) -> None:
    registry = ProjectRegistry(state.db)
    service = ReviewService(registry, state.engine, state.audit)

    router = APIRouter(prefix="/api/projects")

    # --- E4-S1: taxonomy review + approval ---
    @router.get("/{project_id}/taxonomy", response_model=TaxonomyReviewView)
    async def get_taxonomy(
        project_id: str, ctx: AuthContext = Depends(admin_context)
    ) -> TaxonomyReviewView:
        require(ctx, (Role.ADMIN,))
        return await service.get_taxonomy(project_id)

    @router.post("/{project_id}/taxonomy/approve", response_model=DecisionReceipt)
    async def approve_taxonomy(
        project_id: str,
        body: ApproveTaxonomyRequest,
        ctx: AuthContext = Depends(admin_context),
    ) -> DecisionReceipt:
        require(ctx, (Role.ADMIN,))
        return await service.approve_taxonomy(
            cast(AdminPrincipal, ctx.principal), project_id, body
        )

    # --- E4-S2: cluster synthesis + severity-graded critic findings ---
    @router.get("/{project_id}/clusters", response_model=list[ClusterSummaryView])
    async def list_clusters(
        project_id: str, ctx: AuthContext = Depends(admin_context)
    ) -> list[ClusterSummaryView]:
        require(ctx, (Role.ADMIN,))
        return await service.list_clusters(project_id)

    @router.get("/{project_id}/clusters/{cluster_id}", response_model=ClusterDetailView)
    async def get_cluster(
        project_id: str, cluster_id: str, ctx: AuthContext = Depends(admin_context)
    ) -> ClusterDetailView:
        require(ctx, (Role.ADMIN,))
        return await service.get_cluster(project_id, cluster_id)

    # --- E4-S3: cluster approval (DecisionGate + waive/omission) ---
    @router.post("/{project_id}/clusters/{cluster_id}/approve", response_model=DecisionReceipt)
    async def approve_cluster(
        project_id: str,
        cluster_id: str,
        body: ApproveClusterRequest,
        ctx: AuthContext = Depends(admin_context),
    ) -> DecisionReceipt:
        require(ctx, (Role.ADMIN,))
        return await service.approve_cluster(
            cast(AdminPrincipal, ctx.principal), project_id, cluster_id, body
        )

    # --- E4-S4: regenerate (only path to resolve blocking findings; long op → JobId) ---
    @router.post("/{project_id}/clusters/{cluster_id}/regenerate", response_model=DecisionReceipt)
    async def regenerate_cluster(
        project_id: str,
        cluster_id: str,
        body: RegenerateClusterRequest,
        ctx: AuthContext = Depends(admin_context),
    ) -> DecisionReceipt:
        require(ctx, (Role.ADMIN,))
        return await service.regenerate_cluster(
            cast(AdminPrincipal, ctx.principal), project_id, cluster_id, body
        )

    # --- E4-S6: compile-eligibility scoreboard (read-only) ---
    @router.get("/{project_id}/review/gate", response_model=ReviewGateView)
    async def review_gate(
        project_id: str, ctx: AuthContext = Depends(admin_context)
    ) -> ReviewGateView:
        require(ctx, (Role.ADMIN,))
        return await service.review_gate(project_id)

    # --- E4-S1/S3/S4: decision audit trail ---
    @router.get("/{project_id}/decisions", response_model=list[DecisionRecordView])
    def list_decisions(
        project_id: str, ctx: AuthContext = Depends(admin_context)
    ) -> list[DecisionRecordView]:
        require(ctx, (Role.ADMIN,))
        return service.list_decisions(project_id)

    app.include_router(router)
