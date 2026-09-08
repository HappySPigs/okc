# Stack Migration Spec — Rust/axum + Next.js → FastAPI + React SPA

**Status**: AUTHORITATIVE (autopilot, user-authorized 2026-09-08). Single source of truth for the
backend/frontend technology port. Every inception design doc, construction plan, and generated code
artifact MUST conform to the mapping tables below.

**Directive**: "backend Rust로 되어있는거 fastapi로 전체 변경 … 권장안대로 선택 … MVP 범위를 벗어나거나
설계 구현 범위가 커지는 방향으로는 절대 가지마 … front도 fast api 에 맞는 프레임워크로 변경 … autopilot …
설계안 바꾸고 construction까지도."

**Governing principle — FAITHFUL PORT, NOT REDESIGN.** This is a 1:1 technology substitution. Nothing is
added or removed from the product. All of the following are **INVARIANT** and MUST survive verbatim:
- The 29 approved user stories / 5 epics; every FR/NFR id.
- The 5 core design decisions: (1) ADR-0002 single-seam adapter boundary; (2) RBAC-before-core (C-1);
  (3) single engine process + in-process single-writer queue + `PROJECT_BUSY`; (4) Case-B
  `CuratorDecision` with **NO winner-select variant** (C3); (5) hash-bound staleness-cascade ownership (C-4).
- The SQLite state data model (tables, columns, hashed-at-rest set), the OkcError→HTTP mapping table,
  the E4-3 / E5-2 focal data contracts, the screen→story matrix, the epic→unit→module map.
- The frozen UI (`ui-screens.md` + `design-system.md`): routes, screens, tokens, component inventory.
- `INTEROP_SCHEMA_VERSION == 2`; the reserving-vs-non-reserving op partition; the hidden runtime spine
  `integrate(U3)→approve(U4)→compile(U3)→serve(U5)`.

**Grounding for the pivot (already anticipated in the design):**
- `requirements.md` Q8: *"Rust(axum) + okc-interop 직접 링크 ⚠️팀 숙련도 시 Python로 변경"* — Python was the
  documented fallback.
- `requirements.md` C-7 / R-5: the binding path was flagged as the alternative (opaque-JSON payload cost,
  stack choice subject to review). This spec resolves that trade-off in favor of Python.
- okc-core ships first-class Python bindings at `okc-core/bindings/python` (PyPI `okc-compiler` 0.3.0,
  maturin/pyo3, `requires-python>=3.11`) whose surface mirrors `okc-interop` 1:1 (verified against
  `bindings/python/okc/__init__.pyi`).

---

## 1. Backend: Rust/axum → FastAPI (Python 3.11+)

### 1.1 Runtime & framework
| Rust / axum design | FastAPI / Python port |
|---|---|
| Rust 1.97, `edition 2021` | Python **3.11+** (matches `okc-compiler` `requires-python`) |
| `axum` 0.7 single binary | **FastAPI** app served by **uvicorn**, `--workers 1` (single worker is REQUIRED — one process owns the single `OkcClient`; the "single long-lived process" topology is unchanged) |
| `tokio` async runtime | asyncio (uvicorn/ASGI) |
| `tower`/`tower-http` middleware, CORS, static fs | Starlette/FastAPI middleware, `CORSMiddleware`, `StaticFiles` (serves the built SPA in prod) |
| `serde` / `serde_json` + DTO structs | **Pydantic v2** models (`model_config = ConfigDict(...)`) |
| Rust `enum` (Role, Principal, CuratorDecision) | `enum.Enum` + Pydantic **discriminated unions** (tagged by a `Literal` field) |
| `thiserror` error types | Python exception classes (`EngineError`, `ApiError`, `AuthError`, …) |
| `tracing` / `tracing-subscriber` | stdlib `logging` (structured) |
| `Cargo.toml` / `Cargo.lock` | `pyproject.toml` (PEP 621) + `uv.lock` (pip fallback documented) |
| `cargo build` / `cargo test` | `uv sync` / `pytest`; `ruff` (lint+format) + `mypy` (types) |

