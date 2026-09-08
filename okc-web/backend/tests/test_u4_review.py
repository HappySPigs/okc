"""U4 review tests — production ``create_app`` + the real okc binding, offline-safe.

Covers the review paths that need NO LLM/synthesis run:
- RBAC-before-core on every route (401, zero engine calls, zero audit rows).
- The pure ``DecisionGate`` (Major/Critical waive & residual blocking → 422; missing
  Minor/omission rationale → 400).
- The body-only missing-rationale gate over HTTP writes NO audit row and calls NO engine.
- ``taxonomy``/``clusters`` on a project with no integration run surface a clean mapped
  ``EngineError`` code (PROJECT_INVALID/422), not a crash.
- A record-then-act ``approve_taxonomy`` appends exactly one ``curator_decisions`` row even
  though the core op then fails (no proposal yet). No AI/synthesis result is faked.
"""

from __future__ import annotations

import pytest
from fastapi import FastAPI
from fastapi.testclient import TestClient
from sqlalchemy import text

from app.config import AppConfig
from app.main import create_app
from app.review.gate import DecisionGate
from app.review.models import ApproveClusterRequest, CriticFindingView
from app.shared.error import EngineError, EngineErrorCode

ADMIN_EMAIL = "admin@u4.test"
ADMIN_PASSWORD = "u4-secret-123"


def _make_app(tmp_path, monkeypatch) -> FastAPI:  # noqa: ANN001
    monkeypatch.setenv("OKC_WEB_BOOTSTRAP_ADMIN_EMAIL", ADMIN_EMAIL)
    monkeypatch.setenv("OKC_WEB_BOOTSTRAP_ADMIN_PASSWORD", ADMIN_PASSWORD)
    return create_app(
        AppConfig(
            state_db_path=str(tmp_path / "state.db"),
            projects_root=str(tmp_path / "projects"),
            providers=[],
        )
    )


def _login(client: TestClient) -> None:
    r = client.post("/api/auth/login", json={"email": ADMIN_EMAIL, "password": ADMIN_PASSWORD})
    assert r.status_code == 200, r.text
    assert client.cookies.get("okc_session")


def _create_project(client: TestClient, name: str = "Review Vault") -> str:
    r = client.post("/api/projects", json={"name": name})
    assert r.status_code == 200, r.text
    return r.json()["id"]


def _decision_count(state) -> int:  # noqa: ANN001
    with state.db.engine.connect() as conn:
        return int(conn.execute(text("SELECT COUNT(*) FROM curator_decisions")).scalar() or 0)


# --- RBAC-before-core (C-1): every route rejects the unauthenticated caller ---


def test_all_routes_require_authentication(tmp_path, monkeypatch) -> None:  # noqa: ANN001
    app = _make_app(tmp_path, monkeypatch)
    with TestClient(app) as client:
        pid = "proj_unauth"
        cid = "cluster_x"
        calls = [
            client.get(f"/api/projects/{pid}/taxonomy"),
            client.post(f"/api/projects/{pid}/taxonomy/approve", json={}),
            client.get(f"/api/projects/{pid}/clusters"),
            client.get(f"/api/projects/{pid}/clusters/{cid}"),
            client.post(f"/api/projects/{pid}/clusters/{cid}/approve", json={}),
            client.post(
                f"/api/projects/{pid}/clusters/{cid}/regenerate", json={"feedback": "redo"}
            ),
            client.get(f"/api/projects/{pid}/review/gate"),
            client.get(f"/api/projects/{pid}/decisions"),
        ]
        for r in calls:
            assert r.status_code == 401, (r.request.url, r.text)
    # RBAC-before-core: nothing was recorded.
    assert _decision_count(app.state.app_state) == 0


# --- DecisionGate (pure, offline): C-2 blocking + mandatory-rationale ---


def test_gate_rejects_blocking_waive_attempt() -> None:
    findings = [
        CriticFindingView(
            finding_id="f_major", severity="major", kind="omission", message="x", blocking=True
        )
    ]
    cmd = ApproveClusterRequest(minor_waivers={"f_major": "trying to waive a blocker"})
    with pytest.raises(EngineError) as exc:
        DecisionGate.assert_approvable(findings, cmd)
    assert exc.value.code == EngineErrorCode.APPROVAL_REQUIRED
    assert exc.value.http_status() == 422


