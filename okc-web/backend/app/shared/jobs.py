"""``shared.jobs`` — the JobStore, single source of truth for job-status polling
(Q7). Long engine ops return a ``JobId``; the engine worker pumps binding
progress into ``job_events``; clients poll a ``JobSnapshot``.

All writes happen on the single-writer engine worker thread; ``get`` is read by
the polling route (fast SQLite read over WAL). Methods are synchronous.
"""

from __future__ import annotations

from dataclasses import dataclass
from datetime import UTC, datetime

from pydantic import BaseModel
from sqlalchemy import text
from ulid import ULID

from app.shared.error import EngineError, EngineErrorCategory, EngineErrorCode
from app.shared.state import StateDb

JobId = str


def _now() -> str:
    return datetime.now(UTC).isoformat()


@dataclass
class JobProgress:
    """Runtime-neutral progress event (adapter maps a binding event into this,
    keeping binding types inside ``adapter``)."""

    sequence: int
    state: str
    phase: str
    completed: int
    total: int | None
    current_item: str | None


class JobErrorDto(BaseModel):
    code: EngineErrorCode
    category: EngineErrorCategory
    retryable: bool


class JobEventDto(BaseModel):
    sequence: int
    phase: str | None = None
    state: str | None = None
    completed: int | None = None
    total: int | None = None
    current_item: str | None = None
    ts: str


class JobSnapshot(BaseModel):
    id: JobId
    project_id: str | None = None
    kind: str
    state: str
    phase: str | None = None
    progress_completed: int = 0
    progress_total: int | None = None
    error: JobErrorDto | None = None
    events: list[JobEventDto] = []
    created_at: str
    updated_at: str
    finished_at: str | None = None


class JobStore:
    def __init__(self, db: StateDb) -> None:
        self._db = db

    def create(self, kind: str, project_id: str | None, requested_by: str | None) -> JobId:
        job_id = f"job_{ULID()}"
        now = _now()
        with self._db.engine.begin() as conn:
            conn.execute(
                text(
                    "INSERT INTO jobs(id, project_id, kind, state, progress_completed, created_at, updated_at, requested_by) "
                    "VALUES (:id, :pid, :kind, 'queued', 0, :now, :now, :by)"
                ),
                {"id": job_id, "pid": project_id, "kind": kind, "now": now, "by": requested_by},
            )
        return job_id

    def mark_running(self, job_id: str) -> None:
        now = _now()
        with self._db.engine.begin() as conn:
            conn.execute(
                text(
                    "UPDATE jobs SET state='running', started_at=COALESCE(started_at, :now), updated_at=:now WHERE id=:id"
                ),
                {"id": job_id, "now": now},
            )

    def record_progress(self, job_id: str, p: JobProgress) -> None:
        now = _now()
        with self._db.engine.begin() as conn:
            conn.execute(
                text(
                    "INSERT OR REPLACE INTO job_events(job_id, sequence, phase, state, completed, total, current_item, ts) "
                    "VALUES (:jid, :seq, :phase, :state, :completed, :total, :item, :ts)"
                ),
                {
                    "jid": job_id, "seq": p.sequence, "phase": p.phase, "state": p.state,
                    "completed": p.completed, "total": p.total, "item": p.current_item, "ts": now,
                },
            )
            conn.execute(
                text(
                    "UPDATE jobs SET state=:state, phase=:phase, progress_completed=:completed, "
                    "progress_total=:total, updated_at=:ts WHERE id=:jid"
                ),
                {
                    "jid": job_id, "state": p.state, "phase": p.phase,
                    "completed": p.completed, "total": p.total, "ts": now,
                },
            )

    def record_terminal(self, job_id: str, error: EngineError | None) -> None:
        now = _now()
        if error is None:
            state, code, cat, retry = "completed", None, None, None
        else:
            state = "cancelled" if error.code == EngineErrorCode.CANCELLED else "failed"
            code, cat, retry = error.code.value, error.category.value, int(error.retryable)
        with self._db.engine.begin() as conn:
            conn.execute(
                text(
                    "UPDATE jobs SET state=:state, error_code=:code, error_category=:cat, "
                    "error_retryable=:retry, finished_at=:now, updated_at=:now WHERE id=:id"
                ),
                {"id": job_id, "state": state, "code": code, "cat": cat, "retry": retry, "now": now},
            )

    def get(self, job_id: str) -> JobSnapshot | None:
        with self._db.engine.connect() as conn:
            row = conn.execute(
                text(
                    "SELECT id, project_id, kind, state, phase, progress_completed, progress_total, "
                    "error_code, error_category, error_retryable, created_at, updated_at, finished_at "
                    "FROM jobs WHERE id=:id"
                ),
                {"id": job_id},
            ).mappings().first()
            if row is None:
                return None
            events = [
                JobEventDto(
                    sequence=e["sequence"], phase=e["phase"], state=e["state"],
                    completed=e["completed"], total=e["total"], current_item=e["current_item"], ts=e["ts"],
                )
                for e in conn.execute(
                    text(
                        "SELECT sequence, phase, state, completed, total, current_item, ts "
                        "FROM job_events WHERE job_id=:id ORDER BY sequence"
                    ),
                    {"id": job_id},
                ).mappings()
            ]
        error = None
        if row["error_code"]:
            try:
                error = JobErrorDto(
                    code=EngineErrorCode(row["error_code"]),
                    category=EngineErrorCategory(row["error_category"]),
                    retryable=bool(row["error_retryable"]),
                )
            except ValueError:
                error = JobErrorDto(
                    code=EngineErrorCode.INTERNAL, category=EngineErrorCategory.INTERNAL, retryable=False
                )
        return JobSnapshot(
            id=row["id"], project_id=row["project_id"], kind=row["kind"], state=row["state"],
            phase=row["phase"], progress_completed=row["progress_completed"],
            progress_total=row["progress_total"], error=error, events=events,
            created_at=row["created_at"], updated_at=row["updated_at"], finished_at=row["finished_at"],
        )
