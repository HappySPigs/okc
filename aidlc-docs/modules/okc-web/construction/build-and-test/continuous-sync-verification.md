# Continuous synchronization verification

Run from `okc-web/backend` using the existing virtual environment and the real
`okc-compiler` Python binding. `cbor2==5.9.0` is locked in `uv.lock`.

```bash
.venv/bin/python -m pytest tests/test_hooks_sync.py tests/test_adapter_validation.py tests/test_u2_upload.py tests/test_foundation.py tests/test_w1_spine.py -q
.venv/bin/python -m ruff check app/upload app/adapter app/shared/state.py tests/test_hooks_sync.py tests/test_adapter_validation.py tests/test_u2_upload.py
.venv/bin/python -m mypy app/upload app/adapter app/shared/state.py tests/test_hooks_sync.py tests/test_adapter_validation.py tests/test_u2_upload.py
```

Recorded result: 63 tests passed; Ruff clean; mypy clean across 16 files. These
are module-local upload results, not a claim that the full AI/provider flow passed.

The Rust wire test uses `okc-hooks/target/debug/examples/protocol-fixture` when
available. Build it from `okc-hooks` with
`cargo build -p upload-client --example protocol-fixture`. The fixture emits CBOR
using the hooks' actual serde/ciborium types and canonical digest algorithm. In this
verification run it was present and the fixture test passed without skipping.

Meaningful failure cases include authentication before decoding, malformed CBOR,
path escape and aliases, undeclared/missing/corrupt blobs, wrong resume offsets,
stale revision conflicts, token rotation, and failure after core commit but before
SQLite acknowledgment. The last scenario uses a real engine and injects only a
database error; retry closes the control-plane state without another core source.
Crash-window cases cover complete partial files before cache publication (both
valid and corrupted bytes). Native validation cases cover exceptions raised
before a Job can be returned; malformed requests remain typed HTTP errors.

Performance qualification remains limited to configured input bounds and the
ten-source/repeated-revision cases. This is not a throughput or production SLA test.
