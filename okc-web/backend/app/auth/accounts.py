"""``auth.accounts`` — repository over the ``accounts`` table (U1-owned, §9).

Pure persistence, no policy: durable CRUD, role assignment, activation toggle,
last-login stamp, and the active-admin count that backs the last-admin guard.
Business rules (uniqueness, last-admin guard, revoke-all) live in ``AuthService``.
"""

from __future__ import annotations

from datetime import UTC, datetime

from pydantic import BaseModel
from sqlalchemy import text
from ulid import ULID

from app.shared.authz import Role
from app.shared.state import StateDb

AccountId = str


class AccountRecord(BaseModel):
    """Full account row (includes the password hash — internal use only)."""

    id: AccountId
    email: str
    display_name: str
    role: Role
    password_hash: str
    status: str  # 'active' | 'disabled'
    created_at: str
    updated_at: str
    last_login_at: str | None = None


class AccountSummary(BaseModel):
    """Account row WITHOUT the password hash — safe to return to clients."""

    id: AccountId
    email: str
    display_name: str
    role: Role
    status: str
    created_at: str
    last_login_at: str | None = None


def _now() -> str:
    return datetime.now(UTC).isoformat()


class AccountStore:
    def __init__(self, db: StateDb) -> None:
        self._db = db

    def insert(
        self, email: str, display_name: str, role: Role, password_hash: str, now: str
    ) -> AccountRecord:
        account_id = f"acc_{ULID()}"
        with self._db.engine.begin() as conn:
            conn.execute(
                text(
                    "INSERT INTO accounts(id, email, display_name, role, password_hash, status, "
                    "created_at, updated_at) "
                    "VALUES (:id, :email, :name, :role, :hash, 'active', :now, :now)"
                ),
                {
                    "id": account_id, "email": email, "name": display_name,
                    "role": role.value, "hash": password_hash, "now": now,
                },
            )
        return AccountRecord(
            id=account_id, email=email, display_name=display_name, role=role,
            password_hash=password_hash, status="active", created_at=now, updated_at=now,
        )

    def find_by_email(self, email: str) -> AccountRecord | None:
        return self._select_one("email=:v", {"v": email})

    def find_by_id(self, account_id: AccountId) -> AccountRecord | None:
        return self._select_one("id=:v", {"v": account_id})

    def update_role(self, account_id: AccountId, role: Role) -> None:
        with self._db.engine.begin() as conn:
            conn.execute(
                text("UPDATE accounts SET role=:role, updated_at=:now WHERE id=:id"),
                {"id": account_id, "role": role.value, "now": _now()},
            )

    def update_status(self, account_id: AccountId, status: str) -> None:
        with self._db.engine.begin() as conn:
            conn.execute(
                text("UPDATE accounts SET status=:status, updated_at=:now WHERE id=:id"),
                {"id": account_id, "status": status, "now": _now()},
            )

    def update_password_hash(self, account_id: AccountId, password_hash: str) -> None:
        with self._db.engine.begin() as conn:
            conn.execute(
                text("UPDATE accounts SET password_hash=:hash, updated_at=:now WHERE id=:id"),
                {"id": account_id, "hash": password_hash, "now": _now()},
            )

    def touch_last_login(self, account_id: AccountId, at: str) -> None:
        with self._db.engine.begin() as conn:
            conn.execute(
                text("UPDATE accounts SET last_login_at=:at, updated_at=:at WHERE id=:id"),
                {"id": account_id, "at": at},
            )

    def count_active_admins(self) -> int:
        with self._db.engine.connect() as conn:
            n = conn.execute(
                text("SELECT COUNT(*) FROM accounts WHERE role='admin' AND status='active'")
            ).scalar_one()
        return int(n)

    def count_all(self) -> int:
        with self._db.engine.connect() as conn:
            return int(conn.execute(text("SELECT COUNT(*) FROM accounts")).scalar_one())

    def list(self) -> list[AccountSummary]:
        with self._db.engine.connect() as conn:
            rows = conn.execute(
                text(
                    "SELECT id, email, display_name, role, status, created_at, last_login_at "
                    "FROM accounts ORDER BY created_at"
                )
            ).mappings()
            return [
                AccountSummary(
                    id=r["id"], email=r["email"], display_name=r["display_name"],
                    role=Role(r["role"]), status=r["status"], created_at=r["created_at"],
                    last_login_at=r["last_login_at"],
                )
                for r in rows
            ]

    def _select_one(self, where: str, params: dict[str, str]) -> AccountRecord | None:
        with self._db.engine.connect() as conn:
            row = conn.execute(
                text(
                    "SELECT id, email, display_name, role, password_hash, status, "
                    f"created_at, updated_at, last_login_at FROM accounts WHERE {where}"
                ),
                params,
            ).mappings().first()
        if row is None:
            return None
        return AccountRecord(
            id=row["id"], email=row["email"], display_name=row["display_name"],
            role=Role(row["role"]), password_hash=row["password_hash"], status=row["status"],
            created_at=row["created_at"], updated_at=row["updated_at"],
            last_login_at=row["last_login_at"],
        )
