// Wire DTO types mirrored from the FROZEN backend Pydantic models
// (backend/app/**). Kept in sync by hand; the SPA never edits the backend.

export type Role = "admin" | "contributor";

// --- shared error contract (backend/app/shared/error.py) ---
export type ErrorCategory =
  | "auth" | "not_found" | "concurrency" | "project" | "approval"
  | "verification" | "schema" | "consent" | "validation" | "limit"
  | "provider" | "lifecycle" | "io" | "internal";

export interface ErrorBody {
  code: string;
  category: ErrorCategory;
  message: string;
  retryable: boolean;
  retry_after_ms?: number;
}

// --- auth (E1) ---
export interface SessionView {
  account_id: string;
  email: string;
  display_name: string;
  role: Role;
  curator_label: string;
}

export interface AccountSummary {
  id: string;
  email: string;
  display_name: string;
  role: Role;
  status: string; // active | disabled
  created_at: string;
  last_login_at?: string | null;
}

export interface CreateAccountResponse {
  account: AccountSummary;
  temp_password: string;
}

// --- upload / tokens (E2) ---
export const SOURCE_CAP = 10;

export interface IssueTokenRequest {
  owner_display_name?: string | null;
  owner_kind?: string | null; // department | individual
  ttl_seconds?: number | null;
}

export interface IssuedToken {
  token_id: string;
  token: string;
  upload_url: string;
  project_id: string;
  slot_index: number;
  owner_display_name?: string | null;
  owner_kind?: string | null;
  created_at: string;
  expires_at?: string | null;
}

export interface TokenSummary {
  token_id: string;
  project_id: string;
  slot_index: number;
  selector: string;
  status: string; // active | revoked | expired
  owner_display_name?: string | null;
  owner_kind?: string | null;
  created_at: string;
  expires_at?: string | null;
  last_used_at?: string | null;
  registered_source_id?: string | null;
}

export interface SlotUsage {
  used: number;
  limit: number;
}

export interface TokenListView {
  tokens: TokenSummary[];
  slot_usage: SlotUsage;
}

export interface UploadTargetView {
  project_id: string;
  project_name: string;
  slot_index: number;
  owner_display_name?: string | null;
  owner_kind?: string | null;
  status: string;
}

export interface CheckResult {
  name: string;
  status: string; // pass | warn | fail
  code?: string | null;
  detail?: string | null;
}

export interface IngestAccepted {
  job_id: string;
  source_id: string;
  project_id: string;
  slot_index: number;
  content_hash: string;
  owner_display_name?: string | null;
  warnings: CheckResult[];
}

// --- orchestration (E3) ---
export type Checkpoint =
  | "needs_provider" | "needs_sources" | "needs_disclosure"
  | "needs_taxonomy" | "needs_clusters" | "ready_to_compile" | "verified";

export const PIPELINE: Checkpoint[] = [
  "needs_provider", "needs_sources", "needs_disclosure",
  "needs_taxonomy", "needs_clusters", "ready_to_compile", "verified",
];

export interface ProjectView {
  id: string;
  name: string;
  engine_root_abs_path: string;
  curator_id: string;
  freeze_state: string; // frozen | unfrozen
  frozen_at?: string | null;
  source_count: number;
  created_at: string;
}

export interface ProjectStatusView {
  project_id: string;
  checkpoint: Checkpoint | string;
  next_action: string;
  resolver: string; // contributor | u3 | u4 | u5
  progression: string[];
  blocked_steps: string[];
  stale: boolean;
  frozen: boolean;
  source_count: number;
  integration_plan_id?: string | null;
}

export interface FreezeResponse {
  project_id: string;
  freeze_state: string;
  source_set_fingerprint: string;
  frozen_at: string;
  source_count: number;
}

export interface ProviderProfileView {
  name: string;
  kind: string;
  endpoint: string;
  model: string;
}

export interface RouteBoundaryView {
  role: string;
  profile_name: string;
  boundary: string; // local | remote
}

export interface PreflightGateView {
  project_id: string;
  sensitive_findings: number;
  routes: RouteBoundaryView[];
  requires_disclosure: boolean;
}

export interface JobAccepted {
  job_id: string;
  project_id: string;
}

export interface CompileResultView {
  project_id: string;
  path: string;
  integration_plan_id?: string | null;
  file_count: number;
}

