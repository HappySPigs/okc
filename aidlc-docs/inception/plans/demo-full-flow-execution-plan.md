# Workflow Plan — Full-Flow Local Demo

> Umbrella initiative. Requirements: [demo-full-flow-requirements.md](../requirements/demo-full-flow-requirements.md). Units: [demo-full-flow-unit-of-work.md](demo-full-flow-unit-of-work.md).

## Stage decisions (adaptive)

| Stage | Decision | Rationale |
|---|---|---|
| Reverse Engineering | **Skip** | Brownfield RE artifacts + module histories exist; targeted current-state investigation done this session (hooks daemon, mcp tools, web review/serving). |
| Requirements Analysis | **Done** | See requirements doc. Autopilot: recommended options, scope guards applied. |
| User Stories | **Skip** | Demo/integration tooling; no new product-facing user story. Existing okc-web UX unchanged. |
| Workflow Planning | **This doc** | — |
| Application Design | **Skip** | No new components/services; reuse existing screens/APIs. Only an optional `role` param on an existing endpoint. |
| Units Generation | **Light** | 5 small, mostly-parallel units (below). |
| Functional Design (per unit) | **Light, only where needed** | U1 contradiction map; U2 okc-web `role` contract; U5 Playwright beat map. Others are scripting. |
| NFR / Infrastructure Design | **Skip** | Local-only demo; TLS proxy is trivial documented config; security applicable-rules folded into requirements. |
| Code Generation (per unit) | **Execute** | All units. |
| Build & Test | **Execute** | End-to-end `run-demo.sh` dry run on wiped state + success criteria; okc-web change unit test. |

## Unit sequence / dependencies

```
Phase A (parallel):   U1 (dummy vaults)      U2 (provider stack + okc-web role routing)
                            \                    /
Phase B:                     \                  v
                              +----------> U3 (env bring-up + TLS + isolation)
Phase C:                                      |
                                              v
                                     U4 (wiring + seed: tokens, 4 dept seed,
                                         hooks watcher on "mine", mcp register + stdio edit)
Phase D:                                      |
                                              v
                                     U5 (Playwright admin walkthrough: rev1 baseline
                                         + rev2 post-change; run-demo.sh; runbook)
Phase E:                                      |
                                              v
                                     Build & Test (end-to-end on wiped state)
```

Text form: U1 and U2 have no dependencies and run first (in parallel). U3 depends on U2 (needs the okc-web provider env + role routing). U4 depends on U3 (web up) and U1 (vault content). U5 depends on U2 (provider to actually integrate) and U4 (sources seeded). Build & Test runs last, end to end.

## Governance note

Root owns the demo harness (`demo/`) and this coordination plan. The single okc-web change (optional `role` on the provider-bind endpoint) is module-scoped: implemented in `okc-web/backend/` with a focused test, kept minimal per the scope guard, and linked from this initiative rather than spinning up a full module AI-DLC cycle for a ~10-line backward-compatible change.

## Model-download long pole

`ollama pull qwen2.5:14b` (~9 GB) is started at the beginning of construction and proceeds in the background while U1/U2 docs + code are produced.
