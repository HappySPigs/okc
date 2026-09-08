# okc-web

A web platform for **permission-gated integration of departmental Obsidian vaults** into a single, verifiable, merged knowledge base — built on top of the [okc-core](../okc-core) engine (OKC = Obsidian Knowledge Compilation).

An admin (curator) collects successive individual/department vault revisions via reusable upload tokens (revealed once), freezes the source set, runs a multi-step human-in-the-loop integration (provider → disclosure → taxonomy → clusters), reviews critic findings (approving, waiving *minor* only, or regenerating *blocking* ones), compiles a deterministic merged vault, and serves fixed revisions to okc-mcp.

The hooks daemon now connects through authenticated CBOR `/api/sync`; repeated uploads update the same source. Serving supports revision-pinned reads, publication history/restore, and optional private access with separate read tokens. See [continuous sync](aidlc-docs/construction/u2-upload/code/continuous-sync.md), [versioned serving](aidlc-docs/construction/u5-serving/code/versioned-serving.md), and the [four-module verification](../aidlc-docs/construction/build-and-test/build-and-test-summary.md).

> Hackathon MVP. Single-organization, single long-lived process, local/trusted environment.

---

## Architecture

```
                    React + Vite SPA (frontend/)                 okc-core (Rust)
  browser ────────►  admin console + /upload/{token} shell         vault engine
      │              (cookie auth, code/category error UI)          integration/critic
      │  HTTP /api, /u                                              compile/verify/explain
      ▼                                                                    ▲
  FastAPI backend (backend/app/) ── ADR-0002 adapter seam ──────────────────┘
   U0 shared/adapter  ·  U1 auth  ·  U2 upload  ·  U3 orchestration          (okc Python
   U4 review  ·  U5 serving   (single-writer engine worker + SQLite WAL)      bindings:
                                                                              okc-compiler 0.3.0)
```

- **Backend** — FastAPI (Python 3.11+). Consumes okc-core **only** through the `okc` Python bindings, behind a single adapter seam (`app/adapter/`, ADR-0002). One SQLite WAL state DB; a single-writer `ThreadPoolExecutor` serializes every reserving engine op (okc-core has a process-global reservation); a separate read-path client keeps status/serving reads off the write queue. All errors are a stable `{code, category}` contract (never message-parsed); HTTP status derives from the code.
- **Frontend** — React 19 + Vite 6 + TypeScript + React Router v6 + Tailwind v4 (Radix slate/indigo tokens). Admin-only app shell; the contributor's only surface is the token upload shell. Three focal "wow" screens (integration monitor, cluster-review workbench, provenance/verify); the rest are deliberately plain.
- **okc-core** — a separate Rust workspace (kept unmodified). okc-web builds its Python binding (`okc-compiler`, maturin/pyo3) at setup time.

### Units (AI-DLC)
| Unit | Dir | Responsibility |
|---|---|---|
| U0 | `app/shared`, `app/adapter` | RBAC/error/jobs/state primitives + the single okc binding seam + single-writer worker |
| U1 | `app/auth` | admin login (argon2id), sessions, RBAC-before-core, account admin |
| U2 | `app/upload` | upload token lifecycle, multipart and resumable CBOR upload, validation, stable source revisions |
| U3 | `app/orchestration` | project registry, freeze, checkpoint projection, provider/disclosure, integrate/compile |
| U4 | `app/review` | taxonomy/cluster review, DecisionGate (no winner-select; blocking→regenerate-only), audit |
| U5 | `app/serving` | verified publication/history/restore, version-pinned read APIs, freshness and optional read-token access |
| U6 | `frontend/` | the SPA wiring all of the above |

---

## Quickstart

