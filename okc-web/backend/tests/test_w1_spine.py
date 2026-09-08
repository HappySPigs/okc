"""W1 sync barrier: real U1 login -> U2 token -> upload -> okc add_source.

This uses the production ``create_app`` discovery path and a real okc binding.
No auth, engine, or repository mock participates in the spine.
"""

from __future__ import annotations

import time

from fastapi.testclient import TestClient
from sqlalchemy import text

from app.adapter import dto
from app.adapter.engine import OkcEngineImpl, build_client
from app.config import AppConfig
from app.main import create_app


def test_login_issue_token_upload_add_source_spine(tmp_path, monkeypatch) -> None:  # noqa: ANN001
    admin_email = "admin@w1.test"
    admin_password = "w1-secret-123"
    project_id = "proj_w1_spine"
    engine_root = str(tmp_path / "engine" / "w1.okc-project")

    monkeypatch.setenv("OKC_WEB_BOOTSTRAP_ADMIN_EMAIL", admin_email)
    monkeypatch.setenv("OKC_WEB_BOOTSTRAP_ADMIN_PASSWORD", admin_password)

    seeder = OkcEngineImpl(build_client([]))
    seeder.create_project(
        dto.CreateProjectSpec(
            root_abs_path=engine_root,
            name="W1 Spine",
            curator_id="W1 Admin",
        )
    )

    app = create_app(
        AppConfig(
            state_db_path=str(tmp_path / "state.db"),
            projects_root=str(tmp_path / "projects"),
            providers=[],
        )
    )
    state = app.state.app_state
    with state.db.engine.begin() as conn:
        admin_id = str(
            conn.execute(
                text("SELECT id FROM accounts WHERE email=:email"),
                {"email": admin_email},
            ).scalar_one()
        )
        conn.execute(
            text(
                "INSERT INTO projects(id,name,engine_root_abs_path,curator_id,created_by,"
                "freeze_state,created_at,updated_at) "
                "VALUES(:id,'W1 Spine',:root,'W1 Admin',:admin,'unfrozen','t','t')"
            ),
            {"id": project_id, "root": engine_root, "admin": admin_id},
        )

    with TestClient(app) as client:
        login = client.post(
            "/api/auth/login",
            json={"email": admin_email, "password": admin_password},
        )
        assert login.status_code == 200, login.text
        assert client.cookies.get("okc_session")

        issued = client.post(
            f"/api/projects/{project_id}/tokens",
            json={"owner_display_name": "Knowledge Team", "owner_kind": "department"},
        )
        assert issued.status_code == 200, issued.text
        token = issued.json()["token"]

        uploaded = client.post(
            f"/u/{token}/upload",
            files={"file": ("w1.md", b"# W1 spine\n\nReal binding path.\n", "text/markdown")},
        )
        assert uploaded.status_code == 200, uploaded.text
        receipt = uploaded.json()

        deadline = time.time() + 15
        while time.time() < deadline:
            polled = client.get(f"/u/{token}/jobs/{receipt['job_id']}")
            assert polled.status_code == 200, polled.text
            snapshot = polled.json()
            if snapshot["state"] in {"completed", "failed", "cancelled"}:
                break
            time.sleep(0.05)
        else:
            raise AssertionError("W1 add_source job did not reach a terminal state")

        assert snapshot["state"] == "completed", snapshot

    with state.db.engine.connect() as conn:
        source = conn.execute(
            text("SELECT * FROM sources WHERE source_id=:source_id"),
            {"source_id": receipt["source_id"]},
        ).mappings().one()
    assert source["owner_display_name"] == "Knowledge Team"
    assert source["owner_kind"] == "department"

    manifest = OkcEngineImpl(build_client([])).manifest(engine_root)
    assert receipt["source_id"] in {item["source_id"] for item in manifest["sources"]}
