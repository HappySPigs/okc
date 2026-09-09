# Unit of Work Requirement Map

User Stories was skipped because the continuation changes documentation only.
This map assigns the existing session-grounded FRs and normative REQ areas to
the single unit and its modules.

| Intent/requirement group | Unit modules | Acceptance evidence |
|---|---|---|
| FR-1–4; REQ-SNP/SRC/PAR/DED/CNF | Compiler, Application | corpus builder contracts, parser/dedup units, adversarial properties |
| FR-5–7; REQ-AI/INT/MAT | AI, Application, Compiler | provider schema tests, app integration flow, core closure tests |
| FR-8–11; REQ-APP/SEC | Operator, Application | CLI contract, TUI reducer/PTY, provider/credential/project tests |
| FR-12; REQ-REL | Operator, build/release assets | updater tests, cargo-dist config, release gates |
| FR-13; REQ-SDK-002 | Interop, Python, Node | scheduler/error tests, public API/E2E/type/package tests |
| FR-14; REQ-CMP-003 | Compiler, Application, Interop, all surfaces | current-only API tests and temporary retired-marker failures |
| FR-15; REQ-CMP/MAT/PRV | Compiler, Application | compile/verify/explain tests and shared artifact golden |
| FR-16; QG-007 | Guide/docs/all public surfaces | Markdown-link test, VitePress build, CLI help tests |
| FR-17 | Demo/test tooling | demo inventory and end-to-end workflow fixtures |
| NFR-1–9; QG-001–008 | Whole unit | `CURRENT_STATE`, `TRACEABILITY`, and build/test summary |

## Coverage statement

All current product requirements map to UOW-OKC-001. Future-only REQ-MCP-001
and REQ-OBS-001 map to no implemented module. REQ-CMP-002 and REQ-PERF-001
remain open work inside the product boundary. REQ-MEM-001 remains isolated
research/experimental scope.
