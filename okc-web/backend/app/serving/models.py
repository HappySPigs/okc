"""``serving.models`` — U5 request/response DTOs (E5-S1..E5-S5).

Pydantic v2 wire models. No ``okc`` binding type appears here (ADR-0002); engine
payloads reach these only as already-converted ``app.adapter.dto`` views (``Any``
where the underlying okc-core structure stays opaque at the seam).
"""

from __future__ import annotations

from typing import Any

from pydantic import BaseModel, Field

# Serving status is derived (offline before publish; live once published; stale
# when the frozen input set has drifted from the bound manifest — E5-S5).
ServingStatus = str  # 'offline' | 'live' | 'stale'


class PublicationView(BaseModel):
    """E5-S1/E5-S5 publication state (admin surface + reused in machine views).

    ``status`` is the EFFECTIVE status (staleness folded in at read time). The
    bound identity fields say WHICH compiled manifest the URL is pinned to."""

    project_id: str
    revision: str | None = None
    access: str = "public"
    status: ServingStatus
    stale: bool = False
    compiled_vault_path: str | None = None
    bound_integration_plan_id: str | None = None
    bound_corpus_hash: str | None = None
    bound_taxonomy_hash: str | None = None
    published_at: str | None = None
    published_by: str | None = None


class ServingFileListView(BaseModel):
    """E5-S2 file list + E5-S5 served-manifest identity/stale label."""

    project_id: str
    revision: str | None = None
    file_hashes: dict[str, str] = Field(default_factory=dict)
    status: ServingStatus
    stale: bool = False
    bound_integration_plan_id: str | None = None
    files: list[str] = Field(default_factory=list)  # relative paths under the 3 roots


class ServingVerifyView(BaseModel):
    """E5-S3 verify() — integrity/internal-consistency, NOT publisher authenticity."""

    project_id: str
    revision: str | None = None
    status: ServingStatus
    stale: bool = False
    valid: bool = False
    artifact_path: str = ""
    manifest: Any = None
    note: str = (
        "verify() proves the compiled vault's internal consistency with its approved"
        " plan; it is not a guarantee of source/publisher authenticity."
    )


class ServingProvenanceView(BaseModel):
    """E5-S3 explain() — per-file ProvenanceRecord + owner labels (owner_display_name
    is a decorative, unverified label supplied at upload time)."""

    project_id: str
    revision: str | None = None
    status: ServingStatus
    stale: bool = False
    file_path: str
    artifact_path: str = ""
    record: Any = None
    owner_labels: dict[str, Any] = Field(default_factory=dict)  # source_id -> {display_name, kind}
    note: str = (
        "Provenance lineage is preserved as-is; contradictions are kept (no winner is"
        " selected). owner_display_name is an unverified label."
    )


class ContractLocation(BaseModel):
    local_dir: str | None = None  # absolute compiled-vault directory
    read_api_base: str  # base URL for the read-only serving API


class ContractFormat(BaseModel):
    content: str = "Markdown-only (compiled merged vault)"
    layout: list[str] = Field(default_factory=lambda: ["knowledge/", "legacy/", ".okc/"])


class ContractEndpoint(BaseModel):
    method: str
    path: str
    description: str


class McpContractView(BaseModel):
    """E5-S4 okc-mcp RAG-source consumption contract (location + format only).

    Exposes ONLY read-only discovery endpoints; RAG retrieval (chunking/embedding/
    vector-index/query) and the MCP tool surface are okc-mcp's job and OUT of scope."""

    project_id: str
    revision: str | None = None
    protocol_version: int = 1
    access: str = "public"
    status: ServingStatus
    stale: bool = False
    location: ContractLocation
    format: ContractFormat = Field(default_factory=ContractFormat)
    bound_integration_plan_id: str | None = None
    bound_corpus_hash: str | None = None
    bound_taxonomy_hash: str | None = None
    endpoints: list[ContractEndpoint] = Field(default_factory=list)
    out_of_scope: str = (
        "RAG retrieval (chunking, embedding, vector index, query) and the MCP tool"
        " surface are okc-mcp's responsibility and are out of scope here. okc-web"
        " serves read-only Markdown only; core embeddings are ephemeral, so the"
        " consumer may use lexical search or re-embed artifacts for semantic search."
    )


class AccessRequest(BaseModel):
    mode: str


class RestoreRequest(BaseModel):
    revision: str = Field(pattern=r"^[a-f0-9]{64}$")
