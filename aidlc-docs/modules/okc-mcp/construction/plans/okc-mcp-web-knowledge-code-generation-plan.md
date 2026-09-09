# Code generation plan — web knowledge

Authorization: `AIDLC 모두 확인해서 검증하고, 없는 부분들 구현해` plus the retained module autopilot authorization. Prior review identifies the missing integration; this follow-up explicitly extends the local-only scope. No new permission gate is inferred.

## Design and stage decisions

Workspace: `/Users/sihun/workspace/projects/okc/okc-mcp`. Brownfield TypeScript, existing local source authoring and bounded retrieval. Requirements: [web delta](../../inception/requirements/requirements-web-knowledge.md). Existing module design artifacts remain authoritative for the authoring pipeline. Root coordinates web revision/auth contract; no module histories are merged.

Workflow: workspace/document verification; requirements and stories; adapter/application and functional/NFR design; implementation; full build/test. Infrastructure design and further units skipped (one process, no deployment changes).

Components: `WebVault` is a read-only fixed-origin adapter. `WebSnapshot` pins revision and caches bounded note bytes only for one operation. `createServer` routes read tools to local `Vault` or `WebSnapshot`; all writes retain the local `Vault`. CLI handles initialization and diagnostics. Local immutable-artifact guard stays unchanged.

Rules: default web when configured; explicit local is permitted; remote errors are errors. Contract/files carry string `revision`, `status`, `stale`, project ID. Subsequent endpoint requests carry `revision`; responses with inconsistent identity are rejected. Relative note paths remain inside `knowledge/` or `legacy/`; `.okc` metadata is consumed only through verify/explain. Optional HTTP bearer token is never echoed or redirected. Request deadlines, streamed byte bounds, max file count, scan bytes, response bytes, and cancellation apply. Writes cannot target web artifacts. New source initialization refuses existing directories and guarded ancestors.

Testable properties: URL path query round-trip over generated Unicode note paths; snapshot requests always pin the acquired revision; local authoring invariant preserved. The prior seeded-generator deviation is resolved by installing fast-check and migrating the property tests to shrinking structured generators with fixed seeds.

## Execution

- [x] Step 1: Read state, requirements, prior design, existing implementation/tests and review evidence; record scoped requirement/story/design delta.
- [x] Step 2: Implement validated web configuration and fixed-origin snapshot reader in `src/config.ts`, `src/web.ts` (REQ-018/020/021).
- [x] Step 3: Route MCP read tools, keep local authoring explicit, add provenance/verification and source information in `src/server.ts` (REQ-018/019/020).
- [x] Step 4: Add new-source initialization and web-aware diagnostics/config in `src/cli.ts`, `src/setup.ts`; update guide/docs/examples (REQ-019/022).
- [x] Step 5: Add contract, source-selection, errors, revision, bounded IO, authoring and initialization tests; run the `npm run check` constituents (typecheck, test, build). All passed; final suite 76/76. Loopback tests require sandbox escalation (REQ-018..022).
- [x] Step 6: Record verified results, update state/README/build-and-test instructions and audit; report residual limitations. Root helper added for actual inter-module stdio checks; root agent owns its execution.

## NFR and extension assessment

RESILIENCY-01: medium availability impact (retrieval unavailable), high source-integrity importance; web is new read dependency. 02/11/12/13: inherited local backup/manual restore, remote cache ephemeral. 03/04: inherited versioned tarball/local install. 05/06/07: CLI diagnostics and typed errors for local tool; centralized service monitoring N/A. 08/09: cloud deployment/scale N/A. 10: explicit HTTP timeouts, no automatic fallback, bounded reads. 14: behavior failure tests now; production drills N/A. 15: inherited lightweight error/runbook process. Security extension skipped as disabled. PBT partial: 02/03/07/08/09 compliant after fast-check migration; real structured generators, shrinking, fixed seeds and npm test integration.
