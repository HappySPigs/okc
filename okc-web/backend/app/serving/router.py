"""``serving.router`` — U5 registration + routes (E5-S1..E5-S5).

``register(app, state)`` (the ``main.py`` discovery seam) builds the
ServingService and mounts TWO surfaces:

- **Admin-gated** publish/unpublish/status (RBAC-before-core, C-1): the
  ``admin_context`` dependency runs before the body, so a 401/403 reaches zero
  engine calls and zero state change.
- **Machine read-only, UNAUTHENTICATED** (E5-S2, the okc-mcp consumer): GET/HEAD
  only, served only for a published (live|stale) project. GET-only routes make
  mutating verbs a framework 405; missing/traversal/out-of-root paths are 404.
"""

from __future__ import annotations

from typing import TYPE_CHECKING, cast

from fastapi import APIRouter, Depends, FastAPI, Request, Response

from app.orchestration.projects import ProjectRegistry
from app.serving.models import (
    AccessRequest,
    McpContractView,
    PublicationView,
    RestoreRequest,
    ServingFileListView,
    ServingProvenanceView,
    ServingVerifyView,
)
from app.serving.service import ServingService
from app.serving.store import ServingStateStore
from app.shared.authz import AdminPrincipal, AuthContext, Role, admin_context, require

if TYPE_CHECKING:  # avoid a runtime import of the app factory (no cycle)
    from app.main import AppState


def register(app: FastAPI, state: AppState) -> None:
    projects = ProjectRegistry(state.db)
    store = ServingStateStore(state.db)
    service = ServingService(
        db=state.db, engine=state.engine, config=state.config, store=store, projects=projects
    )

    # --- Admin surface (E5-S1) — RBAC(Admin) before any engine/state touch ---
    admin = APIRouter(prefix="/api/projects/{project_id}/serving")

    @admin.post("/publish", response_model=PublicationView)
    async def publish(
        project_id: str, ctx: AuthContext = Depends(admin_context)
    ) -> PublicationView:
        require(ctx, (Role.ADMIN,))
        return await service.publish(project_id, cast(AdminPrincipal, ctx.principal).curator_label)

    @admin.post("/unpublish", response_model=PublicationView)
    def unpublish(project_id: str, ctx: AuthContext = Depends(admin_context)) -> PublicationView:
        require(ctx, (Role.ADMIN,))
        return service.unpublish(project_id)

    @admin.get("", response_model=PublicationView)
    def publication_status(
        project_id: str, ctx: AuthContext = Depends(admin_context)
    ) -> PublicationView:
        require(ctx, (Role.ADMIN,))
        return service.publication_status(project_id)

    @admin.get("/history")
    def history(project_id: str, ctx: AuthContext = Depends(admin_context)) -> list[dict]:
        require(ctx, (Role.ADMIN,))
        projects.get(project_id)
        return service.snapshots.history(project_id)

    @admin.post("/restore", response_model=PublicationView)
    async def restore(project_id: str, body: RestoreRequest,
                      ctx: AuthContext = Depends(admin_context)) -> PublicationView:
        require(ctx, (Role.ADMIN,))
        return await service.restore(project_id, body.revision, cast(AdminPrincipal, ctx.principal).curator_label)

    @admin.put("/access")
    def access(project_id: str, body: AccessRequest, ctx: AuthContext = Depends(admin_context)) -> dict:
        require(ctx, (Role.ADMIN,))
        projects.get(project_id)
        service.snapshots.set_mode(project_id, body.mode)
        return {"project_id": project_id, "mode": body.mode}

    @admin.post("/tokens")
    def issue_token(project_id: str, ctx: AuthContext = Depends(admin_context)) -> dict:
        require(ctx, (Role.ADMIN,))
        projects.get(project_id)
        return service.snapshots.issue_token(project_id)

    @admin.delete("/tokens/{token_id}")
    def revoke_token(project_id: str, token_id: str, ctx: AuthContext = Depends(admin_context)) -> dict:
        require(ctx, (Role.ADMIN,))
        projects.get(project_id)
        service.snapshots.revoke_token(project_id, token_id)
        return {"revoked": True}

    def read_access(project_id: str, request: Request) -> None:
        service.snapshots.authorize(project_id, request.headers.get("Authorization"))

    machine = APIRouter(prefix="/api/serving/{project_id}", dependencies=[Depends(read_access)])

    @machine.get("/files", response_model=ServingFileListView)
    def machine_files(project_id: str, revision: str | None = None) -> ServingFileListView:
        return service.list_files(project_id, revision)

    @machine.get("/file")
    def machine_file(project_id: str, path: str, revision: str | None = None) -> Response:
        body, media_type = service.read_file(project_id, path, revision)
        return Response(content=body, media_type=media_type, headers={"Cache-Control": "private, no-store"})

    @machine.get("/verify", response_model=ServingVerifyView)
    async def machine_verify(project_id: str, revision: str | None = None) -> ServingVerifyView:
        return await service.verify(project_id, revision)

    @machine.get("/explain", response_model=ServingProvenanceView)
    async def machine_explain(project_id: str, path: str, revision: str | None = None) -> ServingProvenanceView:
        return await service.explain(project_id, path, revision)

    @machine.get("/contract", response_model=McpContractView)
    def machine_contract(project_id: str, request: Request, revision: str | None = None) -> McpContractView:
        base = str(request.base_url).rstrip("/") + f"/api/serving/{project_id}"
        return service.contract(project_id, base, revision)

    app.include_router(admin)
    app.include_router(machine)
