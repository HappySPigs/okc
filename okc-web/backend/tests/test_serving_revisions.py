"""Real core + deterministic loopback provider, including the successful web spine.

These tests exercise real HTTP provider calls and real compiler verification;
the provider is synthetic, so this is not live-model quality evidence.
"""

from __future__ import annotations

import importlib.util
import threading
import time
from http.server import ThreadingHTTPServer
from pathlib import Path

import pytest
from fastapi.testclient import TestClient
from sqlalchemy import text

from app.adapter.dto import ProviderSpecView
from app.config import AppConfig
from app.main import create_app


@pytest.fixture
def published_client(tmp_path, monkeypatch):
    fixture_path = Path(__file__).resolve().parents[3] / "okc-core/bindings/python/tests/test_public_api.py"
    spec = importlib.util.spec_from_file_location("core_provider_fixture", fixture_path)
    assert spec and spec.loader
    fixture = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(fixture)
    provider = ThreadingHTTPServer(("127.0.0.1", 0), fixture._FixtureProvider)
    thread = threading.Thread(target=provider.serve_forever, daemon=True)
    thread.start()
    monkeypatch.setenv("OKC_WEB_BOOTSTRAP_ADMIN_EMAIL", "integration@example.test")
    monkeypatch.setenv("OKC_WEB_BOOTSTRAP_ADMIN_PASSWORD", "integration-password")
    app = create_app(AppConfig(state_db_path=str(tmp_path / "web.db"), projects_root=str(tmp_path / "projects"),
        providers=[ProviderSpecView(name="fixture", kind="ollama", model="fixture-model",
                                  endpoint=f"http://127.0.0.1:{provider.server_port}")]))
    try:
        with TestClient(app) as client:
            _ok(client.post("/api/auth/login", json={"email": "integration@example.test", "password": "integration-password"}))
            pid = _ok(client.post("/api/projects", json={"name": "Integrated knowledge"}))["id"]
            token = _ok(client.post(f"/api/projects/{pid}/tokens", json={"owner_display_name": "Author"}))["token"]
            _upload(client, pid, token, b"# First\n\nOriginal knowledge.\n")
            _ok(client.post(f"/api/projects/{pid}/provider", json={"profile_name": "fixture"}))
            first = _compile_publish(client, pid, str(tmp_path / "projects" / "custom-output"))
            yield client, app, pid, token, first
    finally:
        provider.shutdown()
        provider.server_close()
        thread.join(timeout=5)


def _ok(response):
    assert response.status_code == 200, response.text
    return response.json()


def _wait(client, pid: str, job: str) -> None:
    deadline = time.monotonic() + 20
    while time.monotonic() < deadline:
        state = _ok(client.get(f"/api/projects/{pid}/jobs/{job}"))
        if state["state"] == "completed":
            return
        assert state["state"] not in ("failed", "cancelled"), state
        time.sleep(0.01)
    pytest.fail("integration job timed out")


def _upload(client, pid: str, token: str, content: bytes) -> None:
    receipt = _ok(client.post(f"/u/{token}/upload", files={"file": ("Note.md", content, "text/markdown")}))
    _wait(client, pid, receipt["job_id"])


def _compile_publish(client, pid: str, output: str | None = None) -> dict:
    base = f"/api/projects/{pid}"
    _ok(client.post(f"{base}/freeze"))
    _wait(client, pid, _ok(client.post(f"{base}/integrate", json={}))["job_id"])
    _ok(client.post(f"{base}/taxonomy/approve", json={"rationale": "reviewed taxonomy"}))
    _wait(client, pid, _ok(client.post(f"{base}/integrate", json={}))["job_id"])
    clusters = _ok(client.get(f"{base}/clusters"))
    assert clusters
    for cluster in clusters:
        _ok(client.post(f"{base}/clusters/{cluster['cluster_id']}/approve", json={}))
    _wait(client, pid, _ok(client.post(f"{base}/integrate", json={}))["job_id"])
    compiled = _ok(client.post(f"{base}/compile", json={"output_path": output}))
    published = _ok(client.post(f"{base}/serving/publish"))
    assert published["bound_integration_plan_id"] == compiled["integration_plan_id"]
    assert published["compiled_vault_path"] == compiled["path"]
    assert len(published["revision"]) == 64
    assert published["stale"] is False
    return published


