# OKC Integration AI-DLC State

## Project Information

- Workspace: `/Users/sihun/workspace/projects/okc`
- Scope: repository-root umbrella integration; four independent `okc-*` modules.
- Type: Brownfield.
- Started: 2026-09-08T16:48:11Z (2026-09-09 Asia/Seoul).
- Request: verify all four modules against the user's roles; design and implement confirmed functional gaps using AI-DLC.
- Current stage: CONSTRUCTION COMPLETE — module gap implementation, independent review and aggregate verification passed on 2026-09-09 KST.
- Module requirements and histories remain authoritative within each module; this state does not replace module states.

## Stage Progress

- [x] Workspace Detection — four implementations discovered; no existing root state or reverse-engineering artifacts.
- [x] Reverse Engineering — current contracts inspected; complete requested gap review and integration artifact set recorded.
- [x] Requirements Analysis — INT-01..10 recorded from user roles and verified gaps; existing explicit product policies reconciled.
- [x] Workflow Planning — execution-plan.md; cross-module story mapping and unit/interface design recorded with links to module authority.
- [x] Code Generation — hooks, web upload, web serving and MCP continuation units completed with module-local plans and history.
- [x] Build and Test — module gates and actual integration verification passed; [final report](construction/build-and-test/build-and-test-summary.md).
- Operations/external deployment: not requested.

## Extension Configuration

| Extension | Enabled | Status |
|---|---|---|
| Security Baseline | Pending opt-in | Only opt-in prompt loaded; decision belongs to future Requirements Analysis. |
| Resiliency Baseline | Pending opt-in | Only opt-in prompt loaded; decision belongs to future Requirements Analysis. |
| Property-Based Testing | Pending opt-in | Only opt-in prompt loaded; decision belongs to future Requirements Analysis. |

All installed extension rule files have opt-in prompts. No always-on extension was found. Module opt-in decisions are not inherited as umbrella decisions. Root extension compliance is N/A without opt-in; existing module extension configuration and functional/NFR constraints were respected throughout implementation.

## Integration Tracking

- Plan: [integration-review-plan.md](inception/plans/integration-review-plan.md)
- Result: [integration-gap-review.md](inception/reverse-engineering/integration-gap-review.md)
- Initial review validation: 15 MCP tests passed and 103 root links checked. Final implementation verification: core 130 Rust +12 Python, hooks 250, MCP 76, web backend 114 +frontend 8, four-module bridge 1; all passed. AI-DLC inventory/link checks passed for non-draft references.
- Extension compliance: Security Baseline N/A; Resiliency Baseline N/A; Property-Based Testing N/A. Opt-in is pending and no full extension rules are active for this review.
- Preserve pre-existing workflow, release-documentation, and distribution-file changes.
- Execution plan: [execution-plan.md](inception/plans/execution-plan.md), all steps complete. User explicitly authorized implementation and continued design/verification.
- Verification scope: module builds/tests plus focused new cross-module regressions. Do not claim live AI-provider verification without evidence.

---

## Current Initiative (started 2026-09-09 KST): Local Config-Driven Install for okc-hooks + okc-mcp

- Scope: repository-root cross-module coordination (okc-hooks + okc-mcp), shared okc-web credential dependency. Routing rule 4 → root workspace, artifacts in `/aidlc-docs/`.
- Type: Brownfield feature. The prior root initiative above is COMPLETE and preserved; this is a distinct new initiative.
- Request (summary): a config-style file per module where the user fills in values (incl. okc-web api/token) and each module then installs locally on its own — okc-hooks as an auto-running daemon, okc-mcp registered globally into the user's coding agent (Claude/Codex), with the chosen coding agent captured in config.

### Stage Progress (Current Initiative)
- [x] Workspace Detection — root scope confirmed; brownfield; prior RE artifacts + module histories exist. Full Reverse Engineering not re-run; targeted current-state investigation of config/install/token mechanisms dispatched instead.
- [x] Requirements Analysis — clarifying questions answered on autopilot (recommended + minimal-scope) by user directive 2026-09-09; requirements at [local-install-requirements.md](inception/requirements/local-install-requirements.md).
- [x] Workflow Planning — [local-install-execution-plan.md](inception/plans/local-install-execution-plan.md). Skips: User Stories, Application Design, NFR Requirements/Design, Infrastructure Design (rationale in plan). Executes: Units (light), Functional Design (light, per unit), Code Generation, Build & Test.
- [x] Units Generation (light) — two independent units in [local-install-unit-of-work.md](inception/plans/local-install-unit-of-work.md): U1 okc-hooks setup/teardown; U2 okc-mcp setup + agent registration + `agents` config. Parallelizable.
- [x] Construction U1 (okc-hooks) — `watcher-bin setup` + teardown implemented in okc-hooks/; functional design at okc-hooks/aidlc-docs/construction/local-install/. Verified: `cargo test --workspace --features proptest-support` all green (setup.rs 8/8), clippy 0 warnings.
- [x] Construction U2 (okc-mcp) — `setup`/`unregister` + `agents` config implemented in okc-mcp/; functional design at okc-mcp/aidlc-docs/construction/local-install/. Verified: `npm run check` green, 101/101 tests.
- [x] Build & Test (root aggregate) — [local-install-build-and-test-summary.md](construction/build-and-test/local-install-build-and-test-summary.md). Both module gates green. Live OS-service registration / live agent `mcp add` / live okc-web token NOT executed (tests use injectable fakes). No commits made.
- Status: CONSTRUCTION COMPLETE for this initiative (design + implement + verify). Operations/deployment not requested.
- [x] Follow-up (user request 2026-09-09): one-command root installer added — `install.sh` (macOS/Linux), `install.ps1` (Windows), `uninstall.sh`, `okc-install.config.example.json`, and `scripts/okc-install-render.mjs`. Fills a single combined config → builds both modules → runs each `setup`. Verified: bash syntax OK; renderer emits valid per-module JSON at 0600 with correct `/api/sync` join; generated mcp config passes real schema via read-only `doctor` (ok:true). Side-effecting registration/service install intentionally not run during verification.

