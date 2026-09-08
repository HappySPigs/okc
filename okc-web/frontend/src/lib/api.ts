// Tiny typed fetch client for the same-origin okc-web API. Cookie auth
// (`okc_session`, HttpOnly) => every call uses credentials:'include'. Errors are
// normalized into ApiError carrying the stable {code, category} contract
// (backend/app/shared/error.py); the UI branches on code/category, never message.

import type {
  AccountSummary, ApproveClusterRequest, ApproveTaxonomyRequest, ClusterDetailView,
  ClusterSummaryView, CompileResultView, CreateAccountResponse, DecisionReceipt,
  DecisionRecordView, ErrorBody, ErrorCategory, FreezeResponse, IngestAccepted,
  IssuedToken, IssueTokenRequest, JobAccepted, JobSnapshot, McpContractView, PreflightGateView,
  ProjectStatusView, ProjectView, ProviderProfileView, PublicationView,
  RegenerateClusterRequest, ReviewGateView, Role, ServingFileListView,
  ServingProvenanceView, ServingVerifyView, SessionView, TaxonomyReviewView,
  TokenListView, UploadTargetView,
} from "./types";

export class ApiError extends Error {
  code: string;
  category: ErrorCategory;
  retryable: boolean;
  retryAfterMs?: number;
  httpStatus: number;

  constructor(body: ErrorBody, httpStatus: number, retryAfterHeaderMs?: number) {
    super(body.message);
    this.name = "ApiError";
    this.code = body.code;
    this.category = body.category;
    this.retryable = body.retryable;
    this.retryAfterMs = body.retry_after_ms ?? retryAfterHeaderMs;
    this.httpStatus = httpStatus;
  }

  get isBusy(): boolean {
    return this.code === "PROJECT_BUSY" || this.code === "RESOURCE_LIMIT";
  }
  get isAuth(): boolean {
    return this.code === "UNAUTHENTICATED" || this.code === "SESSION_EXPIRED";
  }
}

// Global session-expiry hook installed by the auth provider (redirect to /login).
let unauthorizedHandler: (() => void) | null = null;
export function setUnauthorizedHandler(fn: (() => void) | null): void {
  unauthorizedHandler = fn;
}

type Method = "GET" | "POST" | "PATCH" | "DELETE";

async function toApiError(res: Response): Promise<ApiError> {
  const retryHeader = res.headers.get("Retry-After");
  const retryMs = retryHeader ? Number(retryHeader) * 1000 : undefined;
  let body: ErrorBody;
  try {
    body = (await res.json()) as ErrorBody;
    if (!body || typeof body.code !== "string") throw new Error("shape");
  } catch {
    body = {
      code: res.status === 405 ? "METHOD_NOT_ALLOWED" : "INTERNAL",
      category: "internal",
      message: `HTTP ${res.status}`,
      retryable: false,
    };
  }
  return new ApiError(body, res.status, retryMs);
}

async function request<T>(method: Method, path: string, body?: unknown): Promise<T> {
  let res: Response;
  try {
    res = await fetch(path, {
      method,
      credentials: "include",
      headers: body !== undefined ? { "Content-Type": "application/json" } : undefined,
      body: body !== undefined ? JSON.stringify(body) : undefined,
    });
  } catch (e) {
    throw new ApiError(
      { code: "INTERNAL", category: "io", message: "network error", retryable: true },
      0,
    );
  }
  if (res.ok) {
    if (res.status === 204) return undefined as T;
    const text = await res.text();
    return (text ? JSON.parse(text) : undefined) as T;
  }
  const err = await toApiError(res);
  if (err.isAuth && unauthorizedHandler) unauthorizedHandler();
  throw err;
}

async function requestForm<T>(path: string, form: FormData): Promise<T> {
  let res: Response;
  try {
    res = await fetch(path, { method: "POST", credentials: "include", body: form });
  } catch {
    throw new ApiError(
      { code: "INTERNAL", category: "io", message: "network error", retryable: true },
      0,
    );
  }
  if (res.ok) return (await res.json()) as T;
  throw await toApiError(res);
}

export const api = {
  get: <T>(p: string) => request<T>("GET", p),
  post: <T>(p: string, b?: unknown) => request<T>("POST", p, b),
  patch: <T>(p: string, b?: unknown) => request<T>("PATCH", p, b),
  del: <T>(p: string) => request<T>("DELETE", p),
  postForm: requestForm,
};

// --- endpoint groups (paths mirror the frozen route map) ---

export const authApi = {
  login: (email: string, password: string) =>
    api.post<SessionView>("/api/auth/login", { email, password }),
  logout: () => api.post<{ status: string }>("/api/auth/logout"),
  session: () => api.get<SessionView>("/api/auth/session"),
};

export const accountsApi = {
  list: () => api.get<AccountSummary[]>("/api/accounts"),
  create: (email: string, display_name: string, role: Role) =>
    api.post<CreateAccountResponse>("/api/accounts", { email, display_name, role }),
  setRole: (id: string, role: Role) =>
    api.patch<AccountSummary>(`/api/accounts/${id}/role`, { role }),
  setStatus: (id: string, status: "active" | "disabled") =>
    api.patch<AccountSummary>(`/api/accounts/${id}/status`, { status }),
  changePassword: (id: string, new_password: string) =>
    api.post<{ status: string }>(`/api/accounts/${id}/password`, { new_password }),
};

