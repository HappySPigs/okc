# Integration Implementation Plan

Authorization: user requested checking every module against the four-role requirements and designing/implementing gaps. Preserve module-local AI-DLC histories and approved product policies; root owns shared contracts and aggregate evidence.

- [x] Resume root state and review; verify existing worktree and active tasks.
- [x] Reconcile cross-module requirements and accepted constraints; record INT-01..10.
- [x] Map stories: contributor submits successive revisions (INT-01/02/08); agent reads web and authors local (INT-03/04/05); curator reviews/publishes and controls access (INT-05/06/07); maintainer reproduces all gates (INT-09/10).
- [x] Coordinate components and units through [contracts](../application-design/integration-contracts.md).
- [x] Hooks unit: reliable retry scheduling, coherent snapshot bytes, shared upload session protocol, tests and module records.
- [x] Web upload unit: authenticated receiver, stable source updates, idempotent commits, concurrent revision protection, tests and records.
- [x] Web serving unit: compile receipt, verified manifest publication, fixed revision reads, read-token access, regression tests and records.
- [x] MCP unit: source separation, remote reader, initialization, metadata/provenance, bounded retrieval and tests.
- [x] Inspect core requirements and run proportionate real-core verification; repair only proven scope-relevant defects.
- [x] Exercise cross-module flow and complete all appropriate module gates.
- [x] Validate AI-DLC inventories/links, update current documentation and traceability, report remaining limits precisely.

Final evidence: core 130 Rust +12 Python; hooks 250; MCP 76; web backend 114 + frontend 8; actual four-module bridge 1, all passed. Build/lint/type checks passed. See [aggregate results](../../construction/build-and-test/build-and-test-summary.md).

Text workflow: prior review → requirements reconciliation → shared contract design → parallel module functional/code plans → per-module implementation and tests → actual integration validation → handoff.

Existing module stories/application designs are extended through module-local continuation plans rather than copied into root. Root story mapping is above; module work remains independently owned. Infrastructure design: skipped (no new deployment architecture). NFR requirements/design are scoped to the existing architecture plus the invariants documented in requirements/contracts. Operations/publishing external deployments: not requested.
