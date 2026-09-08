# U3 Orchestration — Functional Design Plan

**Wave**: W2 · **Unit**: U3 (`app/orchestration`) · **Epic**: E3 · **Stories**: E3-S1..E3-S7  
**Depth**: Comprehensive, MVP-only  
**Mode**: AUTOPILOT — earlier user authorization selects the recommended answer and continuation gate; every decision remains visible below.  
**Single source of truth**: This plan governs U3 Functional Design only. It does not authorize U4 review logic, U5 serving, frontend work, new infrastructure, or changes to okc-core.

## Context and boundaries

- Inputs: approved requirements, E3 stories, U3 unit definition/story map, application design, W0 adapter contracts, W1 real source landing.
- Dependencies: U0 `EngineWorker`, adapter DTOs, error/job/state mechanisms; U1 admin principal; U2 `projects`/`sources` rows and real `add_source` output.
- Owned behavior: project registry, logical source freeze, checkpoint projection, provider binding, disclosure gate, preflight/integration dispatch, compile dispatch, and derived staleness.
- Explicit non-ownership: taxonomy/cluster review and `DecisionGate` (U4), serving/publication (U5), frontend (U6), binding types (U0), and any okc-core change.

## Execution checklist

- [x] Step 1 — Load U3 unit definition, dependencies, E3-S1..E3-S7 acceptance criteria, prior application design, and current U0/U1/U2 contracts.
- [x] Step 2 — Inspect the actual `okc-compiler` 0.3.0 binding surface and core checkpoint derivation; verify the seven snake-case checkpoint values and `status()` payload shape.
- [x] Step 3 — Resolve functional ambiguities using the standing recommended-decision authorization; record all choices in this plan.
- [x] Step 4 — Generate `business-logic-model.md`: lifecycle flow, logical freeze, checkpoint/next-action table, provider/preflight/integration flow, compile handoff, and job concurrency gate.
- [x] Step 5 — Generate `business-rules.md`: preconditions, validation, transitions, stable error codes, RBAC-before-core, staleness precedence, and edge cases.
- [x] Step 6 — Generate `domain-entities.md`: U3 entities/value objects, ownership, invariants, persistence mapping, and API contract DTOs.
- [x] Step 7 — Validate Markdown structure and cross-artifact consistency; confirm no diagrams require Mermaid/ASCII validation and no disabled extension rule was loaded. (Two ASCII flow/state diagrams carry text alternatives per ascii-diagram-standards; no Mermaid. Checkpoint values, Q1–Q7 decisions, and E3-S1..S7 mapping are consistent across all three artifacts. Disabled extensions → N/A, no rule file loaded.)
- [x] Step 8 — Record the standardized two-option completion gate, auto-select `Continue to Next Stage`, update `aidlc-state.md`, and hand off to U3 Code Generation planning.

## Functional design decisions

### Question 1
What does source “freeze” mean for this MVP when a contributor later uploads another valid source?

A) Treat freeze as a reproducible snapshot gate: later source changes remain allowed, make the snapshot stale/unfrozen on the next U3 read, and require re-freeze before another run. This matches okc-core hash-bound invalidation and avoids adding a cross-unit write dependency. (Recommended)

B) Permanently reject every later upload until an explicit unfreeze API is added.

C) Allow later uploads without invalidating the frozen snapshot.

D) Other (please describe after [Answer]: tag below)

[Answer]: A — AUTOPILOT recommended decision

### Question 2
How should U3 expose preflight and long-operation results given the frozen U0 `JobStore` stores progress/errors but no arbitrary result payload?

A) Return preflight directly after awaiting it on the engine worker; return `JobId` for integrate/compile, with compile acceptance also carrying its deterministic output path. Read post-integration truth through `status()`. (Recommended)

B) Add a new generic result JSON column to U0 jobs.

C) Store preflight and compile results only in process memory.

D) Other (please describe after [Answer]: tag below)

[Answer]: A — AUTOPILOT recommended decision

### Question 3
How should project and compiled-output paths be selected?

A) Default to server-generated absolute paths under `AppConfig.projects_root`; permit an explicit admin path only when it resolves inside that root. Compile defaults to a fresh per-run child and never overwrites an existing path. (Recommended)

B) Accept any admin-provided absolute filesystem path.

C) Use one fixed global project and compile directory.

D) Other (please describe after [Answer]: tag below)

[Answer]: A — AUTOPILOT recommended decision

### Question 4
How much provider-route customization belongs in the MVP?

A) List only immutable profiles already provisioned in `AppConfig`; bind one selected profile as the default route for all four AI roles. Do not add per-role route editing. (Recommended)

B) Expose separate provider selection for embedding, organizer, synthesis, and critic.

C) Accept raw provider endpoints and secret values from the browser.

D) Other (please describe after [Answer]: tag below)

[Answer]: A — AUTOPILOT recommended decision

### Question 5
How should remote disclosure be enforced?

A) Use the real preflight `routes[].boundary` as truth. If any route is remote, require both disclosure booleans before enqueueing integrate; local-only routes force both booleans false. Core remains the authoritative backstop. (Recommended)

B) Infer remote/local only from provider kind names.

C) Let the core reject missing consent after the job starts without a web pre-check.

D) Other (please describe after [Answer]: tag below)

[Answer]: A — AUTOPILOT recommended decision

### Question 6
How should U3 project staleness without duplicating core authority?

A) Compare a versioned, canonical digest of current registered sources with the digest captured at freeze, then combine that advisory result with the current core checkpoint and existing approval audit history. Any core `APPROVAL_STALE` or checkpoint regression wins. (Recommended)

B) Treat the web digest as authoritative and ignore core status.

C) Remove proactive staleness and show only errors returned during compile.

D) Other (please describe after [Answer]: tag below)

[Answer]: A — AUTOPILOT recommended decision

### Question 7
What should happen when another reserving project job is already queued or running?

A) Reject a duplicate U3 mutation before enqueue with retryable `PROJECT_BUSY`; keep U0's single worker as the serialization backstop. Status/progress continues through `JobStore`, not a second core status call. (Recommended)

B) Queue every duplicate run silently.

C) Start a second process to execute it concurrently.

D) Other (please describe after [Answer]: tag below)

[Answer]: A — AUTOPILOT recommended decision

## Traceability target

| Story | Functional design coverage |
|---|---|
| E3-S1 | Project creation, curator label binding, path and registry consistency |
| E3-S2 | Source cap reconciliation, canonical freeze snapshot, re-freeze rule |
| E3-S3 | Seven-state checkpoint projection, next action, single active mutation |
| E3-S4 | Configured-provider allowlist, direct preflight, remote disclosure gate |
| E3-S5 | Advisory source digest plus authoritative core regression/stale signal |
| E3-S6 | Ready-to-compile precondition, unique output path, no-clobber, U5 handoff |
| E3-S7 | Pollable jobs, stable error codes/categories, retryable project-busy response |

## Extension configuration

Security Baseline, Resiliency Baseline, and Property-Based Testing are disabled in `aidlc-state.md`. Their full rule files remain unloaded; compliance is N/A for this stage. Baseline input validation and real-binding tests remain ordinary project requirements.