export interface RunIntegrationRequest {
  allow_remote_provider: boolean;
  remote_disclosure_confirmed: boolean;
}

// --- jobs (Q7 poll) ---
export interface JobEventDto {
  sequence: number;
  phase?: string | null;
  state?: string | null;
  completed?: number | null;
  total?: number | null;
  current_item?: string | null;
  ts: string;
}

export interface JobErrorDto {
  code: string;
  category: ErrorCategory;
  retryable: boolean;
}

export interface JobSnapshot {
  id: string;
  project_id?: string | null;
  kind: string;
  state: string; // queued | running | completed | failed | cancelled
  phase?: string | null;
  progress_completed: number;
  progress_total?: number | null;
  error?: JobErrorDto | null;
  events: JobEventDto[];
  created_at: string;
  updated_at: string;
  finished_at?: string | null;
}

// --- review (E4) ---
export type Severity = "minor" | "major" | "critical" | string;

export interface CriticFindingView {
  finding_id: string;
  severity: Severity;
  kind: string;
  message: string;
  blocking: boolean;
}

export interface TaxonomyReviewView {
  project_id: string;
  interop_schema_version?: number | null;
  payload: unknown;
}

export interface ClusterSummaryView {
  cluster_id: string;
  finding_count: number;
  blocking_count: number;
  minor_count: number;
  blocking: boolean;
}

export interface ClusterDetailView {
  cluster_id: string;
  proposal_hash?: string | null;
  critic_hash?: string | null;
  taxonomy_hash?: string | null;
  findings: CriticFindingView[];
  blocking: boolean;
  proposal: unknown;
  contradictions: unknown;
}

export interface BlockingItemView {
  cluster_id: string;
  finding_id: string;
  severity: string;
  required_action: string;
}

export interface ReviewGateView {
  project_id: string;
  state: string; // Blocked | PendingApprovals | Ready
  checkpoint?: string | null;
  blocking_items: BlockingItemView[];
}

export interface DecisionRecordView {
  id: string;
  decision_kind: string;
  target_ref?: string | null;
  core_op: string;
  core_job_id?: string | null;
  created_at: string;
}

export interface DecisionReceipt {
  project_id: string;
  audit_id: string;
  decision_kind: string;
  job_id?: string | null;
}

export interface ApproveTaxonomyRequest {
  edited_clusters?: Record<string, unknown>[] | null;
  rationale?: string | null;
}

export interface ApproveClusterRequest {
  minor_waivers: Record<string, string>;
  omission_rationales: Record<string, string>;
}

export interface RegenerateClusterRequest {
  feedback: string;
  allow_remote_provider: boolean;
  remote_disclosure_confirmed: boolean;
}

// --- serving (E5) ---
export type ServingStatus = "offline" | "live" | "stale" | string;

export interface PublicationView {
  project_id: string;
  status: ServingStatus;
  stale: boolean;
  compiled_vault_path?: string | null;
  bound_integration_plan_id?: string | null;
  bound_corpus_hash?: string | null;
  bound_taxonomy_hash?: string | null;
  published_at?: string | null;
  published_by?: string | null;
}

export interface ServingFileListView {
  project_id: string;
  status: ServingStatus;
  stale: boolean;
  bound_integration_plan_id?: string | null;
  files: string[];
}

export interface ServingVerifyView {
  project_id: string;
  status: ServingStatus;
  stale: boolean;
  valid: boolean;
  artifact_path: string;
  manifest: unknown;
  note: string;
}

export interface ServingProvenanceView {
  project_id: string;
  status: ServingStatus;
  stale: boolean;
  file_path: string;
  artifact_path: string;
  record: unknown;
  owner_labels: Record<string, { display_name?: string; kind?: string }>;
  note: string;
}

export interface ContractLocation {
  local_dir?: string | null;
  read_api_base: string;
}

export interface ContractFormat {
  content: string;
  layout: string[];
}

export interface ContractEndpoint {
  method: string;
  path: string;
  description: string;
}

export interface McpContractView {
  project_id: string;
  status: ServingStatus;
  stale: boolean;
  location: ContractLocation;
  format: ContractFormat;
  bound_integration_plan_id?: string | null;
  bound_corpus_hash?: string | null;
  bound_taxonomy_hash?: string | null;
  endpoints: ContractEndpoint[];
  out_of_scope: string;
}
