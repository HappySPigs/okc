"""``adapter.dto`` — okc-web-owned Views/Cmds/Specs that mirror the okc binding
schema v2, plus ``from_native(...)`` parsers over the binding's ``dict`` payloads.

Together with ``engine.py`` this is the ONLY place that reads raw ``okc`` payload
shapes (ADR-0002). Downstream units see okc-web Pydantic types only; binding
churn (and the "opaque JSON payload" concern of req C-7) stops here.

Where a payload wraps a deep okc-core structure (checkpoint, manifest, provenance
record, taxonomy/cluster JSON), we carry it as ``Any`` at the seam; U3/U4/U5
refine these into typed views within their own wave.
"""

from __future__ import annotations

from collections.abc import Mapping
from typing import Any

from pydantic import BaseModel, ConfigDict, Field

# --- Commands / Specs (okc-web -> engine) ---


class CreateProjectSpec(BaseModel):
    root_abs_path: str
    name: str
    curator_id: str
    policy_version: str = "policy-v3"
    language: str | None = None


class AddSourceCmd(BaseModel):
    source_id: str
    absolute_path: str
    owner_display_name: str | None = None
    snapshot_id: str | None = None


class DisclosureCmd(BaseModel):
    allow_remote_provider: bool = False
    remote_disclosure_confirmed: bool = False


class ApproveTaxonomyCmd(BaseModel):
    # okc-core `list[TaxonomyCluster]` shape, opaque at the seam.
    edited_clusters: list[dict[str, Any]] | None = None
    rationale: str | None = None


class ApproveClusterCmd(BaseModel):
    cluster_id: str
    # key = "{document_id}:{target_id}" (core omission_key)
    omission_rationales: dict[str, str] = Field(default_factory=dict)
    # key = Minor finding_id
    minor_waivers: dict[str, str] = Field(default_factory=dict)


class RegenerateClusterCmd(BaseModel):
    cluster_id: str
    feedback: str
    disclosure: DisclosureCmd = Field(default_factory=DisclosureCmd)


class ProviderSpecView(BaseModel):
    """okc-web-owned provider spec (env-var NAME only, never a secret value)."""

    name: str
    kind: str
    endpoint: str
    model: str
    api_key_env: str | None = None


# --- Views (engine -> okc-web) ---


class ProjectRefView(BaseModel):
    root_abs_path: str


class StatusView(BaseModel):
    interop_schema_version: int | None = None
    checkpoint: Any = None
    integration: Any = None

    @classmethod
    def from_native(cls, payload: Mapping[str, Any]) -> StatusView:
        return cls(
            interop_schema_version=payload.get("interop_schema_version"),
            checkpoint=payload.get("checkpoint"),
            integration=payload.get("integration"),
        )


class PreflightView(BaseModel):
    model_config = ConfigDict(extra="allow")

    run_id: str | None = None
    documents: int = 0
    blocks: int = 0
    input_bytes: int = 0
    estimated_tokens_min: int = 0
    estimated_tokens_max: int = 0
    estimated_requests_min: int = 0
    estimated_requests_max: int = 0
    sensitive_findings: int = 0
    routes: Any = None

    @classmethod
    def from_native(cls, payload: Mapping[str, Any]) -> PreflightView:
        return cls.model_validate(dict(payload))


class IntegrationView(BaseModel):
    model_config = ConfigDict(extra="allow")

    interop_schema_version: int | None = None
    run_id: str | None = None
    documents: int = 0
    embedding_inputs: int = 0
    input_bytes: int = 0
    sensitive_findings: int = 0
    semantic_candidates: int = 0
    checkpoint: Any = None
    integration_plan_id: str | None = None

    @classmethod
    def from_native(cls, payload: Mapping[str, Any]) -> IntegrationView:
        return cls.model_validate(dict(payload))


class CompileView(BaseModel):
    path: str
    integration_plan_id: str | None = None
    file_count: int = 0
    manifest: Any = None

    @classmethod
    def from_native(cls, payload: Mapping[str, Any]) -> CompileView:
        return cls(
            path=str(payload.get("path", "")),
            integration_plan_id=payload.get("integration_plan_id"),
            file_count=int(payload.get("file_count", 0) or 0),
            manifest=payload.get("manifest"),
        )


class VerificationView(BaseModel):
    interop_schema_version: int | None = None
    valid: bool = False
    artifact_path: str = ""
    manifest: Any = None

    @classmethod
    def from_native(cls, payload: Mapping[str, Any]) -> VerificationView:
        return cls(
            interop_schema_version=payload.get("interop_schema_version"),
            valid=bool(payload.get("valid", False)),
            artifact_path=str(payload.get("artifact_path", "")),
            manifest=payload.get("manifest"),
        )


class ProvenanceView(BaseModel):
    interop_schema_version: int | None = None
    artifact_path: str = ""
    record: Any = None

    @classmethod
    def from_native(cls, payload: Mapping[str, Any]) -> ProvenanceView:
        return cls(
            interop_schema_version=payload.get("interop_schema_version"),
            artifact_path=str(payload.get("artifact_path", "")),
            record=payload.get("record"),
        )
