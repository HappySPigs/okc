"""U5 serving tests — production ``create_app`` + the real okc binding, offline-safe.

Covers the two U5 surfaces without any LLM/verify faking:
- admin publish gating (RBAC-before-core; a non-Verified project is refused with a
  clean code and zero state change);
- the machine read-only API against a hand-built fake compiled vault on disk
  (file list/body, 405 on mutating verbs, 404 on traversal/out-of-root/unpublished),
  the okc-mcp contract, staleness labelling, and that verify/explain against a
  non-artifact surface a clean ``EngineError`` code-branch (not a crash).
"""

from __future__ import annotations

from pathlib import Path

from fastapi import FastAPI
from fastapi.testclient import TestClient
from sqlalchemy import text

from app.adapter import dto
from app.adapter.engine import OkcEngineImpl, build_client
from app.config import AppConfig
from app.main import create_app
from app.orchestration.projects import ProjectRegistry

ADMIN_EMAIL = "admin@u5.test"
ADMIN_PASSWORD = "u5-secret-123"


def _make_app(tmp_path, monkeypatch) -> FastAPI:
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


def _insert_project(state, project_id: str, engine_root: str) -> None:
    with state.db.engine.begin() as conn:
        conn.execute(
            text(
                "INSERT INTO projects(id, name, engine_root_abs_path, curator_id, freeze_state,"
                " created_at, updated_at) VALUES(:id,'Fake',:root,'Curator','unfrozen','t','t')"
            ),
            {"id": project_id, "root": engine_root},
        )


def _insert_source(state, project_id: str, source_id: str, content_hash: str, slot: int) -> None:
    with state.db.engine.begin() as conn:
        conn.execute(
            text(
                "INSERT INTO sources(source_id, project_id, content_hash, absolute_path,"
                " slot_index, registered_at) VALUES(:sid,:pid,:ch,:path,:slot,'t')"
            ),
            {"sid": source_id, "pid": project_id, "ch": content_hash,
             "path": f"/tmp/{source_id}", "slot": slot},
        )


def _insert_publication(state, project_id: str, vault_path: str, corpus_hash: str | None = None) -> None:
    with state.db.engine.begin() as conn:
        conn.execute(
            text(
                "INSERT INTO serving_publications(project_id, compiled_vault_path,"
                " bound_corpus_hash, status, published_at, published_by)"
                " VALUES(:pid,:path,:ch,'live','t','Curator')"
            ),
            {"pid": project_id, "path": vault_path, "ch": corpus_hash},
        )


def _make_fake_vault(tmp_path, project_id: str) -> str:
    """A Markdown-only 3-root compiled vault, plus a vault-root file that must NOT
    be served (it is outside knowledge/legacy/.okc)."""
    vault = Path(tmp_path) / "projects" / project_id / "compiled" / "v1"
    (vault / "knowledge").mkdir(parents=True)
    (vault / "legacy").mkdir(parents=True)
    (vault / ".okc").mkdir(parents=True)
    (vault / "knowledge" / "a.md").write_text("# A\nknowledge note\n")
    (vault / "legacy" / "b.md").write_text("# B\nlegacy note\n")
    (vault / ".okc" / "manifest.json").write_text('{"schema": 1}')
    (vault / "top-secret.txt").write_text("must not be served")
    return str(vault)


# --- E5-S1: publish gating (RBAC-before-core + Verified precondition) ---


def test_publish_requires_authentication(tmp_path, monkeypatch) -> None:
    app = _make_app(tmp_path, monkeypatch)
    with TestClient(app) as client:
        r = client.post("/api/projects/proj_x/serving/publish")
        assert r.status_code == 401, r.text
    # RBAC-before-core: no publication row was written.
    state = app.state.app_state
    with state.db.engine.connect() as conn:
        assert conn.execute(text("SELECT COUNT(*) FROM serving_publications")).scalar() == 0


def test_publish_refused_when_not_verified(tmp_path, monkeypatch) -> None:
    app = _make_app(tmp_path, monkeypatch)
    state = app.state.app_state
    project_id = "proj_u5"
    engine_root = str(tmp_path / "engine" / "u5.okc-project")
    # A freshly created okc project is far from Verified (checkpoint needs_sources).
    seeder = OkcEngineImpl(build_client([]))
    seeder.create_project(
        dto.CreateProjectSpec(root_abs_path=engine_root, name="U5", curator_id="U5 Admin")
    )
    _insert_project(state, project_id, engine_root)
    with TestClient(app) as client:
        _login(client)
        r = client.post(f"/api/projects/{project_id}/serving/publish")
        assert r.status_code == 422, r.text
        assert r.json()["code"] == "APPROVAL_REQUIRED"
    # Refusal is a clean gate: zero state change.
    with state.db.engine.connect() as conn:
        assert conn.execute(text("SELECT COUNT(*) FROM serving_publications")).scalar() == 0


# --- E5-S2: machine read-only file list / body / method + path guards ---