export const projectsApi = {
  list: () => api.get<ProjectView[]>("/api/projects"),
  create: (name: string, root?: string) =>
    api.post<ProjectView>("/api/projects", { name, root: root ?? null }),
  get: (id: string) => api.get<ProjectView>(`/api/projects/${id}`),
  status: (id: string) => api.get<ProjectStatusView>(`/api/projects/${id}/status`),
  freeze: (id: string) => api.post<FreezeResponse>(`/api/projects/${id}/freeze`),
  providers: (id: string) => api.get<ProviderProfileView[]>(`/api/projects/${id}/providers`),
  bindProvider: (id: string, profile_name: string) =>
    api.post<ProjectStatusView>(`/api/projects/${id}/provider`, { profile_name }),
  preflight: (id: string) => api.post<PreflightGateView>(`/api/projects/${id}/preflight`),
  integrate: (id: string, allow_remote_provider: boolean, remote_disclosure_confirmed: boolean) =>
    api.post<JobAccepted>(`/api/projects/${id}/integrate`, {
      allow_remote_provider,
      remote_disclosure_confirmed,
    }),
  compile: (id: string, output_path?: string) =>
    api.post<CompileResultView>(`/api/projects/${id}/compile`, { output_path: output_path ?? null }),
};

export const tokensApi = {
  list: (projectId: string) => api.get<TokenListView>(`/api/projects/${projectId}/tokens`),
  issue: (projectId: string, body: IssueTokenRequest) =>
    api.post<IssuedToken>(`/api/projects/${projectId}/tokens`, body),
  revoke: (projectId: string, tokenId: string) =>
    api.post<void>(`/api/projects/${projectId}/tokens/${tokenId}/revoke`),
  rotate: (projectId: string, tokenId: string) =>
    api.post<IssuedToken>(`/api/projects/${projectId}/tokens/${tokenId}/rotate`),
};

export const uploadApi = {
  target: (token: string) => api.get<UploadTargetView>(`/u/${token}`),
  upload: (token: string, file: File, ownerDisplayName?: string, ownerKind?: string) => {
    const form = new FormData();
    form.append("file", file);
    if (ownerDisplayName) form.append("owner_display_name", ownerDisplayName);
    if (ownerKind) form.append("owner_kind", ownerKind);
    return api.postForm<IngestAccepted>(`/u/${token}/upload`, form);
  },
};

export const reviewApi = {
  taxonomy: (id: string) => api.get<TaxonomyReviewView>(`/api/projects/${id}/taxonomy`),
  approveTaxonomy: (id: string, body: ApproveTaxonomyRequest) =>
    api.post<DecisionReceipt>(`/api/projects/${id}/taxonomy/approve`, body),
  clusters: (id: string) => api.get<ClusterSummaryView[]>(`/api/projects/${id}/clusters`),
  cluster: (id: string, cid: string) =>
    api.get<ClusterDetailView>(`/api/projects/${id}/clusters/${cid}`),
  approveCluster: (id: string, cid: string, body: ApproveClusterRequest) =>
    api.post<DecisionReceipt>(`/api/projects/${id}/clusters/${cid}/approve`, body),
  regenerate: (id: string, cid: string, body: RegenerateClusterRequest) =>
    api.post<DecisionReceipt>(`/api/projects/${id}/clusters/${cid}/regenerate`, body),
  gate: (id: string) => api.get<ReviewGateView>(`/api/projects/${id}/review/gate`),
  decisions: (id: string) => api.get<DecisionRecordView[]>(`/api/projects/${id}/decisions`),
};

export const servingApi = {
  status: (id: string) => api.get<PublicationView>(`/api/projects/${id}/serving`),
  publish: (id: string) => api.post<PublicationView>(`/api/projects/${id}/serving/publish`),
  unpublish: (id: string) => api.post<PublicationView>(`/api/projects/${id}/serving/unpublish`),
  // Machine read-only surface (unauthenticated; serves only when published).
  files: (id: string) => api.get<ServingFileListView>(`/api/serving/${id}/files`),
  fileUrl: (id: string, filePath: string) =>
    `/api/serving/${id}/file?path=${encodeURIComponent(filePath)}`,
  fileText: (id: string, filePath: string) =>
    fetch(servingApi.fileUrl(id, filePath), { credentials: "include" }).then((r) =>
      r.ok ? r.text() : Promise.reject(new Error(`HTTP ${r.status}`)),
    ),
  verify: (id: string) => api.get<ServingVerifyView>(`/api/serving/${id}/verify`),
  explain: (id: string, filePath: string) =>
    api.get<ServingProvenanceView>(`/api/serving/${id}/explain?path=${encodeURIComponent(filePath)}`),
  contract: (id: string) => api.get<McpContractView>(`/api/serving/${id}/contract`),
};

export const jobsApi = {
  adminJob: (projectId: string, jobId: string) =>
    api.get<JobSnapshot>(`/api/projects/${projectId}/jobs/${jobId}`),
  uploadJob: (token: string, jobId: string) =>
    api.get<JobSnapshot>(`/u/${token}/jobs/${jobId}`),
};

export const TERMINAL_JOB_STATES = new Set(["completed", "failed", "cancelled"]);
