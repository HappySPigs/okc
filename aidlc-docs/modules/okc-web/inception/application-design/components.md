# okc-web — Application Design: Components

**Stage**: INCEPTION / Application Design (Part 2 — synthesis) · Depth: comprehensive
**Scope**: Backend module/component definitions for units U0–U5, plus a U6 frontend-wiring component-group reference. Method signatures live in the companion *Component Methods* doc; orchestration lives in *Services*; wiring/matrix live in *Application Design*.
**Grounding**: okc Python bindings verified at `okc-core/bindings/python/okc/__init__.pyi` (INTEROP_SCHEMA_VERSION=2). UI is frozen by `aidlc-docs/inception/application-design/ui-screens.md` + `design-system.md` — this document defines the API-wiring contract only and does **not** redesign UI.

> **Verifier corrections applied (see Application Design §"Applied Corrections" for the full list):** Protocol gains `preflight` + `status`; `create_project`/`open_project` split; `Role.{Admin,Contributor}` (no `Curator`/`UploadAgent`); `Principal.UploadToken(UploadContext)`; single `EngineError` at the adapter boundary (no `okc` binding type past `adapter`); one project-scoped job-status route; non-reserving reads bypass the queue via a read-path second client; unified verify/explain DTO names (`VerificationView`/`ProvenanceView`); explicit `sources` registry owner; `curator_decisions` carries all three hash bindings; extended HTTP mapping table.

---

## Module ⇄ Unit map (Q1, 1:1 traceability)

| Module | Unit | Epic | Concern |
|---|---|---|---|
| `adapter` (+ `adapter.queue`) | U0 | E3 support | sole `okc` bindings link; Protocol, DTOs, schema guard, engine actor/queue |
| `shared` (`authz`, `error`, `jobs`, `audit`, `state`) | U0 | cross | RBAC guard, HTTP error mapping, job store, curator audit, SQLite pool + registries |
| `auth` | U1 | E1 | accounts, roles, sessions, password hashing |
| `upload` | U2 | E2 | tokens, token-auth upload, validation, source landing, ≤10 cap |
| `orchestration` | U3 | E3 | project lifecycle, freeze, checkpoint loop, compile, staleness projection |
| `review` | U4 | E4 | taxonomy/cluster decision surface, DecisionGate, regenerate, compile-gate scoreboard |
| `serving` | U5 | E5 | read-only compiled-vault API, verify/explain provenance, publish state, mcp contract |
| `web` (React + Vite SPA) | U6 | cross | API-consumption layer (client, error map, query hooks, job polling) |

---

## U0 — `adapter` + cross-cutting `shared` (Foundation)

U0 is the SOLE point that imports the `okc` Python bindings. Every other unit depends only on the `OkcEngine` Protocol, okc-web-owned typed DTOs, `EngineError`/`ApiError`, the `JobStore` polling surface, and the authz guard — **never on `okc.*`**.

### C0.1 `adapter.OkcEngine` (Protocol/ABC) — sole engine boundary
- **Responsibility**: the one abstract seam over okc-core. Declares async, okc-web-DTO-typed operations for the whole lifecycle. No caller outside `adapter` may name an `okc.*` type (ADR-0002).
- **Interface**: `class OkcEngine(Protocol)` — `async def` methods. Read ops return a `*View` DTO; long mutations return `JobId`; fast mutations return `JobOutcome`. **All errors are the okc-web-owned `EngineError`** (never `okc.OkcError`).
- **Corrected surface** (added `preflight`, added `status`, split `create_project`/`open_project`): `create_project`, `open_project`, `status`, `taxonomy`, `clusters`, `manifest`, `verify`, `explain`, `add_source`, `replace_sources`, `preflight`, `integrate`, `approve_taxonomy`, `approve_cluster`, `regenerate_cluster`, `compile`.

