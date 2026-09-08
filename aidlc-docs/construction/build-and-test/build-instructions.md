# Integrated Build Instructions

Use the repository root, Rust matching [core toolchain](../../../okc-core/rust-toolchain.toml), Python 3.12 and Node >=22.13. Module lockfiles own dependencies.

1. Build/install core Python wheel using [web quickstart](../../../okc-web/README.md); install backend locked dependencies including cbor2.
2. In okc-hooks run cargo build --locked --workspace and cargo build --locked -p upload-client --example protocol-fixture.
3. In okc-mcp run npm ci then npm run check.
4. In okc-web/frontend run npm ci then npm run build.
5. Run aggregate checks in [integration-test-instructions.md](integration-test-instructions.md).

Existing local web venv is okc-web/backend/.venv. Build directories and user data were preserved; no release publication or deployment is part of this implementation.
