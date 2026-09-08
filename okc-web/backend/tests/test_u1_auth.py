"""U1 (Auth & RBAC) tests — E1-S1..E1-S5, MVP.

Builds an ISOLATED app (NOT ``app.main.create_app``) so a mid-write sibling unit
can't break auto-discovery: in-memory ``StateDb`` + the real U0 ``AppState`` + a
bare ``FastAPI`` with the U0 error handlers, then ``auth.router.register``. Uses
the REAL argon2 hasher (no mocks) — the okc engine is never touched (U1 is
DB-only, pre-core).
"""

from __future__ import annotations

from datetime import UTC, datetime, timedelta

import pytest
from fastapi import FastAPI
from fastapi.testclient import TestClient
from sqlalchemy import text

from app.auth.accounts import AccountStore
from app.auth.hashing import PasswordHasher
from app.auth.router import _bootstrap_admin, register
from app.auth.sessions import SessionResolver, SessionStore, hash_session_id
from app.config import AppConfig
from app.main import AppState
from app.shared.audit import AuditStore
from app.shared.authz import AdminPrincipal, AuthContext, Role, require
from app.shared.error import EngineError, EngineErrorCode
from app.shared.jobs import JobStore
from app.shared.state import StateDb


@pytest.fixture(autouse=True)
def _clear_bootstrap_env(monkeypatch: pytest.MonkeyPatch) -> None:
    """Bootstrap must be inert unless a test opts in with the env vars."""
    monkeypatch.delenv("OKC_WEB_BOOTSTRAP_ADMIN_EMAIL", raising=False)
    monkeypatch.delenv("OKC_WEB_BOOTSTRAP_ADMIN_PASSWORD", raising=False)


def _make_app() -> tuple[FastAPI, StateDb]:
    from app.shared.error import register_error_handlers

    db = StateDb.open_in_memory()
    cfg = AppConfig(
        state_db_path=":memory:", projects_root="/tmp/okc-projects",
        providers=[], cookie_secure=False, skip_engine=True,
    )
    state = AppState(cfg, db, JobStore(db), AuditStore(db), None)
    app = FastAPI()
    register_error_handlers(app)
    register(app, state)
    return app, db


def _seed(db: StateDb, email: str, password: str, role: Role, *, display_name: str = "User",
          status: str = "active") -> str:
    accounts = AccountStore(db)
    rec = accounts.insert(
        email=email, display_name=display_name, role=role,
        password_hash=PasswordHasher().hash(password), now=datetime.now(UTC).isoformat(),
    )
    if status != "active":
        accounts.update_status(rec.id, status)
    return rec.id


# --- E1-S1: login + session ----------------------------------------------------

def test_login_sets_cookie_and_whoami_works() -> None:
    app, db = _make_app()
    _seed(db, "admin@okc.test", "secret-123", Role.ADMIN, display_name="Ada Admin")
    client = TestClient(app)

    r = client.post("/api/auth/login", json={"email": "admin@okc.test", "password": "secret-123"})
    assert r.status_code == 200, r.text
    assert client.cookies.get("okc_session")
    body = r.json()
    assert body["role"] == "admin"
    assert body["curator_label"] == "Ada Admin"

    who = client.get("/api/auth/session")
    assert who.status_code == 200
    assert who.json()["email"] == "admin@okc.test"


def test_login_bad_password_is_401_and_sets_no_cookie() -> None:
    app, db = _make_app()
    _seed(db, "admin@okc.test", "secret-123", Role.ADMIN)
    client = TestClient(app)

    r = client.post("/api/auth/login", json={"email": "admin@okc.test", "password": "WRONG"})
    assert r.status_code == 401
    assert r.json()["code"] == "UNAUTHENTICATED"
    assert client.cookies.get("okc_session") is None


def test_login_unknown_email_is_401() -> None:
    app, _ = _make_app()
    client = TestClient(app)
    r = client.post("/api/auth/login", json={"email": "nobody@okc.test", "password": "x"})
    assert r.status_code == 401
    assert r.json()["code"] == "UNAUTHENTICATED"


def test_disabled_account_login_is_account_disabled() -> None:
    app, db = _make_app()
    _seed(db, "off@okc.test", "secret-123", Role.ADMIN, status="disabled")
    client = TestClient(app)
    r = client.post("/api/auth/login", json={"email": "off@okc.test", "password": "secret-123"})
    assert r.status_code == 400
    assert r.json()["code"] == "ACCOUNT_DISABLED"


def test_contributor_login_refused_no_session() -> None:
    app, db = _make_app()
    _seed(db, "contrib@okc.test", "secret-123", Role.CONTRIBUTOR)
    client = TestClient(app)
    r = client.post("/api/auth/login", json={"email": "contrib@okc.test", "password": "secret-123"})
    assert r.status_code == 401  # correct creds, but no app-shell session for contributors
    assert client.cookies.get("okc_session") is None


