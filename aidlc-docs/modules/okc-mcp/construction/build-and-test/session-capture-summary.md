# User-selected session capture verification

Status: COMPLETE, 2026-09-09 KST. Scope: okc-mcp only. The user chooses the session/content to record; the connected coding agent chooses relevant local notes and sections within that selection.

## Implemented behavior
- Initial server instructions and capture_session prompt describe an explicit user-selected workflow. Startup, new messages, task completion, compaction and shutdown do not trigger capture.
- Both preparation and application require userSelected=true (default false), checked before scanning/writing. A completed request grants no standing permission for later captures.
- prepare_session_capture supplies local lexical candidates, metadata/section evidence, folder patterns, untrusted organization hints and existing session-item locations. Web-first knowledge reads remain unchanged.
- apply_session_capture previews then applies host-selected placements through the existing authoring/Vault pipeline. It groups items per note, preserves unrelated content/YAML, retains external backups and verifies saved hashes.
- Stable markers support re-capture/reconnect without duplicates. Invalid destinations, planned case/Unicode/file-directory collisions, ambiguous sections and malformed markers are rejected before writes.
- Runtime partial failures return an MCP error with per-file receipts and remaining paths; retry uses fresh preparation and existing identities.

## Executed checks
- Node v24.13.1.
- Final npm run check: exit 0; TypeScript source/test checks passed; 103 tests passed, 0 failed/skipped; build passed.
- Capture plus stdio focused suites: 33 tests passed. New session-specific coverage comprises 17 domain/PBT tests and 3 stdio tests; other test totals include existing authoring/web/install work.
- Two domain-shaped capture properties used fast-check seed 20260909 with shrinking: 200 marker/recapture cases and 100 candidate-ordering cases.
- Actual stdio: selected session -> local candidates while web is unreachable -> section preview/apply -> repeat -> fresh-context update; negative tests reject missing/false selection and demonstrate ordinary reads leave Vault/state unchanged.
- Package dry-run: 49 entries; dist/capture.js, dist/capture-guide.js and dist/server.js included. Compiled output contains the selected-only gate and instructions.
- git diff --check passed. AI-DLC link inventory had no current/historical issues; seven existing draft-only future-layout references are preserved.

The first full run encountered nine sandbox EPERM errors opening local HTTP test ports; rerunning through the approved npm run check escalation passed. No test was weakened or skipped to avoid the restriction.

## Extension compliance
Security baseline remains disabled; existing product path/hash/backup/readonly safeguards were tested.

| Rule | Result | Evidence / applicability |
|---|---|---|
| RESILIENCY-01 | Compliant | Local note integrity is the critical workload; host/capture/Vault dependencies documented. |
| RESILIENCY-02 | Compliant | Existing backup-and-manual-restore targets retained; no invented production SLA. |
| RESILIENCY-03 | Compliant | Existing local Git and AI-DLC change history; explicit implementation and steering recorded. |
| RESILIENCY-04 | N/A | No production deployment; existing build/package/test commands remain compatible. |
| RESILIENCY-05 | Compliant, local | Typed per-file receipts expose failures and verification; centralized cloud telemetry is N/A. |
| RESILIENCY-06 | N/A | No new public service or load balancer; existing local diagnostics remain. |
| RESILIENCY-07 | N/A | No deployed regional/replicated workload introduced. |
| RESILIENCY-08 | N/A | Local single-Vault operation; no regional topology change. |
| RESILIENCY-09 | Compliant | Bounded inputs, corpus reads, candidates, previews and receipt reservation. |
| RESILIENCY-10 | Compliant | No new provider/network dependency; cancellation and local I/O failure behavior explicit. |
| RESILIENCY-11 | Compliant | Existing external backups and manual recovery retained. |
| RESILIENCY-12 | Compliant, local | Backup contents checked; distributed replication is N/A. |
| RESILIENCY-13 | Compliant | Partial-result recovery and fresh-hash retry documented/tested. |
| RESILIENCY-14 | Compliant | Injected mid-batch failure, stale hashes, malformed markers and cancellation tested. |
| RESILIENCY-15 | Compliant, local | Errors carry bounded codes and recovery paths; no new enterprise incident process. |
| PBT-02 | Compliant | Marker session/item identity parse/render round-trip. |
| PBT-03 | Compliant | Unrelated prefix/suffix preservation and deterministic bounded candidates. |
| PBT-07 | Compliant | Structured session/note generators with Korean, Unicode and CRLF. |
| PBT-08 | Compliant | Fixed seed, default shrinking, included in npm test. |
| PBT-09 | Compliant | Existing fast-check dependency and Node test integration. |

Other PBT rules are advisory under the accepted partial mode; idempotence is also explicitly property-tested.

## Practical boundaries
The host model owns semantic relevance and user selection. userSelected is a caller declaration; the server does not independently authenticate human intent from a conversation. The tests use a scripted host for placement and do not claim live-model accuracy. No raw transcript collector, session-end hook, internal model API, real user Vault mutation, installation, publication, commit or push was performed.

Per-note writes are atomic; the batch is not a multi-process or multi-file transaction. Existing web/core review/publication remains separate. Read [usage and recovery](../okc-mcp-session-capture/code/usage.md), [requirements](../../inception/requirements/requirements-session-capture.md) and the [completed plan](../plans/okc-mcp-session-capture-code-generation-plan.md).

