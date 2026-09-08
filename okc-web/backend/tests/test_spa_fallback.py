"""W4 integration barrier: the SPA history-fallback in ``main.py`` (added so the
React deep-links — ``/projects/x``, ``/upload/{token}`` — and refreshes load
index.html instead of 404ing under FastAPI static serving).

Uses a throwaway dist dir so the test does not depend on a real frontend build.
"""

from __future__ import annotations

from fastapi import FastAPI
from fastapi.testclient import TestClient

from app.config import AppConfig
from app.main import create_app


def _app(tmp_path, monkeypatch) -> FastAPI:  # noqa: ANN001
    dist = tmp_path / "dist"
    dist.mkdir()
    (dist / "index.html").write_text("<!doctype html><title>okc-web</title><div id=root></div>")
    (dist / "favicon.ico").write_bytes(b"x")
    monkeypatch.setenv("OKC_WEB_BOOTSTRAP_ADMIN_EMAIL", "admin@spa.test")
    monkeypatch.setenv("OKC_WEB_BOOTSTRAP_ADMIN_PASSWORD", "spa-secret-123")
    return create_app(
        AppConfig(
            state_db_path=str(tmp_path / "state.db"),
            projects_root=str(tmp_path / "projects"),
            providers=[],
            spa_dist=str(dist),
        )
    )


def test_spa_deeplink_falls_back_to_index(tmp_path, monkeypatch) -> None:  # noqa: ANN001
    with TestClient(_app(tmp_path, monkeypatch)) as client:
        # An unknown client-route deep-link must serve the SPA shell, not 404.
        r = client.get("/projects/abc/review")
        assert r.status_code == 200, r.text
        assert "<div id=root>" in r.text
        # The contributor upload deep-link (its direct entry point) must too.
        r2 = client.get("/upload/some-token")
        assert r2.status_code == 200, r2.text
        assert "<div id=root>" in r2.text
        # Real static asset still resolves.
        assert client.get("/favicon.ico").status_code == 200


def test_spa_fallback_does_not_swallow_api_404(tmp_path, monkeypatch) -> None:  # noqa: ANN001
    with TestClient(_app(tmp_path, monkeypatch)) as client:
        # Unknown /api path must NOT return the HTML shell.
        r = client.get("/api/definitely-not-a-route")
        assert r.status_code == 404
        assert "<div id=root>" not in r.text
        # A real upload-capability route (registered, wins over the mount) returns a
        # JSON error for a bad token — never the SPA shell.
        r2 = client.get("/u/badtoken")
        assert "<div id=root>" not in r2.text
        assert r2.json()["code"] == "TOKEN_INVALID"
        # Health endpoint still works (registered route wins over the mount).
        assert client.get("/api/health").json()["status"] == "ok"
