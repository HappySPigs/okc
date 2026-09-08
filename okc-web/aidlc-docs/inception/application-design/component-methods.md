# okc-web — Application Design: Component Methods

**Depth**: comprehensive. Signatures are Rust-style with input/output DTO types and **high-level purpose only**. **Detailed business rules (rationale-required validation, edit-diff semantics, next-action policy tables, diff computation, path-safety algorithms, cap-atomicity) are DEFERRED to Functional Design.**

> All engine-touching methods depend on the U0 `OkcEngine` trait and return `Result<_, EngineError>` (the okc-web-owned error). `okc_interop::OkcError` is converted to `EngineError` **only inside `adapter`** and never appears in U1–U6 signatures (ADR-0002; verifier correction #3). Domain-only ops return `AuthError`/`UploadError`/`StateError`.

---

## U0 `adapter::OkcEngine` (async_trait) — corrected surface

```rust
#[async_trait]
pub trait OkcEngine: Send + Sync {
    // ── lifecycle (split to mirror interop: create_project L953 / open_project L998 / project_handle L1025) ──
    async fn create_project(&self, spec: CreateProjectSpec) -> Result<ProjectRef, EngineError>;
    //   spec = { root: AbsPath, name, curator_id, policy_version, language }
    async fn open_project(&self, root: AbsPath) -> Result<ProjectRef, EngineError>;

    // ── status (RESERVING → routed through the single-writer queue) ──
    async fn status(&self, project: &ProjectRef) -> Result<StatusView, EngineError>;
    //   ADDED per verifier #2: wraps Project::status()->ProjectStatus{checkpoint, integration};
    //   StatusView{ checkpoint: CheckpointView (7-variant, derived), integration: IntegrationStatusView }.

    // ── non-reserving reads (served off the read-path OkcClient clone; BYPASS the queue, verifier #8) ──
    async fn taxonomy(&self, project: &ProjectRef) -> Result<TaxonomyView, EngineError>;
    async fn clusters(&self, project: &ProjectRef) -> Result<ClusterReportView, EngineError>;
    async fn manifest(&self, project: &ProjectRef) -> Result<ManifestView, EngineError>;
    async fn verify(&self, vault: AbsPath) -> Result<VerificationView, EngineError>;          // -> verify_artifact
    async fn explain(&self, vault: AbsPath, note: RelPath) -> Result<ProvenanceView, EngineError>; // -> explain_artifact

    // ── fast mutations (routed through queue; await terminal inline via call()) ──
    async fn add_source(&self, project: &ProjectRef, src: SourceSpec) -> Result<JobOutcome, EngineError>;
    async fn replace_sources(&self, project: &ProjectRef, srcs: Vec<SourceSpec>) -> Result<JobOutcome, EngineError>; // <=10 (C-3) upstream
    async fn approve_taxonomy(&self, project: &ProjectRef, edited: Option<Vec<ClusterEdit>>, rationale: Option<String>) -> Result<JobOutcome, EngineError>;
    async fn approve_cluster(&self, project: &ProjectRef, cmd: ApproveClusterCmd) -> Result<JobOutcome, EngineError>;
    //   cmd = { cluster_id, omission_rationales: BTreeMap<String,String>, minor_waivers: BTreeMap<String,String> }

    // ── long mutations (routed through queue; return JobId immediately, poll for progress) ──
    async fn preflight(&self, project: &ProjectRef) -> Result<JobId, EngineError>;                 // ADDED per verifier #1
    async fn integrate(&self, project: &ProjectRef, consent: DisclosureDecision) -> Result<JobId, EngineError>;
    async fn regenerate_cluster(&self, project: &ProjectRef, cmd: RegenerateClusterCmd, consent: DisclosureDecision) -> Result<JobId, EngineError>;
    async fn compile(&self, project: &ProjectRef, output: AbsPath) -> Result<JobId, EngineError>;
}
```

## U0 `adapter::schema_guard::SchemaGuard`
```rust
fn assert_client(client: &okc_interop::OkcClient) -> Result<(), EngineError>; // api_info().interop_schema_version == 2
fn check_payload(version: u32) -> Result<(), EngineError>;                     // per-DTO decode guard
```

## U0 `adapter::queue::EngineActor` / `EngineHandle`
```rust
fn spawn(client: OkcClient, jobs: JobStore, audit: AuditStore) -> EngineHandle; // dedicated blocking worker thread
async fn call(&self, op: EngineOp) -> Result<EngineValue, EngineError>;         // enqueue + await terminal (fast/reserving ops)
async fn enqueue(&self, op: EngineOp) -> Result<JobId, EngineError>;            // enqueue, return id, poll for progress (long ops)
fn run(self);                                                                    // blocking loop: recv one cmd -> dispatch -> reply
fn drive_job<T: DeserializeOwned>(&self, job: okc_interop::Job<T>, id: &JobId) -> Result<T, okc_interop::OkcError>; // pump events()->JobStore, block on result()
// enum EngineCommand { Call{op, reply: oneshot::Sender<Result<EngineValue,EngineError>>},
//                      Enqueue{op, id: JobId, reply: oneshot::Sender<Result<JobId,EngineError>>} }
// Only reserving/mutating ops are dispatched here; non-reserving reads use OkcEngineImpl's read-path clone directly.
```

## U0 `shared::jobs::JobStore`
```rust
fn create(&self, project: &ProjectRef, operation: &str) -> JobId;
fn record_progress(&self, id: &JobId, ev: &ProgressView);
fn record_terminal(&self, id: &JobId, outcome: Result<(), EngineError>);
async fn get(&self, id: &JobId) -> Option<JobSnapshot>;   // source for the single job-status endpoint (Q7)
```

## U0 `shared::authz`
```rust
async fn authz_layer(State(app): State<AppState>, req: Request, next: Next) -> Result<Response, ApiError>;
impl<S: Send+Sync> FromRequestParts<S> for AuthContext { /* cookie -> AuthProvider.resolve_session -> Principal::Admin; 401/403 before handler */ }
impl<S: Send+Sync> FromRequestParts<S> for UploadContext { /* bearer -> AuthProvider.resolve_token -> Principal::UploadToken payload; 401/403 before handler */ }
trait AuthProvider: Send + Sync {
    async fn resolve_session(&self, raw_cookie_id: &str) -> Result<Option<AuthContext>, AuthError>;   // impl by U1
    async fn resolve_token(&self, raw_token: &str)      -> Result<Option<UploadContext>, AuthError>;  // impl by U2
}
fn require(ctx: &AuthContext, allowed: &[Role]) -> Result<(), ApiError>;  // 403 => guarantees core untouched (C-1)
```

## U0 `shared::error`
```rust
impl From<EngineError> for ApiError;
impl IntoResponse for ApiError;
fn http_status(code: EngineErrorCode, category: EngineErrorCategory) -> StatusCode;
// See Application Design §"OkcError -> HTTP mapping" for the full, corrected table (branch on code/category, NEVER message).
```

## U0 `shared::audit::AuditStore`
```rust
fn append(&self, project: &ProjectRef, principal: &Principal, decision: CuratorDecision, bindings: HashBindings) -> AuditRecord; // append-only
fn list_for_project(&self, project: &ProjectRef) -> Vec<AuditRecord>;
// struct HashBindings { proposal_hash: Option<String>, critic_hash: Option<String>, taxonomy_hash: Option<String> } // all three (verifier LOW #2)
```

## U0 `shared::state::SourceRegistry` (verifier #10 — dangling ref resolved)
```rust
async fn record(&self, row: SourceRow) -> Result<(), StateError>;                       // written by U2 at add_source commit
async fn owner_labels(&self, ids: &[SourceId]) -> Result<OwnerLabelMap, StateError>;     // read by U5 ProvenanceComposer
async fn list_for_project(&self, project: &ProjectId) -> Result<Vec<SourceRow>, StateError>;
```

---

## U1 `auth`

```rust
// AuthService (orchestrator; DB-only, pre-core)
async fn login(&self, creds: PasswordCredentials) -> Result<EstablishedSession, AuthError>;      // verify -> create session -> Set-Cookie; reject disabled/bad-creds (401)
async fn logout(&self, session: SessionIdHash) -> Result<(), AuthError>;
async fn resolve(&self, raw_cookie: &str, now: OffsetDateTime) -> Result<AuthContext, AuthError>; // backs AuthProvider.resolve_session -> Principal::Admin
async fn create_account(&self, actor: &AuthContext, req: NewAccount) -> Result<Account, AuthError>;      // admin-only; email-unique; one-time temp secret
async fn set_role(&self, actor: &AuthContext, id: AccountId, role: Role) -> Result<Account, AuthError>;  // last-admin guard
async fn set_status(&self, actor: &AuthContext, id: AccountId, s: AccountStatus) -> Result<Account, AuthError>; // disable -> revoke_all sessions
async fn change_password(&self, actor: &AuthContext, id: AccountId, req: PasswordChange) -> Result<(), AuthError>; // rehash -> revoke_all
async fn list_accounts(&self, actor: &AuthContext, f: AccountFilter) -> Result<Vec<AccountSummary>, AuthError>;

// AccountStore
async fn insert(&self, rec: AccountRecord) -> Result<(), StateError>;
async fn find_by_email(&self, email: &str) -> Result<Option<AccountRecord>, StateError>;
async fn find_by_id(&self, id: &AccountId) -> Result<Option<AccountRecord>, StateError>;
async fn update_role(&self, id: &AccountId, role: Role) -> Result<(), StateError>;
async fn update_status(&self, id: &AccountId, s: AccountStatus) -> Result<(), StateError>;
async fn update_password_hash(&self, id: &AccountId, h: PasswordHash) -> Result<(), StateError>;
async fn touch_last_login(&self, id: &AccountId, at: OffsetDateTime) -> Result<(), StateError>;
async fn count_active_admins(&self) -> Result<u64, StateError>;
async fn list(&self, f: AccountFilter) -> Result<Vec<AccountSummary>, StateError>;

// PasswordHasher (shared)
fn hash(&self, plaintext: &SecretString) -> Result<PasswordHash, CryptoError>;   // argon2id, random salt
fn verify(&self, plaintext: &SecretString, hash: &PasswordHash) -> Result<bool, CryptoError>; // constant-time

// SessionStore
async fn create(&self, account: &AccountId, ttl: Duration, meta: SessionMeta) -> Result<SessionSecret, StateError>; // plaintext id ONCE; store SHA-256
async fn resolve(&self, id_hash: &SessionIdHash, now: OffsetDateTime) -> Result<Option<SessionRecord>, StateError>;
async fn touch(&self, id_hash: &SessionIdHash, now: OffsetDateTime) -> Result<(), StateError>;
async fn revoke(&self, id_hash: &SessionIdHash) -> Result<(), StateError>;
async fn revoke_all_for_account(&self, id: &AccountId) -> Result<(), StateError>;

// SessionCookieCodec
fn write(&self, id: &SessionSecret) -> HeaderValue;   // HttpOnly; SameSite=Lax; Secure; Path=/
fn clear(&self) -> HeaderValue;
fn read(headers: &HeaderMap) -> Option<RawSessionId>;
```

---

## U2 `upload`

```rust
// UploadTokenService (admin-facing; cookie session + RBAC(admin))
async fn issue(&self, actor: &AuthContext, req: IssueTokenRequest) -> Result<IssuedToken, UploadError>;   // one-time plaintext token + upload URL (E2-2)
async fn list(&self, actor: &AuthContext, project: &ProjectId, f: TokenFilter) -> Result<Vec<TokenSummary>, UploadError>;
async fn rotate(&self, actor: &AuthContext, token: &TokenId) -> Result<IssuedToken, UploadError>;         // revoke old + issue new, same slot
async fn revoke(&self, actor: &AuthContext, token: &TokenId) -> Result<(), UploadError>;                  // immediate upload block
async fn slot_usage(&self, actor: &AuthContext, project: &ProjectId) -> Result<SlotUsage, UploadError>;   // n/10 for CategoryBar

// UploadIngestService (token-facing; capability-authenticated, mutates engine)
async fn ingest(&self, ctx: &UploadContext, body: BodyStream) -> Result<IngestAccepted, UploadError>;     // full pipeline -> returns JobId
async fn registration_status(&self, ctx: &UploadContext, job: &JobId) -> Result<JobSnapshot, UploadError>;// E2-5; reads the SAME JobStore snapshot (route: GET /u/{token}/jobs/{jobId})

// UploadTokenStore
async fn insert(&self, rec: UploadTokenRecord) -> Result<(), StateError>;
async fn find_by_selector(&self, sel: &TokenSelector) -> Result<Option<UploadTokenRecord>, StateError>;
async fn mark_used(&self, id: &TokenId, source_id: &str, at: OffsetDateTime) -> Result<(), StateError>;
async fn revoke(&self, id: &TokenId, at: OffsetDateTime) -> Result<(), StateError>;
async fn list_for_project(&self, project: &ProjectId, f: TokenFilter) -> Result<Vec<UploadTokenRecord>, StateError>;
async fn active_slot_count(&self, project: &ProjectId) -> Result<u32, StateError>;

// UploadTokenSecret (split token)
fn generate() -> (TokenSelector, TokenSecret);                                 // CSPRNG (>=256-bit verifier)
fn verifier_hash(secret: &TokenSecret, pepper: &Pepper, salt: &Salt) -> VerifierHash;
fn present(sel: &TokenSelector, secret: &TokenSecret) -> PlaintextToken;        // "selector.verifier" (one-time)
fn parse(raw: &str) -> Result<(TokenSelector, TokenSecret), UploadError>;       // TOKEN_INVALID on malformed

// UploadReceiver / ArchiveValidator / SourceLander / SlotAccountant
async fn receive(&self, ctx: &UploadContext, body: BodyStream, cap: ByteCap) -> Result<StagedUpload, UploadError>; // stream to temp, cap while streaming
fn inspect(&self, staged: &StagedUpload, policy: &UploadPolicy) -> ValidationReport;   // per-check Pass|Warn|Fail + WebErrorCode
async fn land(&self, ctx: &UploadContext, staged: &StagedUpload) -> Result<LandedSource, UploadError>;  // extract to ABSOLUTE path; content hash; dedup
async fn reserve(&self, project: &ProjectId) -> Result<SlotReservation, UploadError>;  // atomic; SOURCE_CAP_EXCEEDED if >=10
async fn commit(&self, r: SlotReservation, source_id: &str) -> Result<(), UploadError>;// binds SourceId; writes SourceRegistry row
async fn release(&self, r: SlotReservation) -> Result<(), UploadError>;                 // on validation/land failure
```

---

## U3 `orchestration` (returns `EngineError`, not interop `OkcError` — verifier #3)

```rust
// ProjectLifecycleService
fn create_project(&self, req: NewProjectReq) -> Result<ProjectSummary, EngineError>;   // -> engine.create_project(CreateProjectSpec{..}); persists projects row
fn open_project_ref(&self, project_id: ProjectId) -> Result<ProjectRef, EngineError>;  // ProjectId (SQLite) -> ProjectRef (engine handle)
fn list_projects(&self) -> Result<Vec<ProjectSummary>, EngineError>;                    // E3-1
fn get_project_overview(&self, project_id: ProjectId) -> Result<ProjectOverviewView, EngineError>; // E3-3
fn freeze_sources(&self, project_id: ProjectId) -> Result<FreezeReceipt, EngineError>; // E3-4/E3-6 transition

// PipelineService
fn status(&self, project_id: ProjectId) -> Result<PipelineStatusView, EngineError>;    // from engine.status() -> StatusView{checkpoint, integration}
fn next_action(&self, s: &PipelineStatusView) -> NextAction;                            // single recommended action, E3-3 card
fn describe_pipeline(&self, s: &PipelineStatusView) -> PipelineTrackerView;             // 7-block Tracker (E3-3/E3-5)
fn run_preflight(&self, project_id: ProjectId) -> Result<JobId, EngineError>;           // -> engine.preflight()
fn run_integration(&self, project_id: ProjectId, d: DisclosureDecision) -> Result<JobId, EngineError>; // -> engine.integrate(d)

// CompileService
fn can_compile(&self, project_id: ProjectId) -> Result<CompileGateView, EngineError>;   // checkpoint==ReadyToCompile && no blocking findings
fn run_compile(&self, project_id: ProjectId, output: AbsPath) -> Result<JobId, EngineError>; // -> engine.compile(output)

// StalenessProjection
fn project_staleness(&self, project_id: ProjectId) -> Result<StalenessView, EngineError>; // derived, non-authoritative
fn record_fingerprint_at_approval(&self, project_id: ProjectId, kind: ApprovalKind, source_fp: Fingerprint);
```

---

## U4 `review` (returns `EngineError` — verifier #3)

```rust
// C-U4.1 the case-B model — NO winner-select variant (C3 differentiator, ADR-0024)
enum CuratorDecision {
    ApproveTaxonomy   { edited_clusters: Option<Vec<ClusterEdit>>, rationale: Option<String> },
    ApproveCluster    { cluster_id: String,
                        omission_rationales: BTreeMap<String, String>,   // key = "{document_id}:{target_id}" (core omission_key)
                        minor_waivers:       BTreeMap<String, String> },  // key = Minor finding_id
    RegenerateCluster { cluster_id: String, feedback: String },
}

// DecisionGate (pure; runs BEFORE any core call)
fn evaluate(&self, critic: &CriticReportDto) -> GateVerdict;  // {blocking_critical, blocking_major, waivable_minor, approvable}
fn assert_approvable(&self, critic: &CriticReportDto) -> Result<(), EngineError>;
//   returns EngineError{code: ApprovalRequired, category: Approval} (=>HTTP 422) if any Major/Critical remains — engine never called.

// TaxonomyReviewService / ClusterReviewService / RegenerationService / ReviewGateService
fn get_taxonomy(&self, project_id: ProjectId) -> Result<TaxonomyReviewView, EngineError>;
fn approve_taxonomy(&self, project_id: ProjectId, d: CuratorDecision) -> Result<ApprovalReceipt, EngineError>; // audit.append -> engine.approve_taxonomy -> receipt
fn list_clusters(&self, project_id: ProjectId) -> Result<Vec<ClusterSummaryView>, EngineError>;
fn get_cluster_view(&self, project_id: ProjectId, cluster_id: &str) -> Result<ClusterReviewView, EngineError>; // E4-3 focal
fn submit_decision(&self, project_id: ProjectId, d: CuratorDecision) -> Result<DecisionOutcome, EngineError>;
//   ApproveCluster => DecisionGate.assert_approvable() THEN audit.append THEN engine.approve_cluster(cmd)
//   RegenerateCluster => delegates to RegenerationService.start_regeneration
fn start_regeneration(&self, project_id: ProjectId, cluster_id: String, feedback: String, d: DisclosureDecision) -> Result<JobId, EngineError>;
fn get_regeneration_diff(&self, project_id: ProjectId, cluster_id: &str) -> Result<RegenerationDiffView, EngineError>;
fn review_overview(&self, project_id: ProjectId) -> Result<ReviewOverviewView, EngineError>; // E4-1 scoreboard + CompileGateView
```

---

## U5 `serving` (returns `EngineError` — verifier #3)

```rust
// CompiledVaultStore (filesystem, NO core call)
fn open(root: AbsPath) -> Result<CompiledVaultStore, EngineError>;   // validates 3-root layout + .okc marker
fn read_tree(&self) -> Result<VaultTreeDto, EngineError>;
fn read_note(&self, rel: RelPath) -> Result<VaultNoteDto, EngineError>;  // 404 if missing/out-of-root
fn read_manifest(&self) -> Result<ManifestSummaryDto, EngineError>;
fn read_contradiction_index(&self) -> Result<Vec<ContradictionViewDto>, EngineError>;   // .okc/integration-plan.json
fn read_provenance_index(&self) -> Result<ProvenanceIndexDto, EngineError>;             // .okc/provenance.jsonl

// ProvenanceComposer (E5-2 focal)
fn compose_note_view(&self, project: ProjectId, note: RelPath) -> Result<NoteProvenanceView, EngineError>;
//   engine.explain(vault, note) [ProvenanceView] + engine.verify(vault) [VerificationView]
//   + SourceRegistry.owner_labels(source_ids) + store.read_contradiction_index()
fn build_lineage(&self, record: &ProvenanceView, labels: &OwnerLabelMap) -> LineageGraphDto;

// ServingStateStore (SQLite; admin-gated; NO core call)
fn get_status(&self, project: ProjectId) -> Result<ServingStatusDto, EngineError>;
fn publish(&self, project: ProjectId, actor: CuratorLabel) -> Result<ServingStatusDto, EngineError>;   // bind manifest, Live
fn unpublish(&self, project: ProjectId, actor: CuratorLabel) -> Result<ServingStatusDto, EngineError>; // Offline
fn resolve_active(&self) -> Result<Option<ProjectId>, EngineError>;   // for bare /api/serving/* form

// McpContractBuilder + ServingService facade
fn build(&self, project: ProjectId) -> Result<McpContractDto, EngineError>;
fn serve_tree(&self, project: ProjectId) -> Result<VaultTreeDto, EngineError>;                       // GET only; 405 on mutating verbs
fn serve_note(&self, project: ProjectId, note: RelPath) -> Result<VaultNoteDto, EngineError>;
fn serve_verify(&self, project: ProjectId) -> Result<VerificationView, EngineError>;                 // non-mutating; bypasses queue
fn serve_explain(&self, project: ProjectId, note: RelPath) -> Result<ProvenanceView, EngineError>;   // non-mutating; bypasses queue
fn note_provenance_view(&self, project: ProjectId, note: RelPath) -> Result<NoteProvenanceView, EngineError>; // E5-2
fn set_published(&self, project: ProjectId, publish: bool, actor: CuratorLabel) -> Result<ServingStatusDto, EngineError>;
```

---

## U6 client-side wiring signatures (TypeScript, contract only)

```ts
// polling contract (Q7) — mutating long jobs only; canonical project-scoped route
useJobPolling(projectId: string, jobId: string): { state: JobState; events: ProgressEvent[]; error?: OkcErrorDto };
// GET /api/projects/{projectId}/jobs/{jobId}  (admin cookie)   OR  GET /u/{token}/jobs/{jobId} (bearer) — same JobSnapshot shape

// read hooks (examples)
useProjectOverview(projectId): ProjectOverviewView;                 // E3-3
usePipelineStatus(projectId): PipelineStatusView;                   // E3-3/E3-5 (poll while running)
useClusterReview(projectId, clusterId): ClusterReviewView;          // E4-3 focal
useReviewOverview(projectId): ReviewOverviewView;                   // E4-1 gate scoreboard
useCompiledTree(projectId): VaultTreeDto;                           // E5-1
useCompiledNote(projectId, path): VaultNoteDto;                     // E5-1
useNoteProvenance(projectId, path): NoteProvenanceView;             // E5-2 focal (verify+explain+lineage+contradictions)
useServingStatus(projectId): ServingStatusDto;                      // E5-3 (poll while STALE)
useMcpContract(projectId): McpContractDto;                          // E5-4
// mutations
useSubmitClusterDecision(projectId, clusterId): (d: CuratorDecision) => Promise<DecisionOutcome>; // E4-3
usePublishToggle(projectId): (publish: boolean) => Promise<ServingStatusDto>; // E5-3 admin-only
```

> **Deferred to Functional Design**: every method body / business rule — individual rationale-required validation (per-omission & per-Minor-finding), taxonomy edit-diff semantics, next-action policy table, before/after diff computation, path-safety/zip-bomb algorithm parameters, cap-reservation atomicity, exact `CreateProjectSpec`/`SourceSpec` field sets and the local-absolute-path landing convention (C-6), and `RemoteConsent`/`DisclosureDecision` field validation.
