"""``serving.store`` — U5 ServingStateStore over the ``serving_publications`` table.

Publish/unpublish is an okc-web-only, admin-gated PURE SQLite state flip: NO
engine op and NO write-queue involvement (design §U5). The physical schema is
frozen in W0 (``MIGRATION_0001_INIT``); this store only reads/writes rows.
"""

from __future__ import annotations

from dataclasses import dataclass
from datetime import UTC, datetime

from sqlalchemy import text

from app.shared.state import StateDb


def _now() -> str:
    return datetime.now(UTC).isoformat()


@dataclass
class PublicationRow:
    project_id: str
    compiled_vault_path: str
    bound_integration_plan_id: str | None
    bound_corpus_hash: str | None
    bound_taxonomy_hash: str | None
    status: str  # 'offline' | 'live' | 'stale' (persisted; effective staleness is derived)
    published_at: str | None
    published_by: str | None


class ServingStateStore:
    """Read/write the ``serving_publications`` table (one row per project)."""

    def __init__(self, db: StateDb) -> None:
        self._db = db

    def get(self, project_id: str) -> PublicationRow | None:
        with self._db.engine.connect() as conn:
            row = conn.execute(
                text(
                    "SELECT project_id, compiled_vault_path, bound_integration_plan_id,"
                    " bound_corpus_hash, bound_taxonomy_hash, status, published_at, published_by"
                    " FROM serving_publications WHERE project_id=:pid"
                ),
                {"pid": project_id},
            ).mappings().first()
        if row is None:
            return None
        return PublicationRow(
            project_id=row["project_id"],
            compiled_vault_path=row["compiled_vault_path"],
            bound_integration_plan_id=row["bound_integration_plan_id"],
            bound_corpus_hash=row["bound_corpus_hash"],
            bound_taxonomy_hash=row["bound_taxonomy_hash"],
            status=row["status"],
            published_at=row["published_at"],
            published_by=row["published_by"],
        )

    def publish(
        self,
        *,
        project_id: str,
        compiled_vault_path: str,
        bound_integration_plan_id: str | None,
        bound_corpus_hash: str | None,
        bound_taxonomy_hash: str | None,
        published_by: str | None,
    ) -> PublicationRow:
        """Upsert the publication row to ``live``, rebinding it to the given manifest
        identity. (Re)publish never mutates any compiled output — it only flips state."""
        now = _now()
        with self._db.engine.begin() as conn:
            conn.execute(
                text(
                    "INSERT INTO serving_publications(project_id, compiled_vault_path,"
                    " bound_integration_plan_id, bound_corpus_hash, bound_taxonomy_hash,"
                    " status, published_at, published_by)"
                    " VALUES(:pid,:path,:plan,:corpus,:tax,'live',:at,:by)"
                    " ON CONFLICT(project_id) DO UPDATE SET"
                    " compiled_vault_path=excluded.compiled_vault_path,"
                    " bound_integration_plan_id=excluded.bound_integration_plan_id,"
                    " bound_corpus_hash=excluded.bound_corpus_hash,"
                    " bound_taxonomy_hash=excluded.bound_taxonomy_hash,"
                    " status='live', published_at=excluded.published_at,"
                    " published_by=excluded.published_by"
                ),
                {
                    "pid": project_id, "path": compiled_vault_path,
                    "plan": bound_integration_plan_id, "corpus": bound_corpus_hash,
                    "tax": bound_taxonomy_hash, "at": now, "by": published_by,
                },
            )
        got = self.get(project_id)
        assert got is not None  # just inserted
        return got

    def unpublish(self, project_id: str) -> None:
        """Flip to ``offline`` (idempotent — a no-op when there is no row)."""
        with self._db.engine.begin() as conn:
            conn.execute(
                text("UPDATE serving_publications SET status='offline' WHERE project_id=:pid"),
                {"pid": project_id},
            )
