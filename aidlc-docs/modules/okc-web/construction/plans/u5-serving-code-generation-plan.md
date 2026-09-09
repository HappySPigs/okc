# U5 — Serving & okc-mcp Contract — Code Generation Plan

**Unit**: U5 (`app/serving`) · **Epic**: E5 · **Stories**: E5-S1..E5-S5 (MVP only)
**Depth**: lean/MVP · **Stack**: FastAPI + Pydantic v2 (ADR-0025)
**Grounding**: `services.md` (§S0.A read-path policy, §U5 ServingService/ProvenanceComposer/ServingStateStore), `stories.md` (E5-S1..S5), `aidlc-state.md` (W3). Functional Design folded into CodeGen (U5 FD skipped per autopilot).

> Faithful MVP port. FROZEN (READ/IMPORT ONLY, never edit): `main.py`, `shared/**`, `adapter/**`, `config.py`, `app/orchestration/**`, `app/auth/**`, `app/upload/**`, `app/review/**` (sibling agent). `okc` reached ONLY via `state.engine` + `app.adapter.dto` (ADR-0002 — never imported here).

## Key design decisions (reconciled against frozen contracts)
- **Publish/unpublish = pure SQLite flip** on `serving_publications` (NO engine op, NO queue). Publish (E5-S1) precondition: `engine.read(status).checkpoint == "verified"` (read path, non-reserving) else refuse `APPROVAL_REQUIRED` with zero state change. Re-publish rebinds the manifest identity; compiled output stays immutable (that is compile/U3's no-clobber property, not U5's).
- **Compiled-vault-path source**: best-effort peek at `engine.manifest(root)` for an output-path key; primary/robust source = newest dir under `{projects_root}/{project_id}/compiled/` (the U3 compile convention `compiled/{ULID}`). Bind `bound_corpus_hash = ProjectRegistry.source_set_fingerprint(project_id)` at publish; `bound_integration_plan_id`/`bound_taxonomy_hash` from the manifest where available.
- **Two surfaces**: (a) admin-gated (`admin_context` + `require(ADMIN)`) publish/unpublish/status; (b) machine read-only UNAUTHENTICATED (E5-S2) GET/HEAD only, served ONLY for a published (live|stale) project. Mutating verbs on machine paths → 405 (framework partial-match, GET-only routes). Missing/traversal/out-of-root file path → 404. Unpublished project machine endpoints → 404.
- **Path-traversal guard**: `realpath` + `commonpath` must stay under the compiled vault root AND under one of the 3 served roots (`knowledge/`, `legacy/`, `.okc/`); else 404.
- **verify/explain** via `engine.read` (non-reserving, bypass the write queue — S0.A read-path policy); a bad artifact surfaces a clean `EngineError` code (adapter maps `OkcError`), never a crash.
- **Staleness (E5-S5)** derived at read time: published + `current source_set_fingerprint != bound_corpus_hash` ⇒ effective status `stale`; still serve the immutable manifest with a Stale label.
- **Provenance (E5-S3)** enriches `explain` (`ProvenanceView.record`) with owner labels read from the `sources` table (read-only) and a "verification proves internal consistency, not publisher authenticity" note.
- **Contract (E5-S4)** = location (abs local dir + read API base) + format (Markdown-only, `knowledge/`+`legacy/`+`.okc/` layout) + read-only endpoints list + explicit out-of-scope note (RAG chunking/embedding/vector-index/query + MCP tool surface = okc-mcp's job; consumers must re-embed). No okc-mcp-internal call is exposed.
- **Errors**: only EXISTING `EngineErrorCode` (`NOT_FOUND`, `METHOD_NOT_ALLOWED`, `APPROVAL_REQUIRED`, `PATH_UNSAFE`, `FORBIDDEN`). Never parse message strings; never add a code.

## Files
- [x] `app/serving/models.py` — `PublicationView`, `ServingFileListView`, `ServingVerifyView`, `ServingProvenanceView`, `McpContractView` (+ small nested models).
- [x] `app/serving/store.py` — `ServingStateStore` (+ `PublicationRow`): get/publish/unpublish over `serving_publications`.
- [x] `app/serving/service.py` — `ServingService`: publish/unpublish/status, list_files/read_file, verify/explain, contract, staleness, path guard, compiled-vault-path resolver.
- [x] `app/serving/router.py` — `register(app, state)`: admin surface + machine read-only surface.
- [x] `tests/test_u5_serving.py` — real-binding, production `create_app`, offline-safe suite.

## Steps
1. [x] Write this plan.
2. [x] `models.py` — minimal Pydantic wire views for E5-S1..S5.
3. [x] `store.py` — `ServingStateStore` pure-SQLite publish/unpublish/get.
4. [x] `service.py` — `ServingService` facade (publish gating, machine reads, verify/explain, contract, staleness, traversal guard, compiled-vault-path resolver).
5. [x] `router.py` — `register`: `POST /api/projects/{pid}/serving/publish|unpublish`, `GET /api/projects/{pid}/serving` (admin); `GET /api/serving/{pid}/{files,file,verify,explain,contract}` (machine, unauth GET/HEAD).
6. [x] `tests/test_u5_serving.py` — publish gating (non-verified → APPROVAL_REQUIRED, zero state change; unauth → 401 zero state change); machine read over a fake compiled vault (files list, file body, 405 POST, 404 traversal/out-of-root, unpublished → 404); contract (location+format+out-of-scope); verify/explain clean EngineError; staleness label.
7. [x] Verify GREEN: `ruff check app/serving tests/test_u5_serving.py`, `mypy app/serving`, `pytest tests/test_u5_serving.py -q`.
8. [x] Summary — persisted by the orchestrator to `aidlc-docs/construction/u5-serving/code/summary.md` from the agent's verified report (subagent harness blocked its own write).
