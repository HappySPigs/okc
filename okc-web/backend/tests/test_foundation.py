"""U0 foundation smoke tests (W0). These exercise the frozen contracts WITHOUT
the native okc binding (skip_engine): the error table, SQLite migration, JobStore,
the append-only audit + 3-variant CuratorDecision (C3), and the RBAC-before-core
invariant (401/403 before any handler body runs)."""

from __future__ import annotations

import pytest
from fastapi.testclient import TestClient
from pydantic import ValidationError

from app.adapter import schema_guard
from app.config import AppConfig
from app.main import create_app
from app.shared.audit import (
    DECISION_KINDS,
    AuditStore,
    CuratorDecisionAdapter,
    HashBindings,
    NewDecisionRecord,
    RegenerateCluster,
)
from app.shared.error import EngineError, EngineErrorCategory, EngineErrorCode
from app.shared.jobs import JobProgress, JobStore
from app.shared.state import StateDb

# --- error table (§8) ---

@pytest.mark.parametrize(
    ("code", "status"),
    [
        (EngineErrorCode.UNAUTHENTICATED, 401),
        (EngineErrorCode.FORBIDDEN, 403),
        (EngineErrorCode.NOT_FOUND, 404),
        (EngineErrorCode.METHOD_NOT_ALLOWED, 405),
        (EngineErrorCode.PROJECT_BUSY, 409),
        (EngineErrorCode.APPROVAL_REQUIRED, 422),
        (EngineErrorCode.PROJECT_INVALID, 422),
        (EngineErrorCode.SOURCE_CAP_EXCEEDED, 429),
        (EngineErrorCode.PROVIDER_UNAVAILABLE, 502),
        (EngineErrorCode.OUTPUT_DURABILITY_UNCERTAIN, 500),
        (EngineErrorCode.SCHEMA_UNSUPPORTED, 500),
        (EngineErrorCode.INTERNAL, 500),
    ],
)
def test_http_status_table(code: EngineErrorCode, status: int) -> None:
    err = EngineError(code, EngineErrorCategory.INTERNAL, "m")
    assert err.http_status() == status


def test_retry_after_by_code() -> None:
    assert EngineError(EngineErrorCode.PROJECT_BUSY, _cat(), "busy").retry_after_seconds() == 2
    assert EngineError(EngineErrorCode.RESOURCE_LIMIT, _cat(), "limit").retry_after_seconds() == 5
    assert EngineError(EngineErrorCode.NOT_FOUND, _cat(), "x").retry_after_seconds() is None


def _cat() -> EngineErrorCategory:
    return EngineErrorCategory.INTERNAL


# --- schema guard ---

def test_schema_guard_check() -> None:
    schema_guard.check(2)  # ok
    schema_guard.check(None)  # tolerated
    with pytest.raises(EngineError) as e:
        schema_guard.check(1)
    assert e.value.code == EngineErrorCode.SCHEMA_UNSUPPORTED


# --- state migration (§9) ---

def test_migration_creates_all_control_plane_tables() -> None:
    db = StateDb.open_in_memory()
    tables = db.table_names()
    for t in ("accounts", "sessions", "projects", "upload_tokens", "sources",
              "jobs", "job_events", "curator_decisions", "serving_publications"):
        assert t in tables


# --- JobStore (Q7) ---

def test_jobstore_roundtrip() -> None:
    db = StateDb.open_in_memory()
    jobs = JobStore(db)
    jid = jobs.create("integrate", project_id="proj_x", requested_by="acc_1")
    jobs.mark_running(jid)
    jobs.record_progress(jid, JobProgress(1, "running", "embedding", 3, 10, "note.md"))
    jobs.record_terminal(jid, None)
    snap = jobs.get(jid)
    assert snap is not None
    assert snap.state == "completed"
    assert snap.project_id == "proj_x"
    assert len(snap.events) == 1 and snap.events[0].phase == "embedding"


