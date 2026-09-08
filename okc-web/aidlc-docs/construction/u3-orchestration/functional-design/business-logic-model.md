# U3 Orchestration — Business Logic Model

**Wave**: W2 · **Unit**: U3 (`app/orchestration`) · **Epic**: E3 (E3-S1..E3-S7) · **Depth**: Comprehensive, MVP-only
**Decisions**: Q1–Q7 in [`../../plans/u3-orchestration-functional-design-plan.md`](../../plans/u3-orchestration-functional-design-plan.md) (all recommended, AUTOPILOT).

This document is technology-agnostic business logic. It is grounded in the frozen
W0/W1 seams (`app/adapter`, `app/shared`, `app/upload`) and the real
`okc-compiler` 0.3.0 binding, but names them only to fix contracts — no code here.

---

## 1. What U3 is (and is not)

U3 is the **integration orchestration shell**. It owns the okc-web *project
registry*, the *logical freeze* record, *provider binding*, the *remote-disclosure
gate*, and the *dispatch* of preflight / integrate / compile onto the U0 single
writer. It **projects** the multi-step human-in-the-loop loop to the admin and
serializes mutating work.

U3 does **not** own the integration state machine. The authoritative state is
**derived by okc-core** from persisted project state and read back through
`status().checkpoint`. U3 never reconstructs or second-guesses that value; it maps
the current checkpoint to "the one action needed now" and dispatches it.

**Non-ownership (hard boundary):** taxonomy/cluster review and the compile-rejection
`DecisionGate` (U4), serving/publication (U5), the React shell (U6), binding/DTO
types and the worker/JobStore (U0), the `sources` write side (U2), and any
okc-core change.

---

## 2. Core is the state machine (authoritative derivation)

`Project.status()` returns `ProjectStatus { interop_schema_version, checkpoint,
integration }`. `checkpoint` is one of **seven snake_case values**. okc-core derives
it in this **precedence order** (verified in `okc-app::IntegrationService::checkpoint`):

1. no sources registered → `needs_sources`
2. a latest verified compiled output exists and re-verifies → `verified`
3. a latest *approved* integration plan exists → `ready_to_compile`
4. the required provider profiles are not all present → `needs_provider`
5. no integration run yet → `needs_disclosure`
6. preflight OR organizer task for the run is not complete → `needs_disclosure`
7. no approved taxonomy (or the taxonomy hash no longer matches) → `needs_taxonomy`
8. any cluster in the approved taxonomy lacks a current approval → `needs_clusters`
9. otherwise → `ready_to_compile` (rule 3)

Two consequences U3 must respect:

- **Derivation precedence ≠ human progression order.** The admin-facing progression
  is `needs_provider → needs_sources → needs_disclosure → needs_taxonomy →
  needs_clusters → ready_to_compile → verified` (E3-S3), but the *derivation* checks
  sources first and short-circuits on `verified`/`ready_to_compile`. U3 renders the
  human progression for context but always trusts `status().checkpoint` for the
  active step.
- **Hash-bound regression is automatic.** Because rules 7–8 compare stored hashes,
  changing an upstream input silently moves the derived checkpoint *backward* on the
  next read. U3 does not mutate any approval to invalidate it — core already does,
  and U3 surfaces it (E3-S5, §8).

---

## 3. Project lifecycle flow

Text description of the end-to-end flow (the numbered arrows are U3-driven steps;
bracketed owners show where another unit resolves a checkpoint):

```
  admin                     U3 (app/orchestration)                 okc-core (via adapter)
   |                              |                                      |
 1 |-- create project ---------->| create_project(spec, curator_id) --->| project on disk
   |                              |  persist projects row (unfrozen)     |
 2 |   (contributors upload) .....|.... [U2 add_source -> sources rows] .| manifest.sources++
 3 |-- freeze source set ------->| snapshot fingerprint, freeze_state=frozen
 4 |-- run integration --------->| read status().checkpoint  ───────────┐
   |                              |  dispatch by checkpoint:              |
   |   needs_provider     ------->|   set_ai_route(profile) ------------->| provider bound
   |   needs_disclosure   ------->|   preflight() + disclosure gate ----->| routes[].boundary
   |                              |   integrate(disclosure) [JobId] ----->| run + tasks
   |   needs_taxonomy ...........|.... [U4 approve_taxonomy] ............| taxonomy approved
   |   needs_clusters ...........|.... [U4 approve/regenerate cluster] ..| clusters approved
 5 |-- compile ---------------->|   compile(output_path) -------------->| merged vault (no-clobber)
   |   verified ................|.... [U5 serve/publish] ...............| live RAG source
```

