# Contract Test Instructions

## Rust/CLI contract

```bash
cargo test --locked -p okc-core --test sdk_output_golden
cargo test --locked -p okc --test cli_contract
```

Validate:

- Current public types and `CorpusBuilder` boundary.
- Exact current CLI commands/options/exits.
- Absence of retired phase commands, project upgrade, Pack, old policy/workspace,
  and generic explanation shapes.
- Current Schema 3 artifact bytes and inventory golden.

## Interop contract

```bash
cargo test --locked -p okc-interop
```

Validate DTO schema 2, absolute paths, structured errors, job states,
cancellation, bounded queues/events, shared project reservations, and typed
verification/explanation results.

## Python contract

```bash
python3 -m pytest bindings/python/tests -q
python3 -m mypy --strict bindings/python/tests/typing_contract.py
```

Validate ABI3 import, snake_case fields, keyword-only `output_path`, type
stubs, consent, source immutability, current-only errors, and full workflow.

## Node contract

```bash
npm test --prefix bindings/node
npm run typecheck --prefix bindings/node
npm pack --dry-run ./bindings/node
```

Validate ESM/CommonJS, camelCase typed results, mandatory `{outputPath}`,
opaque-map key preservation, current-only errors, and package contents.

## Golden

All three language surfaces must produce inventory SHA-256
`452ca0671e806a93b4f36f218cf9e62da899f6404c74705c2cf0ca14e413c7e5`.