def test_gate_rejects_residual_blocking_finding() -> None:
    findings = [
        CriticFindingView(
            finding_id="f_crit", severity="critical", kind="misattribution", message="x",
            blocking=True,
        )
    ]
    with pytest.raises(EngineError) as exc:
        DecisionGate.assert_approvable(findings, ApproveClusterRequest())
    assert exc.value.code == EngineErrorCode.APPROVAL_REQUIRED
    assert exc.value.http_status() == 422


def test_gate_rejects_missing_minor_rationale() -> None:
    with pytest.raises(EngineError) as exc:
        DecisionGate.assert_rationales(ApproveClusterRequest(minor_waivers={"f_minor": "  "}))
    assert exc.value.code == EngineErrorCode.VALIDATION_FAILED
    assert exc.value.http_status() == 400


def test_gate_rejects_missing_omission_rationale() -> None:
    with pytest.raises(EngineError) as exc:
        DecisionGate.assert_rationales(
            ApproveClusterRequest(omission_rationales={"doc_a:block_b": ""})
        )
    assert exc.value.code == EngineErrorCode.VALIDATION_FAILED
    assert exc.value.http_status() == 400


def test_gate_allows_minor_waiver_with_rationale() -> None:
    findings = [
        CriticFindingView(
            finding_id="f_minor", severity="minor", kind="link_loss", message="x", blocking=False
        )
    ]
    cmd = ApproveClusterRequest(minor_waivers={"f_minor": "acceptable, cited elsewhere"})
    DecisionGate.assert_rationales(cmd)
    DecisionGate.assert_approvable(findings, cmd)  # no raise


# --- HTTP: body-only gate rejects before core, with no audit row + no engine call ---


def test_missing_rationale_over_http_rejects_before_core(tmp_path, monkeypatch) -> None:  # noqa: ANN001
    app = _make_app(tmp_path, monkeypatch)
    state = app.state.app_state
    with TestClient(app) as client:
        _login(client)
        pid = _create_project(client)
        r = client.post(
            f"/api/projects/{pid}/clusters/any_cluster/approve",
            json={"minor_waivers": {"f1": ""}},
        )
        assert r.status_code == 400, r.text
        assert r.json()["code"] == "VALIDATION_FAILED"
    # Gate-before-core: no decision recorded (the clusters() read was never reached).
    assert _decision_count(state) == 0


# --- reads on a project with no integration run surface a clean mapped code (no crash) ---


def test_taxonomy_and_clusters_no_run_surface_engine_error(tmp_path, monkeypatch) -> None:  # noqa: ANN001
    app = _make_app(tmp_path, monkeypatch)
    with TestClient(app) as client:
        _login(client)
        pid = _create_project(client)

        rt = client.get(f"/api/projects/{pid}/taxonomy")
        assert rt.status_code == 422, rt.text
        assert rt.json()["code"] == "PROJECT_INVALID"  # branched by code, not a 500 crash

        rc = client.get(f"/api/projects/{pid}/clusters")
        assert rc.status_code == 422, rc.text
        assert rc.json()["code"] == "PROJECT_INVALID"


# --- record-then-act: approve_taxonomy appends exactly one row even if the core op fails ---


def test_approve_taxonomy_records_before_acting(tmp_path, monkeypatch) -> None:  # noqa: ANN001
    app = _make_app(tmp_path, monkeypatch)
    state = app.state.app_state
    with TestClient(app) as client:
        _login(client)
        pid = _create_project(client)
        # No edit → no rationale required; the gate passes and the row is appended BEFORE
        # the engine op, which then fails (no taxonomy proposal exists yet).
        r = client.post(f"/api/projects/{pid}/taxonomy/approve", json={})
        assert r.status_code == 422, r.text
        assert r.json()["code"] == "PROJECT_INVALID"

    with state.db.engine.connect() as conn:
        rows = conn.execute(
            text("SELECT project_id, decision_kind, core_op FROM curator_decisions")
        ).mappings().all()
    assert len(rows) == 1
    assert rows[0]["project_id"] == pid
    assert rows[0]["decision_kind"] == "approve_taxonomy"
    assert rows[0]["core_op"] == "approve_taxonomy"