def test_machine_read_api_over_fake_vault(tmp_path, monkeypatch) -> None:
    app = _make_app(tmp_path, monkeypatch)
    state = app.state.app_state
    pid = "proj_fake"
    vault = _make_fake_vault(tmp_path, pid)
    _insert_project(state, pid, str(tmp_path / "engine" / "fake.okc-project"))
    _insert_publication(state, pid, vault)

    with TestClient(app) as client:
        # file list spans exactly the three served roots.
        r = client.get(f"/api/serving/{pid}/files")
        assert r.status_code == 200, r.text
        files = r.json()["files"]
        assert "knowledge/a.md" in files
        assert "legacy/b.md" in files
        assert ".okc/manifest.json" in files
        assert "top-secret.txt" not in files  # vault-root file is not served

        # file body.
        r = client.get(f"/api/serving/{pid}/file", params={"path": "knowledge/a.md"})
        assert r.status_code == 200 and "knowledge note" in r.text

        # mutating verb on a GET-only machine path -> 405.
        assert client.post(f"/api/serving/{pid}/files").status_code == 405

        # traversal / out-of-root / missing -> 404.
        assert client.get(
            f"/api/serving/{pid}/file", params={"path": "../../../../etc/passwd"}
        ).status_code == 404
        assert client.get(
            f"/api/serving/{pid}/file", params={"path": "top-secret.txt"}
        ).status_code == 404
        assert client.get(
            f"/api/serving/{pid}/file", params={"path": "knowledge/missing.md"}
        ).status_code == 404

        # an unpublished project's machine endpoints are 404.
        assert client.get("/api/serving/proj_never/files").status_code == 404


# --- E5-S4: okc-mcp consumption contract (location + format + out-of-scope) ---


def test_contract_reports_location_format_and_out_of_scope(tmp_path, monkeypatch) -> None:
    app = _make_app(tmp_path, monkeypatch)
    state = app.state.app_state
    pid = "proj_contract"
    vault = _make_fake_vault(tmp_path, pid)
    _insert_project(state, pid, str(tmp_path / "engine" / "contract.okc-project"))
    _insert_publication(state, pid, vault)

    with TestClient(app) as client:
        r = client.get(f"/api/serving/{pid}/contract")
        assert r.status_code == 200, r.text
        body = r.json()
        assert body["location"]["local_dir"] == vault
        assert body["location"]["read_api_base"].endswith(f"/api/serving/{pid}")
        assert "markdown" in body["format"]["content"].lower()
        assert body["format"]["layout"] == ["knowledge/", "legacy/", ".okc/"]
        assert "re-embed" in body["out_of_scope"].lower()
        # discovery exposes read-only endpoints only — no okc-mcp-internal calls.
        assert {e["method"] for e in body["endpoints"]} == {"GET"}


# --- E5-S3: verify / explain surface a clean EngineError (never a crash) ---


def test_verify_explain_surface_clean_engine_error(tmp_path, monkeypatch) -> None:
    app = _make_app(tmp_path, monkeypatch)
    state = app.state.app_state
    pid = "proj_verify"
    vault = _make_fake_vault(tmp_path, pid)
    _insert_project(state, pid, str(tmp_path / "engine" / "verify.okc-project"))
    _insert_publication(state, pid, vault)

    with TestClient(app) as client:
        rv = client.get(f"/api/serving/{pid}/verify")
        # The fake dir is not a real okc artifact -> a clean, code-tagged error body.
        assert rv.status_code != 200, rv.text
        assert "code" in rv.json()

        re_ = client.get(f"/api/serving/{pid}/explain", params={"path": "knowledge/a.md"})
        assert re_.status_code != 200, re_.text
        assert "code" in re_.json()


# --- E5-S5: staleness derived from the bound corpus hash ---


def test_publication_status_and_staleness(tmp_path, monkeypatch) -> None:
    app = _make_app(tmp_path, monkeypatch)
    state = app.state.app_state
    pid = "proj_stale"
    vault = _make_fake_vault(tmp_path, pid)
    _insert_project(state, pid, str(tmp_path / "engine" / "stale.okc-project"))
    _insert_source(state, pid, "src_a", "hash_a", 0)
    bound = ProjectRegistry(state.db).source_set_fingerprint(pid)
    _insert_publication(state, pid, vault, corpus_hash=bound)

    with TestClient(app) as client:
        _login(client)
        # bound hash matches the current fingerprint -> live.
        r = client.get(f"/api/projects/{pid}/serving")
        assert r.status_code == 200, r.text
        assert r.json()["status"] == "live" and r.json()["stale"] is False
        assert client.get(f"/api/serving/{pid}/files").json()["status"] == "live"

        # drift the frozen input set -> the bound manifest becomes stale.
        _insert_source(state, pid, "src_b", "hash_b", 1)
        r = client.get(f"/api/projects/{pid}/serving")
        assert r.json()["status"] == "stale" and r.json()["stale"] is True

        # E5-S5: still serve the immutable manifest, but labelled stale.
        files = client.get(f"/api/serving/{pid}/files")
        assert files.status_code == 200
        assert files.json()["status"] == "stale"
        assert "knowledge/a.md" in files.json()["files"]


def test_unpublish_is_idempotent_and_offline(tmp_path, monkeypatch) -> None:
    app = _make_app(tmp_path, monkeypatch)
    state = app.state.app_state
    pid = "proj_unpub"
    _insert_project(state, pid, str(tmp_path / "engine" / "unpub.okc-project"))
    with TestClient(app) as client:
        _login(client)
        # unpublishing a never-published project is an idempotent no-op -> offline.
        r = client.post(f"/api/projects/{pid}/serving/unpublish")
        assert r.status_code == 200, r.text
        assert r.json()["status"] == "offline"
        # machine endpoints stay 404 while offline.
        assert client.get(f"/api/serving/{pid}/files").status_code == 404
