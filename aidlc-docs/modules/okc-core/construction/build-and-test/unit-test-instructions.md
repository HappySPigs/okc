# Unit Test Execution Instructions

## Rust unit and integration targets

Run the complete locked workspace suite:

```bash
cargo test --locked --workspace --all-features --no-fail-fast
```

The current static Rust inventory is 130 `#[test]` functions. The command also
runs all explicit integration targets and doc-test targets. A successful run
must report zero failed targets; do not report only the numeric sum if one
target fails.

Focused commands:

```bash
cargo test --locked -p okc-core
cargo test --locked -p okc-ai
cargo test --locked -p okc-app
cargo test --locked -p okc-interop
cargo test --locked -p okc
```

## Python tests

After building/installing the native extension into a clean environment:

```bash
python3 -m pytest bindings/python/tests -q
python3 -m mypy --strict bindings/python/tests/typing_contract.py
```

The runtime suite includes parametrized/subtest coverage beyond the nine
top-level `test_` functions visible by static counting.

## Node.js tests

After building the native addon:

```bash
npm test --prefix bindings/node
npm run typecheck --prefix bindings/node
```

## Result review

1. Preserve the exact failing target/test name and environment.
2. Distinguish product failure, documentation failure, sandbox/network block,
   missing host tool, and unsupported capability.
3. Fix the authoritative source/contract rather than weakening an invariant.
4. Rerun the focused failure, then the complete relevant suite.
5. Update `CURRENT_STATE`, `TRACEABILITY`, and the append-only decision log if behavior/evidence changed.
