# Build Instructions (W5)

## Prerequisites
Python 3.11+, [`uv`](https://docs.astral.sh/uv/), a Rust toolchain (build-time only, for the okc binding), Node 20+/npm, and a sibling checkout of `okc-core` at `../okc-core`.

## 1. okc engine binding (Rust → Python wheel)
```bash
uv venv --python 3.12 backend/.venv
VIRTUAL_ENV=backend/.venv uv pip install "maturin==1.15.0"
(cd ../okc-core && ../okc-web/backend/.venv/bin/maturin build --release --locked \
   --manifest-path bindings/python/Cargo.toml \
   -i ../okc-web/backend/.venv/bin/python --out bindings/python/dist)
VIRTUAL_ENV=backend/.venv uv pip install ../okc-core/bindings/python/dist/okc_compiler-0.3.0-*.whl
# sanity: python -c "import okc; print(okc.INTEROP_SCHEMA_VERSION)"  → 2
```

## 2. Backend
```bash
cd backend
VIRTUAL_ENV=.venv uv sync --frozen     # from uv.lock; drop --frozen to refresh
```

## 3. Frontend
```bash
cd frontend
npm ci                                  # from package-lock.json
npm run build                           # tsc --noEmit && vite build → frontend/dist
```

## 4. Run (single process, single worker)
```bash
export OKC_WEB_BOOTSTRAP_ADMIN_EMAIL=admin@example.com OKC_WEB_BOOTSTRAP_ADMIN_PASSWORD=change-me
cd backend && .venv/bin/python -m uvicorn app.main:create_app --factory --workers 1 --port 8000
```
`--workers 1` is mandatory (single-writer `OkcClient`; okc-core reservation is process-global). The backend serves `frontend/dist` at `/` with SPA history-fallback.

## Reproducibility
Both lockfiles are committed. The okc binding is pinned to `okc-compiler` 0.3.0 / `INTEROP_SCHEMA_VERSION==2`; pin the okc-core commit in CI for full reproducibility.

The 2026-09-09 continuous-upload extension adds locked `cbor2==5.9.0` and an
idempotent SQLite migration on startup. Follow
[continuous-sync verification](continuous-sync-verification.md) for the new receiver.