def test_jobstore_terminal_error() -> None:
    db = StateDb.open_in_memory()
    jobs = JobStore(db)
    jid = jobs.create("compile", project_id=None, requested_by=None)
    jobs.record_terminal(jid, EngineError(EngineErrorCode.PROJECT_BUSY, _cat(), "busy"))
    snap = jobs.get(jid)
    assert snap is not None and snap.state == "failed"
    assert snap.error is not None and snap.error.code == EngineErrorCode.PROJECT_BUSY


# --- CuratorDecision: exactly 3 variants, NO winner-select (C3) ---

def test_curator_decision_has_exactly_three_kinds() -> None:
    assert set(DECISION_KINDS) == {"approve_taxonomy", "approve_cluster", "regenerate_cluster"}
    assert len(DECISION_KINDS) == 3


def test_curator_decision_rejects_winner_select() -> None:
    # A "select_winner"/"resolve_contradiction" decision is not representable.
    with pytest.raises(ValidationError):
        CuratorDecisionAdapter.validate_python({"kind": "select_winner", "cluster_id": "c1"})
    # The three legitimate kinds validate.
    CuratorDecisionAdapter.validate_python({"kind": "approve_taxonomy"})
    CuratorDecisionAdapter.validate_python(
        {"kind": "approve_cluster", "cluster_id": "c1"}
    )


def test_audit_append_and_list_and_db_check() -> None:
    db = StateDb.open_in_memory()
    audit = AuditStore(db)
    # account_id references accounts(id); left None here since this isolated
    # foundation test does not seed an account (U4 supplies a real admin id).
    aid = audit.append(NewDecisionRecord(
        project_id="proj_x", account_id=None, curator_id="curator-label",
        decision=RegenerateCluster(cluster_id="c1", feedback="redo"),
        bindings=HashBindings(proposal_hash="ph", critic_hash="ch", taxonomy_hash="th"),
    ))
    assert aid.startswith("dec_")
    rows = audit.list_for_project("proj_x")
    assert len(rows) == 1 and rows[0]["decision_kind"] == "regenerate_cluster"

    # The DB CHECK also forbids a winner-select row at the storage layer.
    import sqlite3
    with pytest.raises(sqlite3.IntegrityError):
        raw = db.engine.raw_connection()
        try:
            driver = raw.driver_connection
            assert driver is not None
            driver.execute(
                "INSERT INTO curator_decisions(id,project_id,curator_id,decision_kind,payload_json,core_op,created_at)"
                " VALUES ('d','p','c','select_winner','{}','x','t')"
            )
            raw.commit()
        finally:
            raw.close()


# --- RBAC-before-core (C-1): 401/403 raised in the dependency, handler never runs ---

def _client(tmp_path) -> TestClient:  # noqa: ANN001
    cfg = AppConfig(
        state_db_path=str(tmp_path / "state.db"),
        projects_root=str(tmp_path / "projects"),
        providers=[],
        skip_engine=True,
    )
    return TestClient(create_app(cfg))


def test_health(tmp_path) -> None:  # noqa: ANN001
    with _client(tmp_path) as c:
        r = c.get("/api/health")
        assert r.status_code == 200 and r.json()["status"] == "ok"


def test_admin_job_route_401_without_session(tmp_path) -> None:  # noqa: ANN001
    with _client(tmp_path) as c:
        r = c.get("/api/projects/proj_x/jobs/job_1")
        assert r.status_code == 401
        assert r.json()["code"] in {"UNAUTHENTICATED", "SESSION_EXPIRED"}


def test_upload_job_route_401_without_token(tmp_path) -> None:  # noqa: ANN001
    with _client(tmp_path) as c:
        r = c.get("/u/some-token/jobs/job_1")
        # unconfigured token resolver denies -> TOKEN_INVALID (400) before any core call
        assert r.status_code in {400, 401}
        assert r.json()["code"] in {"TOKEN_INVALID", "UNAUTHENTICATED"}
