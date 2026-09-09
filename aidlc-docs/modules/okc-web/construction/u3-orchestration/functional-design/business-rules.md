# U3 Orchestration — Business Rules

**Wave**: W2 · **Unit**: U3 (`app/orchestration`) · **Epic**: E3 · **Depth**: Comprehensive, MVP-only

Rules are stated as testable preconditions → outcome, with the stable
`EngineErrorCode` each rule raises (HTTP status is derived from the code by
`app/shared/error.py`, never by parsing a message). All codes below already exist in
`EngineErrorCode`; U3 adds no new error code.

---

## 1. RBAC-before-core (C-1, invariant BR-U3-RBAC)

**Every mutating U3 operation is `Depends(admin_context)` + `require(ctx, (Role.ADMIN,))`
before any engine method is reached.** Because authz runs inside a FastAPI dependency,
a 401/403 raised there means the route body — the sole engine-dispatch site — never
runs, so okc-core is untouched.

| Class | Operations | Rule |
|---|---|---|
| Mutating (gated) | create project, freeze/re-freeze, run integrate, set provider, confirm disclosure, compile | contributor → **403 `FORBIDDEN`**; unauthenticated → **401 `UNAUTHENTICATED`/`SESSION_EXPIRED`**; core not called |
| Read (not gated by role) | status/checkpoint projection, project detail, job snapshot | admin session required for the admin console; not blocked by the *mutation* gate |

`curator_id` is bound from the authenticated admin (semantics owned by **E1-S4**; E3-S1
only *applies* the binding). It is an unverified label to core — never a trust boundary.

---

## 2. Project creation (E3-S1)

| # | Precondition | Outcome | Code on failure |
|---|---|---|---|
| BR-CP-1 | admin authenticated + `Role.ADMIN` | proceed | 401/403 (rule 1) |
| BR-CP-2 | `name` non-empty; `root` absent or resolves **inside** `projects_root` | create at a server-generated absolute root under `projects_root/{project_id}` (or the validated explicit path) | `VALIDATION_FAILED` (empty name) / `PATH_UNSAFE` (escapes root) |
| BR-CP-3 | engine `create_project(spec)` succeeds | persist a `projects` row: `id`, `name`, `engine_root_abs_path`, `curator_id`=admin label, `created_by`=account id, `freeze_state='unfrozen'` | binding faults mapped by adapter (`PROJECT_INVALID`, `PATH_*`) |
| BR-CP-4 | — | response states `curator_id` is a curation label, not publisher-authenticity; real gating is okc-web RBAC | — |

---

## 3. Source set & freeze (E3-S2)