### Prerequisites
- Python 3.11+ and [`uv`](https://docs.astral.sh/uv/), a Rust toolchain (to build the okc binding), Node 20+ / npm.
- A sibling checkout of `okc-core` (this repo expects `../okc-core`).

### 1. Build + install the okc engine binding
```bash
uv venv --python 3.12 backend/.venv
VIRTUAL_ENV=backend/.venv uv pip install "maturin==1.15.0"
# build the wheel from okc-core (produces okc_compiler-0.3.0-*.whl)
(cd ../okc-core && ../okc-web/backend/.venv/bin/maturin build --release --locked \
   --manifest-path bindings/python/Cargo.toml \
   -i ../okc-web/backend/.venv/bin/python --out bindings/python/dist)
VIRTUAL_ENV=backend/.venv uv pip install ../okc-core/bindings/python/dist/okc_compiler-0.3.0-*.whl
```

### 2. Backend deps + gate
```bash
cd backend
VIRTUAL_ENV=.venv uv sync --frozen --extra dev --inexact  # retain the locally built okc wheel
.venv/bin/python -m ruff check app tests
.venv/bin/python -m mypy app tests
.venv/bin/python -m pytest -q        # includes real-core deterministic-provider publication regressions
```

### 3. Frontend build
```bash
cd frontend
npm install
npm run build     # tsc + vite build → frontend/dist
npm run test      # vitest
```

### 4. Run
```bash
# bootstrap an admin + (optionally) a provider by env, then serve
export OKC_WEB_BOOTSTRAP_ADMIN_EMAIL=admin@example.com OKC_WEB_BOOTSTRAP_ADMIN_PASSWORD=change-me
# optional AI provider (name/endpoint/model + the ENV-VAR NAME of the key — never the key value):
# export OKC_WEB_PROVIDER_ENDPOINT=http://localhost:11434 OKC_WEB_PROVIDER_KIND=ollama OKC_WEB_PROVIDER_MODEL=llama3
cd backend && .venv/bin/python -m uvicorn app.main:create_app --factory --workers 1 --port 8000
# open http://localhost:8000  (serves frontend/dist)
```
For frontend hot-reload during development: `cd frontend && npm run dev` (proxies `/api` and `/u` to `:8000`).

> **Single worker only** (`--workers 1`): the process owns the one write `OkcClient` (okc-core's reservation is process-global). Multiple workers would break the single-writer invariant.

### Configuration (all via env; secrets by **name**, never value)
`OKC_WEB_STATE_DB`, `OKC_WEB_PROJECTS_ROOT`, `OKC_WEB_CORS_ORIGINS`, `OKC_WEB_COOKIE_SECURE`, `OKC_WEB_SPA_DIST`, `OKC_WEB_BOOTSTRAP_ADMIN_*`, `OKC_WEB_PROVIDERS` (JSON) or `OKC_WEB_PROVIDER_*`. AI keys are referenced by env-var **name** (`api_key_env`) and read by the binding when a job runs.

---

## Demo path (§9)
upload (token) → admin login / create project / freeze sources → run integration (provider + remote-disclosure consent) → review (approve / minor-waive / regenerate; blocking findings **refuse compile**) → compile merged vault → serve read-only URL + provenance/verify + okc-mcp contract.

> **Provider note:** the AI-driven middle of the flow (integrate → taxonomy → synthesis → critic → compile) requires a configured LLM provider. Without one, the app still runs and every non-AI seam works; the engine returns real, typed errors (e.g. `needs_provider`, `PROJECT_INVALID`) rather than mocks.

## Design honesty (okc-core hard constraints, surfaced in the UI)
Contradictions are preserved · Major/Critical findings are waive-forbidden and block compile (regenerate only) · ≤10 distinct sources per project (successive revisions do not consume new slots) · freeze-then-run: changing inputs invalidates downstream approvals · read-only compiled vault · retrieval belongs to the implemented sibling okc-mcp module · `curator_id` is an unverified audit label, with actual authorization enforced by okc-web.

---

## How this was built (AI-DLC)
This project was produced with the AI-DLC workflow; the full traceable trail lives in [`aidlc-docs/`](aidlc-docs/): requirements → user stories (29) → application design → units → per-unit functional design + code generation, with every decision logged in [`aidlc-docs/audit.md`](aidlc-docs/audit.md) and state in [`aidlc-docs/aidlc-state.md`](aidlc-docs/aidlc-state.md). See [`aidlc-docs/PROCESS.md`](aidlc-docs/PROCESS.md) for the narrative.

## Testing
Backend: `ruff` + `mypy` + `pytest` using the real okc binding and deterministic loopback-provider integration. Frontend: `tsc` + `vite build` + `vitest`. The root integration test runs actual MCP authoring/reading and Rust hooks wire encoding through web/core. See [`aidlc-docs/construction/build-and-test/`](aidlc-docs/construction/build-and-test/) for current results and limits.

## License / status
Hackathon MVP. okc-core is a separate dependency and is unmodified.
