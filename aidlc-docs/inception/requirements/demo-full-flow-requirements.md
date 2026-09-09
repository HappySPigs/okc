# Requirements — Full-Flow Local Demo

> Root/umbrella initiative (routing rule 4). Coordinates okc-web + okc-hooks + okc-mcp + okc-core with new root-owned `demo/` tooling. Module histories remain authoritative; this doc owns the cross-module demo contract only.

## Intent Analysis

- **User request**: A single, reproducible demo on this MacBook that rides the *entire* okc pipeline end to end — 5 personal vaults on the web, a live MCP edit to one vault, the hooks watcher auto-uploading the change, the web showing it changed, and an admin re-merging + resolving conflicts — with hooks/mcp/web all wired and the web authenticated on this laptop. Build the whole thing (Playwright-driven).
- **Request type**: New Feature (demo/integration tooling), Brownfield.
- **Scope**: Cross-module / repository-wide assembly (4 modules + demo harness).
- **Complexity**: Moderate (orchestration of existing, already-built modules; no new product surfaces).

## Confirmed decisions (clarifying questions)

1. **LLM brain = all-local Ollama.** `qwen2.5:14b` for organizer/synthesis/critic; `nomic-embed-text` for embedding. Verified feasible on Apple M5 / 32 GB / Metal. No Bedrock, no shim, offline after model pull.
2. **MCP edit beat = scripted stdio call** to okc-mcp (`tools/call apply_session_capture`, `dryRun:false`) for reproducibility; also `claude mcp add` so it is real in a live session.
3. **"conflict / changed" = the product's real behavior.** Step 4 = project-level `stale` banner (no per-file diff). Step 5 = preserved contradictions ("승자 없음") + critic-finding disposition (minor-waive w/ rationale, regenerate for blocking) → compile. Dummy vaults are authored with deliberate contradictions so the critic actually raises findings.
4. **Delivery = fully automated + reproducible.** Playwright drives the admin web walkthrough with a screenshot per beat; shell scripts stand up infra + seed sources.

## Functional Requirements

- **DEMO-01** — One-command, idempotent bring-up of a **fresh, isolated** okc-web demo instance (own state DB + projects root under `demo/state/`), single-writer uvicorn on `:8000`, bootstrap admin, on this laptop. Must not clobber the user's existing instance (stop the leftover Bedrock dev instance first; demo state is separate).
- **DEMO-02** — All-local AI provider stack: Ollama serving `qwen2.5:14b` + `nomic-embed-text`. okc-web configured with two provider profiles and **per-role routing**: `embedding → nomic-embed-text`, `organizer/synthesis/critic → qwen2.5:14b`.
- **DEMO-03** — **5 curated Obsidian vaults** (4 department + 1 "mine"), Korean notes, with **deliberate cross-vault contradictions** so the critic yields **≥1 blocking + ≥1 minor** finding at merge time.
- **DEMO-04** — okc-hooks `watcher-bin` runs against the "mine" vault over **HTTPS** (local Caddy TLS proxy with a keychain-trusted cert in front of `:8000/api/sync`): startup sync seeds "mine"; a later `sync-now` uploads the live change.
- **DEMO-05** — okc-mcp registered via `claude mcp add`; a scripted stdio client performs `apply_session_capture(dryRun:false)` writing a new/updated note into "mine" (the note introduces a change that alters the merge).
- **DEMO-06** — Scenario execution: (1) 5 vaults present; **baseline** integrate → compile → publish **revision 1**; (2) MCP edits "mine"; (3) watcher uploads the change; (4) project goes **`stale`** (web shows it); (5) admin re-freeze → re-integrate → taxonomy approve → **ClusterReview** (contradiction preserved, minor-waive, regenerate a blocking finding) → compile → publish **revision 2**.
- **DEMO-07** — **Playwright** drives the admin web flow headed, capturing a screenshot per beat (+ optional video); orchestrated by `demo/run-demo.sh` running steps `00→50` on a wiped state.
- **DEMO-08** — Teardown/reset scripts (`down.sh`, `reset.sh`) that wipe demo state and stop demo-only services, leaving the user's environment as before.

## Non-Functional Requirements / Constraints

