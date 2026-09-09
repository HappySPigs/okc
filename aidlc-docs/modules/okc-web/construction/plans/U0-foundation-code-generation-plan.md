# U0 Foundation — Code Generation Plan (W0)

**Wave**: W0 (blocking) · **Unit**: U0 (adapter + shared) · **Depth**: CodeGen comprehensive (FD/NFR/Infra skipped per autopilot)
**Mode**: AUTOPILOT — this plan is auto-approved (recommended "Continue"); logged in audit.md.
**Grounding**: [services.md](../../inception/application-design/services.md) S0.A–S0.E · [application-design.md](../../inception/application-design/application-design.md) §2/§4/§5/§8/§9 · [unit-of-work.md](../../inception/application-design/unit-of-work.md) §1 U0 · [stack-migration-spec.md](stack-migration-spec.md) · okc Python 바인딩(`okc-compiler` 0.3.0, `bindings/python/okc/__init__.pyi`; INTEROP_SCHEMA_VERSION=2).
**Single source of truth** for W0 code generation. Application code → `okc-web/backend/` (NEVER aidlc-docs/).

## Scope (MVP-only)
Foundation seam that every other unit compiles against. U0 owns **mechanism only**, no domain policy (ADR-0002). Freezes the contracts + the `sources`/SourceRegistry write-read seam so U1–U6 can proceed.

## Steps
> **Re-baselined to FastAPI per ADR-0025** — the Rust/axum implementation is being re-generated in Python(FastAPI); implementation-specific checkboxes reset to `[ ]`. Contracts/decisions/seam are invariant (faithful 1:1 port, not a redesign).

- [x] Step 1 — `backend/pyproject.toml` (PEP 621): deps `fastapi`, `uvicorn[standard]`, `pydantic>=2`, `sqlalchemy>=2`, `argon2-cffi`, `python-ulid`, `python-multipart`, `okc-compiler` (path `../../okc-core/bindings/python`); dev `pytest`, `pytest-asyncio`, `httpx`, `ruff`, `mypy`. Commit `uv.lock` (NFR-PORT-1). *(Building `okc-compiler` needs the Rust toolchain + maturin — build-time only.)*
- [x] Step 2 — `app/shared/error.py` (S0.D §8): `EngineError{code,category,message,retryable,retry_after_ms}` + table-driven code→HTTP exception handler (`@app.exception_handler`), `Retry-After` by code. Branch on code/category, never message.
- [x] Step 3 — `app/shared/state.py` (§9): single SQLite WAL `StateDb` = SQLAlchemy 2.0 `Engine` (`PRAGMA journal_mode=WAL`, `engine.begin()`) + ordered DDL migration `0001_init` (no Alembic) creating ALL tables incl. `sources` (SourceRegistry seam frozen in W0) + `jobs`/`job_events`/`curator_decisions`.
- [x] Step 4 — `app/shared/jobs.py` (Q7): `JobStore` + `JobId` + `JobSnapshot` + `job_events` — polling snapshot single source.
- [x] Step 5 — `app/shared/audit.py` (S0.E, §5, C3): append-only `AuditStore.append`, `HashBindings{proposal_hash,critic_hash,taxonomy_hash}`, `CuratorDecision` Pydantic **discriminated union with exactly 3 variants, no SelectWinner** (winner-select row not representable).
- [x] Step 6 — `app/shared/authz.py` (S0.C §3, C-1): `Role{Admin,Contributor}`, `Principal{Admin,UploadToken}`, `AuthContext`, `AuthProvider` Protocol + `Depends(...)` dependencies (`require_role(*roles)`); invariant: 403/401 ⇒ zero engine calls.
- [x] Step 7 — `app/adapter/schema_guard.py`: assert `okc.INTEROP_SCHEMA_VERSION==2` at startup + each decode; else `SchemaUnsupported`.
- [x] Step 8 — `app/adapter/dto.py`: okc-web `*View/*Cmd/*Spec` Pydantic + `from_native(dict)`; neutral `EngineJobEvent`. Only place besides `engine.py` importing okc types.
- [x] Step 9 — `app/adapter/engine.py` (§2): `OkcEngine` Protocol (create/open/status/add_source/preflight/integrate/taxonomy/approve_taxonomy/clusters/approve_cluster/regenerate_cluster/compile/verify/explain/manifest) + single `OkcEngineImpl` (only class importing `okc`) + `EngineError.from_okc_error`.
- [x] Step 10 — `app/adapter/queue.py` (S0.A §4, NFR-CONC-1): dedicated single-worker `ThreadPoolExecutor(max_workers=1)` owning mutating submissions; asyncio bridge (`loop.run_in_executor` / `call_soon_threadsafe`); `call` (fast) / `enqueue` (long → JobId + pump events→JobStore); read-path bypass (second `OkcClient`) for non-reserving reads.
- [x] Step 11 — `app/main.py` / `app/config.py`: startup SchemaGuard → AppConfig(env, names only) → StateDb migrate → OkcClient → OkcEngineImpl → spawn engine worker → FastAPI router (health + canonical job-status routes §7) → uvicorn serve (`--workers 1`). `AppState` shared.
- [x] Step 12 — real `pytest` + import-smoke (build `okc-compiler` via maturin) instead of `cargo build`/`cargo test`; frozen contracts version-locked.

## 6-criteria gate (W0)
- **C1**: modules 1:1 with unit map (§13). **C3**: `CuratorDecision` 3-variant discriminated union, no winner-select (code-verifiable now). **C4**: real okc Python binding imported (no mock). **C6**: single okc-binding import point (`app/adapter`), single-writer queue, SQLite seam. C2/C5 → U6/README (N/A here).
- **Extensions**: Security/Resiliency/PBT opt-out → N/A. **NFR**: skipped per autopilot.

## Completion (2026-09-08, autopilot)
**DONE — all 12 steps green.** Verification: `ruff` clean · `mypy` clean (19 files) · **23/23 pytest** (error table, schema guard, migration=9 tables, JobStore roundtrip + terminal-error, CuratorDecision exactly-3 + winner-select rejected at type AND DB-CHECK layers, RBAC 401/403-before-core). okc-compiler 0.3.0 wheel built via maturin (cargo 1.98.1) + installed → `import okc` green (schema=2); OkcEngineImpl maps a **real** OkcError end-to-end (relative path → `PATH_NOT_ABSOLUTE`/`validation`/HTTP 400) — no mock (C4).
**Minor deviations (recorded):** (a) `okc-compiler` declared as a pyproject **optional group** (`[engine]`) + installed as a maturin-built wheel, so plain `uv sync` works in rust-less envs while the server/integration tests require the wheel; (b) `uv.lock` generation deferred to Build-and-Test (W5) with the other exit artifacts. Both scope-neutral.
