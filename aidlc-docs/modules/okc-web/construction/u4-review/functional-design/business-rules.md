# U4 Review & Resolution — Functional Design (Business Rules)

**Unit**: U4 `app/review` — Conflict/Critic Review & Resolution (Epic E4).
**Depth**: concise (MVP). One doc. Covers the review-domain rules only; the engine seam,
error contract, RBAC guard, and audit store are frozen U0 assets consumed as-is.

Owning services (design `services.md` §U4): TaxonomyReviewService / ClusterReviewService /
RegenerationService / ReviewGateService — implemented together in `ReviewService`.

---

## 1. Decision surface = the 3 `CuratorDecision` variants (C3 / E4-S5)

The ONLY way to mutate review state is one of the three `shared.audit.CuratorDecision`
variants, each mapping 1:1 to an engine op:

| Decision (audit variant) | Engine op (`app.adapter.dto`) | Kind of op | Story |
|---|---|---|---|
| `ApproveTaxonomy(edited_clusters, rationale)` | `approve_taxonomy` | fast reserving (`engine.call`) | E4-S1 |
| `ApproveCluster(cluster_id, omission_rationales, minor_waivers)` | `approve_cluster` | fast reserving (`engine.call`) | E4-S3 |
| `RegenerateCluster(cluster_id, feedback)` | `regenerate_cluster` | long AI op (`engine.enqueue` → JobId) | E4-S4 |

**No winner-select (C-2 / E4-S5).** There is deliberately no `SelectWinner` /
`ResolveContradiction` decision. Preserved semantic contradictions are surfaced **read-only**
inside the cluster detail (`ClusterDetailView.contradictions`, opaque). Structural (path/link)
conflicts are discarded inside okc-core and never reach the caller. The only way a contradiction
changes is a full cluster **Regenerate**. This absence — enforced by the closed union type and
the `curator_decisions.decision_kind` CHECK constraint — is the code-verifiable C3 differentiator.

## 2. Critic-finding severity (E4-S2)

`clusters()` returns (after the adapter unwraps `VersionedPayload.payload`) a **JSON array** of
`{proposal, critic}` objects — there is no top-level `clusters` key and no per-element `id`. The
cluster id lives at `element.proposal.cluster_id` (and `element.critic.cluster_id`). Each
`critic.findings[]` entry carries `finding_id`, `severity`, `kind`, `message`, `evidence[]`.

Severity serializes from okc-core (`CriticSeverity`, `#[serde(rename_all="snake_case")]`) as the
bare lowercase strings **`"minor"` / `"major"` / `"critical"`**. `CriticFindingView` keeps the raw
lowercase string and derives a `blocking` boolean:

- `major` / `critical` → **blocking** (waive-forbidden).
- `minor` → **non-blocking** (waivable with a rationale).
- any other/unknown value → **blocking (fail-closed)** — we never silently treat an
  unrecognized severity as waivable. Documented, defensive parse (offline severity confirmed
  against okc-core `crates/okc-core/src/integration.rs`).

## 3. DecisionGate — gate-before-core (C-1 analogue, C-2)

`DecisionGate` runs entirely in okc-web BEFORE any engine op, so a rejected decision reaches zero
reserving/mutating engine calls and writes zero audit rows. Necessity is code-verified: okc-core's
blocking-approval rejection surfaces as the generic `PROJECT_INVALID`, not `APPROVAL_REQUIRED`, so
the deterministic 422 must originate here.

- **`assert_rationales(cmd)`** (body-only, no findings needed): any `minor_waivers` value or
  `omission_rationales` value that is blank → `VALIDATION_FAILED` (400). "사유 필수" (E4-S3).
- **`assert_approvable(findings, cmd)`**:
  1. a `finding_id` in `minor_waivers` whose finding is blocking → `APPROVAL_REQUIRED` (422):
     "Major/Critical cannot be waived; regenerate to resolve" (C-2, E4-S3/E4-S4).
  2. any residual blocking finding on the cluster → `APPROVAL_REQUIRED` (422): a cluster with
     Major/Critical findings is not approvable at all; regenerate is the only path.

Regenerate is NOT gated (it is the sanctioned path for blocking findings, E4-S4).

## 4. Record-then-act audit (Q8, `shared.audit`)

Every mutating decision:
1. **Validate/gate** (§3) — nothing recorded on rejection.
2. **`audit.append(NewDecisionRecord(...))`** (append-only) BEFORE the engine op. `HashBindings`
   (`proposal_hash`, `critic_hash`, `taxonomy_hash`) are populated for `approve_cluster` from the
   cluster payload being acted on. `approve_taxonomy` binds `taxonomy_hash` when available;
   `regenerate` records `cluster_id` + `feedback` (bindings empty in MVP — the acted-on state is
   the immediately preceding synthesis).
3. **Engine op** (`engine.call` for fast reserving; `engine.enqueue` → JobId for regenerate).
4. **`audit.link_job(audit_id, job_id)`** after the long op returns its JobId.

The audit row persists even if the engine op subsequently fails (record-then-act = the decision
is recorded regardless of outcome). `curator_id` = the unverified admin `curator_label` (E1-S4).

## 5. ReviewGate scoreboard — compile-eligibility (E4-S6)

Read-only projection; it never triggers compile (compile is U3's endpoint):

- Any cluster with a blocking (Major/Critical) finding → **`Blocked`** (with the blocking list).
- No blocking findings, core checkpoint `ready_to_compile` → **`Ready`**.
- Otherwise (approvals still pending) → **`PendingApprovals`**.

Each blocking item carries its `{cluster_id, finding_id, severity}` and the required action
(Major/Critical → regenerate; Minor → waive+rationale; unapproved → approve_cluster).

## 6. Concurrency & errors

- Mutating decisions call `_guard_not_busy` (a queued/running long op → `PROJECT_BUSY` 409,
  retryable) before enqueuing, consistent with U3, to avoid head-of-line blocking the single
  writer. `PROJECT_BUSY` is retryable (E4-S4).
- All errors are `EngineError` with existing `EngineErrorCode`s only; branching is by code, never
  by parsing okc-core message strings (FR-INT-8). Reads (`taxonomy`, `clusters`) run on the
  non-reserving read path; a project with no completed integration surfaces the mapped code
  cleanly (no crash).

## 7. Story trace

E4-S1 taxonomy read/edit/approve · E4-S2 severity-graded findings view · E4-S3 approve + waive/
omission with mandatory rationale · E4-S4 regenerate (only blocking-resolution path) ·
E4-S5 read-only preserved contradictions, no winner-select · E4-S6 compile-eligibility gate.
