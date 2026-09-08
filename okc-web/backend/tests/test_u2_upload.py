"""U2 (Upload & Token) tests — REAL okc engine, NO mock (C4).

Builds an ISOLATED app (not ``app.main.create_app`` — a sibling unit may be
mid-write and would break auto-discovery): in-file StateDb, real ``EngineWorker``
with an empty provider list (``add_source`` needs no AI provider), U0 error
handlers, U0's canonical job routes, and U2's ``register``. A real okc project is
seeded through the adapter so the ingest pipeline exercises a genuine
``add_source`` landing + registration (the C4 no-mock proof).
"""

from __future__ import annotations

import io
import os
import time
import zipfile
from collections.abc import Iterator
from threading import Event

import pytest
from fastapi import FastAPI
from fastapi.testclient import TestClient
from sqlalchemy import text

from app.adapter.engine import OkcEngineImpl, build_client
from app.adapter.queue import EngineWorker
from app.config import AppConfig
from app.main import AppState, _mount_core_routes
from app.shared.audit import AuditStore
from app.shared.authz import SESSION_COOKIE, AdminPrincipal
from app.shared.error import register_error_handlers
from app.shared.jobs import JobStore
from app.shared.state import StateDb
from app.upload import router as upload_router

ADMIN_COOKIE = "good-cookie"
PROJECT_ID = "proj_u2"


class _StubSessionResolver:
    """Test-only stand-in for the U1 session seam (NOT a mock of okc)."""

    def __init__(self, principal: AdminPrincipal) -> None:
        self._principal = principal

    def resolve_admin(self, session_plaintext: str) -> AdminPrincipal | None:
        return self._principal if session_plaintext == ADMIN_COOKIE else None

    def resolve_upload(self, token_plaintext: str) -> None:
        return None


class Harness:
    def __init__(self, client: TestClient, state: AppState, engine_root: str) -> None:
        self.client = client
        self.state = state
        self.engine_root = engine_root


@pytest.fixture()
def harness(tmp_path) -> Iterator[Harness]:  # noqa: ANN001
    db = StateDb.open(str(tmp_path / "state.db"))
    jobs = JobStore(db)
    audit = AuditStore(db)
    engine = EngineWorker.create([], jobs)
    config = AppConfig(
        state_db_path=str(tmp_path / "state.db"),
        projects_root=str(tmp_path / "projects"),
        providers=[],
    )
    state = AppState(config, db, jobs, audit, engine)

    # Seed a real okc project on disk (U3 does not exist yet in W1).
    engine_root = str(tmp_path / "engine" / "demo.okc-project")
    seeder = OkcEngineImpl(build_client([]))
    from app.adapter import dto

    seeder.create_project(
        dto.CreateProjectSpec(root_abs_path=engine_root, name="Demo", curator_id="Admin One")
    )

    with db.engine.begin() as conn:
        conn.execute(
            text(
                "INSERT INTO accounts(id,email,display_name,role,password_hash,status,created_at,updated_at)"
                " VALUES('acc_admin','a@x','Admin','admin','x','active','t','t')"
            )
        )
        conn.execute(
            text(
                "INSERT INTO projects(id,name,engine_root_abs_path,curator_id,created_by,freeze_state,"
                "created_at,updated_at) VALUES(:id,'Demo',:root,'Admin One','acc_admin','unfrozen','t','t')"
            ),
            {"id": PROJECT_ID, "root": engine_root},
        )

    app = FastAPI()
    app.state.app_state = state
    app.state.session_resolver = _StubSessionResolver(
        AdminPrincipal(account_id="acc_admin", session_id_hash="h", curator_label="Admin One")
    )
    app.state.token_resolver = None
    register_error_handlers(app)
    _mount_core_routes(app)  # U0 canonical job routes (E2-S6 poll)
    upload_router.register(app, state)  # installs the real token resolver

    client = TestClient(app)
    client.cookies.set(SESSION_COOKIE, ADMIN_COOKIE)
    try:
        yield Harness(client, state, engine_root)
    finally:
        engine.shutdown()


# --- helpers ---


def _issue(harness: Harness, owner: str = "HR/Kim", kind: str = "individual") -> dict:
    r = harness.client.post(
        f"/api/projects/{PROJECT_ID}/tokens",
        json={"owner_display_name": owner, "owner_kind": kind},
    )
    assert r.status_code == 200, r.text
    return r.json()


