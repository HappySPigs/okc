# U1 — Auth & RBAC — Code Generation Plan

**Unit**: U1 (`app/auth`) · **Epic**: E1 · **Stories**: E1-S1..E1-S5 (MVP only)
**Pattern**: purely pre-core, DB-only. NEVER touches the engine queue (unit-of-work.md §U1).
**Depends on (FROZEN, read-only)**: `app/main.py` (register seam, `AppState`), `app/shared/authz.py`
(`Role`, `AdminPrincipal`, `SessionResolver` Protocol, `admin_auth`, `admin_context`, `require`,
`SESSION_COOKIE`), `app/shared/error.py` (`EngineError`), `app/shared/state.py` (`accounts`/`sessions`
tables), `app/config.py` (`cookie_secure`).

Split of duties (C-1): **U1 authenticates (supplies the `SessionResolver`)**; **U0 authorizes**.

---

## Steps

- [x] 1. **Password hashing** (`app/auth/hashing.py`) — `PasswordHasher` over `argon2.PasswordHasher`
  (argon2id PHC string), constant-time `verify`, plus `verify_dummy` for enumeration-timing parity.
  → E1-S5 (secret at rest, one-way salted hash).
- [x] 2. **AccountStore** (`app/auth/accounts.py`) — repo over `accounts`: `insert`, `find_by_email`,
  `find_by_id`, `update_role`, `update_status`, `update_password_hash`, `touch_last_login`,
  `count_active_admins`, `list`. Typed `AccountRecord`/`AccountSummary`. `acc_{ULID}` ids.
  → E1-S2 (roles persisted), E1-S5 (password_hash column only).
- [x] 3. **SessionStore + SessionResolver** (`app/auth/sessions.py`) — sessions repo (`create` returns
  plaintext ONCE, stores `sha256` hex; `touch`, `revoke`, `revoke_all_for_account`) + `SessionResolver`
  implementing the frozen Protocol `resolve_admin(session_plaintext)->AdminPrincipal|None` (hash → join
  accounts → not revoked / not past TTL / idle-ok / role==admin / status==active → slide `last_seen_at` →
  `AdminPrincipal(account_id, session_id_hash, curator_label=display_name or email)`). Cookie helpers
  (`set`/`clear`) honoring `cookie_secure`. → E1-S1 (session), E1-S4 (curator_label), E1-S5 (sha256 at rest).
- [x] 4. **AuthService** (`app/auth/service.py`) — orchestrator (DB-only, pre-core):
  - `login(email, password)` → find → verify (constant-time; dummy verify on miss) → bad creds =
    `UNAUTHENTICATED` (401, no leak) → disabled = `ACCOUNT_DISABLED` → non-admin = `UNAUTHENTICATED`
    (no app-shell session) → `SessionStore.create` + `touch_last_login`. → E1-S1.
  - `logout(session_id_hash)` → `SessionStore.revoke`. → E1-S1.
  - `whoami(account_id)` → `AccountSummary` for the session view. → E1-S1/E1-2.
  - `create_account(email, display_name, role)` → email-unique (`VALIDATION_FAILED`) → generate one-time
    temp password → hash → insert → return `(AccountSummary, temp_password)` shown ONCE. → E1-S1/E1-S5, NFR-SEC-1.
  - `set_role(id, role)` → **last-admin guard** (block demote of the final active admin). → E1-S2.
  - `set_status(id, status)` → **last-admin guard** (block disable of final active admin) →
    on disable, **revoke-all** sessions. → E1-S2.
  - `change_password(id, new_password)` → rehash → **revoke-all** sessions. → E1-S5.
  - `list_accounts()` → `list[AccountSummary]`. → E1-S2/E1-3.
- [x] 5. **Router + wiring** (`app/auth/router.py`) — request/response Pydantic models + `APIRouter`:
  - `POST /api/auth/login` (sets `okc_session` cookie) · `POST /api/auth/logout` · `GET /api/auth/session`
    (whoami; `Depends(admin_auth)`).
  - `GET /api/accounts` · `POST /api/accounts` · `PATCH /api/accounts/{id}/role` ·
    `PATCH /api/accounts/{id}/status` · `POST /api/accounts/{id}/password` — each guarded by
    `Depends(admin_context)` + `require(ctx, (Role.ADMIN,))`. → E1-S3 (RBAC-before-core).
  - `register(app, state)`: build stores/service from `state.db` + `state.config.cookie_secure`;
    install `app.state.session_resolver`; run bootstrap-admin; `app.include_router(...)`.
- [x] 6. **Bootstrap first admin** (in `register`) — if `accounts` empty and `OKC_WEB_BOOTSTRAP_ADMIN_EMAIL`
  + `OKC_WEB_BOOTSTRAP_ADMIN_PASSWORD` set (read via `os.environ`), create the first admin. Idempotent.
- [x] 7. **Tests** (`tests/test_u1_auth.py`) — ISOLATED app (no `create_app`): in-memory `StateDb`, U0
  `AppState`, bare `FastAPI` + `register_error_handlers` + `auth.router.register`, `TestClient`. Real argon2.
  Cover: login sets cookie + whoami; bad password 401; disabled → `ACCOUNT_DISABLED`; non-admin login 401;
  session resolve/touch/expire/revoke; account create (temp secret once) + role/status under RBAC (no cookie
  → 401; `require` blocks contributor → 403); last-admin guard blocks demote/disable; revoke-all on disable &
  password change; bootstrap-admin idempotent.
- [x] 8. **Verify to green** (from `backend/`): `ruff check app/auth tests/test_u1_auth.py`,
  `mypy app/auth`, `pytest tests/test_u1_auth.py -q`. Iterate until clean.
- [x] 9. **Summary** (`aidlc-docs/construction/u1-auth/code/summary.md`).