**Text alternative:** (1) Admin creates a project; U3 calls the engine
`create_project` binding the authenticated admin as `curator_id`, and persists a
`projects` row in `unfrozen` state. (2) Contributors register sources through U2's
upload capability (U3 does not upload; it reads the resulting `sources` rows).
(3) Admin freezes the source set; U3 records a canonical fingerprint and flips
`freeze_state` to `frozen`. (4) Admin runs integration; U3 reads the derived
checkpoint and dispatches exactly the action that checkpoint requires — bind a
provider, run preflight and confirm disclosure, then enqueue `integrate`. Taxonomy
and cluster checkpoints are resolved by U4. (5) At `ready_to_compile` (and only
after U4's compile-rejection gate passes) U3 compiles to a fresh output path with
no-clobber; the resulting `verified` state is handed to U5 for serving.

---

## 4. Logical freeze (Q1 = A)

Freeze is a **reproducible-snapshot gate**, not a hard lock on further uploads.

- **Freeze** captures a canonical **source-set fingerprint** (§ domain-entities) over
  the project's current `sources` rows and sets `freeze_state = frozen`,
  `frozen_at = now`, `source_set_fingerprint = <digest>`.
- A later valid upload is **still allowed** (U2 keeps writing `sources` rows up to
  the cap). It does not retroactively rewrite the frozen snapshot.
- On the next U3 read, if the live fingerprint ≠ the frozen fingerprint, the project
  is reported **stale/unfrozen** and a **re-freeze is required** before another run
  or compile. This mirrors okc-core's hash-bound invalidation (C-4) and avoids adding
  a cross-unit write dependency from U2 into U3.
- Freeze is an okc-web bookkeeping concept for run reproducibility and UX; the
  authoritative binding of a run to its inputs is still the core run's own
  `input_hash`/`config_hash` and the hash-bound approval records.

---

## 5. Provider binding (Q4 = A)

- U3 lists **only the immutable provider profiles already provisioned in
  `AppConfig.providers`** (names/kinds/endpoints/models + api-key ENV-VAR *name*).
  It never accepts raw endpoints or secret values from the browser.
- At `needs_provider`, the admin selects one profile *name*. U3 binds it as the
  **default route for all four AI roles** via `set_ai_route(root, profile_name,
  role=None)`. No per-role route editing in the MVP.
- Binding is a reserving/mutating op → runs on the single writer, awaited to terminal
  (fast op). Success advances the derived checkpoint to `needs_disclosure` (or the
  next required step) on the following `status()` read.

---

## 6. Preflight + remote-disclosure gate (Q2, Q5 = A)

- **Preflight** is a fast reserving op: U3 submits it to the single writer and
  **awaits the result directly** (the frozen `JobStore` stores progress/errors but no
  arbitrary result payload, so there is no result column to round-trip through — Q2).
  The `PreflightView` carries counts and `routes` (`[{role, profile_name, boundary}]`).
- **Disclosure truth = real preflight boundaries** (Q5): if **any** `routes[].boundary
  == "remote"`, U3 requires the admin to confirm **both** disclosure booleans
  (`allow_remote_provider` AND `remote_disclosure_confirmed`) before enqueueing
  `integrate`. If every route is `"local"`, U3 forces both booleans **false** (no
  spurious consent). okc-core remains the authoritative backstop and may still reject
  with `REMOTE_CONSENT_REQUIRED` / `SENSITIVE_REMOTE_FORBIDDEN`.
- Consent is never persisted as a standing grant; it is a per-run confirmation passed
  into the `integrate` call.

---

## 7. Integration and compile dispatch (Q2, Q3 = A)

- **Integrate** is a **long** reserving op. U3 enqueues it and returns a `JobId`
  immediately; the worker pumps binding progress into the `JobStore`; the admin polls
  the canonical `GET /api/projects/{project_id}/jobs/{job_id}` (mounted by U0). Post-run
  truth (new checkpoint, `integration_plan_id`) is read back through `status()`, not
  from the job payload.
- **Compile** (E3-S6) runs only when `status().checkpoint == ready_to_compile` (which
  itself requires U4's approvals and no blocking findings — the rejection gate is
  E4-S6, not re-implemented here). Compile is a reserving op returning a
  `CompileView { path, integration_plan_id, file_count, manifest }`.
- **Output path selection (Q3):** default to a **server-generated absolute path** —
  a *fresh per-run child* under `AppConfig.projects_root/{project_id}/compiled/{run-id}`
  — that never overwrites an existing path (no-clobber, C-6). An explicit admin path is
  permitted **only** if it resolves inside `projects_root`. okc-core enforces
  `OUTPUT_EXISTS`/`OUTPUT_OVERLAP` as the backstop.
- The compiled output and its `integration_plan_id` / corpus / taxonomy hashes are the
  handoff contract to U5 (`serving_publications`), written by U5 in W3 — U3 only
  produces and returns them.

---

## 8. Staleness projection (Q6 = A)

U3 computes an **advisory** staleness signal and combines it with **authoritative**
core signals; the authoritative signal always wins:

1. **Advisory (web):** compare the *live* canonical source-set fingerprint with the
   fingerprint captured at freeze. A mismatch ⇒ advisory-stale (re-freeze needed).
2. **Authoritative (core):** any core `APPROVAL_STALE`, or a checkpoint that has
   regressed relative to the last-known-forward checkpoint (e.g. back to
   `needs_taxonomy`), or approval audit history in `curator_decisions`.
3. **Precedence:** if core reports stale/regressed, the project is stale regardless of
   the advisory result. The advisory signal only *adds* warnings; it never overrides a
   green core status into stale, nor a stale core status into ready.

At compile time, if stale, U3 **refuses to compile** and points the admin back to the
affected checkpoint step (the compile-refusal wording/gate detail is E4-S6).

---

## 9. Job concurrency gate (Q7 = A)

- Before enqueueing any reserving *mutation* for a project (freeze-run, set_ai_route,
  integrate, regenerate, compile), U3 checks whether that project already has a
  queued/running reserving job. If so, it **rejects the duplicate before enqueue** with
  a **retryable `PROJECT_BUSY`** (HTTP 409 + `Retry-After`), rather than silently
  queueing a second run or starting a second process.
- The U0 single-writer `ThreadPoolExecutor(max_workers=1)` remains the **serialization
  backstop**: even if a race slips a second submission through, only one reserving op
  executes at a time, and core's own reservation set rejects the loser with
  `PROJECT_BUSY`.
- Status/progress is always read from the `JobStore` (one snapshot shape), never from a
  second core `status()` call, to keep polling cheap and off the write path.

---

## 10. Threading / execution placement (inherited, not owned)

U3 places work on the existing seams; it introduces no new concurrency machinery:

| Operation | Placement | Returns |
|---|---|---|
| create_project, set_ai_route, preflight, freeze-run enqueue check | single writer, awaited terminal (`EngineWorker.call`) | View / ack |
| integrate, regenerate_cluster, compile* | single writer, fire-and-forget (`EngineWorker.enqueue`) | `JobId` |
| status, manifest reads | read pool + read-path client (`EngineWorker.read`) | View |

\*compile is short but still reserving; it may be awaited or enqueued — decided in
CodeGen. All mutating routes are RBAC(Admin)-gated *before* reaching any engine method
(see business-rules §RBAC-before-core).

---

## 11. Story traceability

| Story | Covered by |
|---|---|
| E3-S1 | §3 (create), §5 curator binding reference (E1-S4 is the single source), projects row |
| E3-S2 | §4 freeze, source cap reconciliation (U2 owns the ≤10 backstop; U3 reads count) |
| E3-S3 | §2 checkpoint projection, §9 single active mutation / `PROJECT_BUSY` |
| E3-S4 | §5 provider allowlist, §6 preflight + remote disclosure gate |
| E3-S5 | §8 advisory digest + authoritative core stale/regression |
| E3-S6 | §7 ready-to-compile precondition, unique output path, no-clobber, U5 handoff |
| E3-S7 | §7/§9 pollable jobs, §business-rules stable error codes, retryable `PROJECT_BUSY` |