| # | Precondition | Outcome | Code on failure |
|---|---|---|---|
| BR-FR-1 | project exists | reads live `sources` count/rows (owned physically by U0 schema, written by U2) | `NOT_FOUND` (no project) |
| BR-FR-2 | source count ≥ 1 and ≤ 10 | allow freeze | `VALIDATION_FAILED` (zero sources) |
| BR-FR-3 | 11th source add attempted | okc-web cap check rejects **before** core; message names "max 10 sources (federation not implemented)". (Enforced primarily in U2's `SlotAccountant`; U3 relies on it and re-reads count.) | `SOURCE_CAP_EXCEEDED` (429) |
| BR-FR-4 | freeze requested | capture canonical `source_set_fingerprint`, set `freeze_state='frozen'`, `frozen_at=now` | — |
| BR-FR-5 | run/compile requested while `freeze_state='unfrozen'` OR live fingerprint ≠ frozen fingerprint | refuse; require (re-)freeze first | `VALIDATION_FAILED` (not frozen) |
| BR-FR-6 | valid upload arrives after freeze | allowed (U2); project becomes advisory-stale on next read (re-freeze required) — no retroactive snapshot rewrite (Q1) | — |

---

## 4. Run orchestration & checkpoint projection (E3-S3)

| # | Precondition | Outcome | Code |
|---|---|---|---|
| BR-RUN-1 | frozen project, run requested | read `status().checkpoint`; expose current step + human progression; lock later steps | — |
| BR-RUN-2 | a reserving job for this project is queued/running | reject the new reserving mutation **before enqueue** | `PROJECT_BUSY` (409, retryable, `Retry-After`) |
| BR-RUN-3 | checkpoint requires another unit (`needs_taxonomy`→U4, `needs_clusters`→U4) | U3 surfaces the step but does **not** perform the resolving action | — |
| BR-RUN-4 | `status()` interop_schema_version ≠ 2 | adapter `SchemaGuard` rejects at the seam | `SCHEMA_UNSUPPORTED`/`ARTIFACT_SCHEMA_UNSUPPORTED` |

The checkpoint value is **always** taken from core (never reconstructed). Derivation
precedence and the seven snake_case values are documented in business-logic-model §2.

---

## 5. Provider binding (E3-S4, NeedsProvider)

| # | Precondition | Outcome | Code |
|---|---|---|---|
| BR-PV-1 | selected profile name ∈ `AppConfig.providers` | `set_ai_route(root, name, role=None)` → default route for all roles | `VALIDATION_FAILED` (unknown profile) / `PROVIDER_INVALID` (core) |
| BR-PV-2 | request carries a raw endpoint or secret value | reject — only pre-provisioned profile *names* are accepted (A-2) | `VALIDATION_FAILED` |
| BR-PV-3 | bind succeeds | next `status()` advances past `needs_provider` | — |

## 6. Remote disclosure gate (E3-S4, NeedsDisclosure)

| # | Precondition | Outcome | Code |
|---|---|---|---|
| BR-DIS-1 | run preflight | obtain `routes[].boundary` as truth | provider faults mapped by adapter |
| BR-DIS-2 | any `boundary == "remote"` AND admin did **not** confirm both booleans | block; **no remote call is made** | `REMOTE_CONSENT_REQUIRED` (422) |
| BR-DIS-3 | any `boundary == "remote"` AND both `allow_remote_provider` + `remote_disclosure_confirmed` true | enqueue `integrate` with the confirmed disclosure | — |
| BR-DIS-4 | all routes `"local"` | force both disclosure booleans **false**; enqueue `integrate` | — |
| BR-DIS-5 | core still detects sensitive-remote violation | core rejects after job start | `SENSITIVE_REMOTE_FORBIDDEN` (403) |

---

## 7. Staleness precedence (E3-S5)

| # | Precondition | Outcome | Code |
|---|---|---|---|
| BR-ST-1 | live fingerprint ≠ frozen fingerprint | advisory-stale warning; re-freeze required | — (warning) |
| BR-ST-2 | core returns `APPROVAL_STALE` **or** checkpoint regressed | **authoritative** stale — wins over any advisory result | `APPROVAL_STALE` (422) on compile |
| BR-ST-3 | compile attempted while stale | refuse compile; route admin back to the affected step (gate detail = E4-S6) | `APPROVAL_STALE` / `APPROVAL_REQUIRED` |
| BR-ST-4 | advisory green but core stale | project is stale (core wins); advisory never overrides core to "ready" | — |

---

## 8. Compile (E3-S6, normal path only)

| # | Precondition | Outcome | Code |
|---|---|---|---|
| BR-CO-1 | `status().checkpoint == ready_to_compile` and not stale | proceed | `APPROVAL_REQUIRED` (not ready) |
| BR-CO-2 | output path = fresh per-run child under `projects_root` (or validated explicit in-root path) | never overwrite existing output | `PATH_UNSAFE` (escapes root) |
| BR-CO-3 | target path already exists | do not clobber; core backstop | `OUTPUT_EXISTS` / `OUTPUT_OVERLAP` (409) |
| BR-CO-4 | compile succeeds | return `CompileView` bound to freeze-time corpus hash + `integration_plan_id` (U5 handoff) | `OUTPUT_DURABILITY_UNCERTAIN` (500, core) |

Blocking-findings refusal (unapproved Major/Critical) is **E4-S6**, not duplicated here.

---

## 9. Progress & error surfacing (E3-S7)

| # | Rule |
|---|---|
| BR-ER-1 | Long ops return a `JobId`; progress polled via the U0 canonical job route; UI updates phase/state from the `JobSnapshot`. |
| BR-ER-2 | Errors are branched on `code`/`category` only (never message-string parsing). Adapter has already mapped every binding `OkcError` to an `EngineError`. |
| BR-ER-3 | `PROJECT_BUSY` (and `RESOURCE_LIMIT`) are presented as **retryable** with a `Retry-After`; UI offers a retry/queue action. Retryability is decided by **code**, not the binding's unreliable `retryable` flag. |
| BR-ER-4 | An unexpected worker exception never loses a job — it is recorded as terminal `INTERNAL` on the `JobStore` (U0 behavior U3 relies on). |

---

## 10. Edge cases

- **Zero sources at run:** `needs_sources`; run is blocked with a clear "add sources"
  action (BR-FR-2).
- **Freeze then unfreeze-by-upload then re-freeze:** allowed; each freeze re-captures the
  fingerprint; prior approvals invalidated by core hash-bound rule (surfaced, not
  mutated by U3).
- **Provider profile removed from config between binding and run:** `needs_provider`
  reappears on the next `status()`; U3 re-prompts for binding.
- **Duplicate integrate double-click:** second call hits BR-RUN-2 `PROJECT_BUSY` before
  enqueue; single-writer is the backstop.
- **Compile on stale approvals:** BR-ST-3 refuses with `APPROVAL_STALE`/`APPROVAL_REQUIRED`.
- **Explicit output path outside `projects_root`:** `PATH_UNSAFE`, before core.
- **Schema drift (interop ≠ 2):** rejected at the adapter `SchemaGuard`, never reaches U3
  business logic.

---

## 11. Extension compliance

Security Baseline, Resiliency Baseline, and Property-Based Testing are **disabled** in
`aidlc-state.md` → full rule files not loaded → **N/A** for this stage. Baseline input
validation (path-in-root, non-empty name, profile allowlist, cap re-read) and
real-binding tests remain ordinary project requirements, applied above.