def _upload_md(harness: Harness, token: str, name: str, body: bytes):
    return harness.client.post(
        f"/u/{token}/upload",
        files={"file": (name, body, "text/markdown")},
    )


def _poll(harness: Harness, token: str, job_id: str, timeout: float = 15.0) -> dict:
    deadline = time.time() + timeout
    while time.time() < deadline:
        r = harness.client.get(f"/u/{token}/jobs/{job_id}")
        assert r.status_code == 200, r.text
        snap = r.json()
        if snap["state"] in {"completed", "failed", "cancelled"}:
            return snap
        time.sleep(0.05)
    raise AssertionError("job did not reach terminal state in time")


def _sources_count(harness: Harness) -> int:
    with harness.state.db.engine.connect() as conn:
        return int(
            conn.execute(
                text("SELECT COUNT(*) FROM sources WHERE project_id=:p"), {"p": PROJECT_ID}
            ).scalar()
            or 0
        )


def _make_zip(entries: list[tuple[str, bytes]], symlinks: list[tuple[str, str]] | None = None) -> bytes:
    buf = io.BytesIO()
    with zipfile.ZipFile(buf, "w") as zf:
        for name, data in entries:
            zf.writestr(name, data)
        for name, target in symlinks or []:
            info = zipfile.ZipInfo(name)
            info.external_attr = 0o120777 << 16  # S_IFLNK
            zf.writestr(info, target)
    return buf.getvalue()


# --- E2-S1/E2-S2: token issue + one-time reveal + masking ---


def test_issue_token_one_time_reveal(harness: Harness) -> None:
    issued = _issue(harness)
    assert "." in issued["token"]  # selector.verifier
    assert issued["upload_url"] == f"/u/{issued['token']}"
    assert issued["slot_index"] == 0

    listing = harness.client.get(f"/api/projects/{PROJECT_ID}/tokens").json()
    assert listing["slot_usage"] == {"used": 0, "limit": 10}
    assert len(listing["tokens"]) == 1
    summary = listing["tokens"][0]
    assert summary["status"] == "active"
    # The plaintext token is never echoed back in the list (only the public selector).
    assert issued["token"] not in str(listing)
    assert summary["selector"] == issued["token"].split(".", 1)[0]


def test_issue_requires_admin_session(harness: Harness) -> None:
    harness.client.cookies.clear()
    r = harness.client.post(f"/api/projects/{PROJECT_ID}/tokens", json={})
    assert r.status_code == 401  # admin_auth denies before any core call (C-1)


def test_rotate_reuses_slot_and_revokes_old_secret(harness: Harness) -> None:
    first = _issue(harness, owner="HR")
    second = _issue(harness, owner="Legal")
    assert (first["slot_index"], second["slot_index"]) == (0, 1)

    rotated = harness.client.post(
        f"/api/projects/{PROJECT_ID}/tokens/{first['token_id']}/rotate"
    )
    assert rotated.status_code == 200, rotated.text
    replacement = rotated.json()
    assert replacement["slot_index"] == 0
    assert replacement["token"] != first["token"]
    assert harness.client.get(f"/u/{first['token']}").json()["code"] == "TOKEN_REVOKED"
    assert harness.client.get(f"/u/{replacement['token']}").status_code == 200


# --- E2-S3/E2-S4/E2-S5/E2-S6: the real ingest pipeline (C4 no-mock proof) ---


def test_real_add_source_landing(harness: Harness) -> None:
    token = _issue(harness)["token"]
    r = _upload_md(harness, token, "note.md", b"# Hello\n\nA note about okc integration.\n")
    assert r.status_code == 200, r.text
    accepted = r.json()
    source_id = accepted["source_id"]
    assert source_id.startswith("src_")

    snap = _poll(harness, token, accepted["job_id"])
    assert snap["state"] == "completed", snap

    # (a) the SourceRegistry row was written by U2 at commit.
    with harness.state.db.engine.connect() as conn:
        row = conn.execute(
            text("SELECT * FROM sources WHERE source_id=:s"), {"s": source_id}
        ).mappings().first()
    assert row is not None
    assert row["project_id"] == PROJECT_ID
    assert row["content_hash"] == accepted["content_hash"]
    assert row["upload_token_id"].startswith("tok_")

    # (b) the source is REALLY registered in the okc project (fresh client, on-disk read).
    manifest = OkcEngineImpl(build_client([])).manifest(harness.engine_root)
    registered_ids = [s["source_id"] for s in manifest["sources"]]
    assert source_id in registered_ids

    # the bytes actually landed on disk under the sources root.
    import os

    assert os.path.isfile(os.path.join(row["absolute_path"], "note.md"))

    # the token is now marked used and bound to the source.
    listing = harness.client.get(f"/api/projects/{PROJECT_ID}/tokens").json()
    assert listing["tokens"][0]["registered_source_id"] == source_id


