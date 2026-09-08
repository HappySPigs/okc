"""``orchestration.projects`` — U3 project registry (the ``projects`` table) plus
read-only source/job introspection.

U3 owns the *semantics* of the ``projects`` table; the physical schema was frozen
in W0 (``MIGRATION_0001_INIT``), so there is NO new migration here. The ``sources``
and ``jobs`` tables are read-only from U3 (written by U2 / U0).
"""

from __future__ import annotations

import hashlib
from dataclasses import dataclass
from datetime import UTC, datetime

from sqlalchemy import text

from app.shared.error import EngineError
from app.shared.state import StateDb

# Canonical source-set fingerprint format (versioned so it can evolve).
_FINGERPRINT_PREFIX = "okcweb-srcset-v1"

# Checkpoint -> (next_action, resolver). Values are the seven core snake_case
# checkpoint strings (business-logic-model §2). Unknown -> a safe default.
CHECKPOINT_ACTIONS: dict[str, tuple[str, str]] = {
    "needs_sources": ("register sources (upload)", "contributor"),
    "needs_provider": ("bind a provisioned provider profile", "u3"),
    "needs_disclosure": ("run preflight and confirm remote disclosure, then integrate", "u3"),
    "needs_taxonomy": ("approve taxonomy", "u4"),
    "needs_clusters": ("approve or regenerate clusters", "u4"),
    "ready_to_compile": ("compile the merged vault", "u3"),
    "verified": ("serve / publish", "u5"),
}

# Human-facing progression order (E3-S3). NOTE: this differs from core's
# derivation precedence — the raw checkpoint is always the source of truth.
PROGRESSION: list[str] = [
    "needs_provider", "needs_sources", "needs_disclosure",
    "needs_taxonomy", "needs_clusters", "ready_to_compile", "verified",
]

# Reserving jobs are the only rows that reach the ``jobs`` table (long ops).
_ACTIVE_JOB_STATES = ("queued", "running")


def _now() -> str:
    return datetime.now(UTC).isoformat()


@dataclass
class ProjectRow:
    id: str
    name: str
    engine_root_abs_path: str
    curator_id: str
    freeze_state: str
    frozen_at: str | None
    source_set_fingerprint: str | None
    created_at: str


class ProjectRegistry:
    """Read/write the ``projects`` table; read-only helpers over ``sources``/``jobs``."""

    def __init__(self, db: StateDb) -> None:
        self._db = db

    # --- projects (write) ---

    def create(
        self, project_id: str, name: str, engine_root: str, curator_id: str, created_by: str
    ) -> None:
        now = _now()
        with self._db.engine.begin() as conn:
            conn.execute(
                text(
                    "INSERT INTO projects(id, name, engine_root_abs_path, curator_id, created_by,"
                    " freeze_state, created_at, updated_at)"
                    " VALUES(:id,:name,:root,:cur,:by,'unfrozen',:now,:now)"
                ),
                {"id": project_id, "name": name, "root": engine_root,
                 "cur": curator_id, "by": created_by, "now": now},
            )

    def set_frozen(self, project_id: str, fingerprint: str, frozen_at: str) -> None:
        with self._db.engine.begin() as conn:
            conn.execute(
                text(
                    "UPDATE projects SET freeze_state='frozen', source_set_fingerprint=:fp,"
                    " frozen_at=:at, updated_at=:at WHERE id=:id"
                ),
                {"id": project_id, "fp": fingerprint, "at": frozen_at},
            )

    # --- projects (read) ---

    def get(self, project_id: str) -> ProjectRow:
        with self._db.engine.connect() as conn:
            row = conn.execute(
                text(
                    "SELECT id, name, engine_root_abs_path, curator_id, freeze_state,"
                    " frozen_at, source_set_fingerprint, created_at"
                    " FROM projects WHERE id=:id"
                ),
                {"id": project_id},
            ).mappings().first()
        if row is None:
            raise EngineError.not_found("project not found")
        return ProjectRow(
            id=row["id"], name=row["name"], engine_root_abs_path=row["engine_root_abs_path"],
            curator_id=row["curator_id"], freeze_state=row["freeze_state"],
            frozen_at=row["frozen_at"], source_set_fingerprint=row["source_set_fingerprint"],
            created_at=row["created_at"],
        )

    def list_rows(self) -> list[ProjectRow]:
        with self._db.engine.connect() as conn:
            rows = conn.execute(
                text(
                    "SELECT id, name, engine_root_abs_path, curator_id, freeze_state,"
                    " frozen_at, source_set_fingerprint, created_at"
                    " FROM projects ORDER BY created_at DESC"
                )
            ).mappings().all()
        return [
            ProjectRow(
                id=r["id"], name=r["name"], engine_root_abs_path=r["engine_root_abs_path"],
                curator_id=r["curator_id"], freeze_state=r["freeze_state"], frozen_at=r["frozen_at"],
                source_set_fingerprint=r["source_set_fingerprint"], created_at=r["created_at"],
            )
            for r in rows
        ]

    # --- sources (read-only; written by U2) ---

    def source_count(self, project_id: str) -> int:
        with self._db.engine.connect() as conn:
            return int(
                conn.execute(
                    text("SELECT COUNT(*) FROM sources WHERE project_id=:pid"),
                    {"pid": project_id},
                ).scalar()
                or 0
            )

    def source_set_fingerprint(self, project_id: str) -> str:
        """Deterministic, versioned digest over sorted (source_id, content_hash)."""
        with self._db.engine.connect() as conn:
            rows = conn.execute(
                text(
                    "SELECT source_id, content_hash FROM sources"
                    " WHERE project_id=:pid ORDER BY source_id"
                ),
                {"pid": project_id},
            ).all()
        body = "\n".join(f"{sid}:{ch}" for sid, ch in rows)
        return hashlib.sha256(f"{_FINGERPRINT_PREFIX}\n{body}".encode()).hexdigest()

    # --- jobs (read-only; written by U0) ---

    def has_active_reserving_job(self, project_id: str) -> bool:
        """Any queued/running long op for this project (single active mutation, Q7)."""
        with self._db.engine.connect() as conn:
            row = conn.execute(
                text(
                    "SELECT 1 FROM jobs WHERE project_id=:pid AND state IN ('queued','running') LIMIT 1"
                ),
                {"pid": project_id},
            ).first()
        return row is not None
