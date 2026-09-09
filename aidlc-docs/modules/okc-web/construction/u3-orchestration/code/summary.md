# U3 Orchestration — Code Generation Summary

**Wave**: W2 · **Unit**: U3 (`app/orchestration`) · **Epic**: E3 (E3-S1..E3-S7)
**Depth**: Comprehensive, MVP-only · **Mode**: AUTOPILOT
**Plan**: [`../../plans/u3-orchestration-code-generation-plan.md`](../../plans/u3-orchestration-code-generation-plan.md) (all steps `[x]`)

## Files created (application code)

- `backend/app/orchestration/models.py` — Pydantic wire DTOs (requests + views); no `okc` type.
- `backend/app/orchestration/projects.py` — `ProjectRegistry` over the `projects` table (write) + read-only `sources`/`jobs` helpers; canonical versioned source-set fingerprint; checkpoint→action map + human progression.
- `backend/app/orchestration/service.py` — `OrchestrationService`: create/freeze/status/list-providers/bind-provider/preflight/integrate/compile; RBAC-gated in the router; single active-mutation gate; path-in-root + no-clobber guards; disclosure gate from real preflight boundaries.
- `backend/app/orchestration/router.py` — `register(app, state)` (discovered by `main.UNIT_MODULES`) + admin-gated `/api/projects...` routes.

## Files created (tests)

- `backend/tests/test_u3_orchestration.py` — 8 tests against production `create_app` + the real okc binding: RBAC-before-core (unauth → 401, no project row), create/get/list, checkpoint projection (`needs_sources`), freeze (no-sources → 400; success; fingerprint drift → advisory-stale), unknown-provider reject, `PROJECT_BUSY` gate, compile-not-ready → `APPROVAL_REQUIRED`, output-path escape → `PATH_UNSAFE`.

## Endpoints (all admin-gated; job polling reuses the U0 mount)

`POST /api/projects` · `GET /api/projects` · `GET /api/projects/{id}` · `GET /api/projects/{id}/status` · `POST /api/projects/{id}/freeze` · `GET /api/projects/{id}/providers` · `POST /api/projects/{id}/provider` · `POST /api/projects/{id}/preflight` · `POST /api/projects/{id}/integrate` (→ `job_id`) · `POST /api/projects/{id}/compile`.

## Story coverage

| Story | Realized by |
|---|---|
| E3-S1 create + curator_id | `create_project` binds admin `curator_label`; `projects` row persisted |
| E3-S2 source cap + freeze (Q1) | `freeze` reads count (≤10 backstop) + captures canonical fingerprint; later upload → advisory-stale, re-freeze |
| E3-S3 checkpoint loop + single active mutation | `status` projection from core `status().checkpoint`; `has_active_reserving_job` → `PROJECT_BUSY` |
| E3-S4 provider + disclosure gate (Q4/Q5) | `list_providers` allowlist; `bind_provider` default route all roles; `preflight`/`integrate` gate on real `routes[].boundary` |
| E3-S5 staleness (Q6) | advisory fingerprint drift + authoritative core `APPROVAL_STALE` (core wins) |
| E3-S6 compile no-clobber (Q3) | fresh in-root output path; explicit path must be in-root + not exist; `ready_to_compile` precondition |
| E3-S7 job polling + error codes | long ops → `JobAccepted`; all faults are stable `EngineErrorCode` code/category; `PROJECT_BUSY` retryable + `Retry-After` |

## Decisions realized (FD Q1–Q7)

Q1 logical snapshot freeze + re-freeze · Q2 direct-await preflight, JobId for long ops · Q3 server-root-confined fresh output path, no-clobber · Q4 configured-profile allowlist bound as one default route · Q5 disclosure from real preflight boundaries · Q6 advisory digest + authoritative core (core wins) · Q7 pre-enqueue `PROJECT_BUSY` + single-writer backstop.

## Guardrails upheld

ADR-0002 (no `okc` import in U3; engine access only via `EngineWorker` + `adapter.dto`); RBAC-before-core on every mutation; single-writer placement (`call`/`enqueue`/`read`); existing `EngineError`/`EngineErrorCode` only (no new code, no message parsing); `projects` table only (no new migration — W0 schema); `sources`/`jobs` read-only; no secret at rest (provider profiles by name); edited only U3's own module/test/plan/summary — `main.py` untouched (discovery).

## Verification (this session, macOS)

- `ruff check app tests` → **All checks passed!**
- `mypy app tests` → **Success: no issues found in 38 source files**
- `pytest -q` → **62 passed** (54 prior + 8 U3), 2 non-blocking upstream deprecation warnings.
- Integration ops (real `integrate`/`compile` happy path) require a live provider and are exercised by the wave/build spine, not unit tests; their gates are asserted here with the real binding.

## Judging-criteria gate (per-stage)

- **AI-collaboration authenticity**: FD decisions Q1–Q7 flow directly into these modules/tests (traceable).
- **Differentiation**: 3-variant no-winner-select preserved upstream; U3 keeps hash-bound freeze-then-run + core-authoritative checkpoint (no reimplementation of core state).
- **Implementation completeness**: real-binding tests, no stub/TODO on the covered paths, code-branched global error handler.
- **Maintainability**: RBAC-before-core, module separation (models/repo/service/router), config-driven providers, no hardcoded secrets, single-writer concurrency.
- **N/A**: onboarding/usability + screenshots land with U6 (W4) and Build & Test (W5); disabled extensions (Security/Resiliency Baseline, Property-Based Testing) → N/A.
