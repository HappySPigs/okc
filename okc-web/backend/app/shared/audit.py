"""``shared.audit`` — S0.E CuratorAuditService (record-then-act, Q8/C3).

For every U4 Case-B decision and U3 freeze/approval, the audit row is appended
(with ``HashBindings``) BEFORE the engine op runs; the resulting ``JobId`` is
linked after. ``CuratorDecision`` is a Pydantic **discriminated union with
exactly three variants** and **no winner-select variant** — the code-verifiable
C3 differentiator (ADR-0024). The DB CHECK on ``decision_kind`` enforces it again.
"""

from __future__ import annotations

from datetime import UTC, datetime
from typing import Annotated, Literal

from pydantic import BaseModel, Field, TypeAdapter
from sqlalchemy import text
from ulid import ULID

from app.shared.error import EngineError
from app.shared.state import StateDb


class HashBindings(BaseModel):
    """The three hash bindings recorded with each decision (C-2/C-4)."""

    proposal_hash: str | None = None
    critic_hash: str | None = None
    taxonomy_hash: str | None = None


class ApproveTaxonomy(BaseModel):
    kind: Literal["approve_taxonomy"] = "approve_taxonomy"
    # okc-core `list[TaxonomyCluster]` shape, opaque here. Rationale required iff edited.
    edited_clusters: list[dict] | None = None
    rationale: str | None = None


class ApproveCluster(BaseModel):
    kind: Literal["approve_cluster"] = "approve_cluster"
    cluster_id: str
    omission_rationales: dict[str, str] = Field(default_factory=dict)  # key "{document_id}:{target_id}"
    minor_waivers: dict[str, str] = Field(default_factory=dict)  # key = Minor finding_id


class RegenerateCluster(BaseModel):
    kind: Literal["regenerate_cluster"] = "regenerate_cluster"
    cluster_id: str
    feedback: str


# The ONLY way to mutate review/approval state. Exactly three variants;
# deliberately NO `SelectWinner`/`ResolveContradiction` (C3, ADR-0024). A
# winner-select decision is not representable in this type.
CuratorDecision = Annotated[
    ApproveTaxonomy | ApproveCluster | RegenerateCluster,
    Field(discriminator="kind"),
]

CuratorDecisionAdapter: TypeAdapter[object] = TypeAdapter(CuratorDecision)

# The complete, closed set of decision kinds (mirrors the DB CHECK constraint).
DECISION_KINDS: tuple[str, ...] = ("approve_taxonomy", "approve_cluster", "regenerate_cluster")


def decision_kind(d: ApproveTaxonomy | ApproveCluster | RegenerateCluster) -> str:
    return d.kind


def core_op(d: ApproveTaxonomy | ApproveCluster | RegenerateCluster) -> str:
    return d.kind  # 1:1 with the engine op name


def target_ref(d: ApproveTaxonomy | ApproveCluster | RegenerateCluster) -> str | None:
    if isinstance(d, ApproveTaxonomy):
        return None
    return d.cluster_id


class NewDecisionRecord(BaseModel):
    project_id: str
    account_id: str | None = None
    curator_id: str
    decision: CuratorDecision
    bindings: HashBindings = Field(default_factory=HashBindings)


class AuditStore:
    def __init__(self, db: StateDb) -> None:
        self._db = db

    def append(self, record: NewDecisionRecord) -> str:
        """Append-only insert BEFORE the engine op. Returns the audit row id."""
        audit_id = f"dec_{ULID()}"
        now = datetime.now(UTC).isoformat()
        d = record.decision
        payload = d.model_dump_json()
        with self._db.engine.begin() as conn:
            conn.execute(
                text(
                    "INSERT INTO curator_decisions"
                    "(id, project_id, account_id, curator_id, decision_kind, target_ref,"
                    " proposal_hash, critic_hash, taxonomy_hash, payload_json, core_op, created_at)"
                    " VALUES (:id,:pid,:aid,:cid,:kind,:target,:ph,:ch,:th,:payload,:op,:now)"
                ),
                {
                    "id": audit_id, "pid": record.project_id, "aid": record.account_id,
                    "cid": record.curator_id, "kind": decision_kind(d), "target": target_ref(d),
                    "ph": record.bindings.proposal_hash, "ch": record.bindings.critic_hash,
                    "th": record.bindings.taxonomy_hash, "payload": payload,
                    "op": core_op(d), "now": now,
                },
            )
        return audit_id

    def link_job(self, audit_id: str, job_id: str) -> None:
        with self._db.engine.begin() as conn:
            conn.execute(
                text("UPDATE curator_decisions SET core_job_id=:jid WHERE id=:id"),
                {"id": audit_id, "jid": job_id},
            )

    def list_for_project(self, project_id: str) -> list[dict]:
        with self._db.engine.connect() as conn:
            return [
                dict(r)
                for r in conn.execute(
                    text(
                        "SELECT id, decision_kind, target_ref, core_op, core_job_id, created_at "
                        "FROM curator_decisions WHERE project_id=:pid ORDER BY created_at"
                    ),
                    {"pid": project_id},
                ).mappings()
            ]

    def raise_if_invalid(self, project_id: str) -> None:  # pragma: no cover - guard hook
        if not project_id:
            raise EngineError.validation("project_id required")
