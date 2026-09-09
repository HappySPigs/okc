# Web knowledge build and verification

Date: 2026-09-09 KST. Scope: REQ-018..022 in the [requirements delta](../../inception/requirements/requirements-web-knowledge.md). No publication or real user Vault modifications performed by verification; temporary test fixtures are cleaned by the test harness.

## Implemented evidence

| Requirement | Implementation | Verification |
|---|---|---|
| Web default / absent-config local | config.ts, server.ts read routing | Remote+local same session, default web, explicit local, absent web, all remote failure paths |
| Independent local authoring | server.ts writeReply; unchanged authoring/Vault pipeline | Remote 403 leaves local writing usable; full prior hash/backup/readonly/immutable-root regression suite |
| Revision / provenance | web.ts snapshots; verify_vault/explain_note | Pinned multi-file requests, revision mismatch, preserved contradictions, stale metadata, actual stdio consumer |
| Bounded safe IO | web.ts fixed endpoints, schemas, streams, timeout | Malformed contract/wrong project/path, redirect, status errors, oversized stream, scan cache bound, timeout/cancel, secret redaction |
| New source initialization | setup.ts / cli.ts init; template resource | Empty conventional folders, no overwrite, guarded artifact/symlink ancestors |
| Enabled PBT framework gap | package.json/lock, properties.pbt.test.ts | Real fast-check with structured generators, shrinking and seeded reproduction |

## Commands and results

`npm run typecheck` and `npm run build`: pass. Final `npm test`: 76/76 passed, 0 failed, 0 skipped, including the core emitted-path regression. This executes all constituents of `npm run check`; loopback tests ran with the approved npm test escalation. Package installation reported 0 npm audit vulnerabilities.

The first network fixture run inside the sandbox could not bind loopback (EPERM); rerun used the approved npm test escalation. One test setup was corrected to keep its statePath in the temporary directory. Fast-check produced a shrunk duplicate-tag counterexample for a new test: analyzeNote is a deduplicating projection, so serialization round-trip assertions correctly use raw YAML parsing instead. Neither finding was hidden by deleting tests.

Build/test reproduction: `npm ci`, then `npm run check` from okc-mcp. Loopback sockets must be permitted for the web contract tests. Tests create no external service dependency: a synthetic HTTP fixture serves revisioned Markdown. Real okc-core/web/hooks integration belongs to the root suite, which uses `scripts/mcp-integration-client.mjs` and this module's built stdio CLI.

Root coordinator subsequently reported the real integration test passing: MCP local authoring, Rust hooks CBOR upload, real core/web integration and publication, then private fixed-revision MCP read/search/verify/explain. Root owns the aggregate command/output evidence in `scripts/test_integration.py` and umbrella AI-DLC records. A root AI-DLC link audit also found and corrected one pre-existing relative link in first-unit nfr-design/logical-components.md.

Performance scope: explicit byte/file/timeout limits are verified; no production latency/throughput SLA or real-Vault OS matrix is claimed. Known product bounds: Markdown retrieval, lexical search, documented wikilink subset, manual local recovery, and read-only publication consumption.
