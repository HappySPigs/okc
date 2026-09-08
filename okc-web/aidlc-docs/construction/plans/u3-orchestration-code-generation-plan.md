# U3 Orchestration — Code Generation Plan

**Wave**: W2 · **Unit**: U3 (`app/orchestration`) · **Epic**: E3 (E3-S1..E3-S7)
**Depth**: Comprehensive, MVP-only · **Mode**: AUTOPILOT (Part 1 auto-approved; recommended continuation auto-selected at the 2-option gate).
**Single source of truth**: This plan governs U3 Code Generation only.

## Unit context

- **Stories implemented**: E3-S1 (create + curator_id), E3-S2 (source cap read + freeze), E3-S3 (checkpoint projection + single active mutation), E3-S4 (provider bind + remote-disclosure gate), E3-S5 (staleness UX signal), E3-S6 (compile normal path + no-clobber), E3-S7 (job polling + code/category errors + retryable PROJECT_BUSY).
- **Design inputs**: `construction/u3-orchestration/functional-design/{business-logic-model,business-rules,domain-entities}.md`.
- **Dependencies (all FROZEN — U3 may NOT edit them)**: U0 `app/adapter/{engine,dto,queue,schema_guard}`, `app/shared/{authz,error,jobs,state,audit}`, `app/config`, `app/main` (register discovery). U1 `AdminPrincipal`/`admin_context`/`require`/`Role`. U2 `app/upload` `sources` writer.
- **Interfaces produced**: `register(app, state)` in `app.orchestration.router` (discovered by `main.UNIT_MODULES`), admin-gated project routes, and the U5 handoff shape (`CompileResultView` + status hashes).
- **DB entities owned**: the `projects` table (already created in W0 `MIGRATION_0001_INIT`; no new migration). Read-only over `sources`/`jobs`/`curator_decisions`.
- **Code location** (greenfield monolith): `backend/app/orchestration/` (app code) + `backend/tests/test_u3_orchestration.py` (tests). Docs → `aidlc-docs/construction/u3-orchestration/code/`.

## Boundaries / guardrails

- MVP-only: no per-role provider editing, no SSE, no viewer role, no federation, no okc-core change (ADR-0002 — all engine access via `EngineWorker` + `adapter.dto`, no `okc` import in U3).
- RBAC-before-core: every mutating route `Depends(admin_context)` + `require(ctx,(Role.ADMIN,))` before any engine call.
- Single-writer discipline: reserving ops via `EngineWorker.call`/`.enqueue`; reads via `.read`. No new concurrency machinery.
- Errors: raise existing `EngineError`/`EngineErrorCode` only; never parse messages; status derived by the shared handler.
- U3 edits ONLY `app/orchestration/*`, `tests/test_u3_orchestration.py`, this plan, and `construction/u3-orchestration/code/summary.md`.

## Execution checklist

- [x] **Step 1 — Module DTOs** (`app/orchestration/models.py`): Pydantic wire models from domain-entities §5 — `CreateProjectRequest`, `ProjectView`, `ProjectStatusView`, `FreezeResponse`, `ProviderProfileView`, `BindProviderRequest`, `RunIntegrationRequest`, `JobAccepted`, `PreflightGateView`, `CompileRequest`, `CompileResultView`. No `okc` type. _(E3-S1..S7 contracts)_
- [x] **Step 2 — Project registry repository** (`app/orchestration/projects.py`): `ProjectRegistry(StateDb)` — `create/get/list/set_freeze/touch` over the `projects` table; `source_count(project_id)` and `source_set_fingerprint(project_id)` (versioned SHA-256 over sorted `sources` rows, read-only). Includes the checkpoint→next-action/resolver map + human progression. _(E3-S1, E3-S2, E3-S5)_
- [x] **Step 3 — Repository unit tests**: fingerprint determinism + change-detection; freeze state transitions; project CRUD. _(E3-S2, E3-S5)_
- [x] **Step 4 — Orchestration service** (`app/orchestration/service.py`): `OrchestrationService` methods — `create_project`, `freeze`, `status` (checkpoint projection + advisory/authoritative staleness, core wins), `list_providers`, `bind_provider`, `preflight` (direct await; build disclosure gate view from `routes[].boundary`), `integrate` (disclosure gate → `PROJECT_BUSY` pre-enqueue check → `enqueue` → JobId), `compile` (ready_to_compile precondition + fresh in-root output path + no-clobber). Path-in-root validation; profile allowlist; active-reserving-job gate. _(E3-S1,S2,S3,S4,S5,S6,S7)_
- [x] **Step 5 — Service unit tests**: RBAC-before-core (contributor/unauth → core untouched); provider allowlist reject; disclosure gate blocks remote without both booleans; `PROJECT_BUSY` on duplicate reserving mutation; compile refused when not `ready_to_compile`/stale; output-path-escape → `PATH_UNSAFE`. Real binding where feasible; DB-only pre-core assertions otherwise. _(E3-S3,S4,S5,S6,S7)_
- [x] **Step 6 — API layer + register** (`app/orchestration/router.py`): `register(app, state)` builds services and includes the router; routes per domain-entities §5 (`POST/GET /api/projects`, `/status`, `/freeze`, `/providers`, `/provider`, `/preflight`, `/integrate`, `/compile`). Job polling reuses the U0 mount. All mutating routes admin-gated. _(E3-S1..S7)_
- [x] **Step 7 — API layer tests**: 401/403/200 gating matrix; create→freeze→status happy path against production `create_app`; integrate returns `job_id`; error bodies carry `code`/`category`. _(E3-S1,S3,S4,S7)_
- [x] **Step 8 — Own-module gate**: `ruff check app/orchestration tests/test_u3_orchestration.py` clean; `mypy app/orchestration` clean; `pytest tests/test_u3_orchestration.py -q` green.
- [x] **Step 9 — Full-suite integration gate**: `ruff check app tests` + `mypy app tests` + `pytest -q` across the whole backend stays green (U3 registers via discovery without editing `main.py`); confirm U3 route appears and W1 spine still passes.
- [x] **Step 10 — Summary doc** (`aidlc-docs/construction/u3-orchestration/code/summary.md`): files created, endpoints, story coverage, decisions realized (Q1–Q7), verification results.
- [x] **Step 11 — Gate**: emit the standardized 2-option Code Generation completion message; under autopilot auto-select "Continue to Next Stage"; update `aidlc-state.md` (W2 complete → W3).

## Story traceability

| Story | Steps |
|---|---|
| E3-S1 create + curator_id | 1, 2, 4, 6, 7 |
| E3-S2 source cap + freeze | 1, 2, 3, 4 |
| E3-S3 checkpoint loop + single active mutation | 2, 4, 5, 6, 7 |
| E3-S4 provider + disclosure gate | 1, 4, 5, 6 |
| E3-S5 staleness UX | 2, 4, 5 |
| E3-S6 compile no-clobber | 1, 4, 5 |
| E3-S7 job polling + error codes + PROJECT_BUSY | 1, 4, 5, 6, 7 |

## Extension configuration

Security/Resiliency Baseline + Property-Based Testing disabled → N/A (no rule file loaded). Baseline input validation + real-binding tests remain ordinary requirements.