### 1.2 Engine boundary (ADR-0002) — the load-bearing seam
| Rust `okc-interop` (path dep) | `okc` Python package (`okc-compiler` 0.3.0) |
|---|---|
| `okc_interop::OkcClient` | `okc.OkcClient` |
| `okc_interop::Project` | `okc.Project` |
| `okc_interop::Job<T>` (`.state()`, `.events()`, `.result()` — **blocks on Condvar**) | `okc.Job[_T]` (`.state`, `.events()`, `.result()` — **blocks**, GIL released by the native ext) |
| `okc_interop::OkcError{code,category,message,retryable,details}` | `okc.OkcError(code, category, message, retryable, details)` |
| `INTEROP_SCHEMA_VERSION` (==2) | `okc.INTEROP_SCHEMA_VERSION` (==2) |
| `OkcClient::api_info()` | `OkcClient.api_info()` |
| typed result structs (`VerificationResult`, `TaxonomyView`, …) | mostly `dict[str, Any]` / `TypedDict` (`VerificationResult`, `ExplanationResult`) → **the adapter parses these into Pydantic DTOs** (directly answers C-7's "opaque JSON payload" note) |
| `SourceInput{source_id,path,owner_display_name,snapshot_id}` | `okc.SourceInput(source_id, path, owner_display_name=, snapshot_id=)` |
| `ProviderProfile` | `okc.ProviderProfile(...)` |

**Method surface (identical names on `okc.OkcClient` / `okc.Project`)**: `create_project`, `open_project`,
`verify_artifact`, `explain_artifact`, `test_provider`, `api_info`; `manifest`, `status`, `add_source`,
`rebind_source`, `replace_sources`, `set_language`, `set_ai_route`, `preflight`, `integrate`, `taxonomy`,
`approve_taxonomy`, `clusters`, `approve_cluster`, `regenerate_cluster`, `compile`.

**Install/build**: `okc-compiler` is a local maturin/pyo3 package → installed from
`../okc-core/bindings/python` (path dependency; `maturin develop` or built wheel). Building it needs the
Rust toolchain + maturin — a **build-time** dependency of the binding, documented in README/CI (the
already-installed cargo 1.98.1 remains relevant for this reason only).

### 1.3 Module map (unchanged units; Rust crate module → Python package)
`backend/` becomes a Python package. Module boundaries and unit ownership are **unchanged**.

| Unit | Rust module | Python package |
|---|---|---|
| U0 | `adapter` (+`adapter::queue`, `dto`, `schema_guard`) | `app/adapter/` (`engine.py`, `queue.py`, `dto.py`, `schema_guard.py`) |
| U0 | `shared` (`authz`,`error`,`jobs`,`audit`,`state`) | `app/shared/` (`authz.py`, `error.py`, `jobs.py`, `audit.py`, `state.py`) |
| U1 | `auth` | `app/auth/` |
| U2 | `upload` | `app/upload/` |
| U3 | `orchestration` | `app/orchestration/` |
| U4 | `review` | `app/review/` |
| U5 | `serving` | `app/serving/` |
| — | `main.rs`/`lib.rs` | `app/main.py` (FastAPI app factory + startup wiring) + `app/config.py` |

### 1.4 Cross-cutting mechanism ports
| Concern | Rust idiom | FastAPI / Python port (semantics preserved) |
|---|---|---|
| **Single-writer queue** (S0.A, NFR-CONC-1) | dedicated **blocking** `EngineActor` OS thread owning the sole `OkcClient`; bounded `tokio::mpsc` + `oneshot`; because `Job::result()` blocks on a Condvar | a dedicated **single worker thread** = `ThreadPoolExecutor(max_workers=1)` owning the sole `OkcClient`. Async handlers submit an `EngineCommand` and `await` via `loop.run_in_executor(engine_pool, ...)` (or a `Future` + `call_soon_threadsafe`). The worker drives each `okc.Job` to terminal (pump `.events()` → `JobStore`, then blocking `.result()`) before the next. **Exactly one reserving/mutating op at a time.** |
| **Read-path bypass** (verifier #8) | non-reserving reads use a read-path `OkcClient` clone, skip the queue | a **second `OkcClient`** + a small `ThreadPoolExecutor` for `taxonomy`/`clusters`/`manifest`/`verify`/`explain` → never blocked behind a long `integrate`/`compile` |
| `EngineHandle.call()` / `.enqueue()` | fast (await terminal) / long (return `JobId`, pump events) | `async def call(op)` / `async def enqueue(op) -> JobId`; long ops return the id immediately, worker pumps events into `JobStore` |
| **RBAC-before-core** (S0.C, C-1) | tower middleware + `FromRequestParts` extractors run before the handler | FastAPI **`Depends(...)`** dependencies resolve auth before the handler body. A 401/403 raised in the dependency ⇒ handler never runs ⇒ **zero engine calls** (the C-1 testable invariant holds). `require_role(*roles)` = a dependency factory. |
| `AuthContext` / `UploadContext` extractors | `FromRequestParts` | dependency callables returning the `AuthContext` / `UploadContext` Pydantic models |
| `AuthProvider` trait (injected) | trait; U1 session impl, U2 token impl | `Protocol`; `SessionAuthProvider` (U1), `TokenAuthProvider` (U2) |
| **Error mapping** (S0.D, Q9) | `ApiError` enum + `IntoResponse` + `http_status(code,category)` table | FastAPI `@app.exception_handler(ApiError)` / `EngineError` → table-driven (`dict`) `code/category`→status. Body `{code,category,message,retryable,retry_after_ms?}`. `Retry-After` synthesized **by code**. Branch on code/category, **never** message. Same extended table (`ProjectInvalid→422`, `ArtifactSchemaUnsupported→422`, `PathUnsupported→400`, `OutputDurabilityUncertain→500`, safe default 500). |
| **Schema guard** | `SchemaGuard::assert_client` / `check_payload` | `SchemaGuard` class: assert `okc.INTEROP_SCHEMA_VERSION == 2` at startup + check the `interop_schema_version` field on each decoded payload; else raise `EngineError(code="SchemaUnsupported")` |
| **DTOs + conversions** | `adapter::dto` `*View/*Cmd/*Spec` + `From<okc_interop::…>` | Pydantic models; conversion = classmethod `from_native(payload: dict) -> Model` / `model_validate(...)`. `adapter/dto.py` is the ONLY module that reads raw binding dicts. |
| **JobStore** (Q7) | SQLite `jobs`+`job_events` + in-mem snapshot | same; `JobStore` class over SQLAlchemy; `JobSnapshot`/`JobStatus` mirror `okc.JobState` literals (`queued/running/cancelling/publishing/completed/failed/cancelled`) |
| **AuditStore + CuratorDecision** (S0.E, C3) | append-only SQLite; `enum CuratorDecision` 3 variants, **no SelectWinner** | `AuditStore` (append-only, no UPDATE/DELETE); `CuratorDecision` = Pydantic **discriminated union with exactly 3 members** (`ApproveTaxonomy`,`ApproveCluster`,`RegenerateCluster`) — winner-select is **structurally unrepresentable** (C3, code-verifiable) |
| **DecisionGate** (U4) | pure fn `evaluate`/`assert_approvable` → deterministic `422 APPROVAL_REQUIRED` | pure function; raises `ApiError`→422 before any engine call (core's blocking-approval rejection maps to generic `ProjectInvalid`, so the clean 422 must originate here — reason preserved) |
| **State DB** (Q3) | `StateDb` = single-file SQLite pool (sqlx/rusqlite), WAL | `StateDb` = **SQLAlchemy 2.0 `Engine`** over `sqlite3`, WAL (`PRAGMA journal_mode=WAL`); repositories use `engine.begin()`. `Migrations` = ordered DDL runner (`0001_init`, …) — **no Alembic** (MVP). Same tables/columns/constraints. |
| **Crypto** | `argon2`+`password-hash` (argon2id), `sha2`, `rand`, `hex`, `ulid` | `argon2-cffi` (argon2id PHC), `hashlib.sha256` (session lookup hash), `hmac` (token verifier alt), `secrets` (CSPRNG selector/verifier), `python-ulid` (`acc_`/`tok_`/`proj_`/`job_` ids) |
| `SecretString` | wrap plaintext | keep plaintext only in locals; never log/persist; one-time reveal preserved |

### 1.5 Recommended dependency set (`pyproject.toml`)
`fastapi`, `uvicorn[standard]`, `pydantic>=2`, `sqlalchemy>=2`, `argon2-cffi`, `python-ulid`,
`python-multipart` (upload streaming), `okc-compiler` (path). Dev: `pytest`, `pytest-asyncio`, `httpx`,
`ruff`, `mypy`. **Excluded (beyond-MVP / scope-neutral):** Alembic, Celery/Redis, any message broker,
ORM-heavy patterns, SSE/WebSocket libs. HTTP polling and the in-process queue are retained exactly.

---

## 2. Frontend: Next.js (App Router) → React + Vite SPA

FastAPI is a pure JSON API (unlike Next.js, itself a full-stack framework). The natural, scope-minimal
"framework suited to FastAPI" is a **pure client-side React SPA**. The entire design system and screen
inventory are **preserved**; only the Next.js shell changes.

| Next.js design (`design-system.md`/`ui-screens.md`) | React + Vite SPA port |
|---|---|
| Next.js **App Router** (`app/`, server + client components) | **Vite + React 18 + TypeScript** SPA; all components are client components |
| file-system routes, dynamic segments `/projects/[id]` | **React Router v6** declarative routes `/projects/:id` (1:1 with the `ui-screens.md` route table) |
| server components (shell/static) vs client (review/upload/polling) | single client tree; data via TanStack Query (no server-component split needed) |
| `next/font` self-host Geist | self-hosted Geist via `@fontsource`/`@font-face` |
| `next/link`, `next/image` | React Router `<Link>`, plain `<img>` |
| `next build` / `next dev` | `vite build` (static assets) / `vite dev` (proxy `/api`,`/u` → uvicorn) |
| **Tailwind** | Tailwind (unchanged) |
| **shadcn/ui** (Radix) | shadcn/ui (framework-agnostic; Vite + path aliases) — unchanged |
| **Tremor** charts | Tremor (React) — unchanged |
| **Lucide** icons | `lucide-react` — unchanged |
| **Radix Colors** tokens | unchanged (CSS variables) |
| **TanStack Query** + **TanStack Table (DataTable)** | unchanged (framework-agnostic React) |
| `apiClient` (`lib/api/client.ts`), 3 auth contexts | unchanged (`fetch`, `credentials:'include'` for admin cookie; `Authorization: Bearer` for `/u/{token}`; read-only serving) |
| `okcErrorMap`, `queryKeys`, `useJobPolling` | unchanged (pure React; code/category branch only) |
| `frontend/` (Next.js) | `frontend/` (Vite): `src/`, `index.html`, `vite.config.ts`, `package.json`, `package-lock.json` |

**Prod serving**: FastAPI mounts the built `frontend/dist` via `StaticFiles` (single-origin → admin cookie
just works). **Dev**: Vite dev server proxies `/api` + `/u` to uvicorn. Tests: **Vitest**; **Playwright**
for the mandated e2e screenshots (`screenshots/`).

---

## 3. Build / Test / CI exit-artifact remap (Build-and-Test, W5 hard gate)
| Rust+Node design | FastAPI+React port |
|---|---|
| `cargo build` / `cargo test` | `uv sync` + `pytest` (+ `ruff check`, `mypy`) |
| `npm/pnpm build` (Next.js) | `npm ci` + `vite build` (+ `vitest`) |
| lockfiles: `Cargo.lock` + JS lock | `uv.lock` (Python) + `package-lock.json` (frontend) |
| CI (`.github/workflows`) building Rust + Node | GitHub Actions: (a) build `okc-compiler` via maturin (needs Rust) → `uv sync` → `pytest`/`mypy`; (b) `npm ci` → `vite build`/`vitest` |
| screenshots/ · README · PROCESS narrative · secret scan | unchanged (hard-MUST); README documents the maturin/Rust build-time step |

---

## 4. Topology story (unchanged in substance)
Single uvicorn process (**one worker**) + one SQLite WAL file + single-writer engine thread + read-path
client. React SPA served as static assets from the same origin (prod) or via Vite proxy (dev). okc-mcp
consumes the read-only `GET /api/serving/*`. This is the same "single long-lived process, single SQLite
state, single-writer engine queue" design — re-expressed in Python.

## 5. Terminology find/replace guide for doc edits (apply consistently)
- "Rust (axum)" → "Python (FastAPI)"; "axum single binary" → "FastAPI/uvicorn single process (one worker)".
- "`okc-interop` (path dep, commit-pinned)" → "`okc` Python bindings (`okc-compiler` 0.3.0, maturin path install)".
- "`okc_interop::X`" → "`okc.X`"; "`OkcError`" surface unchanged.
- "trait `OkcEngine`" → "`OkcEngine` Protocol/ABC"; "`OkcEngineImpl`" → "`OkcEngineImpl` (the one class naming `okc.*`)".
- "`#[async_trait]`/`Send+Sync`/`mpsc`/`oneshot`/`Condvar`/`spawn_blocking`" → the Python equivalents in §1.4 (single-worker executor + asyncio bridge).
- "tower middleware / `FromRequestParts` extractor" → "FastAPI `Depends` dependency".
- "`rusqlite`/`sqlx`" → "SQLAlchemy 2.0 Core over sqlite3 (WAL)".
- "`argon2` crate" → "`argon2-cffi`"; "`sha2`" → "`hashlib`"; "`ulid` crate" → "`python-ulid`".
- "Rust enum … variants" → "Pydantic discriminated union / `enum.Enum`" (keep variant names + the C3 no-winner-select property).
- "Next.js (App Router)" → "React + Vite SPA (React Router v6)"; "server components" → "client components + TanStack Query".
- "`cargo build`/`cargo test`" → "`pytest`/`uv sync`"; keep "real, no-mock" wording (C4).
- Keep ALL ids, decisions, tables, and the frozen UI verbatim.
