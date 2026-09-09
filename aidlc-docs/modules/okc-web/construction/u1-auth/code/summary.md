# Code Generation Summary — U1 (Auth & RBAC)

**Wave**: W1 · **Unit**: U1 (`app/auth/`) · **Depth**: CodeGen comprehensive, MVP-only
**Status**: COMPLETE · GREEN (independently re-verified by orchestrator)
**Stories**: E1-S1..E1-S5 (only). No scope expansion; viewer role (FR-AUTH-2) deferred.

## Pattern
Purely **pre-core, DB-only** — U1 never touches the engine queue. U1 *authenticates* (supplies
the `SessionResolver`); U0 `shared.authz` *authorizes* (owns the guard, ordering, canonical
`Role`/`Principal`/`AuthContext`). No edits to U0/shared/adapter/config/pyproject/upload.

## Files created
| File | Responsibility |
|---|---|
| `backend/app/auth/hashing.py` | `PasswordHasher` — argon2id PHC, constant-time verify, dummy-verify for timing parity (no user enumeration) |
| `backend/app/auth/accounts.py` | `AccountStore` + `AccountRecord`/`AccountSummary`; `acc_{ULID}` ids; `count_active_admins()` |
| `backend/app/auth/sessions.py` | `SessionStore` (opaque plaintext id once, sha256 hex at rest), `SessionResolver` (frozen sync Protocol impl), cookie helpers, TTL/idle constants |
| `backend/app/auth/service.py` | `AuthService` — login/logout/whoami + account admin + invariants |
| `backend/app/auth/router.py` | HTTP models + `APIRouter` + `register(app, state)` |
| `backend/tests/test_u1_auth.py` | 16 tests, isolated app, real argon2 (no mock) |
| `aidlc-docs/construction/plans/u1-auth-code-generation-plan.md` | CodeGen plan (all steps [x]) |

## HTTP endpoints
- `POST /api/auth/login` (public; sets `okc_session` cookie)
- `POST /api/auth/logout` (admin_auth; revoke + clear cookie)
- `GET  /api/auth/session` (admin_auth; whoami → `{account_id,email,display_name,role,curator_label}`)
- `GET  /api/accounts` · `POST /api/accounts` (returns `{account, temp_password}` once)
- `PATCH /api/accounts/{id}/role` · `PATCH /api/accounts/{id}/status` · `POST /api/accounts/{id}/password`
- All `/api/accounts` routes: `Depends(admin_context)` + `require(ctx, (Role.ADMIN,))` (RBAC-before-core, C-1).

## Key decisions (deferred-question resolutions — session/token hashing + Role model)
- **Sessions**: `secrets.token_urlsafe(32)` plaintext lives ONLY in the HttpOnly / SameSite=Lax / Path=/ cookie (Secure per `config.cookie_secure`); `sha256` hex stored in `sessions.id_hash`. Absolute TTL 12h + idle 60m with sliding `last_seen_at` on resolve.
- **curator_label (E1-S4)**: `display_name or email` — the unverified label later passed to core as `curator_id`.
- **Bootstrap admin**: in `register()`, if `accounts` empty AND `OKC_WEB_BOOTSTRAP_ADMIN_EMAIL` + `OKC_WEB_BOOTSTRAP_ADMIN_PASSWORD` set (read via `os.environ`, no config.py edit) → create first admin (idempotent, fires only when table empty).
- **Refusal codes** (docs didn't pin): unknown-email and bad-password both → `UNAUTHENTICATED` (401) with a dummy argon2 verify on miss (anti-enumeration); disabled → `ACCOUNT_DISABLED` (400); a correctly-authenticated CONTRIBUTOR is also refused 401 (no app-shell session — matches "admin만 착지, contributor는 셸 없음").
- **Invariants**: last-admin guard (blocks demote/disable of the final active admin) and email-duplicate → `VALIDATION_FAILED` (400; enum has no dedicated code); revoke-all sessions on disable and on password change; one-time temp password via `secrets.token_urlsafe`, never persisted or re-shown (NFR-SEC-1).

## U0-contract friction (flagged, NOT worked around)
1. Frozen `SessionResolver.resolve_admin` is **synchronous**, but `component-methods.md` shows async `resolve(...)`. Implementation matched the FROZEN `authz.py` Protocol (sync) — consistent with the sync U0 repos (`jobs.py`/`audit.py`). Doc/contract discrepancy noted; no contract change needed.
2. `/api/*` auth paths were chosen by the orchestrator brief (docs pin SPA routes, not API paths; no frontend `apiClient` exists yet). To reconcile with U6 (W4) if it expects different account paths.

## Verification (independently re-run by orchestrator, from `backend/`)
- `ruff check app/auth tests/test_u1_auth.py` → `All checks passed!`
- `mypy app/auth` → `Success: no issues found in 6 source files`
- `pytest tests/test_u1_auth.py -q` → `16 passed` (2 pre-existing starlette/httpx deprecation warnings, not from U1)

## Judging-criteria touchpoints
- **C1** (traceability): E1-S1..S5 implemented per services.md §U1 / unit-of-work.md §U1.
- **C4** (real, no-mock): real argon2 hasher; RBAC-before-core proven by tests (auth-fail ⇒ core untouched).
- **C6** (maintainability): secrets from env; hashed-at-rest (argon2id passwords, sha256 sessions); input validation; RBAC + input checks in place.

Wave-barrier integration (full-suite + login→issue-token→upload spine) is orchestrator-owned and runs after U2 lands.
