"""``orchestration.router`` — U3 registration + routes (E3-S1..E3-S7).

``register(app, state)`` builds the OrchestrationService and mounts the admin
project-orchestration console. Every route is admin-gated (RBAC-before-core, C-1):
the dependency runs before the handler body, so a 401/403 reaches zero engine calls.
Job-status polling reuses the U0 mount ``GET /api/projects/{id}/jobs/{job_id}``.
"""

from __future__ import annotations

from typing import TYPE_CHECKING, cast

from fastapi import APIRouter, Depends, FastAPI

from app.orchestration.models import (
    BindProviderRequest,
    CompileRequest,
    CompileResultView,
    CreateProjectRequest,
    FreezeResponse,
    JobAccepted,
    PreflightGateView,
    ProjectStatusView,
    ProjectView,
    ProviderProfileView,
    RunIntegrationRequest,
)
from app.orchestration.projects import ProjectRegistry
from app.orchestration.service import OrchestrationService
from app.shared.authz import AdminPrincipal, AuthContext, Role, admin_context, require

if TYPE_CHECKING:  # avoid a runtime import of the app factory (no cycle)
    from app.main import AppState


def register(app: FastAPI, state: AppState) -> None:
    registry = ProjectRegistry(state.db)
    service = OrchestrationService(registry, state.engine, state.config)

    router = APIRouter(prefix="/api/projects")

    # --- E3-S1: project registry ---
    @router.post("", response_model=ProjectView)
    async def create_project(
        body: CreateProjectRequest, ctx: AuthContext = Depends(admin_context)
    ) -> ProjectView:
        require(ctx, (Role.ADMIN,))
        return await service.create_project(cast(AdminPrincipal, ctx.principal), body)

    @router.get("", response_model=list[ProjectView])
    def list_projects(ctx: AuthContext = Depends(admin_context)) -> list[ProjectView]:
        require(ctx, (Role.ADMIN,))
        return service.list_projects()

    @router.get("/{project_id}", response_model=ProjectView)
    def get_project(project_id: str, ctx: AuthContext = Depends(admin_context)) -> ProjectView:
        require(ctx, (Role.ADMIN,))
        return service.get_project(project_id)

    # --- E3-S3/E3-S5: checkpoint projection ---
    @router.get("/{project_id}/status", response_model=ProjectStatusView)
    async def project_status(
        project_id: str, ctx: AuthContext = Depends(admin_context)
    ) -> ProjectStatusView:
        require(ctx, (Role.ADMIN,))
        return await service.status(project_id)

    # --- E3-S2: freeze ---
    @router.post("/{project_id}/freeze", response_model=FreezeResponse)
    def freeze(project_id: str, ctx: AuthContext = Depends(admin_context)) -> FreezeResponse:
        require(ctx, (Role.ADMIN,))
        return service.freeze(project_id)

    # --- E3-S4: provider binding ---
    @router.get("/{project_id}/providers", response_model=list[ProviderProfileView])
    def list_providers(
        project_id: str, ctx: AuthContext = Depends(admin_context)
    ) -> list[ProviderProfileView]:
        require(ctx, (Role.ADMIN,))
        return service.list_providers()

    @router.post("/{project_id}/provider", response_model=ProjectStatusView)
    async def bind_provider(
        project_id: str, body: BindProviderRequest, ctx: AuthContext = Depends(admin_context)
    ) -> ProjectStatusView:
        require(ctx, (Role.ADMIN,))
        return await service.bind_provider(project_id, body.profile_name, body.role)

    # --- E3-S4: preflight + disclosure gate ---
    @router.post("/{project_id}/preflight", response_model=PreflightGateView)
    async def preflight(
        project_id: str, ctx: AuthContext = Depends(admin_context)
    ) -> PreflightGateView:
        require(ctx, (Role.ADMIN,))
        return await service.preflight(project_id)

    # --- E3-S3/E3-S7: integrate (long op -> JobId) ---
    @router.post("/{project_id}/integrate", response_model=JobAccepted)
    async def integrate(
        project_id: str, body: RunIntegrationRequest, ctx: AuthContext = Depends(admin_context)
    ) -> JobAccepted:
        require(ctx, (Role.ADMIN,))
        return await service.integrate(project_id, body)

    # --- E3-S6: compile ---
    @router.post("/{project_id}/compile", response_model=CompileResultView)
    async def compile_project(
        project_id: str, body: CompileRequest, ctx: AuthContext = Depends(admin_context)
    ) -> CompileResultView:
        require(ctx, (Role.ADMIN,))
        return await service.compile(project_id, body.output_path)

    app.include_router(router)
