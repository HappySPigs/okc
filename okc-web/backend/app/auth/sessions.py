"""``auth.sessions`` — opaque server-side sessions + the U1 ``SessionResolver``.

The opaque session id (CSPRNG) is returned to the caller exactly ONCE and lives
only in the ``okc_session`` cookie; the DB stores only its SHA-256 hex lookup
hash (§9, E1-S5). Sessions carry an absolute TTL and a sliding idle timeout.

``SessionResolver`` implements the frozen ``shared.authz.SessionResolver``
Protocol: it authenticates (U1) and yields an ``AdminPrincipal`` that U0's RBAC
guard authorizes. Only ``role == 'admin'`` + ``status == 'active'`` accounts
resolve — contributors have no app-shell session (uploads are token capabilities).
``curator_label`` is the unverified label later passed to core as ``curator_id``
(E1-S4, single source of that binding meaning).
"""

from __future__ import annotations

import hashlib
import secrets
from datetime import UTC, datetime, timedelta

from fastapi import Response
from sqlalchemy import text

from app.shared.authz import SESSION_COOKIE, AdminPrincipal
from app.shared.state import StateDb

SessionIdHash = str

# Absolute lifetime of a session and the sliding idle window. Hardcoded MVP
# constants (no speculative config): a session dies at ``created + TTL`` or after
# ``IDLE`` of inactivity, whichever comes first.
SESSION_TTL = timedelta(hours=12)
SESSION_IDLE = timedelta(minutes=60)


def hash_session_id(plaintext: str) -> SessionIdHash:
    return hashlib.sha256(plaintext.encode("utf-8")).hexdigest()


class SessionStore:
    def __init__(self, db: StateDb) -> None:
        self._db = db

    def create(
        self, account_id: str, now: datetime,
        user_agent: str | None = None, ip: str | None = None,
    ) -> str:
        """Create a session; return the opaque plaintext id (shown ONCE)."""
        plaintext = secrets.token_urlsafe(32)
        id_hash = hash_session_id(plaintext)
        expires = now + SESSION_TTL
        with self._db.engine.begin() as conn:
            conn.execute(
                text(
                    "INSERT INTO sessions(id_hash, account_id, created_at, last_seen_at, "
                    "expires_at, user_agent, ip) "
                    "VALUES (:h, :acc, :now, :now, :exp, :ua, :ip)"
                ),
                {
                    "h": id_hash, "acc": account_id, "now": now.isoformat(),
                    "exp": expires.isoformat(), "ua": user_agent, "ip": ip,
                },
            )
        return plaintext

    def touch(self, id_hash: SessionIdHash, now: datetime) -> None:
        with self._db.engine.begin() as conn:
            conn.execute(
                text("UPDATE sessions SET last_seen_at=:now WHERE id_hash=:h"),
                {"h": id_hash, "now": now.isoformat()},
            )

    def revoke(self, id_hash: SessionIdHash) -> None:
        with self._db.engine.begin() as conn:
            conn.execute(
                text("UPDATE sessions SET revoked_at=:now WHERE id_hash=:h AND revoked_at IS NULL"),
                {"h": id_hash, "now": datetime.now(UTC).isoformat()},
            )

    def revoke_all_for_account(self, account_id: str) -> None:
        with self._db.engine.begin() as conn:
            conn.execute(
                text(
                    "UPDATE sessions SET revoked_at=:now "
                    "WHERE account_id=:acc AND revoked_at IS NULL"
                ),
                {"acc": account_id, "now": datetime.now(UTC).isoformat()},
            )


class SessionResolver:
    """U1's concrete resolver, installed on ``app.state.session_resolver`` so U0's
    ``admin_auth`` dependency can authenticate cookie sessions (C-1)."""

    def __init__(self, db: StateDb, sessions: SessionStore) -> None:
        self._db = db
        self._sessions = sessions

    def resolve_admin(self, session_plaintext: str) -> AdminPrincipal | None:
        id_hash = hash_session_id(session_plaintext)
        now = datetime.now(UTC)
        with self._db.engine.connect() as conn:
            row = conn.execute(
                text(
                    "SELECT s.account_id AS account_id, s.expires_at AS expires_at, "
                    "s.last_seen_at AS last_seen_at, s.revoked_at AS revoked_at, "
                    "a.display_name AS display_name, a.email AS email, "
                    "a.role AS role, a.status AS status "
                    "FROM sessions s JOIN accounts a ON a.id = s.account_id "
                    "WHERE s.id_hash = :h"
                ),
                {"h": id_hash},
            ).mappings().first()
        if row is None:
            return None
        if row["revoked_at"] is not None:
            return None
        if now > datetime.fromisoformat(row["expires_at"]):
            return None
        if now - datetime.fromisoformat(row["last_seen_at"]) > SESSION_IDLE:
            return None
        # Only active admins hold an app-shell session (a demoted/disabled account
        # with a lingering cookie must NOT resolve). Contributors never have one.
        if row["role"] != "admin" or row["status"] != "active":
            return None
        self._sessions.touch(id_hash, now)  # sliding renewal
        return AdminPrincipal(
            account_id=row["account_id"],
            session_id_hash=id_hash,
            curator_label=row["display_name"] or row["email"],
        )


def set_session_cookie(response: Response, plaintext: str, secure: bool) -> None:
    response.set_cookie(
        key=SESSION_COOKIE,
        value=plaintext,
        max_age=int(SESSION_TTL.total_seconds()),
        httponly=True,
        samesite="lax",
        secure=secure,
        path="/",
    )


def clear_session_cookie(response: Response, secure: bool) -> None:
    response.delete_cookie(key=SESSION_COOKIE, path="/", httponly=True, samesite="lax", secure=secure)
