# U4 Review & Resolution — Code Generation Summary

**Wave**: W3 · **Unit**: U4 (`app/review`) · **Epic**: E4 (E4-S1..E4-S6)
**Depth**: Concise FD + CodeGen · MVP-only, lean · **Mode**: AUTOPILOT (real parallel with U5)
**Plan**: [`../../plans/u4-review-code-generation-plan.md`](../../plans/u4-review-code-generation-plan.md) · **FD**: [`../functional-design/business-rules.md`](../functional-design/business-rules.md)

_Persisted by the orchestrator from the `u4-review` agent's verified report (the agent's harness blocked its own summary write; content is authoritative, independently re-verified at the W3 wave barrier)._

## Files created (application code)

- `backend/app/review/__init__.py`
- `backend/app/review/models.py` — DTOs + the critic-payload parser (`CriticFindingView` with raw lowercase severity + derived `blocking`; `iter_clusters`/`parse_findings`/`summarize_cluster`; taxonomy/cluster/gate/decision views; request bodies).
- `backend/app/review/gate.py` — `DecisionGate` (pure, offline-unit-testable): `assert_rationales` (body-only) + `assert_approvable` (blocking rules).
- `backend/app/review/service.py` — `ReviewService` (record-then-act throughout).
- `backend/app/review/router.py` — `register(app, state)`; `main.py` NOT edited (discovered via `UNIT_MODULES`).

## Files created (tests + FD)

- `backend/tests/test_u4_review.py` — real binding + production `create_app`, offline-safe, 9 tests.
- `aidlc-docs/construction/u4-review/functional-design/business-rules.md` — concise 1-doc FD.

## Endpoints (all admin-gated: `admin_context` + `require(ADMIN)`, under `/api/projects/{project_id}`)

- `GET .../taxonomy` → TaxonomyReviewView (E4-S1)
- `POST .../taxonomy/approve` → DecisionReceipt (E4-S1)
- `GET .../clusters` → list[ClusterSummaryView] (E4-S2)
- `GET .../clusters/{cid}` → ClusterDetailView (E4-S2/E4-S5; preserved contradictions read-only)
- `POST .../clusters/{cid}/approve` → DecisionReceipt (E4-S3)
- `POST .../clusters/{cid}/regenerate` → DecisionReceipt{job_id} (E4-S4; long op via `engine.enqueue`)
- `GET .../review/gate` → ReviewGateView `Blocked|PendingApprovals|Ready` (E4-S6)
- `GET .../decisions` → list[DecisionRecordView] (audit trail)

## Story coverage

E4-S1 taxonomy read/edit/approve (rationale required iff edited) · E4-S2 severity-graded findings + schema-guarded parse · E4-S3 approve + Minor waive/omission with mandatory rationale (DecisionGate) · E4-S4 regenerate = the only blocking-resolution path (`PROJECT_BUSY` retryable) · E4-S5 read-only preserved contradictions, **NO winner-select** (only the 3 `CuratorDecision` variants used) · E4-S6 compile-eligibility scoreboard.

## How critic-finding severity was determined (verified, not guessed)

Read okc-core source (`crates/okc-core/src/integration.rs`, `crates/okc-interop`) AND confirmed at runtime with the real binding. `clusters()` returns — after the adapter unwraps `VersionedPayload.payload` — a JSON **array** of `{proposal, critic}`; there is no top-level `clusters` key and no per-element id; the cluster id is `element.proposal.cluster_id`. `critic.findings[]` carries `finding_id`/`severity`/`kind`/`message`/`evidence[]`. `severity` is `CriticSeverity` with `#[serde(rename_all="snake_case")]` over unit variants → bare lowercase `"minor"`/`"major"`/`"critical"`. The parser lowercases defensively and treats **unknown severity as blocking (fail-closed)**. `major`/`critical` = blocking (waive-forbidden, regenerate-only); `minor` = waivable with a rationale. Confirmed offline: `taxonomy()`/`clusters()`/`approve_taxonomy()` on a project with no integration run all raise `PROJECT_INVALID`/422 via the real binding — tests branch on that code, no crash.

## DecisionGate / record-then-act

`assert_rationales` runs body-only BEFORE any engine call (missing Minor/omission rationale → `VALIDATION_FAILED`/400); `assert_approvable` rejects any Major/Critical waive attempt AND any residual blocking finding → `APPROVAL_REQUIRED`/422 (C-2). Tests prove: unauth→401 on all routes with zero engine calls + zero audit rows; pure-gate rejections; HTTP body-only rejection writes NO audit row and reaches NO engine op; `approve_taxonomy` appends EXACTLY ONE `curator_decisions` row (record-then-act) even when the core op then fails. No AI/synthesis result faked.

## Frozen-contract friction (flagged, not worked around)

`services.md` §S0.A lists `status()` as routed through the reserving queue, but the frozen U3 `OrchestrationService` reads `status()` via the non-reserving `engine.read` path. U4's review-gate follows the **frozen U3 behavior** (`engine.read` for status) for consistency. No frozen file modified. Non-blocking.

## Verification (agent-reported; re-verified at the W3 wave barrier)

- `ruff check app/review tests/test_u4_review.py` → All checks passed!
- `mypy app/review` → Success: no issues found in 5 source files
- `pytest tests/test_u4_review.py -q` → 9 passed
- `grep -rn "import okc" app/review` → none (ADR-0002).
