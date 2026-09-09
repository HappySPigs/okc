# Units of Work — Full-Flow Local Demo

> Light decomposition. All demo tooling is root-owned under `demo/` unless noted. One module-scoped change lives in `okc-web/`.

## U1 — Dummy vaults + contradiction map
- **Deliver**: `demo/vaults/{eng,product,sales,support,mine}/` — 5 small Obsidian vaults, Korean `.md` notes with YAML frontmatter, on shared topics (예: 환불 정책, 배포 절차, 온콜 순번, 가격) with **deliberate cross-vault contradictions** (e.g., 환불 기간 30일 vs 14일) → guarantees critic findings.
- **Deliver**: a short contradiction map (in `demo/README.md`) listing which notes disagree and the expected severities.
- **Depends on**: nothing.
- **Functional design (light)**: the contradiction/topic matrix.

## U2 — Provider stack + okc-web per-role routing
- **Deliver**: preflight that installs/starts Ollama and pulls `qwen2.5:14b` + `nomic-embed-text` (started early, background).
- **Deliver**: `demo/config/providers.json` (or `OKC_WEB_PROVIDERS`) with two Ollama profiles: `ollama-gen` (qwen2.5:14b) and `ollama-embed` (nomic-embed-text).
- **Deliver (module change)**: `okc-web` — optional `role` on `POST /api/projects/{id}/provider` (service `bind_provider(project_id, profile_name, role=None)` passes `role` to the adapter, which already supports it; router accepts optional body field). One focused pytest. Backward compatible; UI unchanged.
- **Deliver**: a `bind` helper the setup calls twice: default→`ollama-gen`, then `embedding`→`ollama-embed`.
- **Depends on**: nothing (parallel with U1).
- **Functional design (light)**: the `role` request/response contract + provider `test_profile` verification for both roles.

## U3 — Environment bring-up + TLS + isolation
- **Deliver**: `demo/scripts/10_up.sh` — stop the leftover dev instance (uvicorn + shim); start Ollama (if not running); start fresh okc-web with isolated `OKC_WEB_STATE_DB`/`OKC_WEB_PROJECTS_ROOT` under `demo/state/`, bootstrap admin, `OKC_WEB_PROVIDERS`; health-wait `:8000`.
- **Deliver**: Caddy TLS proxy config (`demo/config/Caddyfile`) → `https://localhost:8443` reverse-proxy to `:8000`; `caddy trust` for the local CA; health-wait.
- **Deliver**: `demo/scripts/down.sh` + `reset.sh` (wipe `demo/state/`, stop demo services).
- **Depends on**: U2 (provider env).

## U4 — Wiring + seeding
- **Deliver**: `demo/scripts/20_seed.sh` — admin login; issue 5 upload tokens; seed the 4 department vaults via the most robust scripted path (upload API or a headless one-shot watcher — decided in this unit's light functional design after inspecting the U2 upload contract); write the okc-hooks watcher config for "mine" pointing at the HTTPS proxy; run `watcher-bin` (startup sync seeds "mine"); `consent acknowledge && grant`.
- **Deliver**: okc-mcp registration (`claude mcp add`) + `demo/scripts/40_mcp_edit.mjs` scripted stdio client that calls `apply_session_capture(dryRun:false)`.
- **Depends on**: U3 (web up), U1 (vaults).
- **Functional design (light)**: choose + document the 4-vault seeding mechanism; the stdio JSON-RPC request shape.

## U5 — Playwright admin walkthrough + runbook
- **Deliver**: `demo/e2e/` Playwright config + `demo.spec.ts` driving the admin flow via `data-testid`: login → sources `5/10` → freeze → integration (bind providers, run) → taxonomy approve → ClusterReview (show preserved contradictions, minor-waive w/ rationale, regenerate a blocking finding) → compile → serving publish/verify. **Baseline pass = revision 1**; after the U4 MCP edit + watcher sync, a **second pass = revision 2**; screenshot per beat.
- **Deliver**: `demo/scripts/30_baseline.sh` + `50_sync.sh` (sync-now; assert `stale`) + `demo/run-demo.sh` orchestrating `00→50`.
- **Deliver**: `demo/README.md` runbook — what each beat proves, the honest caveats, prerequisites, teardown.
- **Depends on**: U2 (provider), U4 (sources seeded).
- **Functional design (light)**: the per-beat selector/assertion + screenshot map.

## Build & Test
- End-to-end `run-demo.sh` on a wiped state; assert the requirements' success criteria; run the okc-web `role` pytest. Root aggregate summary in `construction/build-and-test/`.
