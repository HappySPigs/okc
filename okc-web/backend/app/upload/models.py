"""``upload.models`` — U2 request/response DTOs (E2-S1..E2-S6).

Pydantic v2 models on the wire. No ``okc`` binding type appears here (ADR-0002);
engine access is entirely through ``state.engine`` / the adapter DTOs.
"""

from __future__ import annotations

from pydantic import BaseModel, Field

# The hard per-project source cap (C-3 / ADR-0016). okc-web owns this ahead of
# okc-core's ``ResourceLimit`` backstop.
SOURCE_CAP = 10


class IssueTokenRequest(BaseModel):
    """E2-S1 issue payload. ``owner_*`` are decorative, unverified labels."""

    owner_display_name: str | None = None
    owner_kind: str | None = None  # 'department' | 'individual'
    ttl_seconds: int | None = None  # optional expiry window


class IssuedToken(BaseModel):
    """E2-S1/E2-S2 one-time issue result. ``token`` is the plaintext
    ``selector.verifier`` shown EXACTLY once and never persisted or re-shown."""

    token_id: str
    token: str
    upload_url: str
    project_id: str
    slot_index: int
    owner_display_name: str | None = None
    owner_kind: str | None = None
    created_at: str
    expires_at: str | None = None
    sync_endpoint: str = "/api/sync"


class TokenSummary(BaseModel):
    """E2-S2 masked list row — never exposes the plaintext token."""

    token_id: str
    project_id: str
    slot_index: int
    selector: str
    status: str  # active | revoked | expired
    owner_display_name: str | None = None
    owner_kind: str | None = None
    created_at: str
    expires_at: str | None = None
    last_used_at: str | None = None
    registered_source_id: str | None = None


class SlotUsage(BaseModel):
    used: int
    limit: int = SOURCE_CAP


class TokenListView(BaseModel):
    tokens: list[TokenSummary]
    slot_usage: SlotUsage


class UploadTargetView(BaseModel):
    """E2-3 portal target info yielded by the token capability."""

    project_id: str
    project_name: str
    slot_index: int
    owner_display_name: str | None = None
    owner_kind: str | None = None
    status: str = "active"


class CheckResult(BaseModel):
    """One ArchiveValidator check outcome (E2-4 checklist)."""

    name: str
    status: str  # pass | warn | fail
    code: str | None = None
    detail: str | None = None


class ValidationReport(BaseModel):
    ok: bool = True
    warnings: list[CheckResult] = Field(default_factory=list)
    markdown_files: int = 0
    total_files: int = 0


class IngestAccepted(BaseModel):
    """E2-S3/E2-S5 accepted receipt. ``job_id`` is polled at
    ``GET /u/{token}/jobs/{jobId}`` (E2-S6, mounted by U0)."""

    job_id: str
    source_id: str
    project_id: str
    slot_index: int
    content_hash: str
    owner_display_name: str | None = None
    warnings: list[CheckResult] = Field(default_factory=list)