# --- E2-S2/E2-S3: revoked / invalid tokens rejected BEFORE core ---


def test_revoked_token_blocked(harness: Harness) -> None:
    issued = _issue(harness)
    token, token_id = issued["token"], issued["token_id"]
    rv = harness.client.post(f"/api/projects/{PROJECT_ID}/tokens/{token_id}/revoke")
    assert rv.status_code == 204

    r = _upload_md(harness, token, "note.md", b"# x\n")
    assert r.status_code == 400
    assert r.json()["code"] in {"TOKEN_REVOKED", "TOKEN_INVALID"}
    assert _sources_count(harness) == 0  # core untouched (C-1)


def test_invalid_token_rejected(harness: Harness) -> None:
    r = harness.client.get("/u/not-a-real.token")
    assert r.status_code == 400
    assert r.json()["code"] == "TOKEN_INVALID"


# --- E2-S4: byte cap enforced while streaming ---


def test_byte_cap_upload_too_large(harness: Harness, monkeypatch) -> None:  # noqa: ANN001
    monkeypatch.setenv("OKC_WEB_MAX_UPLOAD_BYTES", "512")
    token = _issue(harness)["token"]
    r = _upload_md(harness, token, "big.md", b"x" * 4096)
    assert r.status_code == 400
    assert r.json()["code"] == "UPLOAD_TOO_LARGE"
    assert _sources_count(harness) == 0


def test_invalid_owner_kind_rejected_before_core_and_slot_released(harness: Harness) -> None:
    token = _issue(harness)["token"]
    rejected = harness.client.post(
        f"/u/{token}/upload",
        data={"owner_kind": "superuser"},
        files={"file": ("note.md", b"# invalid owner\n", "text/markdown")},
    )
    assert rejected.status_code == 400
    assert rejected.json()["code"] == "VALIDATION_FAILED"
    assert _sources_count(harness) == 0
    assert OkcEngineImpl(build_client([])).manifest(harness.engine_root)["sources"] == []

    accepted = _upload_md(harness, token, "valid.md", b"# valid owner\n")
    assert accepted.status_code == 200, accepted.text
    assert accepted.json()["slot_index"] == 0
    assert _poll(harness, token, accepted.json()["job_id"])["state"] == "completed"


# --- E2-S4: hostile-input blocked pre-landing (FR-UP-3) ---


def test_zip_traversal_blocked(harness: Harness) -> None:
    token = _issue(harness)["token"]
    payload = _make_zip([("../evil.md", b"# escape\n"), ("ok.md", b"# ok\n")])
    r = harness.client.post(f"/u/{token}/upload", files={"file": ("v.zip", payload, "application/zip")})
    assert r.status_code == 400
    assert r.json()["code"] == "PATH_UNSAFE"
    assert _sources_count(harness) == 0


def test_zip_symlink_blocked(harness: Harness) -> None:
    token = _issue(harness)["token"]
    payload = _make_zip([("ok.md", b"# ok\n")], symlinks=[("link.md", "/etc/passwd")])
    r = harness.client.post(f"/u/{token}/upload", files={"file": ("v.zip", payload, "application/zip")})
    assert r.status_code == 400
    assert r.json()["code"] == "PATH_UNSAFE"
    assert _sources_count(harness) == 0


def test_valid_zip_lands_and_registers(harness: Harness) -> None:
    token = _issue(harness)["token"]
    payload = _make_zip([("a.md", b"# A\n"), ("sub/b.md", b"# B\n")])
    r = harness.client.post(f"/u/{token}/upload", files={"file": ("v.zip", payload, "application/zip")})
    assert r.status_code == 200, r.text
    snap = _poll(harness, token, r.json()["job_id"])
    assert snap["state"] == "completed", snap
    manifest = OkcEngineImpl(build_client([])).manifest(harness.engine_root)
    assert r.json()["source_id"] in [s["source_id"] for s in manifest["sources"]]


