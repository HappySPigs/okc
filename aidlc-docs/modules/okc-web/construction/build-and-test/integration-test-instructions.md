# Integration Test Instructions (W5)

Integration here = the cross-unit spine through the **production app + real okc binding**, plus the frontend↔backend contract.

## 1. Automated no-mock spine (offline, in CI)
`tests/test_w1_spine.py` exercises the real cross-unit path with no mock:
bootstrap admin → **U1** login (cookie) → **U2** issue token → contributor upload → **U0** single-writer worker → native `okc.add_source` → authenticated job polling → **U3** `SourceRegistry` + fresh engine manifest read.
```bash
cd backend && .venv/bin/python -m pytest tests/test_w1_spine.py -q
```

## 2. AI-driven runtime spine (provider-dependent — MANUAL)
The full `create → freeze → integrate → (taxonomy → clusters review) → compile → publish → serve` path drives okc-core's AI stages (embedding/organizer/synthesis/critic) and therefore needs a **configured LLM provider**. It is NOT run in CI (no provider). To run locally:
```bash
# 1. configure a provider (e.g. local ollama), then start the app (build-instructions §4)
export OKC_WEB_PROVIDER_ENDPOINT=http://localhost:11434 OKC_WEB_PROVIDER_KIND=ollama OKC_WEB_PROVIDER_MODEL=llama3
# 2. via the SPA or curl (cookie from login):
#    POST /api/projects → POST /api/projects/{id}/tokens → POST /u/{token}/upload
#    POST /api/projects/{id}/freeze → POST /api/projects/{id}/provider {profile_name}
#    POST /api/projects/{id}/preflight → POST /api/projects/{id}/integrate  (poll /jobs/{id})
#    GET  /api/projects/{id}/taxonomy → POST .../taxonomy/approve
#    GET  .../clusters → POST .../clusters/{cid}/approve | regenerate
#    POST /api/projects/{id}/compile   (checkpoint must be ready_to_compile)
#    POST /api/projects/{id}/serving/publish → GET /api/serving/{id}/files|verify|explain|contract
```
Verify each hop with `GET /api/projects/{id}/status` (checkpoint should advance `needs_provider → needs_disclosure → needs_taxonomy → needs_clusters → ready_to_compile → verified`).

**Boundary behavior proven offline** (no provider): the engine returns real typed errors at each unmet precondition (`needs_provider`, `PROJECT_INVALID`, `APPROVAL_REQUIRED`, `VERIFICATION_FAILED`) — the adapter maps them to the stable `{code, category}` contract, confirming the ADR-0002 seam end-to-end without faking AI output.

## 3. Frontend ↔ backend contract
- Dev: `cd frontend && npm run dev` proxies `/api` and `/u` to `:8000`; exercise the demo path in the browser.
- Prod: `npm run build` then serve via the backend; the SPA history-fallback makes deep-links/refresh and the contributor `/upload/{token}` entry load correctly (`tests/test_spa_fallback.py`).
- Error contract: the SPA branches on `{code, category}` (401→login, `PROJECT_BUSY`→retry, validation→inline) — never on the message string.

## 4. Hooks receiver and repeated source revisions

Use [continuous-sync verification](continuous-sync-verification.md) for actual
Rust CBOR fixtures through authenticated HTTP routes and native source rebinding.
Commit responses indicate completed registration; semantic review and publication
remain the separate flow in section 2.