### Extension Configuration (Current Initiative)
| Extension | Enabled | Decided At |
|---|---|---|
| Security Baseline | Yes (applicable rules only; infra/web rules N/A) | Requirements Analysis |
| Resiliency Baseline | No | Requirements Analysis |
| Property-Based Testing | Partial (config parse/serialize round-trips) | Requirements Analysis |

- Autopilot directive: take recommended options, deprioritize scope-expanding choices, proceed without per-gate blocking. Concise checkpoints surfaced instead of hard approval gates.
- Security applicable rules for this initiative: SECURITY-03, 05, 06, 09, 10, 12, 13, 15 (+ SECURITY-11 credential separation-of-concerns). N/A: SECURITY-01 (no data store; token at 0600, full secure-store deferred), 02, 04, 07, 08, 14.

---

## Current Initiative (started 2026-09-09 KST): Full-Flow Local Demo (hooks → web → mcp, all-local Ollama)

- Scope: repository-root umbrella coordination (okc-web + okc-hooks + okc-mcp + okc-core) + new root-owned `demo/` tooling. Routing rule 4 → root workspace, artifacts in `/aidlc-docs/`. Prior two root initiatives remain COMPLETE and preserved.
- Type: Brownfield feature (demo/integration harness over already-built modules).
- Request (summary): a one-command, reproducible local demo riding the entire pipeline — 5 personal vaults uploaded, a live MCP edit to "my" vault, hooks auto-upload, web shows it changed (stale), admin re-merges + disposes of critic findings — Playwright-driven, web authenticated on this laptop.
- Autopilot directive (user, 2026-09-09): "권장안으로 선택하고 대부분 진행하되 구현/설계 범위가 너무 커지는 방향은 피해. autopilot으로 자동으로 진행." → recommended options, avoid scope expansion, no hard per-gate blocking, concise checkpoints.

### Key decisions
- LLM brain = all-local Ollama `qwen2.5:14b` (organizer/synthesis/critic) + `nomic-embed-text` (embedding). Verified: Apple M5 / 32 GB / Metal. No Bedrock/shim.
- MCP edit = scripted stdio `apply_session_capture(dryRun:false)` (+ real `claude mcp add`).
- "conflict/changed" = product's real behavior (project-level `stale` banner; ClusterReview preserved-contradictions + minor-waive/regenerate). Dummy vaults carry deliberate contradictions.
- Delivery = fully automated + reproducible; Playwright drives the admin web flow with per-beat screenshots.
- Scope guards: no new web UI, no per-file diff view, okc-web limited to an optional `role` on the existing provider-bind endpoint, no CI/packaging/OS-autostart for demo processes.

### Stage Progress (Current Initiative)
- [x] Workspace Detection — root/umbrella scope; brownfield; new initiative. Targeted current-state investigation (hooks daemon, mcp tools, web review/serving) instead of full Reverse Engineering.
- [x] Requirements Analysis — [demo-full-flow-requirements.md](inception/requirements/demo-full-flow-requirements.md).
- [x] Workflow Planning — [demo-full-flow-execution-plan.md](inception/plans/demo-full-flow-execution-plan.md). Skips: Reverse Engineering, User Stories, Application Design, NFR/Infra Design. Executes: Units (light), Functional Design (light, where needed), Code Generation, Build & Test.
- [x] Units Generation (light) — [demo-full-flow-unit-of-work.md](inception/plans/demo-full-flow-unit-of-work.md): U1 dummy vaults; U2 provider stack + okc-web role routing; U3 env bring-up + TLS; U4 wiring + seed; U5 Playwright + runbook.
- [x] Construction — DONE. Deliverables under top-level `demo/` (5 curated vaults, `scripts/` for preflight/up/seed/mcp-edit/sync/down/reset, `config/` providers+Caddyfile, `curation_provider.py`, `e2e/` Playwright, `run-demo.sh`, `README.md`). okc-web enhancements (per-role `role`, provider `timeout_ms`) with tests. Step-5 generation pivoted to a deterministic curation provider (fully local; real LLM cannot reliably satisfy okc-core's strict synthesis/critic contracts — okc's own tests use a deterministic fixture).
- [x] Build & Test — DONE. `run-demo.sh --fresh` PASSES end-to-end (~2 min): baseline Playwright → revision 1, MCP edit → real watcher detection → upload → STALE, re-merge Playwright → revision 2, publication history = 2, 5 preserved contradictions, 11 screenshots. okc-web gate: ruff + mypy clean, orchestration tests 9/9. Summary: [demo-full-flow-build-and-test-summary.md](construction/build-and-test/demo-full-flow-build-and-test-summary.md).
- Status: CONSTRUCTION COMPLETE for this initiative. Operations/deployment not requested. No commits made.

### Extension Configuration (Current Initiative)
| Extension | Enabled | Decided At |
|---|---|---|
| Security Baseline | Yes (applicable rules only — token/secret hygiene, TLS, no secrets in logs/commits/state) | Requirements Analysis |
| Resiliency Baseline | No | Requirements Analysis |
| Property-Based Testing | No (thin integration tooling + Playwright e2e; okc-web tweak uses existing pytest style) | Requirements Analysis |
