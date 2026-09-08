# U4 Review & Resolution — Code Generation Plan (W3)

**Unit**: U4 (`app/review`) — Conflict/Critic Review & Resolution.
**Scope**: MVP-only, Epic E4 stories E4-S1..E4-S6. Lean; no speculative abstraction.
**Guardrails**: ADR-0002 (no `import okc`; engine only via `state.engine` + `app.adapter.dto`);
RBAC-before-core on every route (C-1); 3-variant `CuratorDecision`, no winner-select (C3);
record-then-act audit (Q8); error branching by `EngineErrorCode` only (never message strings).

Edit ONLY: `app/review/*.py`, `tests/test_u4_review.py`, this plan,
`aidlc-docs/construction/u4-review/**`. Everything else is FROZEN (read/import only).

---

## Step 1 — Functional Design (concise, 1 doc)
- [x] Write `aidlc-docs/construction/u4-review/functional-design/business-rules.md`:
  DecisionGate rules, the no-winner-select invariant (C3/E4-S5), record-then-act,
  the ReviewGate scoreboard (E4-S6), critic-finding severity parsing. Traces E4-S1..E4-S6.

## Step 2 — Models (`app/review/models.py`) [E4-S2, E4-S1, E4-S3, E4-S6]
- [x] `CriticFindingView` (typed finding: `finding_id`, `severity` raw lowercase, `kind`,
  `message`, `blocking` derived) + defensive severity→blocking parser (unknown = fail-closed).
- [x] `parse_clusters(payload)` over the opaque `clusters()` array (`[{proposal, critic}]`).
- [x] `TaxonomyReviewView`, `ClusterSummaryView`, `ClusterDetailView` (proposal + contradictions
  opaque, E4-S5 read-only), `ReviewGateView` (`Blocked|PendingApprovals|Ready`),
  `DecisionRecordView`, `DecisionReceipt`.
- [x] Request bodies: `ApproveTaxonomyRequest`, `ApproveClusterRequest`, `RegenerateClusterRequest`.

## Step 3 — DecisionGate (`app/review/gate.py`) [E4-S3, E4-S4, E4-S6]
- [x] `assert_rationales(cmd)` — body-only: Minor-waive / omission missing rationale →
  `VALIDATION_FAILED` (400). Runs before any engine call.
- [x] `assert_approvable(findings, cmd)` — Major/Critical waive attempt AND any residual
  blocking finding → `APPROVAL_REQUIRED` (422). (C-2: blocking resolved only by regenerate.)
- [x] `scoreboard(findings)` — compile-eligibility helper (blocking present → Blocked).

## Step 4 — Service (`app/review/service.py`) [E4-S1..E4-S6]
- [x] `ReviewService`: taxonomy read/approve; cluster list/detail/approve/regenerate;
  review-gate scoreboard; decisions list.
- [x] Every mutating decision is record-then-act: gate → `audit.append(NewDecisionRecord(...))`
  BEFORE the engine op; for regenerate (long op) `audit.link_job(audit_id, job_id)` after.
- [x] No winner-select path exists (only the 3 `CuratorDecision` variants).

## Step 5 — Router (`app/review/router.py`) [E4-S1..E4-S6]
- [x] `register(app, state)` (discovered by `main`, no edit to main.py). Admin-gated routes:
  `GET/POST .../taxonomy(/approve)`, `GET .../clusters`, `GET .../clusters/{cid}`,
  `POST .../clusters/{cid}/approve`, `POST .../clusters/{cid}/regenerate`,
  `GET .../review/gate`, `GET .../decisions`. Each `Depends(admin_context)` + `require(ADMIN)`.
- [x] `app/review/__init__.py`.

## Step 6 — Tests (`tests/test_u4_review.py`) [real binding, offline-safe]
- [x] Unauth → 401 on every route (RBAC-before-core; zero engine calls, zero audit rows).
- [x] DecisionGate (pure): Major/Critical waive → 422; residual blocking → 422;
  missing minor rationale → 400; missing omission rationale → 400.
- [x] Missing-rationale via HTTP → 400 with NO audit row and NO engine call (body-only gate).
- [x] taxonomy/clusters on a no-run project → clean mapped `EngineError` code (no crash/500).
- [x] approve_taxonomy (record-then-act) appends exactly one `curator_decisions` row.

## Step 7 — Verify (report exact output) & summary
- [x] `ruff check app/review tests/test_u4_review.py` green.
- [x] `mypy app/review` green.
- [x] `pytest tests/test_u4_review.py -q` green.
- [x] Code summary delivered to the team lead in the final report (harness blocks writing
  `code/summary.md` as a report file; the FD `business-rules.md` remains the durable U4 doc).
</content>
</invoke>