def test_logout_revokes_session() -> None:
    app, db = _make_app()
    _seed(db, "admin@okc.test", "secret-123", Role.ADMIN)
    client = TestClient(app)
    client.post("/api/auth/login", json={"email": "admin@okc.test", "password": "secret-123"})
    assert client.get("/api/auth/session").status_code == 200
    assert client.post("/api/auth/logout").status_code == 200
    # The (now revoked) cookie no longer resolves.
    r = client.get("/api/auth/session")
    assert r.status_code == 401


# --- E1-S1/E1-S4: SessionResolver resolve / touch / expire / revoke -------------

def _set_session_times(db: StateDb, id_hash: str, *, last_seen: datetime | None = None,
                       expires: datetime | None = None) -> None:
    sets, params = [], {"h": id_hash}
    if last_seen is not None:
        sets.append("last_seen_at=:ls")
        params["ls"] = last_seen.isoformat()
    if expires is not None:
        sets.append("expires_at=:ex")
        params["ex"] = expires.isoformat()
    with db.engine.begin() as conn:
        conn.execute(text(f"UPDATE sessions SET {', '.join(sets)} WHERE id_hash=:h"), params)


def _read_last_seen(db: StateDb, id_hash: str) -> datetime:
    with db.engine.connect() as conn:
        v = conn.execute(
            text("SELECT last_seen_at FROM sessions WHERE id_hash=:h"), {"h": id_hash}
        ).scalar_one()
    return datetime.fromisoformat(v)


def test_session_resolver_resolves_active_admin_and_slides() -> None:
    db = StateDb.open_in_memory()
    acc = _seed(db, "admin@okc.test", "x", Role.ADMIN, display_name="Ada")
    store = SessionStore(db)
    resolver = SessionResolver(db, store)
    plaintext = store.create(acc, datetime.now(UTC))
    id_hash = hash_session_id(plaintext)

    # Simulate 5 minutes of inactivity (well within the idle window), then resolve.
    _set_session_times(db, id_hash, last_seen=datetime.now(UTC) - timedelta(minutes=5))
    principal = resolver.resolve_admin(plaintext)
    assert principal is not None
    assert principal.account_id == acc
    assert principal.session_id_hash == id_hash
    assert principal.curator_label == "Ada"
    # Sliding renewal moved last_seen_at forward on resolve.
    assert datetime.now(UTC) - _read_last_seen(db, id_hash) < timedelta(seconds=30)


def test_session_resolver_rejects_expired_idle_and_revoked() -> None:
    db = StateDb.open_in_memory()
    acc = _seed(db, "admin@okc.test", "x", Role.ADMIN)
    store = SessionStore(db)
    resolver = SessionResolver(db, store)

    # idle timeout exceeded
    p1 = store.create(acc, datetime.now(UTC))
    _set_session_times(db, hash_session_id(p1), last_seen=datetime.now(UTC) - timedelta(hours=2))
    assert resolver.resolve_admin(p1) is None

    # absolute TTL exceeded
    p2 = store.create(acc, datetime.now(UTC))
    _set_session_times(db, hash_session_id(p2), expires=datetime.now(UTC) - timedelta(minutes=1))
    assert resolver.resolve_admin(p2) is None

    # revoked
    p3 = store.create(acc, datetime.now(UTC))
    store.revoke(hash_session_id(p3))
    assert resolver.resolve_admin(p3) is None

    # unknown session id
    assert resolver.resolve_admin("not-a-real-session") is None


def test_session_resolver_rejects_demoted_or_disabled_account() -> None:
    db = StateDb.open_in_memory()
    acc = _seed(db, "admin@okc.test", "x", Role.ADMIN)
    store = SessionStore(db)
    resolver = SessionResolver(db, store)
    accounts = AccountStore(db)

    p = store.create(acc, datetime.now(UTC))
    assert resolver.resolve_admin(p) is not None
    accounts.update_role(acc, Role.CONTRIBUTOR)  # demoted -> lingering cookie must not resolve
    assert resolver.resolve_admin(p) is None
    accounts.update_role(acc, Role.ADMIN)
    accounts.update_status(acc, "disabled")  # disabled -> must not resolve
    assert resolver.resolve_admin(p) is None


# --- E1-S3: RBAC-before-core gate on account-admin routes -----------------------

def test_account_routes_require_admin_session() -> None:
    app, _ = _make_app()
    client = TestClient(app)  # no cookie
    assert client.get("/api/accounts").status_code == 401
    assert client.patch("/api/accounts/acc_x/role", json={"role": "admin"}).status_code == 401
    assert client.post("/api/accounts", json={"email": "a@b.c", "display_name": "A"}).status_code == 401


def test_require_guard_blocks_non_admin_role() -> None:
    # The U0 guard raises FORBIDDEN (403) for a non-admin AuthContext — core untouched.
    ctx = AuthContext(
        principal=AdminPrincipal(account_id="acc_x", session_id_hash="h", curator_label="X"),
        role=Role.CONTRIBUTOR,
    )
    with pytest.raises(EngineError) as e:
        require(ctx, (Role.ADMIN,))
    assert e.value.code == EngineErrorCode.FORBIDDEN
    assert e.value.http_status() == 403