- **Local & offline** after the one-time model pull; no cloud credentials (Bedrock dropped for the demo).
- **Secret hygiene** (Security Baseline, applicable rules): upload tokens + serving read tokens live in `0600` files, never printed in full, never committed; `demo/state/` and rendered configs are git-ignored; TLS-only hooks endpoint preserved.
- **No mocks in the pipeline** — real okc-core engine + real Ollama. Honest error surfacing (typed errors, not fabricated success).
- **Reproducibility** — fixed dummy content; `run-demo.sh` re-runnable from a clean state. (okc-core's semantic candidate step is deterministic; LLM stages are non-deterministic but bounded by the curated 5-vault dataset.)
- **Honesty of framing** — step 4 is a project-level stale signal (not a per-file diff); step 5 is finding disposition (not pick-a-side). Documented in the runbook.

## Scope Guards (per "avoid scope expansion")

- **No new web UI**; reuse existing screens and `data-testid` selectors.
- **No per-file diff view.**
- **okc-web change is limited** to an optional `role` field on the existing `POST /api/projects/{id}/provider` endpoint (backward compatible; the adapter already forwards `role`). Implemented in `okc-web/` with one focused test; documented under root initiative + a short note in okc-web.
- **No CI, no packaging, no OS-service autostart** for demo processes (foreground/`brew services` only, torn down by scripts).

## Extension Configuration (this initiative)

| Extension | Enabled | Rationale |
|---|---|---|
| Security Baseline | **Yes — applicable rules only** | Real upload/serving tokens + TLS certs + no-secrets-in-commits genuinely apply. Infra/web-platform rules N/A. |
| Resiliency Baseline | **No** | Local, ephemeral demo; not a production workload. |
| Property-Based Testing | **No** | Thin integration/orchestration + Playwright e2e; the okc-web tweak follows okc-web's existing pytest style. |

## Construction findings & adjustments (2026-09-09)

Discovered while implementing/validating; these refine the requirements without expanding scope:

- **DEMO-04 amended** — the okc-hooks watcher trusts only the bundled Mozilla `webpki-roots` (no platform verifier / CA-file config), so it cannot trust a local self-signed TLS front and `caddy trust` is moot. Per user decision, okc-hooks is left unmodified: the watcher still runs on "mine" and demonstrates **real detection** (file-watch → debounce → manifest diff → consent; its upload then fails with `UnknownIssuer`, which the demo shows honestly), and the **transfer** of the changed vault is performed by the local contributor upload API (`POST /u/{token}/upload`) on the same token/source. Verified: a re-upload yields the same `source_id` with a new `content_hash` → new revision → project `stale`. Fully local, no external tunnel.
- **DEMO-02 amended** — okc-core's per-provider-call timeout defaults to 120s; a cold 9 GB model load / large taxonomy generation exceeds it. Fix (no okc-core change): okc-web now forwards an optional `timeout_ms` (set to 600 000 in `providers.json`), and bring-up **pre-warms + pins** both models (`keep_alive 30m`). The generative model may be **qwen2.5:7b** instead of 14b for a watchable demo (config-only swap in `providers.json`), since the local 14b organizer/synthesis/critic is slow; nomic-embed-text stays for embeddings. Per-role routing verified through the real engine (preflight shows embedding→ollama-embed, others→ollama-gen, all local).
- **okc-web changes (module-scoped, in the allowed demo module)**: (1) optional `role` on the provider-bind endpoint; (2) optional `timeout_ms` on the provider spec forwarded to the engine profile. Both backward-compatible with focused tests / gate green.

## Success Criteria (verifiable)

- Ollama `test_profile` passes for **both** roles: gen (structured JSON) → `{generation:true}`, embed → `{embeddings:true}`.
- Watcher completes a real HTTPS `/api/sync` (negotiate→blob→commit) and okc-web registers the source; sources read `5/10`.
- Baseline reaches `verified` and publishes **revision 1**; after the MCP edit + watcher sync the project is **`stale`**.
- Re-merge surfaces **≥1 blocking + ≥1 minor** critic finding; regenerate clears blocking; compile publishes **revision 2**; `serving/verify` validates it.
- `demo/run-demo.sh` runs `00→50` clean on a wiped state with a screenshot captured per beat.
