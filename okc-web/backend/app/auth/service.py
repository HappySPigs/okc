"""``auth.service`` — the U1 AuthService orchestrator (DB-only, pre-core).

Owns the login / session / account-admin lifecycle. NEVER touches the engine
queue (unit-of-work.md §U1). Business invariants live here:
- email uniqueness (E1-S1);
- **last-admin guard** — the final active admin cannot be demoted or disabled (E1-S2);
- **revoke-all** sessions on disable and on password change (E1-S5);
- one-time temp secret on create, shown ONCE (NFR-SEC-1).

The authenticated admin becomes the unverified ``curator_id`` label via the
``SessionResolver`` (E1-S4) — a label, never a trust boundary (C-1).
"""

from __future__ import annotations

import secrets
from datetime import UTC, datetime

from app.auth.accounts import AccountRecord, AccountStore, AccountSummary
from app.auth.hashing import PasswordHasher
from app.auth.sessions import SessionStore
from app.shared.authz import Role
from app.shared.error import EngineError, EngineErrorCategory, EngineErrorCode


def _summary(rec: AccountRecord) -> AccountSummary:
    return AccountSummary(
        id=rec.id, email=rec.email, display_name=rec.display_name, role=rec.role,
        status=rec.status, created_at=rec.created_at, last_login_at=rec.last_login_at,
    )


class LoginResult:
    """Outcome of a successful login: the opaque session plaintext (to place in
    the cookie) + the account summary for the whoami/session view."""

    def __init__(self, session_plaintext: str, account: AccountSummary) -> None:
        self.session_plaintext = session_plaintext
        self.account = account


class AuthService:
    def __init__(
        self, accounts: AccountStore, sessions: SessionStore, hasher: PasswordHasher
    ) -> None:
        self._accounts = accounts
        self._sessions = sessions
        self._hasher = hasher

    # --- login / session (E1-S1) ---

    def login(self, email: str, password: str) -> LoginResult:
        rec = self._accounts.find_by_email(email)
        if rec is None:
            self._hasher.verify_dummy(password)  # enumeration-timing parity
            raise EngineError.unauthenticated("invalid credentials")
        if not self._hasher.verify(password, rec.password_hash):
            raise EngineError.unauthenticated("invalid credentials")
        if rec.status != "active":
            raise EngineError(
                EngineErrorCode.ACCOUNT_DISABLED, EngineErrorCategory.VALIDATION, "account disabled"
            )
        # Only admins get an app-shell session; a valid contributor is refused with
        # the same generic 401 (no session, no role enumeration on the login card).
        if rec.role != Role.ADMIN:
            raise EngineError.unauthenticated("invalid credentials")
        now = datetime.now(UTC)
        plaintext = self._sessions.create(rec.id, now)
        self._accounts.touch_last_login(rec.id, now.isoformat())
        return LoginResult(plaintext, _summary(rec))

    def logout(self, session_id_hash: str) -> None:
        self._sessions.revoke(session_id_hash)

    def whoami(self, account_id: str) -> AccountSummary:
        rec = self._accounts.find_by_id(account_id)
        if rec is None:
            raise EngineError.not_found("account not found")
        return _summary(rec)

    # --- account admin (E1-S2 / E1-S5), guarded by U0 RBAC(Admin) at the router ---

    def create_account(
        self, email: str, display_name: str, role: Role
    ) -> tuple[AccountSummary, str]:
        """Create an account with a generated one-time temp password (NFR-SEC-1).
        The plaintext is returned ONCE and never persisted or re-shown."""
        if self._accounts.find_by_email(email) is not None:
            raise EngineError.validation("email already in use")
        temp_password = secrets.token_urlsafe(12)
        now = datetime.now(UTC).isoformat()
        rec = self._accounts.insert(
            email=email, display_name=display_name, role=role,
            password_hash=self._hasher.hash(temp_password), now=now,
        )
        return _summary(rec), temp_password

    def set_role(self, account_id: str, role: Role) -> AccountSummary:
        rec = self._require_account(account_id)
        if rec.role == Role.ADMIN and role != Role.ADMIN:
            self._assert_not_last_admin(rec)
        self._accounts.update_role(account_id, role)
        return self.whoami(account_id)

    def set_status(self, account_id: str, status: str) -> AccountSummary:
        if status not in ("active", "disabled"):
            raise EngineError.validation("status must be 'active' or 'disabled'")
        rec = self._require_account(account_id)
        if status == "disabled":
            self._assert_not_last_admin(rec)
            self._sessions.revoke_all_for_account(account_id)  # kill live sessions
        self._accounts.update_status(account_id, status)
        return self.whoami(account_id)

    def change_password(self, account_id: str, new_password: str) -> None:
        self._require_account(account_id)
        self._accounts.update_password_hash(account_id, self._hasher.hash(new_password))
        self._sessions.revoke_all_for_account(account_id)  # force re-login

    def list_accounts(self) -> list[AccountSummary]:
        return self._accounts.list()

    # --- helpers ---

    def _require_account(self, account_id: str) -> AccountRecord:
        rec = self._accounts.find_by_id(account_id)
        if rec is None:
            raise EngineError.not_found("account not found")
        return rec

    def _assert_not_last_admin(self, rec: AccountRecord) -> None:
        """Block demote/disable of the final active admin (E1-S2 last-admin guard)."""
        if rec.role == Role.ADMIN and rec.status == "active" and self._accounts.count_active_admins() == 1:
            raise EngineError.validation("system requires at least one active admin")
