"""U3 orchestration tests — production ``create_app`` + the real okc binding.

Exercises the offline-safe orchestration paths (create / status / freeze / provider
allowlist / PROJECT_BUSY gate / compile readiness + no-clobber path guard). Integrate
and a real compile need a live provider, so those happy paths are covered by the
wave/build spine, not here; the gates around them are asserted with the real binding.
"""

from __future__ import annotations

import os

from fastapi import FastAPI
from fastapi.testclient import TestClient
from sqlalchemy import text

from app.config import AppConfig
from app.main import create_app

ADMIN_EMAIL = "admin@u3.test"
ADMIN_PASSWORD = "u3-secret-123"


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


def _create_project(client: TestClient, name: str = "Team Vault") -> dict:
    r = client.post("/api/projects", json={"name": name})
    assert r.status_code == 200, r.text
    return r.json()


def _insert_source(state, project_id: str, source_id: str, content_hash: str, slot: int) -> None:  # noqa: ANN001
    with state.db.engine.begin() as conn:
        conn.execute(
            text(
                "INSERT INTO sources(source_id, project_id, content_hash, absolute_path,"
                " slot_index, registered_at) VALUES(:sid,:pid,:ch,:path,:slot,'t')"
            ),
            {"sid": source_id, "pid": project_id, "ch": content_hash,
             "path": f"/tmp/{source_id}", "slot": slot},
        )


def _insert_running_job(state, project_id: str) -> None:  # noqa: ANN001
    with state.db.engine.begin() as conn:
        conn.execute(
            text(
                "INSERT INTO jobs(id, project_id, kind, state, progress_completed,"
                " created_at, updated_at) VALUES('job_u3_active',:pid,'integrate','running',0,'t','t')"
            ),
            {"pid": project_id},
        )


# --- E3-S1 / RBAC-before-core ---


def test_create_project_requires_authentication(tmp_path, monkeypatch) -> None:  # noqa: ANN001
    app = _make_app(tmp_path, monkeypatch)
    with TestClient(app) as client:
        r = client.post("/api/projects", json={"name": "No Auth"})
        assert r.status_code == 401, r.text
        # RBAC-before-core: no project row was written.
    state = app.state.app_state
    with state.db.engine.connect() as conn:
        assert conn.execute(text("SELECT COUNT(*) FROM projects")).scalar() == 0


def test_create_get_and_list_project(tmp_path, monkeypatch) -> None:  # noqa: ANN001
    app = _make_app(tmp_path, monkeypatch)
    with TestClient(app) as client:
        _login(client)
        created = _create_project(client, "Knowledge Base")
        assert created["curator_id"]  # bound to the admin label (E1-S4)
        assert created["freeze_state"] == "unfrozen"
        root = created["engine_root_abs_path"]
        assert os.path.commonpath(
            [os.path.abspath(root), os.path.abspath(str(tmp_path / "projects"))]
        ) == os.path.abspath(str(tmp_path / "projects"))

        got = client.get(f"/api/projects/{created['id']}")
        assert got.status_code == 200 and got.json()["id"] == created["id"]

        listed = client.get("/api/projects")
        assert listed.status_code == 200
        assert any(p["id"] == created["id"] for p in listed.json())


def test_status_projection_needs_sources(tmp_path, monkeypatch) -> None:  # noqa: ANN001
    app = _make_app(tmp_path, monkeypatch)
    with TestClient(app) as client:
        _login(client)
        created = _create_project(client)
        r = client.get(f"/api/projects/{created['id']}/status")
        assert r.status_code == 200, r.text
        body = r.json()
        assert body["checkpoint"] == "needs_sources"
        assert body["resolver"] == "contributor"
        assert body["frozen"] is False
        assert body["stale"] is True  # unfrozen => advisory-stale
        assert "verified" in body["progression"]


# --- E3-S2: freeze (Q1 reproducible snapshot) ---


def test_freeze_requires_sources_then_succeeds_and_drifts(tmp_path, monkeypatch) -> None:  # noqa: ANN001
    app = _make_app(tmp_path, monkeypatch)
    state = app.state.app_state
    with TestClient(app) as client:
        _login(client)
        pid = _create_project(client)["id"]

        # No sources yet => 400.
        r = client.post(f"/api/projects/{pid}/freeze")
        assert r.status_code == 400, r.text
        assert r.json()["code"] == "VALIDATION_FAILED"

        _insert_source(state, pid, "src_a", "hash_a", 0)
        frozen = client.post(f"/api/projects/{pid}/freeze")
        assert frozen.status_code == 200, frozen.text
        fp = frozen.json()["source_set_fingerprint"]
        assert fp and frozen.json()["freeze_state"] == "frozen"

        status = client.get(f"/api/projects/{pid}/status").json()
        assert status["frozen"] is True and status["stale"] is False

        # A later valid upload drifts the fingerprint => advisory-stale (re-freeze).
        _insert_source(state, pid, "src_b", "hash_b", 1)
        drifted = client.get(f"/api/projects/{pid}/status").json()
        assert drifted["stale"] is True


# --- E3-S4: provider allowlist ---


def test_bind_unknown_provider_rejected(tmp_path, monkeypatch) -> None:  # noqa: ANN001
    app = _make_app(tmp_path, monkeypatch)
    with TestClient(app) as client:
        _login(client)
        pid = _create_project(client)["id"]
        r = client.post(f"/api/projects/{pid}/provider", json={"profile_name": "ghost"})
        assert r.status_code == 400, r.text
        assert r.json()["code"] == "VALIDATION_FAILED"


# --- E3-S3/E3-S7: single active mutation (PROJECT_BUSY) ---


def test_project_busy_blocks_second_mutation(tmp_path, monkeypatch) -> None:  # noqa: ANN001
    app = _make_app(tmp_path, monkeypatch)
    state = app.state.app_state
    with TestClient(app) as client:
        _login(client)
        pid = _create_project(client)["id"]
        _insert_running_job(state, pid)
        r = client.post(f"/api/projects/{pid}/compile", json={})
        assert r.status_code == 409, r.text
        assert r.json()["code"] == "PROJECT_BUSY"
        assert r.json()["retryable"] is True
        assert r.headers.get("Retry-After")


# --- E3-S6: compile readiness + no-clobber path guard ---


def test_compile_not_ready_returns_approval_required(tmp_path, monkeypatch) -> None:  # noqa: ANN001
    app = _make_app(tmp_path, monkeypatch)
    state = app.state.app_state
    with TestClient(app) as client:
        _login(client)
        pid = _create_project(client)["id"]
        _insert_source(state, pid, "src_a", "hash_a", 0)
        assert client.post(f"/api/projects/{pid}/freeze").status_code == 200
        r = client.post(f"/api/projects/{pid}/compile", json={})
        assert r.status_code == 422, r.text
        assert r.json()["code"] == "APPROVAL_REQUIRED"


def test_compile_output_path_escape_rejected(tmp_path, monkeypatch) -> None:  # noqa: ANN001
    app = _make_app(tmp_path, monkeypatch)
    with TestClient(app) as client:
        _login(client)
        pid = _create_project(client)["id"]
        r = client.post(
            f"/api/projects/{pid}/compile",
            json={"output_path": os.path.abspath(os.sep.join(["", "tmp", "escape", "out"]))},
        )
        assert r.status_code == 400, r.text
        assert r.json()["code"] == "PATH_UNSAFE"
