"""``auth.router`` — U1 HTTP surface + unit registration (E1).

Exposes ``register(app, state)`` (the ``main.py`` discovery seam): it builds the
stores/service from ``state``, installs the U1 ``SessionResolver`` on
``app.state.session_resolver`` (how U0's ``admin_auth`` authenticates), runs the
one-shot bootstrap-admin step, and mounts the router.

Endpoints:
- ``POST /api/auth/login``  — password login → ``okc_session`` cookie (E1-S1).
- ``POST /api/auth/logout`` — revoke the current session + clear the cookie.
- ``GET  /api/auth/session`` — whoami for the app shell (E1-2).
- ``GET  /api/accounts`` · ``POST /api/accounts`` · ``PATCH /api/accounts/{id}/role`` ·
  ``PATCH /api/accounts/{id}/status`` · ``POST /api/accounts/{id}/password`` — account
  admin, each guarded by U0 RBAC(Admin) (E1-S2/E1-S3/E1-S5).
"""

from __future__ import annotations

import os
from datetime import UTC, datetime
from typing import TYPE_CHECKING, Literal

from fastapi import APIRouter, Depends, FastAPI, Request, Response
from pydantic import BaseModel, Field

from app.auth.accounts import AccountStore, AccountSummary
from app.auth.hashing import PasswordHasher
from app.auth.service import AuthService
from app.auth.sessions import (
    SessionResolver,
    SessionStore,
    clear_session_cookie,
    set_session_cookie,
)
from app.shared.authz import AdminPrincipal, AuthContext, Role, admin_auth, admin_context, require

if TYPE_CHECKING:  # avoid an import cycle at runtime (main imports this module)
    from app.main import AppState

router = APIRouter()


# --- request / response models -------------------------------------------------

class LoginRequest(BaseModel):
    email: str = Field(min_length=1)
    password: str = Field(min_length=1)


class SessionView(BaseModel):
    """The authenticated admin as the app shell sees it. ``curator_label`` is the
    unverified label later passed to core as ``curator_id`` (E1-S4)."""

    account_id: str
    email: str
    display_name: str
    role: Role
    curator_label: str


class CreateAccountRequest(BaseModel):
    email: str = Field(min_length=1)
    display_name: str = Field(min_length=1)
    role: Role = Role.CONTRIBUTOR  # least privilege by default (E1-S2)


class CreateAccountResponse(BaseModel):
    account: AccountSummary
    temp_password: str  # shown ONCE (NFR-SEC-1); never persisted or re-shown


class SetRoleRequest(BaseModel):
    role: Role


class SetStatusRequest(BaseModel):
    status: Literal["active", "disabled"]


class ChangePasswordRequest(BaseModel):
    new_password: str = Field(min_length=1)


class OkResponse(BaseModel):
    status: Literal["ok"] = "ok"


def _session_view(account: AccountSummary) -> SessionView:
    return SessionView(
        account_id=account.id, email=account.email, display_name=account.display_name,
        role=account.role, curator_label=account.display_name or account.email,
    )


# --- dependencies ---------------------------------------------------------------

def _service(request: Request) -> AuthService:
    return request.app.state.auth_service


def _cookie_secure(request: Request) -> bool:
    return bool(request.app.state.cookie_secure)


def _require_admin(ctx: AuthContext = Depends(admin_context)) -> AuthContext:
    """U0 RBAC(Admin) gate — a 403 here means the handler body never runs (C-1)."""
    require(ctx, (Role.ADMIN,))
    return ctx


# --- auth routes (E1-S1) --------------------------------------------------------

@router.post("/api/auth/login")
def login(
    body: LoginRequest,
    response: Response,
    service: AuthService = Depends(_service),
    cookie_secure: bool = Depends(_cookie_secure),
) -> SessionView:
    result = service.login(body.email, body.password)
    set_session_cookie(response, result.session_plaintext, cookie_secure)
    return _session_view(result.account)


@router.post("/api/auth/logout")
def logout(
    response: Response,
    principal: AdminPrincipal = Depends(admin_auth),
    service: AuthService = Depends(_service),
    cookie_secure: bool = Depends(_cookie_secure),
) -> OkResponse:
    service.logout(principal.session_id_hash)
    clear_session_cookie(response, cookie_secure)
    return OkResponse()


@router.get("/api/auth/session")
def whoami(
    principal: AdminPrincipal = Depends(admin_auth),
    service: AuthService = Depends(_service),
) -> SessionView:
    return _session_view(service.whoami(principal.account_id))


# --- account admin routes (E1-S2 / E1-S3 / E1-S5) -------------------------------

@router.get("/api/accounts")
def list_accounts(
    _ctx: AuthContext = Depends(_require_admin),
    service: AuthService = Depends(_service),
) -> list[AccountSummary]:
    return service.list_accounts()


@router.post("/api/accounts")
def create_account(
    body: CreateAccountRequest,
    _ctx: AuthContext = Depends(_require_admin),
    service: AuthService = Depends(_service),
) -> CreateAccountResponse:
    account, temp_password = service.create_account(body.email, body.display_name, body.role)
    return CreateAccountResponse(account=account, temp_password=temp_password)


@router.patch("/api/accounts/{account_id}/role")
def set_role(
    account_id: str,
    body: SetRoleRequest,
    _ctx: AuthContext = Depends(_require_admin),
    service: AuthService = Depends(_service),
) -> AccountSummary:
    return service.set_role(account_id, body.role)


@router.patch("/api/accounts/{account_id}/status")
def set_status(
    account_id: str,
    body: SetStatusRequest,
    _ctx: AuthContext = Depends(_require_admin),
    service: AuthService = Depends(_service),
) -> AccountSummary:
    return service.set_status(account_id, body.status)


@router.post("/api/accounts/{account_id}/password")
def change_password(
    account_id: str,
    body: ChangePasswordRequest,
    _ctx: AuthContext = Depends(_require_admin),
    service: AuthService = Depends(_service),
) -> OkResponse:
    service.change_password(account_id, body.new_password)
    return OkResponse()


# --- unit registration (main.py discovery seam) ---------------------------------

def _bootstrap_admin(accounts: AccountStore, hasher: PasswordHasher) -> None:
    """Create the first admin from env vars if ``accounts`` is empty (idempotent).
    Enables the login spine on a fresh deployment. Secrets come from env only."""
    if accounts.count_all() > 0:
        return
    email = os.environ.get("OKC_WEB_BOOTSTRAP_ADMIN_EMAIL")
    password = os.environ.get("OKC_WEB_BOOTSTRAP_ADMIN_PASSWORD")
    if not email or not password:
        return
    accounts.insert(
        email=email, display_name=email, role=Role.ADMIN,
        password_hash=hasher.hash(password), now=datetime.now(UTC).isoformat(),
    )


def register(app: FastAPI, state: AppState) -> None:
    hasher = PasswordHasher()
    accounts = AccountStore(state.db)
    sessions = SessionStore(state.db)
    service = AuthService(accounts, sessions, hasher)

    app.state.session_resolver = SessionResolver(state.db, sessions)  # U0 authenticates via this
    app.state.auth_service = service
    app.state.cookie_secure = state.config.cookie_secure

    _bootstrap_admin(accounts, hasher)
    app.include_router(router)
