# Integration Sync Repair Plan

## Scope and authorization

Workspace: `okc-hooks/`. This is the module implementation workstream of the root integration review. Existing module requirements and approved amendments remain authoritative. The user requested: "AIDLC 모두 확인해서 검증하고, 없는 부분들 구현해". The existing autopilot authorization and this implementation request cover the bounded corrections below.

The local daemon, manifest scanning, state persistence, consent, config token storage, and native watcher already exist. Preserve requirements section 12 (latest-state replacement, no direct core dependency) and section 13 (config token default). Requirements FR-15 explicitly selects raw upload without exclusions; no new filtering policy is introduced. AutoUpdater distribution, Windows IPC, and optional secure-store remain independent deferred features.

## Requirements and design delta

- FR-11 / NFR-04 / U8 R-U8-09: a transient failure schedules another complete scan/upload cycle when backoff expires, without requiring a new file event. Pending events coalesce during the delay. Shutdown and stop are observed during waiting.
- FR-22 / U3 R-REVERIFY-01: freeze wanted file bytes into a temporary spool while hashing; verify against the manifest before sending. Transmit exactly those verified bytes. Memory use remains bounded by the transfer chunk size.
- FR-07..10 / DEP-03: agree with the web receiver on the existing CBOR shapes plus additive negotiation `session_id` and authoritative `resume_offsets`. Send the session header on blob and commit requests, set CBOR content type, and reject unusable offsets. The server confirms commit only after source revision application and receipt persistence.
- FR-08: preserve the large-file chunk path and recover failed single requests through bounded chunk transfer where appropriate.

## Execution steps

- [x] 1. Inspect module state, requirement amendments, applicable rules, existing daemon/upload implementation, tests, and web consumer contract.
- [x] 2. Record the module scope, existing non-goals, concrete corrections, and coordinated wire shape in this plan.
- [x] 3. Implement deadline-driven retry scheduling and responsive shutdown; add regression tests for retry without incoming events, backoff coalescing, and stop during backoff. Focused tests passed (three threaded regressions).
- [x] 4. Implement coherent verified blob spooling and session-aware negotiation/resume; add byte mutation, session propagation, authoritative resume, and codec tests. Focused tests passed (three examples plus two properties). Added the Rust protocol-fixture example for the root contract test.
- [x] 5. Run focused regression/property tests, full workspace tests, build, and clippy. Full suite: 246 passed, zero failed. All ten crates build; all-target clippy passes. Explicit toolchain paths avoided unrelated rustup shim updates.
- [x] 6. Update module verification summaries, audit, and state; report remaining deployment/platform gaps and root integration verification ownership. Added setup/contract instructions and retained earlier baseline history.

## Follow-up: target identity isolation

- [x] 7. Persist a fingerprint binding committed state to the canonical vault, HTTPS endpoint, and token selector. Reject mismatches or unbound legacy committed state without deleting or uploading anything. Guard config reload before snapshot swap and add regression/property tests. This prevents an unchanged manifest from falsely skipping an upload after a destination change. Four tests pass, including idempotency and verifier independence.
- [x] 8. Re-run affected tests/build/clippy and update the final verification count and setup instructions. Full workspace: 250 passed, zero failed; build and all-target clippy pass. State records contain only a versioned SHA-256 fingerprint, never the token or selector.
- [x] 9. Correct pre-existing pseudo-links in the unit dependency table to plain notation after the root link audit; dependency semantics and audit history are preserved.

## Initial repair compliance

PBT-01..07/09/10: compliant for the changed paths (identified properties, actual codec round-trip, transmitted-byte invariant, inherited dedup/idempotency properties, generated retry sequence reference count, domain generators, existing proptest and explicit regressions). PBT-08: deterministic seed 20260909 used with shrinking retained; root `.github/workflows/hooks-ci.yml` now includes the property suite and seed for three OSes and Rust 1.89.0/1.97.1. Remote execution of that matrix is not claimed. No statistical coverage percentage is claimed.

RESILIENCY-01/02/05/06/10/14/15: compliant for this correction through existing daemon state/recovery/logging/status plus newly tested bounded retry waits, interruption, and verified-byte transfer. RESILIENCY-03 remains the approved exemption. RESILIENCY-04/07/08/09/11/12/13: N/A to this local correction; the existing deployment and platform follow-ups remain explicit. Security Baseline remains disabled and skipped. No user content exclusion policy changed.

## Testable properties and validation

PBT-01/03/05/06: retry state obeys the reference rule that pending failures cannot run before their deadline and coalesced events do not suppress a scheduled retry. PBT-02: session-aware negotiate response round-trips through CBOR. PBT-03: all transmitted bytes for a committed blob equal the bytes whose hash matched the manifest, for arbitrary file contents and chunk boundaries. Existing proptest generators/framework/seed support are reused (PBT-07..10). Real threaded regressions complement pure/property tests.

Resiliency applicability: 01/02 preserve the existing high-criticality local daemon and latest-state recovery model; 03 remains the approved exemption; 04/07/08/09/11/12/13 are N/A to this local synchronization correction (no deployment topology change); 05/06/15 preserve existing logs/status/alerts; 10/14 are directly verified by timeout, retry, interruption, and upload regression tests. Security Baseline is disabled by existing configuration and is skipped.

Content validation: plain Markdown and code identifiers only; no diagrams or embedded data formats requiring additional parsing. Relative artifact links and application-code paths remain inside the module's ownership boundary.
