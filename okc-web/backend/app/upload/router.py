"""``upload.router`` — U2 registration + routes (E2-S1..E2-S6).

``register(app, state)`` builds the U2 services, installs the token authenticate
seam on ``app.state.token_resolver`` (U0's ``upload_auth`` authorizes with it),
and mounts the admin token console + the token-capability upload endpoints.

The job-status poll ``GET /u/{token}/jobs/{jobId}`` (E2-S6) is ALREADY mounted by
U0 in ``main.py`` — it is NOT re-added here.
"""

from __future__ import annotations

from typing import TYPE_CHECKING

from fastapi import APIRouter, Depends, FastAPI, File, Form, Request, UploadFile

from app.shared.authz import AuthContext, Role, UploadContext, admin_context, require, upload_auth
from app.upload.ingest import (
    ArchiveValidator,
    SlotAccountant,
    SourceLander,
    SourceRegistry,
    UploadIngestService,
    UploadReceiver,
)
from app.upload.models import IngestAccepted, IssuedToken, IssueTokenRequest, TokenListView, UploadTargetView
from app.upload.tokens import UploadTokenResolver, UploadTokenService, UploadTokenStore

if TYPE_CHECKING:  # avoid a runtime import of the app factory (no cycle)
    from app.main import AppState


def register(app: FastAPI, state: AppState) -> None:
    store = UploadTokenStore(state.db)
    token_service = UploadTokenService(store, state.db)
    registry = SourceRegistry(state.db)
    ingest_service = UploadIngestService(
        db=state.db,
        engine=state.engine,
        config=state.config,
        tokens=store,
        registry=registry,
        slots=SlotAccountant(registry),
        receiver=UploadReceiver(),
        validator=ArchiveValidator(),
        lander=SourceLander(state.config),
    )

    # Install the token authenticate seam consumed by U0's upload_auth.
    app.state.token_resolver = UploadTokenResolver(store)

    router = APIRouter()

    # --- admin-facing token console (E2-S1/E2-S2) — RBAC(Admin) before core (C-1) ---
    @router.post("/api/projects/{project_id}/tokens", response_model=IssuedToken)
    def issue_token(
        project_id: str, body: IssueTokenRequest, ctx: AuthContext = Depends(admin_context)
    ) -> IssuedToken:
        require(ctx, (Role.ADMIN,))
        return token_service.issue(ctx, project_id, body)

    @router.get("/api/projects/{project_id}/tokens", response_model=TokenListView)
    def list_tokens(project_id: str, ctx: AuthContext = Depends(admin_context)) -> TokenListView:
        require(ctx, (Role.ADMIN,))
        return token_service.list(ctx, project_id)

    @router.post("/api/projects/{project_id}/tokens/{token_id}/revoke", status_code=204)
    def revoke_token(
        project_id: str, token_id: str, ctx: AuthContext = Depends(admin_context)
    ) -> None:
        require(ctx, (Role.ADMIN,))
        token_service.revoke(ctx, project_id, token_id)

    @router.post("/api/projects/{project_id}/tokens/{token_id}/rotate", response_model=IssuedToken)
    def rotate_token(
        project_id: str, token_id: str, ctx: AuthContext = Depends(admin_context)
    ) -> IssuedToken:
        require(ctx, (Role.ADMIN,))
        return token_service.rotate(ctx, project_id, token_id)

    # --- token-capability upload shell (E2-S3/E2-S4/E2-S5) ---
    @router.get("/u/{token}", response_model=UploadTargetView)
    def upload_portal(token: str, ctx: UploadContext = Depends(upload_auth)) -> UploadTargetView:
        return token_service.target_view(ctx)

    @router.post("/u/{token}/upload", response_model=IngestAccepted)
    async def upload_source(
        token: str,
        request: Request,
        file: UploadFile = File(...),
        owner_display_name: str | None = Form(default=None),
        owner_kind: str | None = Form(default=None),
        ctx: UploadContext = Depends(upload_auth),
    ) -> IngestAccepted:
        return await ingest_service.ingest(ctx, file, owner_display_name, owner_kind, request)

    app.include_router(router)