### C0.2 `adapter.OkcEngineImpl` — the single concrete impl
- **Responsibility**: the only class holding an `okc.OkcClient`. Translates Protocol calls into binding calls and converts binding result/error types into okc-web DTOs/`EngineError` at this boundary. Holds no engine lock: the `EngineActor` owns the live handle; `OkcEngineImpl` holds a shareable `EngineHandle` (for reserving/mutating ops) **plus a read fast-path second `OkcClient`** used for non-reserving reads (queue-bypass, see Services §read-path policy).
- **Interface**: `class OkcEngineImpl(OkcEngine)`; `OkcEngineImpl(handle: EngineHandle, reader: OkcClient, guard: SchemaGuard)`.

### C0.3 `adapter.dto` — okc-web-owned typed DTOs (binding schema v2 mirror) + conversions
- **Responsibility**: define okc-web's own Pydantic DTOs and `from_native(dict)`/`model_validate` conversions; the only translation layer that references binding result types. Downstream units decouple from binding churn behind this boundary.
- **DTOs**: `ProjectRef`, `CreateProjectSpec`, `SourceSpec`, `DisclosureDecision`, `CheckpointView` (wraps the derived 7-variant `IntegrationCheckpoint (NeedsProvider, NeedsSources, NeedsDisclosure, NeedsTaxonomy, NeedsClusters, ReadyToCompile, Verified)`), `IntegrationStatusView`, **`StatusView(checkpoint, integration)`** (added so U3 gets `IntegrationStatus` from the single reserving `status()` call), `TaxonomyView`, `ClusterReportView`, `ClusterReviewView`, `ManifestView`, `CompileView`, **`VerificationView`** (verify — canonical, was U5 `VerifyReportDto`), **`ProvenanceView`** (explain — canonical, == `ProvenanceRecord`, was U5 `ProvenanceDto`), `ProgressView`, `ApproveClusterCmd`, `RegenerateClusterCmd`, `ClusterEdit`, `JobOutcome`, `JobId`.
- **Naming reconciliation**: `ProjectRef` = engine-level handle (canonical absolute root path + engine identity) returned by the adapter; `ProjectId` = okc-web SQLite project PK. Services take `ProjectId` and resolve to `ProjectRef`. `DisclosureDecision` replaces the earlier `RemoteConsent` and is shared by `integrate` (U3) and `regenerate_cluster` (U4).

### C0.4 `adapter.schema_guard.SchemaGuard` — single schema-version guard point
- **Responsibility**: the ONE place asserting compatibility with `okc.INTEROP_SCHEMA_VERSION` (==2). Checked at client init against `OkcClient.api_info().interop_schema_version` and again at each DTO decode boundary. Mismatch aborts startup or yields `EngineError(code="SchemaUnsupported")` rather than mis-decoding.
- **Interface**: `assert_client(client: okc.OkcClient) -> None`; `check_payload(version: int) -> None` (both raise `EngineError` on mismatch).

### C0.5 `adapter.queue.EngineActor` + `EngineHandle` — single-owner worker (NFR-CONC-1)
- **Responsibility**: a dedicated single worker thread (`ThreadPoolExecutor(max_workers=1)`) that exclusively owns the process's single `OkcClient` and a cache of `okc.Project` handles keyed by canonical path. Receives `EngineCommand`s submitted by async handlers; processes them ONE AT A TIME; drives each `okc.Job[_T]` to terminal (pumping `Job.events()` into the `JobStore`, then reading the blocking `Job.result()` on the worker thread, GIL released by the native ext). Serializes **only reserving/mutating** engine ops (all mutations plus the reserving `status()`); non-reserving reads take the read-path second client and never enqueue (avoids head-of-line blocking behind a multi-minute `integrate`/`compile`).
- **Interface**: `EngineHandle` (shareable, held by handlers); `async EngineHandle.call(op) -> EngineValue` (submit + await terminal via `run_in_executor`, fast ops); `async EngineHandle.enqueue(op) -> JobId` (id immediately, worker runs the job, client polls); `EngineActor.spawn(client, jobs, audit) -> EngineHandle`.
- **Code-verified rationale**: `okc.Job.result()` blocks (the native ext releases the GIL) → the owner MUST be a dedicated worker thread bridged via `run_in_executor`, not an asyncio task on the event loop (reconciles Q6 "one task owns the handle"). Concurrency is also enforced by a process-global `PROJECT_RESERVATIONS`; keeping exactly one `OkcClient` here makes the okc-web queue the primary serializer and `PROJECT_BUSY→409` the residual safety net.

