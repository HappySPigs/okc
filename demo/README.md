# OKC Full-Flow Demo

A one-command, fully local, reproducible demo that rides the **entire** OKC pipeline on this laptop:

> 5 personal vaults on the web → I edit my vault through **okc-mcp** in a session → the **okc-hooks** watcher detects it → the web shows it **changed (stale)** → the admin **re-merges** (preserving contradictions) → a new compiled "brain" **revision** is published for **okc-mcp** to read.

Steps 1–4 (upload, MCP authoring, hook detection, stale) are **real**. The AI merge (step 5) runs through a **deterministic curation provider** — see [Why a deterministic provider](#why-a-deterministic-provider) — so integrate → review → compile → publish completes reliably and reproducibly. The admin walkthrough is driven by **Playwright** with a screenshot per beat.

Verified: `run-demo.sh --fresh` completes in ~2 minutes and publishes **two revisions** (baseline + after the edit).

---

## What each scenario step maps to (honest)

| Scenario | What actually happens | Surface |
|---|---|---|
| 1. 5 vaults uploaded | 4 dept vaults + "mine" uploaded via the real contributor upload API (zip) | Sources `5/10` |
| 2. I update "mine" via MCP | scripted stdio `apply_session_capture(dryRun:false)` on the real okc-mcp server writes a note into `demo/vaults/mine/` | file on disk |
| 3. hook detects → uploads | the real `watcher-bin` **detects** the change (file-watch → debounce → manifest diff → consent — see its log). Its own HTTPS upload is untrusted here (see *TLS note*), so the **transfer** is done by the upload API on the same token/source | watcher log + new source revision |
| 4. web shows the change | the project goes **`stale`** ("소스가 변경되어 이전 승인이 무효화되었습니다") — a project-level signal, **not** a per-file diff | AppShell / Sources banner |
| 5. admin merges + resolves conflicts | re-freeze → re-integrate → approve taxonomy → cluster review with **preserved contradictions ("승자 없음")** → compile → publish **revision 2** | ClusterReview workbench |

**Honesty notes (okc design, surfaced not hidden):**
- "Conflict resolution" is **not** a git-style pick-a-side. Contradictions are preserved; the admin disposes of critic findings (minor→waive, major/critical→regenerate-only) until the compile gate clears.
- "Changed" is a project-level `stale` flag; there is no per-file before/after diff screen.
- Steps 1–4 use the real okc-web/okc-hooks/okc-mcp code paths. Step 5's merge uses a deterministic provider (below).

## Why a deterministic provider

okc-core's synthesis/critic contracts are strict: the provider must echo the engine's exact `content_hash`/`block_id`/`document_id` identifiers and never, e.g., put a metadata id in a block field. okc-core's own tests compile against a **deterministic fixture provider** (its README calls the provider tests "not live-model quality evidence"), not a live LLM. A real model (local qwen, or Claude via a Bedrock shim) doesn't reliably satisfy these contracts, so the merge fails to decode.

`demo/scripts/curation_provider.py` is a small, **input-aware** deterministic provider that speaks the Ollama wire (`/api/embed`, `/api/chat`). It echoes the engine's identifiers exactly (so decode passes), groups the near-duplicate note pairs into topic clusters, and **preserves the cross-team contradictions** ("승자 없음"). It's honestly synthetic, but it drives the whole real pipeline — integrate → taxonomy → synthesis → critic → compile → serve — reproducibly, with no external model. (Wiring a real Anthropic/OpenAI key via okc-core's native provider is possible but not reliable against the strict contracts; that's a known okc-core limitation, not a demo bug.)

## The 5 dummy vaults + designed contradictions

Fictional company **너울(Neoul)**, `demo/vaults/{eng,product,sales,support,mine}/`. Each contradiction is a **near-duplicate pair** (same `title`, ~identical body, one differing fact) so the pair clusters together and the disagreement surfaces (see `vaults/_contradiction-map.md`):

| Topic (shared title) | Vault A | Vault B |
|---|---|---|
| 환불 정책 | sales: 30일 | support: 14일 |
| 배포 절차 | eng: 자동(무승인) | product: PM 승인 필수 |
| 온콜 로테이션 | eng: 월요일 | support: 목요일 |
| Pro 요금제 | sales: $49 | product: $39 |
| 로그 보관 정책 | eng: 30일 삭제 | support: 90일 보관 |

`mine/` has 2 neutral notes; the MCP edit in step 2 adds a note that changes the source set so the re-merge produces **revision 2**.

## Prerequisites

- macOS/Linux, `python3`, Node 20+, `jq`, `zip`, `curl`, `lsof`, **Caddy** (`brew install caddy`).
- Sibling checkouts built: `okc-web` (venv + `okc_compiler` wheel + `frontend/dist`), `okc-hooks` (`watcher-bin`), `okc-mcp` (`dist/cli.js`).
- Playwright browser: `cd demo/e2e && npm install && npx playwright install chromium`.
- **No Ollama/Bedrock/API keys needed** — the merge uses the local deterministic provider.

## Run it

```bash
cd demo
./run-demo.sh --fresh                       # wipe state, run the whole thing (~2 min)
OKC_DEMO_HEADED=1 ./run-demo.sh --fresh      # watch the browser live
```

Phases: `00 preflight → 10 up → 20 seed → baseline merge (Playwright, rev1) → 40 MCP edit → 50 detect+upload (stale) → re-merge (Playwright, rev2)`.

Individual steps (`demo/scripts/`, all source `lib.sh`):

```bash
scripts/00_preflight.sh   # verify tools + built modules + SPA build + curation provider
scripts/10_up.sh          # curation provider (:8797) + fresh okc-web (:8000, isolated state) + Caddy (:8443)
scripts/20_seed.sh        # create project, bind curation provider, upload 5 vaults, start watcher, register mcp
node scripts/40_mcp_edit.mjs   # okc-mcp writes a new note into "mine"
scripts/50_sync.sh        # watcher detects; upload the change; assert STALE
scripts/down.sh           # stop services (keep state)
scripts/reset.sh          # stop + wipe demo/state
```

Screenshots land in `state/screenshots/` (`01-overview … 10-served-published`, plus `01-stale-banner` on the re-merge). Admin console: <http://localhost:8000> (`admin@okc.demo` / `okc-demo-1234`, local bootstrap only).

## Architecture

```
 Playwright (admin) ─► okc-web :8000 (fresh state) ─ okc binding ─► okc-core engine
                            │  provider: curation (:8797, deterministic, all roles)
   scripted stdio ─► okc-mcp ─writes─► demo/vaults/mine ──watched──► watcher-bin (detection)
                            ▲                                  changed vault ─upload API─► :8000
   (okc-mcp also reads the published "brain" from :8000/api/serving/{project})
```

## TLS note (why the transfer uses the upload API)

`watcher-bin` (ureq + rustls) trusts only the bundled Mozilla roots, so it cannot trust a local self-signed cert, and `caddy trust` is moot. Rather than modify okc-hooks, this demo keeps the watcher for **real detection** and performs the **transfer** via the contributor upload API on the same token/source — fully local, no external tunnel. (In production the watcher uploads directly to a publicly-trusted okc-web over HTTPS.)

## okc-web enhancements used (module-scoped, backward-compatible, tested)

- optional `role` on `POST /api/projects/{id}/provider` (per-role AI routing)
- optional `timeout_ms` on the provider spec (forwarded to the engine profile)

## Notes

- `demo/state/` (DB, tokens, cookies, logs, screenshots) is git-ignored and wiped by `reset.sh`.
- The generated `mine/세션-메모-*.md` note (from the MCP edit) is recreated each run; `reset.sh` starts clean.
