"""``shared.authz`` — S0.C RBAC-before-core (C-1, §3).

U0 owns the canonical ``Role``/``Principal``/``AuthContext``, the FastAPI
dependencies, and the ordering guarantee. U1/U2 only *supply the resolvers*
(authenticate); U0 *authorizes*. Invariant (testable): any request that fails
authz reaches zero ``OkcEngine`` methods — a 401/403 raised inside a dependency
means the route body (the sole enqueue site) never runs, so okc-core is untouched.
"""

from __future__ import annotations

from enum import Enum
from typing import Protocol

from fastapi import Depends, Request
from pydantic import BaseModel

from app.shared.error import EngineError, EngineErrorCategory, EngineErrorCode

SESSION_COOKIE = "okc_session"


class Role(str, Enum):
    ADMIN = "admin"
    CONTRIBUTOR = "contributor"


class AdminPrincipal(BaseModel):
    """Admin principal (cookie session). ``curator_label`` is the unverified label
    later passed to core as ``curator_id`` (FR-AUTH-4)."""

    account_id: str
    session_id_hash: str
    curator_label: str


class UploadContext(BaseModel):
    """Upload-capability principal (bearer token, ``/u/{token}/*``). ``owner_*``
    are decorative/unverified labels."""

    token_id: str
    project_id: str
    slot_index: int
    owner_display_name: str | None = None
    owner_kind: str | None = None


Principal = AdminPrincipal | UploadContext


class AuthContext(BaseModel):
    """Canonical ``{principal, role}`` established before any handler runs."""

    principal: Principal
    role: Role


class SessionResolver(Protocol):
    """Authenticate seam supplied by U1 (session)."""

    def resolve_admin(self, session_plaintext: str) -> AdminPrincipal | None:
        """Resolve (and slide/touch) a plaintext session id into an admin principal.
        ``None`` = no valid session → 401 (core untouched)."""
        ...


class TokenResolver(Protocol):
    """Authenticate seam supplied by U2 (upload token)."""

    def resolve_upload(self, token_plaintext: str) -> UploadContext | None:
        ...


class UnconfiguredResolver:
    """W0 default until U1/U2 wire real resolvers: denies everything, which
    trivially upholds the "auth-fail => core untouched" invariant."""

    def resolve_admin(self, _session_plaintext: str) -> AdminPrincipal | None:
        return None

    def resolve_upload(self, _token_plaintext: str) -> UploadContext | None:
        return None


def _session_resolver(request: Request) -> SessionResolver:
    return getattr(request.app.state, "session_resolver", None) or UnconfiguredResolver()


def _token_resolver(request: Request) -> TokenResolver:
    return getattr(request.app.state, "token_resolver", None) or UnconfiguredResolver()


def admin_auth(request: Request) -> AdminPrincipal:
    """Dependency for admin-authenticated routes. Only ``Role.ADMIN`` yields a
    session; contributors have no app-shell session (uploads are capabilities)."""
    plaintext = request.cookies.get(SESSION_COOKIE)
    if not plaintext:
        raise EngineError.unauthenticated("no session cookie")
    principal = _session_resolver(request).resolve_admin(plaintext)
    if principal is None:
        raise EngineError.session_expired()
    return principal


def admin_context(principal: AdminPrincipal = Depends(admin_auth)) -> AuthContext:
    return AuthContext(principal=principal, role=Role.ADMIN)


def upload_auth(request: Request, token: str) -> UploadContext:
    """Dependency for ``/u/{token}/*`` upload-capability routes."""
    if not token:
        raise EngineError(
            EngineErrorCode.TOKEN_INVALID, EngineErrorCategory.AUTH, "missing upload token"
        )
    ctx = _token_resolver(request).resolve_upload(token)
    if ctx is None:
        raise EngineError(
            EngineErrorCode.TOKEN_INVALID, EngineErrorCategory.AUTH, "invalid or revoked upload token"
        )
    return ctx


def require(ctx: AuthContext, allowed: tuple[Role, ...]) -> None:
    if ctx.role not in allowed:
        raise EngineError.forbidden(f"role {ctx.role.value} not permitted")