### C0.6 `shared.jobs.JobStore` — job state + progress (FR-INT-8 / Q7)
- **Responsibility**: authoritative okc-web job state persisted in SQLite (`jobs` + `job_events`) plus an in-memory live snapshot. Updated by `EngineActor` from binding `ProgressEvent`s (coalesced upstream at 64). Read by the single project-scoped job-status endpoint that TanStack Query polls.
- **Interface**: `create(project, operation) -> JobId`; `record_progress(job_id, progress: ProgressView)`; `record_terminal(job_id, outcome)`; `get(job_id) -> JobSnapshot | None`. `JobSnapshot(id, project, operation, state: JobStatus, phase, completed, total, current_item, error: ApiErrorBody | None, updated_at)`; `JobStatus` mirrors `okc.JobState` `{Queued, Running, Cancelling, Publishing, Completed, Failed, Cancelled}`.

### C0.7 `shared.authz` — RBAC-before-core guard (C-1)
- **Responsibility**: FastAPI `Depends(...)` dependencies that authenticate the caller and enforce role BEFORE the handler runs, so a denied request never reaches an engine call (a 403 guarantees okc-core was not touched). U0 owns the enforcement machinery + ordering guarantee; U1/U2 supply concrete validators via an injected `AuthProvider`.
- **Canonical types (corrected)**:
  - `class Role(enum.Enum)` with members `Admin`, `Contributor` — no `Curator`, no `UploadAgent`. "Curator" is the capacity in which an admin acts, recorded as okc-core's unverified `curator_id` label (FR-AUTH-4). Upload is a *capability*, modeled as `Principal.UploadToken`, not a role.
  - `Principal` = a discriminated union (tagged by `kind`): `Admin(account_id, session_id_hash, curator_label) | UploadToken(UploadContext)`.
  - `class AuthContext(BaseModel): principal: Principal; role: Role` — the session-authenticated principal for admin routes (U0-owned canonical type; U1's `account_id`/`curator_label`/`session_id_hash` fold into `Principal.Admin`).
  - `class UploadContext(BaseModel)` with fields `token_id, project_id, slot_index, owner_display_name, owner_kind` — the concrete payload of `Principal.UploadToken` (single source of truth; U2's `/u/{token}/*` dependency yields this).
- **Interface**: `require_role(*roles)` — a `Depends` dependency factory; `AuthContext` and `UploadContext` resolved as `Depends(...)` dependencies (401 if unauth, 403 if role insufficient); `require(ctx, allowed: list[Role]) -> None` (raises `ApiError`).

### C0.8 `shared.error.ApiError` — the single OkcError→HTTP mapping (Q9)
- **Responsibility**: the ONE module converting `EngineError` + okc-web-origin errors into HTTP responses, branching on `code`/`category` and NEVER on message strings. Emits a stable JSON body `{ code, category, message, retryable, retry_after_ms? }`. Synthesizes `Retry-After` for `PROJECT_BUSY`/`RESOURCE_LIMIT` by **code** (the binding's `retryable` flag is inconsistent for reservation-collision `ProjectBusy`, so it is never trusted).
- **Interface**: `class ApiError(Exception)` with variants `Unauthenticated | Forbidden | NotFound | MethodNotAllowed | Validation(str) | Engine(EngineError)`; `ApiError.from_engine(EngineError)`; a FastAPI `@app.exception_handler(ApiError)`; `http_status(code: EngineErrorCode, category: EngineErrorCategory) -> int` (extended table dict incl. `ProjectInvalid→422`, `ArtifactSchemaUnsupported→422`, `PathUnsupported→400`, `OutputDurabilityUncertain→500`, safe default 500).

### C0.9 `shared.audit.AuditStore` + `CuratorDecision` — append-only decision record (Q8, C3 differentiator)
- **Responsibility**: append-only SQLite store recording each curator decision as an explicit typed event BEFORE it is mapped to a binding call, capturing principal, project, target, and **all three hash bindings** (`proposal_hash`, `critic_hash`, `taxonomy_hash`) for staleness/provenance (C-4), plus the resulting `JobId`. Records are never updated or deleted. **Single owner = U0** (`shared.audit`); records are *produced by* U4 and U3.
- **Interface**: `CuratorDecision` = a discriminated union (tagged by `kind`) `ApproveCluster(cluster_id, omission_rationales, minor_waivers) | RegenerateCluster(cluster_id, feedback) | ApproveTaxonomy(edited_clusters, rationale)` — **deliberately NO winner-select variant** (ADR-0024, C-2; a verifiable C3 structural differentiator, confirmed by the verifier in both U0 and U4). `append(project, principal, decision, HashBindings) -> AuditRecord`; `list_for_project(project) -> list[AuditRecord]`. `class HashBindings(BaseModel): proposal_hash: str | None; critic_hash: str | None; taxonomy_hash: str | None`.

### `shared.state` — SQLite substrate (Q3)
- **C0.10 `StateDb`** — a single **SQLAlchemy 2.0 `Engine`** over `sqlite3` (`PRAGMA journal_mode=WAL`), shared by every repository; repositories run inside `engine.begin()`. Single-process topology → transactional integrity without external coordination.
- **C0.11 `Migrations`** — an ordered DDL runner (`0001_init`, …; no Alembic for MVP) creating all tables + indices (see Application Design §"SQLite State Data Model").
- **C0.12 `SourceRegistry`** (repository over the `sources` table) — **resolves the previously dangling U5 dependency**. Maps okc-core `SourceId`/`DocumentId → owner_display_name` (+ `owner_kind`, `content_hash`, `absolute_path`, `slot_index`). **Owner = U3 orchestration** (project-registry domain); **written by U2** at `add_source` commit; **read by U5** `ProvenanceComposer` for owner-label enrichment. Interface: `record(row: SourceRow)`, `owner_labels(ids: list[SourceId]) -> OwnerLabelMap`, `list_for_project(project: ProjectId)`.

---

## U1 — `auth` (Epic E1)

- **`AccountStore`** (repository over `accounts`): durable CRUD; role assignment; activation toggle; last-login stamp; active-admin count (last-admin guard). Pure persistence, no policy. Returns typed `AccountRecord`/`AccountSummary`.
- **`PasswordHasher`** (in `shared`, first consumed by U1): argon2id via `argon2-cffi` (PHC), embedded per-hash salt + tuned params, constant-time verify (NFR-SEC-1). Plaintext lives only in a transient local `str` (never logged or persisted).
- **`SessionStore`** (repository over `sessions`): create opaque server-side sessions (returns plaintext id ONCE, stores only SHA-256 lookup hash), resolve by hashed id with TTL/idle expiry, sliding renewal, single + bulk revoke (on disable / password change).
- **`SessionCookieCodec`**: build/clear the auth cookie — `HttpOnly; SameSite=Lax; Secure; Path=/`, value = opaque id only; parse the raw id from headers.
- **`AuthProvider` impl (session)**: U1's concrete resolver injected into U0's `authz`. `async resolve_session(raw_cookie_id) -> AuthContext | None` performs the hashed lookup + role/curator-label bind, producing `Principal.Admin(...)`. (Authentication = U1; role enforcement = U0.)
- **`AuthService`** (orchestrator): login/logout/session-resolve + account admin (create/set_role/set_status/change_password/list) with email uniqueness, last-admin guard, and session revoke-all on disable/password-change.

---

## U2 — `upload` (Epic E2)

- **`UploadTokenStore`** (repository over `upload_tokens`): persist issued tokens (public `selector` + secret `verifier_hash`), lookup-by-selector, mark-used (bind SourceId), revoke, list-per-project with filters, active-slot counting for the ≤10 cap.
- **`UploadTokenSecret`** (value object/crypto): split-token generation — CSPRNG `selector` (public, indexed) + `verifier` (≥256-bit secret); verifier hashed at rest (argon2id or HMAC-SHA256 + server pepper + per-row salt); format `selector.verifier` shown as plaintext exactly once at issue (E2-2); parse presented tokens back into `(selector, verifier)`. O(1) lookup by selector, one hash-verify per request.
- **`AuthProvider` impl (token)** / **`UploadContext` dependency** (`Depends(...)`, guards `/u/{token}/*`): parse token → selector lookup → constant-time verifier check → status/expiry/revocation/slot checks → yield `UploadContext` (== `Principal.UploadToken` payload). Emits `TOKEN_INVALID`/`TOKEN_EXPIRED`/`TOKEN_REVOKED`/`SOURCE_CAP_EXCEEDED` BEFORE any core call (C-1).
- **`UploadReceiver`**: stream the multipart body to a temp staging path enforcing a hard byte cap *during* streaming; reject over-cap early (`UPLOAD_TOO_LARGE`).
- **`ArchiveValidator`**: inspect the staged artifact → structured `ValidationReport` matching the E2-4 checklist: (1) format = zip/tar.zst only, (2) size cap, (3) markdown-ratio (non-blocking warn), (4) path safety (reject `..`/absolute), (5) symlink rejection with offender list, (6) zip-bomb via declared-vs-actual ratio. okc-web's early guard honestly duplicating okc-core hostile-input defenses (FR-UP-3); core remains the backstop.
- **`SourceLander`**: materialize validated bytes to a deterministic ABSOLUTE landing path under the project sources root (C-6: disk THEN `add_source`), compute content hash for dedup, report file_count/md_count (E2-5).
- **`SlotAccountant`**: enforce the ≤10 cap at the okc-web layer BEFORE the core call — atomic reserve (fail `SOURCE_CAP_EXCEEDED` when active ≥10), commit on success (bind SourceId → `SourceRegistry`), release on validation/land failure. okc-core `ResourceLimit` is the backstop.
- **`UploadTokenService`** (admin-facing orchestrator) and **`UploadIngestService`** (token-facing orchestrator) — see Services.

---

## U3 — `orchestration` (Epic E3)

- **`ProjectLifecycleService`**: owns the `projects` registry rows (id, engine root abs-path, name, unverified `curator_id` label per C-1, created_by admin, source-set fingerprint, freeze state). Handles create/open and the source-freeze transition. Never calls the engine for authz — RBAC runs in `shared.authz` first. Interface incl. `create_project`, `open_project_ref` (ProjectId → ProjectRef), `list_projects`, `get_project_overview`, `freeze_sources`.
- **`PipelineService`**: drives the DERIVED `IntegrationCheckpoint` loop, treating checkpoint as engine-owned truth re-derived each `status()` call (never stored in okc-web). Builds the E3-3 Next-Action card and E3-5 gates from **`StatusView(checkpoint, integration)`** (the corrected Protocol op supplies both `IntegrationCheckpoint` and `IntegrationStatus` in one reserving call). Owns Provider/Disclosure surface and the long-running `preflight`/`integrate` enqueues.
- **`DisclosureDecision`** (value type + policy): okc-web representation of `(allow_remote_provider, remote_disclosure_confirmed)`, collected in E3-5 and flowed into `integrate` (U3) and `regenerate_cluster` (U4). Encapsulates the local-provider "disclosure not required" branch. (Shared type, defined in `adapter.dto`.)
- **`CompileService`**: compile execution (no-clobber). Re-checks `checkpoint == ReadyToCompile` via `PipelineService.status()` before enqueuing `compile`; surfaces `CompileView` (artifact path, integration_plan_id, file_count, manifest). Trigger button lives on E5-1 but calls this U3 service; E4-1 is the read-only gate.
- **`StalenessProjection`**: owns the DERIVED, NON-AUTHORITATIVE freeze-then-run staleness label. Compares the source-set/config fingerprint recorded at each approval (in the audit store) against the current fingerprint to pre-warn (E3-6/E3-4/E4-1 banners). Authoritative staleness stays in okc-core (hash binding) and surfaces as `ApprovalStale` at run/approve/compile time; on any engine `ApprovalStale`, okc-web defers to core.
- **`OrchestrationRoutes`** (FastAPI routers): E3 REST endpoints, cookie-session + admin-guarded. Includes the canonical **`GET /api/projects/{id}/jobs/{jobId}`** job-status poll (Q7).

---

## U4 — `review` (Epic E4)

- **`CuratorDecision`** (discriminated union — the case-B model): 3 variants, NO winner-select variant (C3 differentiator; identical to C0.9). Every constructed decision is appended to U0's `AuditStore` (with `curator_id` + the hashes it was taken against) BEFORE the engine op; the receipt/`JobId` is linked after.
- **`DecisionGate`** (pure function): computes cluster review state from `critic.findings` in okc-web — counts Critical/Major (blocking, non-waivable) vs Minor (waivable) — and enforces the Major/Critical→regenerate-only rule BEFORE any `approve_cluster` engine call, returning a deterministic `422 APPROVAL_REQUIRED` when blocked. **Code-verified necessity**: core rejects blocking approvals with `AppError::InvalidProject("major or critical critic findings require regeneration")`, which the `okc` bindings' `map_project_message` maps to the generic `ProjectInvalid`, NOT `ApprovalRequired` — so the clean 422 must come from this okc-web gate. Interface: `evaluate(critic: CriticReportDto) -> GateVerdict`, `assert_approvable(critic: CriticReportDto) -> None` (raises `EngineError`).
- **`TaxonomyReviewService`**: loads the taxonomy proposal (E4-2), applies admin edits into `list[ClusterEdit]`, approves via `approve_taxonomy(edited, rationale)` (rationale required when edited).
- **`ClusterReviewService`**: the E4-3 focal engine. Loads `list[ClusterReviewView]` from `clusters()` (each = proposal + critic), builds the case-B decision surface, dispatches `ApproveCluster`/omission/Minor-waive through `DecisionGate` then `approve_cluster`. Builds `omission_rationales` keyed `"{document_id}:{target_id}"` (matches core `omission_key`) and `minor_waivers` keyed by Minor `finding_id`.
- **`RegenerationService`**: Major/Critical resolution (E4-4). Enqueues `regenerate_cluster(cluster_id, feedback, disclosure)` (feedback non-empty), then produces a before/after `RegenerationDiffView` once the job completes.
- **`ReviewGateService`**: E4-1 compile-gate scoreboard — aggregates blocking findings across clusters, approval-complete/pending counts, preserved-contradiction/waiver tallies → `CompileGateView(Blocked|PendingApprovals|Ready)` + per-cluster status. Read-only; never triggers compile.
- **`ReviewRoutes`** (FastAPI routers): E4 REST endpoints, admin-only (contributor → 403).

---

## U5 — `serving` (Epic E5)

- **`ServingRouter`**: mounts (a) the machine read-API `/api/serving/{project}/{tree|note|verify|explain}` (okc-mcp + E5-4 discovery) and (b) admin data endpoints under `/api/projects/{id}/serving*` and `/api/projects/{id}/compiled*` (E5-1/2/3/4). Read-only enforcement: `/api/serving/*` accepts GET/HEAD only; mutating verbs → 405; out-of-root/missing → 404. The bare `/api/serving/*` form resolves the project via the single LIVE publication.
- **`CompiledVaultStore`** (okc-web-owned filesystem reader): sandboxed read-only reads over one compiled-vault root — exists **because okc-core exposes no tree/note API** (C-5). Owns path safety (confine to root, reject `..`/`.`/absolute-escape/symlink; only under `knowledge/`+`legacy/`+`.okc/`). Parses the immutable sidecar `.okc/manifest.json`, `.okc/integration-plan.json` (per-cluster contradictions), `.okc/provenance.jsonl` → post-compile serving is self-contained (no live U4 dependency).
- **`ProvenanceComposer`** (E5-2 focal assembler): builds `NoteProvenanceView` by fusing (1) `engine.explain` → `ProvenanceView` (authoritative per-note record), (2) `engine.verify` → `VerificationView` (whole-vault PASS/FAIL + per-file rows), (3) `owner_display_name` from the shared `SourceRegistry`, (4) `ContradictionViewDto` from the `.okc/` index. No mock data.
- **`ServingStateStore`** (SQLite `serving_publications`): per-project publication row (bound `{integration_plan_id, corpus_hash, taxonomy_hash}`, `compiled_vault_path` abs, `status ∈ {Offline, Live, Stale}`, `published_at`, `published_by` = curator_id label). Publish/unpublish is an okc-web-only, admin-gated state flip that calls NO core op and does not enter the queue (E5-S1: manifest rebinding, compiled output immutable). Staleness derived by comparing bound hashes to U3's current frozen-input hashes.
- **`McpContractBuilder`** (FR-SRV-3): builds `McpContractDto` (E5-4) — consumption location, Markdown-only 3-root format, bound manifest identity, and honest boundary declarations (okc-mcp Deferred; chunking/embedding/query out-of-scope; consumer must RE-EMBED). Exposes NO mcp-tool trigger.
- **`ServingDtos`**: `VaultTreeDto`, `VaultNodeDto`, `VaultNoteDto`, `ManifestSummaryDto`, `VerificationView`, `VerifyCheckRowDto`, `ProvenanceView`, `NoteProvenanceView`, `LineageGraphDto`(nodes+edges), `ContradictionViewDto`, `ContradictionClaimViewDto`, `ServingStatusDto`, `McpContractDto` — all Pydantic models with a schema-version-guarded field.

---

## U6 — `web` (frontend component-group reference; wiring contract only, no UI redesign)

UI screens/routes/tokens are frozen in `ui-screens.md`/`design-system.md`. U6 defines only the API-consumption layer.

- **`apiClient`** (`frontend/lib/api/client.ts`): typed fetch wrapper, three auth contexts (Q9) — admin routes send the HTTP-only session cookie (`credentials: include`); `/u/{token}/*` uploads send `Authorization: Bearer <token>`; `/api/serving/*` is read-only. Emits typed `OkcErrorDto{code, category}`.
- **`okcErrorMap`** (`frontend/lib/api/errors.ts`): client mirror of the single OkcError code/category → UI branch table. Branches on `code`/`category` ONLY, never on `message`. Feeds the E1-4 401/403 interceptor, PROJECT_BUSY(409) retry banners, APPROVAL_REQUIRED(422) gate callouts.
- **`queryKeys` + TanStack Query hooks** (`frontend/lib/api/queries.ts`): one hook per read surface. All components are client components; TanStack Query drives data (Q10).
- **`useJobPolling`** (`frontend/lib/api/job-polling.ts`): the Q7 contract — `useQuery` against the canonical job route with `refetchInterval` until terminal, draining `events[]` into the append-only log, branching PROJECT_BUSY via `okcErrorMap`. Consumers: E3-5, E4-2, E4-4, E5-1. NOT used for serving verify/explain (synchronous, non-mutating).
- **Wiring deliverables (documents)**: screen→endpoint map, polling contract, screen→story-ID matrix (C1), epic-aligned module map (C6) — in the *Application Design* doc.
