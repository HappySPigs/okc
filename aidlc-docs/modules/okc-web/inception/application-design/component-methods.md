# okc-web — Application Design: Component Methods

**Depth**: comprehensive. Signatures are Python-style (type hints) with input/output DTO types and **high-level purpose only**. **Detailed business rules (rationale-required validation, edit-diff semantics, next-action policy tables, diff computation, path-safety algorithms, cap-atomicity) are DEFERRED to Functional Design.**

> All engine-touching methods depend on the U0 `OkcEngine` Protocol and raise `EngineError` (the okc-web-owned error) on failure. `okc.OkcError` is converted to `EngineError` **only inside `adapter`** and never appears in U1–U6 signatures (ADR-0002; verifier correction #3). Domain-only ops raise `AuthError`/`UploadError`/`StateError`.

---

## U0 `adapter.OkcEngine` (Protocol) — corrected surface

```python
class OkcEngine(Protocol):
    # ── lifecycle (split to mirror the bindings: create_project L953 / open_project L998 / project_handle L1025) ──
    async def create_project(self, spec: CreateProjectSpec) -> ProjectRef: ...
    #   spec = { root: AbsPath, name, curator_id, policy_version, language }
    async def open_project(self, root: AbsPath) -> ProjectRef: ...

    # ── status (RESERVING → routed through the single-writer queue) ──
    async def status(self, project: ProjectRef) -> StatusView: ...
    #   ADDED per verifier #2: wraps Project.status()->ProjectStatus(checkpoint, integration);
    #   StatusView(checkpoint: CheckpointView (7-variant, derived), integration: IntegrationStatusView).

    # ── non-reserving reads (served off the read-path second OkcClient; BYPASS the queue, verifier #8) ──
    async def taxonomy(self, project: ProjectRef) -> TaxonomyView: ...
    async def clusters(self, project: ProjectRef) -> ClusterReportView: ...
    async def manifest(self, project: ProjectRef) -> ManifestView: ...
    async def verify(self, vault: AbsPath) -> VerificationView: ...            # -> verify_artifact
    async def explain(self, vault: AbsPath, note: RelPath) -> ProvenanceView: ...  # -> explain_artifact

    # ── fast mutations (routed through queue; await terminal inline via call()) ──
    async def add_source(self, project: ProjectRef, src: SourceSpec) -> JobOutcome: ...
    async def replace_sources(self, project: ProjectRef, srcs: list[SourceSpec]) -> JobOutcome: ...  # <=10 (C-3) upstream
    async def approve_taxonomy(self, project: ProjectRef, edited: list[ClusterEdit] | None, rationale: str | None) -> JobOutcome: ...
    async def approve_cluster(self, project: ProjectRef, cmd: ApproveClusterCmd) -> JobOutcome: ...
    #   cmd = { cluster_id, omission_rationales: dict[str, str], minor_waivers: dict[str, str] }

    # ── long mutations (routed through queue; return JobId immediately, poll for progress) ──
    async def preflight(self, project: ProjectRef) -> JobId: ...                 # ADDED per verifier #1
    async def integrate(self, project: ProjectRef, consent: DisclosureDecision) -> JobId: ...
    async def regenerate_cluster(self, project: ProjectRef, cmd: RegenerateClusterCmd, consent: DisclosureDecision) -> JobId: ...
    async def compile(self, project: ProjectRef, output: AbsPath) -> JobId: ...
```

## U0 `adapter.schema_guard.SchemaGuard`
```python
def assert_client(client: okc.OkcClient) -> None: ...  # raises EngineError unless api_info().interop_schema_version == 2
def check_payload(version: int) -> None: ...           # per-DTO decode guard (raises on mismatch)
```

## U0 `adapter.queue.EngineActor` / `EngineHandle`
```python
def spawn(client: okc.OkcClient, jobs: JobStore, audit: AuditStore) -> EngineHandle: ...  # dedicated single worker thread (ThreadPoolExecutor(max_workers=1))
async def call(self, op: EngineOp) -> EngineValue: ...   # submit + await terminal via run_in_executor (fast/reserving ops)
async def enqueue(self, op: EngineOp) -> JobId: ...       # submit, return id, poll for progress (long ops)
def run(self) -> None: ...                                # worker loop: take one cmd -> dispatch -> set result
def drive_job(self, job: okc.Job[_T], id: JobId) -> _T: ...  # pump events()->JobStore, block on result() (raises okc.OkcError)
# @dataclass EngineCommand: Call(op, future: asyncio.Future) | Enqueue(op, id: JobId, future: asyncio.Future)
#   the worker resolves each Future via loop.call_soon_threadsafe.
# Only reserving/mutating ops are dispatched here; non-reserving reads use OkcEngineImpl's read-path second client directly.
```

## U0 `shared.jobs.JobStore`
```python
def create(self, project: ProjectRef, operation: str) -> JobId: ...
def record_progress(self, id: JobId, ev: ProgressView) -> None: ...
def record_terminal(self, id: JobId, outcome: EngineError | None) -> None: ...  # None = success
async def get(self, id: JobId) -> JobSnapshot | None: ...   # source for the single job-status endpoint (Q7)
```

## U0 `shared.authz`
```python
def require_role(*roles: Role) -> Callable: ...   # Depends(...) factory; enforces auth+role before the handler body runs
async def auth_context(request: Request) -> AuthContext: ...      # Depends: cookie -> AuthProvider.resolve_session -> Principal.Admin; raise 401/403 before handler
async def upload_context(request: Request) -> UploadContext: ...  # Depends: bearer -> AuthProvider.resolve_token -> Principal.UploadToken payload; raise 401/403 before handler

class AuthProvider(Protocol):
    async def resolve_session(self, raw_cookie_id: str) -> AuthContext | None: ...   # impl by U1
    async def resolve_token(self, raw_token: str) -> UploadContext | None: ...       # impl by U2

def require(ctx: AuthContext, allowed: list[Role]) -> None: ...  # raise 403 => guarantees core untouched (C-1)
```

## U0 `shared.error`
```python
def api_error_from_engine(err: EngineError) -> ApiError: ...
@app.exception_handler(ApiError)
async def handle_api_error(request: Request, err: ApiError) -> JSONResponse: ...
def http_status(code: EngineErrorCode, category: EngineErrorCategory) -> int: ...
# See Application Design §"OkcError -> HTTP mapping" for the full, corrected table (branch on code/category, NEVER message).
```

## U0 `shared.audit.AuditStore`
```python
def append(self, project: ProjectRef, principal: Principal, decision: CuratorDecision, bindings: HashBindings) -> AuditRecord: ...  # append-only
def list_for_project(self, project: ProjectRef) -> list[AuditRecord]: ...
# class HashBindings(BaseModel): proposal_hash: str | None; critic_hash: str | None; taxonomy_hash: str | None  # all three (verifier LOW #2)
```

## U0 `shared.state.SourceRegistry` (verifier #10 — dangling ref resolved)
```python
async def record(self, row: SourceRow) -> None: ...                       # written by U2 at add_source commit (raises StateError)
async def owner_labels(self, ids: list[SourceId]) -> OwnerLabelMap: ...    # read by U5 ProvenanceComposer
async def list_for_project(self, project: ProjectId) -> list[SourceRow]: ...
```

---

## U1 `auth`

```python
# AuthService (orchestrator; DB-only, pre-core)
async def login(self, creds: PasswordCredentials) -> EstablishedSession: ...      # verify -> create session -> Set-Cookie; reject disabled/bad-creds (401)
async def logout(self, session: SessionIdHash) -> None: ...
async def resolve(self, raw_cookie: str, now: datetime) -> AuthContext: ...        # backs AuthProvider.resolve_session -> Principal.Admin
async def create_account(self, actor: AuthContext, req: NewAccount) -> Account: ...      # admin-only; email-unique; one-time temp secret
async def set_role(self, actor: AuthContext, id: AccountId, role: Role) -> Account: ...  # last-admin guard
async def set_status(self, actor: AuthContext, id: AccountId, s: AccountStatus) -> Account: ...  # disable -> revoke_all sessions
async def change_password(self, actor: AuthContext, id: AccountId, req: PasswordChange) -> None: ...  # rehash -> revoke_all
async def list_accounts(self, actor: AuthContext, f: AccountFilter) -> list[AccountSummary]: ...

# AccountStore
async def insert(self, rec: AccountRecord) -> None: ...
async def find_by_email(self, email: str) -> AccountRecord | None: ...
async def find_by_id(self, id: AccountId) -> AccountRecord | None: ...
async def update_role(self, id: AccountId, role: Role) -> None: ...
async def update_status(self, id: AccountId, s: AccountStatus) -> None: ...
async def update_password_hash(self, id: AccountId, h: PasswordHash) -> None: ...
async def touch_last_login(self, id: AccountId, at: datetime) -> None: ...
async def count_active_admins(self) -> int: ...
async def list(self, f: AccountFilter) -> list[AccountSummary]: ...

# PasswordHasher (shared)
def hash(self, plaintext: str) -> PasswordHash: ...   # argon2id (argon2-cffi), random salt; plaintext is a transient local
def verify(self, plaintext: str, hash: PasswordHash) -> bool: ...  # constant-time

# SessionStore
async def create(self, account: AccountId, ttl: timedelta, meta: SessionMeta) -> SessionSecret: ...  # plaintext id ONCE; store SHA-256 (hashlib)
async def resolve(self, id_hash: SessionIdHash, now: datetime) -> SessionRecord | None: ...
async def touch(self, id_hash: SessionIdHash, now: datetime) -> None: ...
async def revoke(self, id_hash: SessionIdHash) -> None: ...
async def revoke_all_for_account(self, id: AccountId) -> None: ...

# SessionCookieCodec
def write(self, id: SessionSecret) -> str: ...   # HttpOnly; SameSite=Lax; Secure; Path=/
def clear(self) -> str: ...
def read(headers: Headers) -> RawSessionId | None: ...
```

---

## U2 `upload`

```python
# UploadTokenService (admin-facing; cookie session + RBAC(admin))
async def issue(self, actor: AuthContext, req: IssueTokenRequest) -> IssuedToken: ...   # one-time plaintext token + upload URL (E2-2)
async def list(self, actor: AuthContext, project: ProjectId, f: TokenFilter) -> list[TokenSummary]: ...
async def rotate(self, actor: AuthContext, token: TokenId) -> IssuedToken: ...          # revoke old + issue new, same slot
async def revoke(self, actor: AuthContext, token: TokenId) -> None: ...                 # immediate upload block
async def slot_usage(self, actor: AuthContext, project: ProjectId) -> SlotUsage: ...    # n/10 for CategoryBar

# UploadIngestService (token-facing; capability-authenticated, mutates engine)
async def ingest(self, ctx: UploadContext, body: BodyStream) -> IngestAccepted: ...     # full pipeline -> returns JobId
async def registration_status(self, ctx: UploadContext, job: JobId) -> JobSnapshot: ... # E2-5; reads the SAME JobStore snapshot (route: GET /u/{token}/jobs/{jobId})

# UploadTokenStore
async def insert(self, rec: UploadTokenRecord) -> None: ...
async def find_by_selector(self, sel: TokenSelector) -> UploadTokenRecord | None: ...
async def mark_used(self, id: TokenId, source_id: str, at: datetime) -> None: ...
async def revoke(self, id: TokenId, at: datetime) -> None: ...
async def list_for_project(self, project: ProjectId, f: TokenFilter) -> list[UploadTokenRecord]: ...
async def active_slot_count(self, project: ProjectId) -> int: ...

# UploadTokenSecret (split token)
def generate() -> tuple[TokenSelector, TokenSecret]: ...                        # CSPRNG via secrets (>=256-bit verifier)
def verifier_hash(secret: TokenSecret, pepper: Pepper, salt: Salt) -> VerifierHash: ...
def present(sel: TokenSelector, secret: TokenSecret) -> PlaintextToken: ...     # "selector.verifier" (one-time)
def parse(raw: str) -> tuple[TokenSelector, TokenSecret]: ...                   # TOKEN_INVALID on malformed

# UploadReceiver / ArchiveValidator / SourceLander / SlotAccountant
async def receive(self, ctx: UploadContext, body: BodyStream, cap: ByteCap) -> StagedUpload: ...  # stream to temp, cap while streaming
def inspect(self, staged: StagedUpload, policy: UploadPolicy) -> ValidationReport: ...   # per-check Pass|Warn|Fail + WebErrorCode
async def land(self, ctx: UploadContext, staged: StagedUpload) -> LandedSource: ...  # extract to ABSOLUTE path; content hash; dedup
async def reserve(self, project: ProjectId) -> SlotReservation: ...  # atomic; SOURCE_CAP_EXCEEDED if >=10
async def commit(self, r: SlotReservation, source_id: str) -> None: ...  # binds SourceId; writes SourceRegistry row
async def release(self, r: SlotReservation) -> None: ...              # on validation/land failure
```

---

## U3 `orchestration` (raises `EngineError`, not binding `OkcError` — verifier #3)

```python
# ProjectLifecycleService
async def create_project(self, req: NewProjectReq) -> ProjectSummary: ...   # -> engine.create_project(CreateProjectSpec(..)); persists projects row
async def open_project_ref(self, project_id: ProjectId) -> ProjectRef: ...  # ProjectId (SQLite) -> ProjectRef (engine handle)
async def list_projects(self) -> list[ProjectSummary]: ...                   # E3-1
async def get_project_overview(self, project_id: ProjectId) -> ProjectOverviewView: ...  # E3-3
async def freeze_sources(self, project_id: ProjectId) -> FreezeReceipt: ...  # E3-4/E3-6 transition

# PipelineService
async def status(self, project_id: ProjectId) -> PipelineStatusView: ...    # from engine.status() -> StatusView(checkpoint, integration)
def next_action(self, s: PipelineStatusView) -> NextAction: ...             # single recommended action, E3-3 card
def describe_pipeline(self, s: PipelineStatusView) -> PipelineTrackerView: ...  # 7-block Tracker (E3-3/E3-5)
async def run_preflight(self, project_id: ProjectId) -> JobId: ...          # -> engine.preflight()
async def run_integration(self, project_id: ProjectId, d: DisclosureDecision) -> JobId: ...  # -> engine.integrate(d)

# CompileService
async def can_compile(self, project_id: ProjectId) -> CompileGateView: ...  # checkpoint==ReadyToCompile && no blocking findings
async def run_compile(self, project_id: ProjectId, output: AbsPath) -> JobId: ...  # -> engine.compile(output)

# StalenessProjection
async def project_staleness(self, project_id: ProjectId) -> StalenessView: ...  # derived, non-authoritative
def record_fingerprint_at_approval(self, project_id: ProjectId, kind: ApprovalKind, source_fp: Fingerprint) -> None: ...
```

---

## U4 `review` (raises `EngineError` — verifier #3)

```python
# C-U4.1 the case-B model — NO winner-select variant (C3 differentiator, ADR-0024)
class ApproveTaxonomy(BaseModel):
    kind: Literal["approve_taxonomy"] = "approve_taxonomy"
    edited_clusters: list[ClusterEdit] | None = None
    rationale: Optional[str] = None

class ApproveCluster(BaseModel):
    kind: Literal["approve_cluster"] = "approve_cluster"
    cluster_id: str
    omission_rationales: dict[str, str]   # key = "{document_id}:{target_id}" (core omission_key)
    minor_waivers: dict[str, str]         # key = Minor finding_id

class RegenerateCluster(BaseModel):
    kind: Literal["regenerate_cluster"] = "regenerate_cluster"
    cluster_id: str
    feedback: str

# discriminated union, tagged by `kind` — EXACTLY 3 variants.
# NO SelectWinner / ResolveContradiction variant — structurally unrepresentable (C3), deliberately.
CuratorDecision = Annotated[Union[ApproveTaxonomy, ApproveCluster, RegenerateCluster], Field(discriminator="kind")]

# DecisionGate (pure; runs BEFORE any core call)
def evaluate(self, critic: CriticReportDto) -> GateVerdict: ...  # (blocking_critical, blocking_major, waivable_minor, approvable)
def assert_approvable(self, critic: CriticReportDto) -> None: ...
#   raises EngineError(code="ApprovalRequired", category="Approval") (=>HTTP 422) if any Major/Critical remains — engine never called.

# TaxonomyReviewService / ClusterReviewService / RegenerationService / ReviewGateService
async def get_taxonomy(self, project_id: ProjectId) -> TaxonomyReviewView: ...
async def approve_taxonomy(self, project_id: ProjectId, d: CuratorDecision) -> ApprovalReceipt: ...  # audit.append -> engine.approve_taxonomy -> receipt
async def list_clusters(self, project_id: ProjectId) -> list[ClusterSummaryView]: ...
async def get_cluster_view(self, project_id: ProjectId, cluster_id: str) -> ClusterReviewView: ...  # E4-3 focal
async def submit_decision(self, project_id: ProjectId, d: CuratorDecision) -> DecisionOutcome: ...
#   ApproveCluster => DecisionGate.assert_approvable() THEN audit.append THEN engine.approve_cluster(cmd)
#   RegenerateCluster => delegates to RegenerationService.start_regeneration
async def start_regeneration(self, project_id: ProjectId, cluster_id: str, feedback: str, d: DisclosureDecision) -> JobId: ...
async def get_regeneration_diff(self, project_id: ProjectId, cluster_id: str) -> RegenerationDiffView: ...
async def review_overview(self, project_id: ProjectId) -> ReviewOverviewView: ...  # E4-1 scoreboard + CompileGateView
```

---

## U5 `serving` (raises `EngineError` — verifier #3)

```python
# CompiledVaultStore (filesystem, NO core call)
def open(root: AbsPath) -> CompiledVaultStore: ...   # validates 3-root layout + .okc marker
def read_tree(self) -> VaultTreeDto: ...
def read_note(self, rel: RelPath) -> VaultNoteDto: ...  # 404 if missing/out-of-root
def read_manifest(self) -> ManifestSummaryDto: ...
def read_contradiction_index(self) -> list[ContradictionViewDto]: ...   # .okc/integration-plan.json
def read_provenance_index(self) -> ProvenanceIndexDto: ...             # .okc/provenance.jsonl

# ProvenanceComposer (E5-2 focal)
async def compose_note_view(self, project: ProjectId, note: RelPath) -> NoteProvenanceView: ...
#   engine.explain(vault, note) [ProvenanceView] + engine.verify(vault) [VerificationView]
#   + SourceRegistry.owner_labels(source_ids) + store.read_contradiction_index()
def build_lineage(self, record: ProvenanceView, labels: OwnerLabelMap) -> LineageGraphDto: ...

# ServingStateStore (SQLite; admin-gated; NO core call)
async def get_status(self, project: ProjectId) -> ServingStatusDto: ...
async def publish(self, project: ProjectId, actor: CuratorLabel) -> ServingStatusDto: ...   # bind manifest, Live
async def unpublish(self, project: ProjectId, actor: CuratorLabel) -> ServingStatusDto: ... # Offline
async def resolve_active(self) -> ProjectId | None: ...   # for bare /api/serving/* form

# McpContractBuilder + ServingService facade
async def build(self, project: ProjectId) -> McpContractDto: ...
async def serve_tree(self, project: ProjectId) -> VaultTreeDto: ...                       # GET only; 405 on mutating verbs
async def serve_note(self, project: ProjectId, note: RelPath) -> VaultNoteDto: ...
async def serve_verify(self, project: ProjectId) -> VerificationView: ...                 # non-mutating; bypasses queue
async def serve_explain(self, project: ProjectId, note: RelPath) -> ProvenanceView: ...   # non-mutating; bypasses queue
async def note_provenance_view(self, project: ProjectId, note: RelPath) -> NoteProvenanceView: ...  # E5-2
async def set_published(self, project: ProjectId, publish: bool, actor: CuratorLabel) -> ServingStatusDto: ...
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
