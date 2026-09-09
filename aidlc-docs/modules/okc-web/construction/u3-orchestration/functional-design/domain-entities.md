# U3 Orchestration — Domain Entities

**Wave**: W2 · **Unit**: U3 (`app/orchestration`) · **Epic**: E3 · **Depth**: Comprehensive, MVP-only

U3's persistent state is the **`projects`** row plus **reads** of the U0-owned
`sources` / `jobs` / `curator_decisions` tables. It introduces **no new table** — the
control-plane schema was created up front in W0 (`MIGRATION_0001_INIT`). This document
fixes the entities, their ownership, invariants, persistence mapping, and the API
contract DTOs that U3 CodeGen will implement.

---

## 1. Entities and value objects

### Project (aggregate root, persisted)
The okc-web registry record for one integration project. Owns the pointer to the
engine project on disk and the freeze bookkeeping.

| Field | Meaning | Source |
|---|---|---|
| `id` | okc-web project id (`proj_<ULID>`) | U3 (created) |
| `name` | display name | admin input |
| `engine_root_abs_path` | absolute root of the okc-core project on disk | U3 (server-generated under `projects_root`, or validated in-root) |
| `curator_id` | unverified curation label (= admin's `curator_label`, E1-S4) | U1 principal |
| `created_by` | account id of the creating admin | U1 principal |
| `source_set_fingerprint` | canonical digest captured at freeze; `NULL` until first freeze | U3 |
| `freeze_state` | `unfrozen` \| `frozen` | U3 |
| `frozen_at` | timestamp of last freeze | U3 |
| `created_at` / `updated_at` | audit timestamps | U3 |

### SourceSetFingerprint (value object, derived)
A **versioned canonical digest** of the project's registered sources, used only for the
advisory freeze/staleness signal (business-rules §3, §7).

- Definition: `SHA-256("okcweb-srcset-v1\n" + "\n".join(sorted("{source_id}:{content_hash}"
  for each row in sources where project_id = P)))`.
- Deterministic (sorted), versioned (prefix allows format evolution), and read-only over
  the U0 `sources` table (written by U2). U3 never writes `sources`.
- Advisory only — it never overrides core's authoritative status (Q6).

### CheckpointProjection (value object, derived, not persisted)
The admin-facing view of the current step, computed from `status()`:

| Field | Meaning |
|---|---|
| `checkpoint` | the raw snake_case core value (source of truth) |
| `next_action` | the one action needed now (see table §4) |
| `resolver` | which unit resolves it (`u3` \| `u4` \| `u5` \| `contributor`) |
| `progression` | ordered human progression for context |
| `blocked_steps` | later steps kept locked |
| `stale` | authoritative-or-advisory staleness (core wins) |

### ProviderBinding (value object, transient)
The selected provider *profile name* bound as the default route for all four AI roles.
No secret value is stored — only the reference (Q4 / A-2). Persistence lives in the
engine project state via `set_ai_route`; okc-web holds no provider secret at rest.

### DisclosureDecision (value object, per-run)
`{ allow_remote_provider, remote_disclosure_confirmed }` — a per-run confirmation, never
a standing grant. Derived from the preflight `routes[].boundary` gate (Q5), then passed
into `integrate`.

### IntegrationRunRef / CompileOutputRef (value objects, derived)
Thin references read back from the engine: `run_id`, `integration_plan_id`, compiled
`path`, `file_count`, corpus/taxonomy hashes. These are the **U5 handoff** contract
(the `serving_publications` row is written by U5 in W3, not by U3).

### JobRef (value object, U0-owned)
The `JobId` + `JobSnapshot` returned/polled for long ops. U3 produces the id; U0 owns
the `jobs`/`job_events` tables and the snapshot shape.

---

## 2. Ownership map (what U3 may write vs read)

| Table | U3 access | Owner / writer |
|---|---|---|
| `projects` | **read + write** | U3 (semantics), U0 (physical schema) |
| `sources` | **read only** (count, fingerprint) | U2 writes at `add_source` commit |
| `jobs` / `job_events` | read (poll) + create-via-enqueue | U0 |
| `curator_decisions` | read (approval history for staleness) | U4/U3 append at decision time |
| `serving_publications` | — | U5 (W3) |
| `accounts` / `sessions` / `upload_tokens` | — | U1 / U2 |

Invariant **BR-U3-OWN**: U3 never writes `sources`, `serving_publications`, or any auth
table; it only writes `projects`. This keeps the frozen W0 seam intact and preserves the
single-writer discipline.

---

## 3. Invariants

- **INV-1 (curator binding):** every project row has a non-null `curator_id` equal to the
  authenticated admin's label at creation (E1-S4 owns the semantics).
- **INV-2 (path containment):** `engine_root_abs_path` and any compile output path resolve
  strictly inside `AppConfig.projects_root` (BR-CP-2, BR-CO-2).
- **INV-3 (freeze fingerprint):** `freeze_state='frozen'` ⇒ `source_set_fingerprint`
  non-null and `frozen_at` set; a live-vs-frozen mismatch ⇒ advisory-stale until re-freeze.
- **INV-4 (core authority):** the reported checkpoint always equals the last
  `status().checkpoint`; U3 stores no derived-state duplicate that could diverge.
- **INV-5 (single reserving mutation):** at most one queued/running reserving job per
  project (BR-RUN-2 + single-writer backstop).
- **INV-6 (no secret at rest):** provider secrets are referenced by env-var name only;
  no secret value is persisted by U3.

---

## 4. Checkpoint → next-action contract

| `checkpoint` (core, snake_case) | next_action | resolver |
|---|---|---|
| `needs_sources` | register sources (upload) | contributor (U2) |
| `needs_provider` | bind a provisioned provider profile | u3 (E3-S4) |
| `needs_disclosure` | run preflight; confirm remote disclosure if any route is remote; integrate | u3 (E3-S4) |
| `needs_taxonomy` | approve taxonomy | u4 (E4-S1) |
| `needs_clusters` | approve / regenerate clusters | u4 (E4-S2..S4) |
| `ready_to_compile` | compile (after E4-S6 gate) | u3 (E3-S6) |
| `verified` | serve / publish | u5 |

---

## 5. API contract DTOs (guides CodeGen; final shapes may refine in code)

Wire models are okc-web-owned Pydantic types; no `okc` binding type appears in U3
request/response bodies (ADR-0002). Engine access is via `app.adapter.dto` + the
`EngineWorker`.

- `CreateProjectRequest { name: str, root: str | None }`
  → `ProjectView { id, name, engine_root_abs_path, curator_id, freeze_state, frozen_at,
    source_count, created_at }`
- `ProjectStatusView { checkpoint: str, next_action: str, resolver: str,
    progression: list[str], stale: bool, integration_plan_id: str | None,
    source_count: int, frozen: bool }`
- `FreezeResponse { freeze_state: str, source_set_fingerprint: str, frozen_at: str,
    source_count: int }`
- `ProviderProfileView { name, kind, endpoint, model }` (list; **no** api-key value)
- `BindProviderRequest { profile_name: str }` → `202/200` ack + refreshed status
- `RunIntegrationRequest { allow_remote_provider: bool = False,
    remote_disclosure_confirmed: bool = False }` → `JobAccepted { job_id, project_id }`
- `PreflightView` (reuse `adapter.dto.PreflightView`) surfaced as a disclosure-gate view:
    `{ sensitive_findings, routes: [{role, profile_name, boundary}], requires_disclosure: bool }`
- `CompileRequest { output_path: str | None }` → `CompileResultView { path,
    integration_plan_id, file_count }`
- Errors: the shared `{ code, category, message, retryable, retry_after_ms? }` body
  (`app/shared/error.py`); status derived from `code`.

Routes (admin-gated unless noted) — exact prefixes finalized in CodeGen, consistent with
existing `/api/projects/{project_id}/...`:
`POST /api/projects` · `GET /api/projects/{id}` · `GET /api/projects/{id}/status` ·
`POST /api/projects/{id}/freeze` · `GET /api/projects/{id}/providers` ·
`POST /api/projects/{id}/provider` · `POST /api/projects/{id}/preflight` ·
`POST /api/projects/{id}/integrate` · `POST /api/projects/{id}/compile`.
Job polling reuses the U0 `GET /api/projects/{id}/jobs/{job_id}`.

---

## 6. Persistence mapping summary

All U3 persistence maps onto the **existing** `projects` columns (§1). No migration is
added; W0 froze the schema. The `sources`, `jobs`, `job_events`, `curator_decisions`, and
`serving_publications` tables are read-only or owned by other units per §2.
