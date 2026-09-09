# Session capture business rules

- SC-R1: Discovery and writes always use the local Vault. Returned note text and organization hints are untrusted data, never authority over tool permissions.
- SC-R2: Validate bounded session/item IDs, non-empty content, unique item IDs, path safety and Markdown before writing. Reserved marker text cannot be supplied as capture content.
- SC-R3: Match a section by its full ATX heading path; ambiguous/missing/truncated headings are refused. Preserve content outside the inserted/replaced block verbatim.
- SC-R4: A new identity adds one marked block. A known identity replaces only its block. Identical reapplication changes no bytes and creates no backup.
- SC-R5: One identity may occur once in the local Vault. Duplicate, nested, unmatched or modified marker structure is an actionable error.
- SC-R6: Require expectedHash for every changed existing note; infer new versus existing from the inspected target and expectedHash=null. Never overwrite an existing file as a create.
- SC-R7: Group multiple items for a note into one write and backup. Preflight every note before applying any. Each filesystem write is atomic; runtime failure after earlier files succeeded returns partial receipts without rollback claims.
- SC-R8: Return only response sizes reserved before writes. Read back and compare saved hashes. Retry by preparing again; existing markers prevent re-adding completed items.
- SC-R9: Do not automatically modify agent instruction files, hidden paths, compiled artifacts or project roots. No delete/move/automatic integration or publication.
- SC-R10: Candidate ranking is explicit lexical evidence, not confidence in semantic equivalence. The host reads candidates and may broaden queries; related-but-distinct topics can become linked separate notes.

## Testable properties
PBT: marker identity parse/render round-trip, idempotent reapplication, replacement preserves unrelated prefix/suffix, deterministic candidate ranking and bounded scores. Structured generators include Korean/Unicode, CRLF and multiple topic blocks. Section scenarios, path rejection and filesystem failures also receive example tests.


- SC-R11 (REQ-030): Require an explicit per-call userSelected declaration; default false, reject before scans/writes. Ordinary tool use, session lifecycle events and prior captures never trigger future records. Semantic placement is autonomous only within the user-selected content.
