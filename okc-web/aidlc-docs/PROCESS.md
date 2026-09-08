# PROCESS — How okc-web was built (AI-DLC narrative)

This project was produced end-to-end with the **AI-DLC** workflow. Every stage's output feeds the next, and every decision is logged. This narrative is the map; the durable evidence is in `aidlc-docs/` (`audit.md` = full timestamped trail, `aidlc-state.md` = authoritative state).

## The chain (each stage consumed the previous one's output)
1. **Requirements** (`inception/requirements/`) — from the 5 original asks (①–⑤: permission-gated cross-dept vault integration, admin-only, conflict handling, upload tokens, okc-mcp serving). Includes an okc-core capability analysis and decision records (ADRs), incl. **ADR-0025** (stack pivot to FastAPI + React) and **ADR-0002** (single adapter seam; okc-core unmodified).
2. **User Stories** (`inception/user-stories/`) — 29 stories / 5 epics with personas; the "no winner-select" reinterpretation of ask ③ is fixed here (okc-core has no winner API; contradictions are preserved).
3. **Application Design** (`inception/application-design/`) — components, services, dependencies, the 24-screen UI map, and the design system. 9 okc-core APIs code-verified against the binding.
4. **Units** (`inception/application-design/unit-of-work*.md`) — decomposition into U0–U6 with a dependency DAG and story map.
5. **Construction** (`construction/`) — a **balanced-wave** schedule (`parallel-execution-plan.md`): W0 foundation → W1 U1‖U2 → W2 U3 → W3 U4‖U5 → W4 U6 → W5 build&test. Per-unit: (selective) functional design → code generation, each behind a standardized 2-option gate.

## How the constraints propagated (traceability example)
okc-core hard constraints (C-1 no auth, C-2 no winner-select, C-3 ≤10 sources, C-4 freeze-then-run, C-6 deterministic offline compile) were captured in requirements → echoed in every story's AC → encoded in application design (S0.C RBAC-before-core, S0.E record-then-act) → realized in code: `shared/authz.py` (RBAC before any engine call), `shared/audit.py` (`CuratorDecision` = exactly 3 variants, no `SelectWinner` — enforced by type AND a DB CHECK), `app/adapter/` (the single okc seam), and surfaced honestly in the U6 UI.

## Execution mode
Run under an authorized **autopilot** with a **lean-MVP** directive: recommended options auto-selected at each per-unit gate (all logged), MVP-approved stories only, no speculative abstraction. Parallel waves (W1 U1‖U2, W3 U4‖U5) ran as real concurrent sub-agents on disjoint module dirs; the orchestrator independently re-verified every wave barrier (never trusting an agent's self-report) and kept `main.py`/U0 frozen so units registered via discovery.

## Verification discipline
No engine mock anywhere — tests run against the real `okc` binding through the production `create_app`. Each wave barrier re-ran the full gate: final state is **backend ruff/mypy clean (49 files) + 80 pytest passed**, **frontend tsc + vite build + 8 vitest passed**, secret scan clean.

## Honest limits (surfaced, not hidden)
- The AI-driven runtime spine (integrate→…→compile) needs a live LLM provider not present in the build env; all non-AI seams are proven and the engine returns real typed errors offline.
- Automated screenshots need a browser (absent here) + (for the full flow) a provider — see `screenshots/README.md`. Not faked.

## Where to look
- Decisions & timeline: `aidlc-docs/audit.md`. State: `aidlc-docs/aidlc-state.md`.
- Per-unit design + code summaries: `aidlc-docs/construction/{unit}/`.
- Wave plan: `aidlc-docs/construction/plans/parallel-execution-plan.md`. Build/test: `aidlc-docs/construction/build-and-test/`.
