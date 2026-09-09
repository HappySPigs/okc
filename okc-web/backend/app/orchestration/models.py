"""``orchestration.models`` — U3 request/response DTOs (E3-S1..E3-S7).

Pydantic v2 wire models. No ``okc`` binding type appears here (ADR-0002); engine
access is entirely through ``app.adapter.dto`` + the ``EngineWorker``.
"""

from __future__ import annotations

from pydantic import BaseModel, Field

# --- requests ---


class CreateProjectRequest(BaseModel):
    """E3-S1. ``root`` is optional; when given it must resolve inside projects_root."""

    name: str = Field(min_length=1)
    root: str | None = None


class BindProviderRequest(BaseModel):
    """E3-S4 NeedsProvider. Only a pre-provisioned profile NAME (A-2).

    ``role`` optionally targets a single AI role
    (``embedding|organizer|synthesis|critic``); omit it to set the default
    profile applied to all roles. The adapter forwards ``role`` to okc-core's
    ``set_ai_route``; routes persist in the project manifest.
    """

    profile_name: str = Field(min_length=1)
    role: str | None = None


class RunIntegrationRequest(BaseModel):
    """E3-S4 NeedsDisclosure. Per-run remote consent (never a standing grant)."""

    allow_remote_provider: bool = False
    remote_disclosure_confirmed: bool = False


class CompileRequest(BaseModel):
    """E3-S6. ``output_path`` optional; when given it must resolve inside projects_root."""

    output_path: str | None = None


# --- responses ---


class ProjectView(BaseModel):
    """E3-S1 project detail."""

    id: str
    name: str
    engine_root_abs_path: str
    curator_id: str
    freeze_state: str
    frozen_at: str | None = None
    source_count: int = 0
    created_at: str


class ProjectStatusView(BaseModel):
    """E3-S3 checkpoint projection over the core-authoritative ``status()``."""

    project_id: str
    checkpoint: str
    next_action: str
    resolver: str  # contributor | u3 | u4 | u5
    progression: list[str]
    blocked_steps: list[str]
    stale: bool
    frozen: bool
    source_count: int
    integration_plan_id: str | None = None


class FreezeResponse(BaseModel):
    """E3-S2 reproducible-snapshot gate result."""

    project_id: str
    freeze_state: str
    source_set_fingerprint: str
    frozen_at: str
    source_count: int


class ProviderProfileView(BaseModel):
    """E3-S4 allowlist row — NEVER exposes the api-key env-var name/value."""

    name: str
    kind: str
    endpoint: str
    model: str


class RouteBoundaryView(BaseModel):
    role: str
    profile_name: str
    boundary: str  # 'local' | 'remote'


class PreflightGateView(BaseModel):
    """E3-S4 disclosure gate — ``requires_disclosure`` = any route is remote."""

    project_id: str
    sensitive_findings: int = 0
    routes: list[RouteBoundaryView] = Field(default_factory=list)
    requires_disclosure: bool = False


class JobAccepted(BaseModel):
    """E3-S7 long-op receipt; poll GET /api/projects/{id}/jobs/{job_id} (U0)."""

    job_id: str
    project_id: str


class CompileResultView(BaseModel):
    """E3-S6 compiled-vault handoff to U5."""

    project_id: str
    path: str
    integration_plan_id: str | None = None
    file_count: int = 0