@pytest.mark.parametrize("entries", [
    [("Caf\u00e9.md", b"composed"), ("Cafe\u0301.md", b"decomposed")],
    [("Caf\u00e9", b"file"), ("Cafe\u0301/note.md", b"ancestor alias")],
    [("NOTE.md", b"upper"), ("note.md", b"lower")],
])
def test_zip_platform_aliases_rejected_before_materialization(harness: Harness, entries: list[tuple[str, bytes]]) -> None:
    token = _issue(harness)["token"]
    original = _upload_md(harness, token, "keep.md", b"# Existing knowledge")
    assert _poll(harness, token, original.json()["job_id"])["state"] == "completed"
    before = OkcEngineImpl(build_client([])).manifest(harness.engine_root)["sources"]
    response = harness.client.post(f"/u/{token}/upload", files={
        "file": ("aliases.zip", _make_zip(entries), "application/zip"),
    })
    assert response.status_code == 400, response.text
    assert response.json()["code"] == "PATH_UNSAFE"
    assert OkcEngineImpl(build_client([])).manifest(harness.engine_root)["sources"] == before
    assert _sources_count(harness) == 1
    revision_parent = os.path.dirname(before[0]["path"])
    assert len(os.listdir(revision_parent)) == 1


# --- E2-S5: ≤10 source cap enforced ahead of core ---


def test_source_cap_exceeded(harness: Harness) -> None:
    token = _issue(harness)["token"]
    with harness.state.db.engine.begin() as conn:
        for i in range(10):
            conn.execute(
                text(
                    "INSERT INTO sources(source_id,project_id,content_hash,absolute_path,slot_index,registered_at)"
                    " VALUES(:s,:p,:h,:path,:i,'t')"
                ),
                {"s": f"src_seed{i}", "p": PROJECT_ID, "h": f"h{i}", "path": f"/tmp/s{i}", "i": i},
            )
    r = _upload_md(harness, token, "eleventh.md", b"# 11th\n")
    assert r.status_code == 429
    assert r.json()["code"] == "SOURCE_CAP_EXCEEDED"
    assert _sources_count(harness) == 10  # no 11th source


# --- E2-S5: duplicate content is an idempotent no-op ---


def test_duplicate_source_idempotent(harness: Harness) -> None:
    token = _issue(harness)["token"]
    body = b"# Dup\n\nsame content re-uploaded.\n"
    first = _upload_md(harness, token, "dup.md", body)
    assert first.status_code == 200, first.text
    assert _poll(harness, token, first.json()["job_id"])["state"] == "completed"
    assert _sources_count(harness) == 1

    second = _upload_md(harness, token, "dup.md", body)
    assert second.status_code == 400
    assert second.json()["code"] == "DUPLICATE_SOURCE"
    assert _sources_count(harness) == 1  # idempotent — no second registration


def test_concurrent_duplicate_jobs_commit_once_with_stable_reservation(harness: Harness) -> None:
    """Both HTTP pre-checks may pass while the writer is busy; the serialized
    authoritative check must commit exactly one revision of the stable source."""
    token = _issue(harness)["token"]
    started = Event()
    release = Event()

    def block_writer(_engine, _progress) -> None:  # noqa: ANN001
        started.set()
        if not release.wait(timeout=5):
            raise AssertionError("test writer blocker timed out")

    assert harness.state.engine is not None
    harness.state.engine.enqueue("test_blocker", PROJECT_ID, "test", block_writer)
    assert started.wait(timeout=2)

    body = b"# Same\n\nqueued twice before either source commits.\n"
    try:
        first = _upload_md(harness, token, "first.md", body)
        second = _upload_md(harness, token, "first.md", body)
        assert first.status_code == 200, first.text
        assert second.status_code == 200, second.text
        assert {first.json()["slot_index"], second.json()["slot_index"]} == {0}
        assert first.json()["source_id"] == second.json()["source_id"]
    finally:
        release.set()

    first_snap = _poll(harness, token, first.json()["job_id"])
    second_snap = _poll(harness, token, second.json()["job_id"])
    assert first_snap["state"] == "completed"
    assert second_snap["state"] == "failed"
    assert second_snap["error"]["code"] == "DUPLICATE_SOURCE"
    assert _sources_count(harness) == 1

    # Rejected bytes never replace the successful revision's native binding.
    sources = OkcEngineImpl(build_client([])).manifest(harness.engine_root)["sources"]
    assert len(sources) == 1
    assert os.path.isfile(os.path.join(sources[0]["path"], "first.md"))
