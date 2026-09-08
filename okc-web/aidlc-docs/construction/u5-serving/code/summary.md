# U5 Serving & okc-mcp Contract — Code Generation Summary

**Wave**: W3 · **Unit**: U5 (`app/serving`) · **Epic**: E5 (E5-S1..E5-S5)
**Depth**: CodeGen only (FD folded in) · MVP-only, lean · **Mode**: AUTOPILOT (real parallel with U4)
**Plan**: [`../../plans/u5-serving-code-generation-plan.md`](../../plans/u5-serving-code-generation-plan.md)

_Persisted by the orchestrator from the `u5-serving` agent's verified report (the agent's harness blocked its own summary write; content is authoritative, independently re-verified at the W3 wave barrier)._

## Files created (application code)

- `backend/app/serving/models.py` — `PublicationView`, `ServingFileListView`, `ServingVerifyView`, `ServingProvenanceView`, `McpContractView` (+ `ContractLocation`/`ContractFormat`/`ContractEndpoint`).
- `backend/app/serving/store.py` — `ServingStateStore` (+`PublicationRow`): `get`/`publish` (upsert→live)/`unpublish` (→offline). Pure SQLite flip on `serving_publications`; no engine op, no queue.
- `backend/app/serving/service.py` — `ServingService` facade: publish gating, machine file list/body, verify/explain, contract, staleness, path-traversal guard, compiled-vault-path resolver, read-only owner labels.
- `backend/app/serving/router.py` — `register(app, state)`; `main.py` NOT edited (discovered via `UNIT_MODULES`).

## Files created (tests)

- `backend/tests/test_u5_serving.py` — 7 tests, production `create_app` + real okc binding, offline-safe.

## Endpoints

- **Admin-gated** (`admin_context` + `require(ADMIN)`; 401/403 in the dependency ⇒ zero engine calls / zero state change): `POST /api/projects/{pid}/serving/publish`, `POST .../serving/unpublish`, `GET /api/projects/{pid}/serving` (status).
- **Machine read-only, UNAUTHENTICATED, GET/HEAD only** (served only for a published live|stale project): `GET /api/serving/{pid}/files`, `/file?path=...`, `/verify`, `/explain?path=...`, `/contract`. Mutating verbs → 405; missing/traversal/out-of-root → 404; unpublished project → 404.

## Story coverage

- **E5-S1** publish/unpublish; Verified precondition (`status.checkpoint == "verified"` via read path) else `APPROVAL_REQUIRED` (422) with zero state change; republish rebinds manifest, compiled output immutable; RBAC(Admin).
- **E5-S2** file list over `knowledge/`+`legacy/`+`.okc/`; raw body; 405 on mutating verbs; 404 traversal/out-of-root/missing.
- **E5-S3** `verify()`/`explain()` via non-reserving `engine.read` (queue bypass, S0.A); provenance enriched with owner labels read read-only from `sources`; internal-consistency-not-publisher-authenticity disclaimer.
- **E5-S4** contract = location (abs local dir + read API base) + format (Markdown-only, 3-root layout) + read-only endpoints list + explicit out-of-scope note (RAG chunking/embedding/vector-index/query + MCP tool surface = okc-mcp's job; consumer MUST re-embed). All GET, no okc-mcp-internal call.
- **E5-S5** staleness derived at read time (bound corpus hash vs current `source_set_fingerprint`); immutable manifest still served, labelled stale.

## Notes / decisions

- **Compiled-vault-path source**: best-effort peek at `engine.manifest(root)` for an output-path key, then robust fallback = newest sub-dir under the U3 compile convention `{projects_root}/{project_id}/compiled/{run}`. The project manifest does not carry a compiled output path in this build, so filesystem discovery is the effective source; the manifest peek is retained defensively.
- Real binding returns `VERIFICATION_FAILED` (422, "artifact manifest is invalid JSON") for verify/explain against a fake vault — a genuine adapter-mapped `EngineError`, not a crash, not faked.

## Frozen-contract friction (flagged, not worked around)

1. `SourceRegistry` (`app/upload/ingest.py`) exposes no owner-label read method → owner labels read directly from the frozen `sources` table via a read-only SELECT inside `app/serving` (no frozen file edited).
2. `app.review.router` was absent at run time; `main.py._register_units` is `ModuleNotFoundError`-tolerant, so U5 registers independently.
3. Routed `status()` through `engine.read` (non-reserving read path) per the S0.A read-path policy.

## Verification (agent-reported; re-verified at the W3 wave barrier)

- `ruff check app/serving tests/test_u5_serving.py` → All checks passed!
- `mypy app/serving` → Success: no issues found in 5 source files
- `pytest tests/test_u5_serving.py -q` → 7 passed (2 non-blocking upstream deprecation warnings)
