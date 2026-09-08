# Unit / Component Test Instructions (W5)

## Backend
```bash
cd backend
.venv/bin/python -m ruff check app tests     # lint
.venv/bin/python -m mypy app tests           # types (49 files clean)
.venv/bin/python -m pytest -q                # 80 passed
```
Run a single unit's suite: `pytest tests/test_u3_orchestration.py -q` (also `test_u1_auth`, `test_u2_upload`, `test_u4_review`, `test_u5_serving`, `test_foundation`, `test_spa_fallback`, `test_w1_spine`).

**No engine mock.** Tests build the app through the production `create_app` and use the real `okc` binding (`build_client([])`). Offline-safe suites assert on real adapter-mapped error codes (e.g. `PROJECT_INVALID`, `VERIFICATION_FAILED`, `needs_sources`) rather than stubbing okc-core.

Coverage highlights: RBAC-before-core (401/403 → zero engine calls); U1 argon2/session/last-admin-guard; U2 slot cap + duplicate + hostile-input + rotation; U3 freeze/fingerprint/checkpoint-projection/PROJECT_BUSY/compile-gate/path-guard; U4 DecisionGate (blocking-waive forbidden, rationale required, record-then-act, exactly-one audit row, no winner-select); U5 publish gating + machine read API (405/404/traversal) + verify/explain error-branch; SPA history-fallback.

## Frontend
```bash
cd frontend
npm run test        # vitest: lib.test.ts (ApiError code/category/retry precedence + format), badges.test.tsx (severity, fail-closed)
npm run build       # tsc typecheck is part of the build gate
```

## Conventions
- Backend: pytest + FastAPI `TestClient`; each test builds an isolated app (own SQLite temp DB) to avoid cross-test state.
- Frontend: vitest + Testing Library; stable `data-testid` (`{screen}-{role}`) on interactive elements for future e2e.