def test_real_publication_pins_revision_and_restores(published_client):
    client, _app, pid, token, first = published_client
    base = f"/api/serving/{pid}"
    revision = first["revision"]
    contract = _ok(client.get(f"{base}/contract"))
    assert contract["revision"] == revision
    assert contract["bound_corpus_hash"] and contract["bound_taxonomy_hash"]
    files = _ok(client.get(f"{base}/files", params={"revision": revision}))
    note = next(p for p in files["files"] if p.startswith("knowledge/") and p.endswith(".md"))
    original = client.get(f"{base}/file", params={"path": note, "revision": revision})
    assert original.status_code == 200
    assert _ok(client.get(f"{base}/verify", params={"revision": revision}))["valid"]
    explanation = _ok(client.get(f"{base}/explain", params={"path": note, "revision": revision}))
    assert explanation["record"]["output_path"] == note
    assert explanation["owner_labels"]

    _upload(client, pid, token, b"# Second\n\nNew knowledge supersedes the local source.\n")
    assert _ok(client.get(f"{base}/contract"))["stale"]
    second = _compile_publish(client, pid)
    assert second["revision"] != revision
    assert _ok(client.get(f"{base}/contract"))["revision"] == second["revision"]
    assert _ok(client.get(f"{base}/contract", params={"revision": revision}))["revision"] == revision
    assert client.get(f"{base}/file", params={"path": note, "revision": revision}).content == original.content
    assert client.get(f"{base}/files", params={"revision": "0" * 64}).status_code == 404
    assert len(_ok(client.get(f"/api/projects/{pid}/serving/history"))) == 2
    restored = _ok(client.post(f"/api/projects/{pid}/serving/restore", json={"revision": revision}))
    assert restored["revision"] == revision and restored["stale"]


def test_private_reads_token_revocation_and_unpublish(published_client):
    client, app, pid, upload_token, first = published_client
    admin = f"/api/projects/{pid}/serving"
    base = f"/api/serving/{pid}"
    read = _ok(client.post(f"{admin}/tokens"))
    _ok(client.put(f"{admin}/access", json={"mode": "private"}))
    assert client.get(f"{base}/contract").status_code == 401
    assert client.get(f"{base}/files", headers={"Authorization": f"Bearer {upload_token}"}).status_code == 401
    headers = {"Authorization": f"Bearer {read['token']}"}
    assert _ok(client.get(f"{base}/contract", headers=headers))["revision"] == first["revision"]
    with app.state.app_state.db.engine.connect() as conn:
        stored = conn.execute(text("SELECT token_hash FROM serving_read_tokens WHERE id=:id"), {"id": read["id"]}).scalar()
        assert stored != read["token"]
    _ok(client.delete(f"{admin}/tokens/{read['id']}"))
    assert client.get(f"{base}/files", headers=headers).status_code == 401
    _ok(client.put(f"{admin}/access", json={"mode": "public"}))
    _ok(client.post(f"{admin}/unpublish"))
    assert client.get(f"{base}/contract", params={"revision": first["revision"]}).status_code == 404


def test_published_file_tampering_is_rejected(published_client):
    client, _app, pid, _token, first = published_client
    base = f"/api/serving/{pid}"
    files = _ok(client.get(f"{base}/files"))
    note = next(p for p in files["files"] if p.startswith("knowledge/") and p.endswith(".md"))
    (Path(first["compiled_vault_path"]) / note).write_text("tampered", encoding="utf-8")
    result = client.get(f"{base}/file", params={"path": note, "revision": first["revision"]})
    assert result.status_code == 422 and result.json()["code"] == "VERIFICATION_FAILED"

    # A crash/corrupt-storage recovery may recompile the same approved plan to
    # a fresh no-clobber path. The same content revision must resolve to it.
    compiled = _ok(client.post(f"/api/projects/{pid}/compile", json={}))
    assert compiled["path"] != first["compiled_vault_path"]
    repaired = _ok(client.post(f"/api/projects/{pid}/serving/publish"))
    assert repaired["revision"] == first["revision"]
    assert repaired["compiled_vault_path"] == compiled["path"]
    assert client.get(f"{base}/file", params={"path": note, "revision": first["revision"]}).status_code == 200


def test_recompile_refreshes_context_after_rejected_review(published_client):
    client, _app, pid, _token, first = published_client
    # A well-typed but incomplete taxonomy gets audited and rejected by core.
    rejected = client.post(f"/api/projects/{pid}/taxonomy/approve", json={
        "edited_clusters": [], "rationale": "test invalid empty taxonomy",
    })
    # The current native facade classifies incomplete taxonomy as a typed
    # provider-validation failure; the web keeps that code/status mapping.
    assert rejected.status_code in (400, 422, 502), rejected.text
    assert _ok(client.get(f"/api/serving/{pid}/contract"))["stale"]
    _ok(client.post(f"/api/projects/{pid}/compile", json={}))
    republished = _ok(client.post(f"/api/projects/{pid}/serving/publish"))
    assert republished["revision"] == first["revision"]
    assert republished["stale"] is False
