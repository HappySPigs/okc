# Module Verification Commands

- Core: cargo test --locked --workspace --all-features --no-fail-fast from okc-core.
- Core Python: backend venv python -m pytest okc-core/bindings/python/tests/test_public_api.py from root.
- Hooks: PROPTEST_RNG_SEED=20260909 cargo test --locked --workspace --features proptest-support from okc-hooks; then all-target clippy with the same features.
- MCP: npm run check from okc-mcp (typecheck, all tests, build).
- Web backend: .venv/bin/python -m ruff check app tests; .venv/bin/python -m mypy app tests; .venv/bin/python -m pytest -q from okc-web/backend.
- Web frontend: npm run build and npm test from okc-web/frontend.

Some tests open loopback sockets for deterministic HTTP providers or MCP; a sandbox may require permission for local socket binding. They do not require external LLM credentials. Native OS service installation and remote CI matrices are distinct deployment checks.
