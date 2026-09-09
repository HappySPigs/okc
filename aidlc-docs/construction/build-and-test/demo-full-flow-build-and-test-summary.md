# Build & Test — Full-Flow Local Demo

Initiative: [Full-Flow Local Demo](../../inception/requirements/demo-full-flow-requirements.md). Root-owned demo tooling under [`demo/`](../../../demo/). Fully local, deterministic, reproducible.

## Result: PASS (end-to-end)

`demo/run-demo.sh --fresh` (~2 min) executed the whole scenario:

| Beat | Evidence |
|---|---|
| Preflight + bring-up | curation provider `:8797`, fresh okc-web `:8000` (isolated state), Caddy `:8443` |
| 1. 5 vaults uploaded | `source_count = 5/10` (4 dept via upload API + "mine") |
| Baseline merge (Playwright) | integrate → taxonomy → synthesis+critic → resolve → compile (23 files) → **publish revision 1** `2b1c4dc4…` (test PASS, 10.1s) |
| 2. MCP edits "mine" | `apply_session_capture(dryRun:false)` created the note (`verified:true`) |
| 3. hook detects → upload | real `watcher-bin` cycle logged (incl. honest `UnknownIssuer` TLS line); change transferred via upload API (same token → new source revision) |
| 4. web shows changed | project **`stale`** (banner + `01-stale-banner` screenshot) |
| 5. re-merge (Playwright) | re-freeze → re-integrate → review (5 **preserved contradictions**) → compile → **publish revision 2** `8f9f2520…` (test PASS, 10.3s) |
| Versioning | **publication history = 2 revisions** |
| Artifacts | 11 screenshots `01-overview … 10-served-published` + `01-stale-banner` in `demo/state/screenshots/` |

## Module gate (okc-web enhancements)

Two backward-compatible additions, both in the allowed demo module:
- optional `role` on `POST /api/projects/{id}/provider` (per-role AI routing).
- optional `timeout_ms` on the provider spec (forwarded to the engine profile).

`ruff` clean · `mypy` clean · `tests/test_u3_orchestration.py` 9/9 (incl. new `test_bind_provider_role_routing_and_validation`).

## Key engineering decisions (see audit.md for the full trail)

1. **Local model → deterministic curation provider for step 5.** Local qwen and Claude-via-Bedrock-shim both failed okc-core's strict synthesis/critic contracts (exact `content_hash`/`block_id`/`document_id` echoing; `metadata_` id rejected in a `BlockId` field). okc-core's own tests compile against a deterministic fixture provider ("not live-model quality evidence"). `demo/scripts/curation_provider.py` is a small input-aware deterministic provider that echoes identifiers exactly, groups near-duplicate note pairs into topic clusters, and preserves contradictions — driving the full real pipeline reproducibly with no external model.
2. **Hooks TLS.** `watcher-bin` trusts only bundled Mozilla roots, so a local self-signed cert can't be trusted (per user decision okc-hooks is unmodified). The watcher demonstrates real **detection**; the **transfer** goes via the upload API on the same token/source.
3. **Re-entrant integrate.** The Playwright flow drives `integrate` again after each approval (organizer → taxonomy → synthesis+critic → clusters → ready → compile), mutating via the authenticated session API (reliable) with the UI navigated+screenshotted per beat; reads tolerate `409 PROJECT_BUSY`.

## Honest boundaries

- Steps 1–4 exercise the real okc-web/okc-hooks/okc-mcp code paths.
- Step-5 merge **content** is deterministic-synthetic (labeled in `demo/README.md`); the pipeline, review, contradiction-preservation, compile, verification, versioning and serving are all real okc-core.
- Fully local/offline: no Ollama, no Bedrock, no API keys required.
- No commits/pushes made.

## Extension compliance (this initiative)

- Security Baseline (applicable rules only): tokens/cookies/DB confined to gitignored `demo/state/` at `0600`; demo admin bootstrap password is local-only; no secrets logged or committed. Infra/web-platform rules N/A.
- Resiliency Baseline: N/A (local ephemeral demo).
- Property-Based Testing: N/A (thin integration tooling + Playwright e2e; okc-web change uses okc-web's existing pytest style).
