# End-to-End Test Instructions

## CLI/TUI workflow

Build the binary and run the POSIX synthetic-provider harness:

```bash
cargo build --locked -p okc
python3 tests/tui_pty_smoke.py target/debug/okc
```

Expected workflow:

1. Discover/select Vaults from cwd.
2. Configure a bounded loopback provider.
3. Run preflight before any disclosure.
4. Generate and explicitly approve taxonomy.
5. Generate/review/approve every cluster.
6. Compile without further provider calls.
7. Independently verify output.
8. Restart without provider calls for completed tasks.
9. Confirm source bytes/mtime and terminal state remain unchanged/restored.

This does not qualify Windows ConPTY, remote vendors, long cancellation, or process kill.

## Python full workflow

Run the public suite against a clean-installed wheel or developed native extension:

```bash
python3 -m pytest bindings/python/tests/test_public_api.py -q
```

Expected: explicit project/source paths, synthetic provider integration,
taxonomy and cluster approvals, compile, verify, explain, source immutability,
structured failures, and the shared golden.

## Node full workflow

```bash
npm test --prefix bindings/node
```

Expected: the same workflow through CommonJS/ESM with camelCase projections,
mandatory consent/outputPath, typed errors, metadata-key preservation, and shared golden.

## Package-install E2E

CI must install Python wheel and sdist in clean environments and install the
root plus matching platform npm tarball without source-tree fallback. Imports,
API version, native loading, typed declarations, and a bounded operation must pass.

## Cleanup

Use test-owned temporary directories. Never delete an unresolved path or source
Vault. Compiled destinations are always new and unique per test.
