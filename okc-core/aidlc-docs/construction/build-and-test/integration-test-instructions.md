# Integration Test Instructions

## Purpose

Verify interactions among core, AI, application state/services, public
surfaces, bindings, and artifact bytes.

## Scenario 1 — hostile source to sealed corpus

- **Setup**: Temporary directory/ZIP/tar.zst Vault fixtures with safe and hostile members.
- **Execution**: `cargo test --locked -p okc-core --test corpus_builder_contract`.
- **Expected**: safe sources produce order-invariant corpus; unsafe aliases,
  traversal, duplicate content/IDs, and workspace overlap fail without source mutation.
- **Cleanup**: Temporary test directories are owned by the test harness.

## Scenario 2 — complete approval to offline artifact

- **Setup**: Current Schema 3 corpus, provider recordings, taxonomy, cluster proposal/critic/approvals.
- **Execution**: core integration tests and `sdk_output_golden`.
- **Expected**: exactly one complete plan compiles twice to identical bytes;
  stale/forged/incomplete authority fails; verify/explain succeed only for the exact artifact.

## Scenario 3 — app project and provider workflow

- **Setup**: Temporary project, fixed/synthetic provider configuration, explicit source bindings.
- **Execution**: `cargo test --locked -p okc-app` and `cargo test --locked -p okc-ai`.
- **Expected**: preflight/consent precedes calls; tasks/recordings resume under
  exact keys; approvals/invalidation/journal/worker/credential rules hold.

## Scenario 4 — interop and language parity

- **Setup**: Built Python and Node native modules and equivalent loopback provider fixtures.
- **Execution**: Rust interop, Python, and Node public suites.
- **Expected**: equivalent API-v1 workflows, interop schema 2, structured
  errors, explicit paths, and the same artifact inventory digest.

## Scenario 5 — documentation and guide integration

```bash
cargo test --locked -p okc-core --test documentation_contract
npm run docs:build --prefix guide
```

Expected: every repository-relative root/docs/guide Markdown link resolves and
the VitePress site builds. AI-DLC links are additionally checked by the
documentation-only validation described in the build summary.

## Scenario 6 — current-only boundary

- Run core/app/interop/CLI/Python/Node tests that create temporary recognizable
  Schema 1/2 markers.
- Expect `ARTIFACT_SCHEMA_UNSUPPORTED` with supported schema 3 and detected details.
- Expect mixed, symlinked, malformed, oversized, corrupt, and unknown artifacts
  to fail closed, never dispatch to a retired reader.