# --- E1-S1/E1-S5: account create (one-time secret) + RBAC -----------------------

def _login(client: TestClient, email: str, password: str) -> None:
    r = client.post("/api/auth/login", json={"email": email, "password": password})
    assert r.status_code == 200, r.text


def test_create_account_returns_one_time_secret_and_enforces_uniqueness() -> None:
    app, db = _make_app()
    _seed(db, "admin@okc.test", "secret-123", Role.ADMIN)
    client = TestClient(app)
    _login(client, "admin@okc.test", "secret-123")

    r = client.post(
        "/api/accounts",
        json={"email": "new-admin@okc.test", "display_name": "Newbie", "role": "admin"},
    )
    assert r.status_code == 200, r.text
    body = r.json()
    temp_password = body["temp_password"]
    assert temp_password  # shown ONCE
    assert body["account"]["role"] == "admin"
    assert body["account"]["status"] == "active"

    # The one-time secret is the real password: the new admin can log in with it.
    other = TestClient(app)
    _login(other, "new-admin@okc.test", temp_password)

    # Duplicate email is rejected.
    dup = client.post(
        "/api/accounts", json={"email": "new-admin@okc.test", "display_name": "Dupe"}
    )
    assert dup.status_code == 400
    assert dup.json()["code"] == "VALIDATION_FAILED"

    # Default role is least-privilege contributor.
    default = client.post("/api/accounts", json={"email": "c@okc.test", "display_name": "C"})
    assert default.status_code == 200
    assert default.json()["account"]["role"] == "contributor"


# --- E1-S2: last-admin guard ----------------------------------------------------

def test_last_admin_guard_blocks_demote_and_disable() -> None:
    app, db = _make_app()
    admin_a = _seed(db, "a@okc.test", "secret-123", Role.ADMIN)
    client = TestClient(app)
    _login(client, "a@okc.test", "secret-123")

    # Only one active admin -> cannot demote or disable it.
    r = client.patch(f"/api/accounts/{admin_a}/role", json={"role": "contributor"})
    assert r.status_code == 400 and r.json()["code"] == "VALIDATION_FAILED"
    r = client.patch(f"/api/accounts/{admin_a}/status", json={"status": "disabled"})
    assert r.status_code == 400 and r.json()["code"] == "VALIDATION_FAILED"

    # Add a second admin -> demoting the first is now allowed.
    created = client.post(
        "/api/accounts", json={"email": "b@okc.test", "display_name": "B", "role": "admin"}
    )
    assert created.status_code == 200
    ok = client.patch(f"/api/accounts/{admin_a}/role", json={"role": "contributor"})
    assert ok.status_code == 200 and ok.json()["role"] == "contributor"


# --- E1-S5: revoke-all on disable / password change -----------------------------

def test_revoke_all_sessions_on_disable() -> None:
    app, db = _make_app()
    _seed(db, "a@okc.test", "secret-123", Role.ADMIN)
    admin_b = _seed(db, "b@okc.test", "secret-123", Role.ADMIN)
    client_a, client_b = TestClient(app), TestClient(app)
    _login(client_a, "a@okc.test", "secret-123")
    _login(client_b, "b@okc.test", "secret-123")
    assert client_b.get("/api/auth/session").status_code == 200

    r = client_a.patch(f"/api/accounts/{admin_b}/status", json={"status": "disabled"})
    assert r.status_code == 200
    assert client_b.get("/api/auth/session").status_code == 401  # B's live session revoked


def test_revoke_all_sessions_on_password_change() -> None:
    app, db = _make_app()
    _seed(db, "a@okc.test", "secret-123", Role.ADMIN)
    admin_b = _seed(db, "b@okc.test", "old-pass-123", Role.ADMIN)
    client_a, client_b = TestClient(app), TestClient(app)
    _login(client_a, "a@okc.test", "secret-123")
    _login(client_b, "b@okc.test", "old-pass-123")
    assert client_b.get("/api/auth/session").status_code == 200

    r = client_a.post(f"/api/accounts/{admin_b}/password", json={"new_password": "new-pass-456"})
    assert r.status_code == 200
    assert client_b.get("/api/auth/session").status_code == 401  # old session revoked

    fresh = TestClient(app)
    _login(fresh, "b@okc.test", "new-pass-456")  # new password works


# --- bootstrap-admin ------------------------------------------------------------

def test_bootstrap_admin_creates_first_admin_and_is_idempotent(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    monkeypatch.setenv("OKC_WEB_BOOTSTRAP_ADMIN_EMAIL", "root@okc.test")
    monkeypatch.setenv("OKC_WEB_BOOTSTRAP_ADMIN_PASSWORD", "boot-pass-123")
    app, db = _make_app()  # register() runs the bootstrap step

    accounts = AccountStore(db)
    assert accounts.count_all() == 1
    assert accounts.count_active_admins() == 1

    client = TestClient(app)
    _login(client, "root@okc.test", "boot-pass-123")

    # Running bootstrap again is a no-op (table non-empty).
    _bootstrap_admin(accounts, PasswordHasher())
    assert accounts.count_all() == 1
