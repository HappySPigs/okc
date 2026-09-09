# Build & Test — Summary (W5)

## Current continuation results — 2026-09-09

Final backend: **114 pytest passed**, Ruff clean, mypy clean (55 files). Frontend: **8 tests passed**, typecheck and Vite build passed. The full synthetic-provider/real-core success path and actual four-module bridge now pass. Unicode aliases, native validation, source/DB/chunk-crash recovery, version pinning and publication-storage recovery are covered. See [versioned serving](../u5-serving/code/versioned-serving.md) and the [root evidence](../../../../construction/build-and-test/build-and-test-summary.md).

The original W5 results below remain historical. Live-provider quality, installed native services, screenshots and remotely run CI were not verified. Root CI now includes hooks and the integration bridge; dependency sync preserves the separately built native wheel.

**Scope**: okc-web, both stacks. Backend = FastAPI (Python 3.11+) consuming okc-core via the `okc-compiler` 0.3.0 Python binding; Frontend = React/Vite SPA. okc-core stays Rust and unmodified.

## Exit artifacts (hard-MUST) — status
| Artifact | Status |
|---|---|
| Lockfiles | ✅ `backend/uv.lock` + `frontend/package-lock.json` |
| CI (both stacks) | ✅ `.github/workflows/backend-ci.yml` + `frontend-ci.yml` (module-scoped; hoist to repo-root `.github/` for the monorepo — see the note in each file) |
| README | ✅ `README.md` (architecture, quickstart, demo path, config, design-honesty) |
| PROCESS narrative | ✅ `aidlc-docs/PROCESS.md` (AI-DLC traceability) |
| Secret scan | ✅ 0 hardcoded-secret candidates; secrets are env-var **names** only |
| Screenshots | ⚠️ capture guide + script provided (`screenshots/`); automated capture needs a browser (absent in the build env) and the full flow needs a live LLM provider — see `screenshots/README.md`. Honest gap, not faked. |

## Verified gates (this environment, real okc binding — no engine mock)
- Backend: `ruff check app tests` clean · `mypy app tests` clean (49 files) · `pytest -q` → **80 passed**.
- Frontend: `tsc --noEmit` clean · `vite build` → `frontend/dist` · `vitest` → **8 passed**.

## What is / isn't covered by automated tests
- **Covered (offline, real binding)**: app factory + unit discovery; RBAC-before-core (401/403 reach zero engine calls); U1 auth/argon2/sessions; U2 token issue/rotate/revoke + hostile-input upload → real `add_source`; U3 create/freeze/status/provider-allowlist/PROJECT_BUSY/compile-gate/path-guard; U4 DecisionGate (blocking-waive forbidden, rationale required, record-then-act, no winner-select); U5 publish gating + read-only machine API (405/404/traversal) + verify/explain error-branch; SPA history-fallback; the W1 no-mock spine (login→token→upload→native add_source→poll→manifest).
- **NOT covered here (provider-dependent)**: the AI-driven happy path integrate→taxonomy→synthesis→critic→compile→verified. Requires a configured LLM provider; the seams are proven (engine returns real typed errors offline). Run it manually per the integration-test doc with a provider configured.

## Judging-criteria mapping (per-stage gate)
1. **AI-collaboration authenticity** → `aidlc-docs/` full trail + `PROCESS.md`; each stage's decisions feed the next (FD Q1–Q7 → code).
2. **Problem definition** → `README.md` + requirements.
3. **Differentiation** → no winner-select (3-variant `CuratorDecision`, code-enforced), hash-bound freeze-then-run, core-authoritative checkpoint.
4. **Working implementation** → real-binding tests (80 + 8), CI, lockfiles, global error handler, no stub on covered paths. Screenshot gap flagged honestly.
5. **Onboarding/usability** → SPA loading/empty/error states, 3 focal screens, honest constraint surfacing.
6. **Maintainability** → RBAC-before-core, module separation, config/secrets by env, single-writer concurrency, ADR-0002 seam.

## Documents
`build-instructions.md` · `unit-test-instructions.md` · `integration-test-instructions.md` · `performance-test-instructions.md` (this dir).

## Continuous upload extension — 2026-09-09

Upload/adapter/foundation/production-spine focused aggregate: **63 pytest passed**, Ruff
clean, mypy clean for 16 owned files. The Rust-generated CBOR fixture ran and passed.
Real add/rebind, durable session/receipt retries, deletion/rename, empty Vaults,
token rotation, stale-base rejection, and DB failure recovery are covered. These
results extend the historical W5 evidence above. Full module integration and serving
verification are separately coordinated by the root initiative.
Details: [continuous-sync verification](continuous-sync-verification.md).
