"""Native validation can fail before returning a Job; preserve typed HTTP errors."""

from __future__ import annotations

import pytest
from fastapi.testclient import TestClient

from app.adapter import dto
from app.adapter.engine import OkcEngineImpl, build_client
from app.shared.error import EngineError, EngineErrorCode
from tests.test_u4_review import _create_project, _login, _make_app


@pytest.mark.parametrize("method", ["open_project", "status", "preflight", "taxonomy", "clusters", "manifest", "verify"])
def test_native_absolute_path_validation_is_mapped(method: str) -> None:
    engine = OkcEngineImpl(build_client([]))
    with pytest.raises(EngineError) as caught:
        getattr(engine, method)("relative.okc-project")
    assert caught.value.code == EngineErrorCode.PATH_NOT_ABSOLUTE
    assert caught.value.http_status() == 400


def test_native_taxonomy_schema_error_is_mapped_before_job(tmp_path) -> None:
    engine = OkcEngineImpl(build_client([]))
    root = str(tmp_path / "test.okc-project")
    engine.create_project(dto.CreateProjectSpec(root_abs_path=root, name="Test", curator_id="Curator"))
    with pytest.raises(EngineError) as caught:
        engine.approve_taxonomy(root, dto.ApproveTaxonomyCmd(edited_clusters=[{"invalid": True}], rationale="test"))
    assert caught.value.code == EngineErrorCode.INVALID_ARGUMENT
    assert caught.value.http_status() == 400


def test_malformed_taxonomy_returns_structured_http_error(tmp_path, monkeypatch) -> None:
    app = _make_app(tmp_path, monkeypatch)
    with TestClient(app) as client:
        _login(client)
        project_id = _create_project(client)
        response = client.post(f"/api/projects/{project_id}/taxonomy/approve", json={
            "edited_clusters": [{"invalid": True}], "rationale": "malformed edit",
        })
        assert response.status_code == 400, response.text
        assert response.json()["code"] == "INVALID_ARGUMENT"
        assert response.json()["category"] == "validation"
