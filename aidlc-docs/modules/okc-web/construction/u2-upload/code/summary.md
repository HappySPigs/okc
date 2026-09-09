# Code Generation Summary — U2 (Upload & Token)

**Wave**: W1 · **Unit**: U2 (`app/upload/`) · **Depth**: CodeGen comprehensive, MVP-only  
**Status**: COMPLETE · GREEN (independently re-verified by orchestrator)  
**Stories**: E2-S1..E2-S6 only. No scope expansion.

## Pattern

Admin token lifecycle is DB-only and protected by U0's admin dependency. Contributor upload is capability-authenticated before the handler reaches the engine. The only engine mutation is `add_source`, dispatched through U0's single-writer worker; the `sources` row and token usage marker are written only after the real binding call succeeds.

## Files created

| File | Responsibility |
|---|---|
| `backend/app/upload/models.py` | Pydantic request, response, validation, token-list, and ingest-receipt DTOs |
| `backend/app/upload/tokens.py` | Split-token generation and verification, token repository, lifecycle service, resolver seam |
| `backend/app/upload/ingest.py` | Streaming receiver, archive validation, atomic in-flight slot accounting, landing, source registry, ingest orchestration |
| `backend/app/upload/router.py` | Admin token routes, contributor upload routes, and `register(app, state)` wiring |
| `backend/tests/test_u2_upload.py` | 14 U2 tests with a real `okc` binding and real `add_source` path |
| `aidlc-docs/construction/plans/u2-upload-code-generation-plan.md` | Executed CodeGen plan with every step marked `[x]` |

## HTTP endpoints

- `POST /api/projects/{project_id}/tokens`
- `GET /api/projects/{project_id}/tokens`
- `POST /api/projects/{project_id}/tokens/{token_id}/revoke`
- `POST /api/projects/{project_id}/tokens/{token_id}/rotate`
- `GET /u/{token}`
- `POST /u/{token}/upload`
- U0-owned polling route used by U2: `GET /u/{token}/jobs/{job_id}`

## Key decisions and invariants

- Token format is `selector.verifier`; both parts use 256-bit CSPRNG input. Only the selector and salted SHA-256 verifier digest are persisted. Plaintext is returned once.
- Token issue selects the lowest free slot while holding an in-process lock. Rotation invalidates the old secret and reuses its slot.
- Upload reservations include committed and in-flight slots, so concurrent requests cannot receive the same slot and the ten-source cap remains enforceable before core.
- Size, format, traversal, symlink, archive expansion, duplicate-content, and owner-kind checks occur before `add_source` whenever determinable.
- Duplicate-content and cap checks run again on the serialized engine worker to close the pre-check race. A rejected queued duplicate releases its reservation and removes its unregistered landing directory.
- Successful `add_source` is proven against a fresh real `okc` client; no engine mock is used.

## Verification

- `ruff check app/upload tests/test_u2_upload.py` → `All checks passed!`
- `mypy app/upload` → `Success: no issues found in 5 source files`
- `pytest tests/test_u2_upload.py -q` → `14 passed`
- Known non-blocking warnings: two upstream Starlette/httpx deprecation warnings.

## Judging-criteria touchpoints

- **C1**: E2-S1..E2-S6 trace directly to the U2 plan, routes, tests, and this summary.
- **C4**: real login-independent upload unit path reaches native `okc.add_source`, persists `SourceRegistry`, and verifies the manifest with a fresh client.
- **C5**: stable one-time token, validation codes, slot usage, and pollable job receipt support the frozen contributor flow.
- **C6**: secrets are hashed at rest, hostile archives are rejected, concurrent slots are serialized, and binding access stays behind U0's adapter.
