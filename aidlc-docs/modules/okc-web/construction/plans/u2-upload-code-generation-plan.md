# U2 — Upload & Token — Code Generation Plan

**Unit**: U2 (`app/upload`) · **Epic**: E2 · **Stories**: E2-S1..E2-S6 (MVP only)
**Depth**: standard · **Stack**: FastAPI + Pydantic v2 (ADR-0025)
**Grounding**: `services.md` (U2 7-step pipeline), `component-methods.md` (U2), `components.md` (U2), `application-design.md` (§7 job poll, §8 error table, §9 data model), `ui-screens.md` (E2-1..E2-5), `stories.md` (E2-S1..S6), `requirements.md` (FR-UP-1..4, NFR-SEC-1).

> Faithful MVP port. Do NOT edit frozen U0 (`main.py`, `shared/**`, `adapter/**`, `config.py`) or `app/auth/**`. `okc` is reached ONLY via `state.engine` (never imported here).

## Key design decisions (reconciled against frozen contracts)
- **Token format** `selector.verifier` — both `secrets.token_urlsafe(32)` (≈256-bit). `selector` stored plaintext (UNIQUE lookup); `verifier_hash = sha256(verifier_salt + verifier)` hex, per-row random `verifier_salt`; constant-time compare (`hmac.compare_digest`). Plaintext returned ONCE (E2-S1). id = `tok_{ULID}`, source id = `src_{ULID}`.
- **Token resolver** installed at `app.state.token_resolver`. `resolve_upload` returns `None` for malformed / selector-miss / verifier-mismatch (→ U0 maps to `TOKEN_INVALID`); RAISES `EngineError(TOKEN_REVOKED|TOKEN_EXPIRED)` for a verifier-valid token that is revoked/expired (propagates through `upload_auth` → global handler). Core untouched either way (C-1).
- **Two slot counts** (both honour ≤10): token issue selects the lowest free active-token slot under an in-process lock; rotation revokes and reissues on the same slot. Ingest `SlotAccountant.reserve` atomically selects the lowest free registered-or-in-flight source slot. Authoritative cap and duplicate checks run again on the single-writer thread; synchronous pre-check returns 429 fast.
- **DUPLICATE_SOURCE**: okc-core does NOT dedup identical content at `add_source` (verified empirically), so okc-web owns it — content-hash (sha256 of uploaded bytes) compared to `sources` rows for the project → idempotent no-op `DUPLICATE_SOURCE` (400), retry-safe (E2-S5/E2-5).
- **Formats**: `.zip` (story-canonical vault archive; full hostile-input defenses) + single `.md` (one-note vault, enables the real add_source proof). `tar.zst` recognized but rejected (`zstandard` not installed; cannot add a dep) with a clear message. All land to an absolute DIRECTORY under `<projects_root>/<project_id>/sources/<source_id>/` (server-generated path — no user input in the path, no traversal).
- **add_source ordering (C-1/C-6)**: pipeline steps 1–5 (auth → reserve → receive+hash → validate/dup → land) all run BEFORE core and guarantee core-untouched on rejection. Step 6 `state.engine.enqueue("add_source", ...)` returns a pollable `JobId`; `run(engine, progress)` calls `engine.add_source(root, cmd)` and, ONLY on success, performs commit + `SourceRegistry.record` (write `sources` row) + `mark_used`. On `add_source` failure the worker records the terminal error; slot "release" is a no-op (reserve wrote nothing). Poll reflects real completion.
- **Audit**: no upload-specific table exists in the frozen schema, and `curator_decisions` has a CHECK that forbids a non-decision row. The append-only trail for an upload = the `sources` row + `upload_tokens.last_used_at`/`registered_source_id` + the `jobs` record. No write to `curator_decisions`.

## Files
- [x] `app/upload/models.py` — request/response DTOs.
- [x] `app/upload/tokens.py` — `UploadTokenSecret`, `UploadTokenStore`, `UploadTokenService`, `UploadTokenResolver`.
- [x] `app/upload/ingest.py` — `UploadReceiver`, `ArchiveValidator`, `SourceLander`, `SlotAccountant`, `SourceRegistry` (write), `UploadIngestService`.
- [x] `app/upload/router.py` — `register(app, state)`: build services, install token resolver, mount routes.
- [x] `tests/test_u2_upload.py` — real-okc (no mock) suite.

## Steps
1. [x] Write this plan.
2. [x] `models.py`: `IssueTokenRequest`, `IssuedToken`, `TokenSummary`, `TokenListView`(+`SlotUsage`), `UploadTargetView`, `IngestAccepted`, `ValidationReport`. (E2-S1..S6)
3. [x] `tokens.py` — `UploadTokenSecret.generate/verifier_hash/present/parse` (E2-S1). (FR-UP-1, NFR-SEC-1)
4. [x] `tokens.py` — `UploadTokenStore` (insert/find_by_selector/list_for_project/active_slot_count/revoke/mark_used). (E2-S1/S2)
5. [x] `tokens.py` — `UploadTokenService.issue/list/revoke/rotate/slot_usage` (RBAC(Admin), one-time reveal, ≤10 issue cap). (E2-S1/S2)
6. [x] `tokens.py` — `UploadTokenResolver.resolve_upload` → `UploadContext` / None / raise revoked|expired. (E2-S3, C-1)
7. [x] `ingest.py` — `SlotAccountant.reserve/commit/release` (source ≤10, SOURCE_CAP_EXCEEDED). (E2-S5)
8. [x] `ingest.py` — `UploadReceiver.receive` (chunked stream to temp, hard byte cap, incremental sha256). (E2-S4, UPLOAD_TOO_LARGE)
9. [x] `ingest.py` — `ArchiveValidator.inspect` (format, traversal, symlink, zip-bomb blocking; markdown-ratio + duplicate warn). (E2-S4, FR-UP-3, PATH_UNSAFE)
10. [x] `ingest.py` — `SourceLander.land` (materialize to absolute dir under sources root). (E2-S5, C-6)
11. [x] `ingest.py` — `SourceRegistry.record` (write `sources` row) + `UploadIngestService.ingest` (full ordered pipeline → JobId via `state.engine.enqueue`). (E2-S5/S6)
12. [x] `router.py` — `register`: install `app.state.token_resolver`; routes: `POST/GET /api/projects/{pid}/tokens`, `POST .../{tid}/revoke`, `POST .../{tid}/rotate`, `GET /u/{token}`, `POST /u/{token}/upload`. (E2-S1..S6)
13. [x] `tests/test_u2_upload.py` — isolated app + real EngineWorker; real okc project seed; real add_source landing + poll to completed + assert sources row & okc manifest; token reveal/rotation; revoked→TOKEN_REVOKED; byte-cap; traversal/symlink pre-landing block; owner-kind pre-core validation; 10/10 cap; synchronous and concurrent duplicate idempotency; unique in-flight slot reservations.
14. [x] Verify GREEN: `ruff check app/upload tests/test_u2_upload.py`, `mypy app/upload`, `pytest tests/test_u2_upload.py -q`.
15. [x] Write `aidlc-docs/construction/u2-upload/code/summary.md`.
