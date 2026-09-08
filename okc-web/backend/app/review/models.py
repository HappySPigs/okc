"""``review.models`` — U4 request/response DTOs + the critic-payload parser (E4).

Pydantic v2 wire models. No ``okc`` binding type appears here (ADR-0002); the raw
``clusters()`` / ``taxonomy()`` payloads arrive as opaque JSON (already unwrapped by
``app.adapter.engine``) and are parsed into typed review views here.

Payload shape (verified against okc-core ``crates/okc-core/src/integration.rs`` and
``crates/okc-interop``): ``clusters()`` returns a JSON **array** of ``{proposal, critic}``.
The cluster id lives at ``element.proposal.cluster_id``. Each ``critic.findings[]`` entry
carries ``finding_id``/``severity``/``kind``/``message``/``evidence[]``; ``severity`` is the
bare lowercase string ``"minor"``/``"major"``/``"critical"`` (serde ``rename_all=snake_case``).
"""

from __future__ import annotations

from collections.abc import Mapping
from typing import Any

from pydantic import BaseModel, Field

# okc-core CriticSeverity values (snake_case unit variants → plain lowercase).
BLOCKING_SEVERITIES = frozenset({"major", "critical"})


def is_blocking(severity_raw: str) -> bool:
    """Major/Critical block approval; Minor is waivable. An unrecognized severity is
    treated as **blocking (fail-closed)** — we never silently treat an unknown value as
    waivable (defensive parse; E4-S2)."""
    sev = (severity_raw or "").strip().lower()
    if sev == "minor":
        return False
    if sev in BLOCKING_SEVERITIES:
        return True
    return True  # unknown/unparseable → fail-closed


# --- requests (okc-web -> engine) ---


class ApproveTaxonomyRequest(BaseModel):
    """E4-S1. ``edited_clusters`` is the opaque okc-core taxonomy shape; ``rationale``
    is required iff the taxonomy was edited (enforced in the service)."""

    edited_clusters: list[dict[str, Any]] | None = None
    rationale: str | None = None


class ApproveClusterRequest(BaseModel):
    """E4-S3. Rationale is mandatory for every waiver/omission (DecisionGate)."""

    # key = Minor ``finding_id`` -> rationale
    minor_waivers: dict[str, str] = Field(default_factory=dict)
    # key = core omission_key "{document_id}:{target_id}" -> rationale
    omission_rationales: dict[str, str] = Field(default_factory=dict)


class RegenerateClusterRequest(BaseModel):
    """E4-S4. Feedback drives the new synthesis; per-run remote consent only (never
    a standing grant), same shape as U3 integrate."""

    feedback: str = Field(min_length=1)
    allow_remote_provider: bool = False
    remote_disclosure_confirmed: bool = False


# --- views (engine -> okc-web) ---


class CriticFindingView(BaseModel):
    """E4-S2 severity-graded finding. ``severity`` is the raw lowercase core value;
    ``blocking`` is the derived waive-forbidden flag."""

    finding_id: str
    severity: str
    kind: str
    message: str
    blocking: bool


class TaxonomyReviewView(BaseModel):
    """E4-S1 taxonomy proposal — carried opaque (okc-core taxonomy shape)."""

    project_id: str
    interop_schema_version: int | None = None
    payload: Any = None


class ClusterSummaryView(BaseModel):
    """E4-S2 cluster list row — severity roll-up for the review board."""

    cluster_id: str
    finding_count: int = 0
    blocking_count: int = 0
    minor_count: int = 0
    blocking: bool = False


class ClusterDetailView(BaseModel):
    """E4-S2/E4-S5 cluster detail. ``proposal`` and ``contradictions`` are opaque and
    **read-only** (contradictions are preserved, never resolved — C-2)."""

    cluster_id: str
    proposal_hash: str | None = None
    critic_hash: str | None = None
    taxonomy_hash: str | None = None
    findings: list[CriticFindingView] = Field(default_factory=list)
    blocking: bool = False
    proposal: Any = None
    contradictions: Any = None


class BlockingItemView(BaseModel):
    """E4-S6 unresolved item + the action that clears it."""

    cluster_id: str
    finding_id: str
    severity: str
    required_action: str  # "regenerate"


class ReviewGateView(BaseModel):
    """E4-S6 compile-eligibility scoreboard. Read-only; never triggers compile."""

    project_id: str
    state: str  # "Blocked" | "PendingApprovals" | "Ready"
    checkpoint: str | None = None
    blocking_items: list[BlockingItemView] = Field(default_factory=list)


class DecisionRecordView(BaseModel):
    """A row from the append-only ``curator_decisions`` audit trail (E4-S1/S3/S4)."""

    id: str
    decision_kind: str
    target_ref: str | None = None
    core_op: str
    core_job_id: str | None = None
    created_at: str


class DecisionReceipt(BaseModel):
    """Ack for a recorded curator decision. ``job_id`` is set only for the long
    regenerate op (poll GET /api/projects/{id}/jobs/{job_id})."""

    project_id: str
    audit_id: str
    decision_kind: str
    job_id: str | None = None


# --- parsers over the opaque clusters() payload ---


def parse_findings(critic: Mapping[str, Any]) -> list[CriticFindingView]:
    """Parse ``critic.findings[]`` into typed, severity-graded views (defensive)."""
    out: list[CriticFindingView] = []
    for f in critic.get("findings") or []:
        if not isinstance(f, Mapping):
            continue
        severity = str(f.get("severity", ""))
        out.append(
            CriticFindingView(
                finding_id=str(f.get("finding_id", "")),
                severity=severity.strip().lower(),
                kind=str(f.get("kind", "")),
                message=str(f.get("message", "")),
                blocking=is_blocking(severity),
            )
        )
    return out


def iter_clusters(payload: Any) -> list[tuple[str, Mapping[str, Any], Mapping[str, Any]]]:
    """Yield ``(cluster_id, proposal, critic)`` for each ``{proposal, critic}`` element
    of the opaque ``clusters()`` array. Defensive against shape drift."""
    rows: list[tuple[str, Mapping[str, Any], Mapping[str, Any]]] = []
    if not isinstance(payload, list):
        return rows
    for element in payload:
        if not isinstance(element, Mapping):
            continue
        raw_proposal = element.get("proposal")
        raw_critic = element.get("critic")
        proposal: Mapping[str, Any] = raw_proposal if isinstance(raw_proposal, Mapping) else {}
        critic: Mapping[str, Any] = raw_critic if isinstance(raw_critic, Mapping) else {}
        cluster_id = str(proposal.get("cluster_id") or critic.get("cluster_id") or "")
        rows.append((cluster_id, proposal, critic))
    return rows


def summarize_cluster(cluster_id: str, critic: Mapping[str, Any]) -> ClusterSummaryView:
    findings = parse_findings(critic)
    blocking = [f for f in findings if f.blocking]
    return ClusterSummaryView(
        cluster_id=cluster_id,
        finding_count=len(findings),
        blocking_count=len(blocking),
        minor_count=len(findings) - len(blocking),
        blocking=bool(blocking),
    )
