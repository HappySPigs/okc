"""Real native binding + real Rust CBOR fixture coverage for hooks reception."""

from __future__ import annotations

import hashlib
import json
import os
import subprocess
from pathlib import Path

import cbor2
import pytest
from sqlalchemy import event, text

from app.adapter.engine import OkcEngineImpl, build_client
from app.orchestration.projects import ProjectRegistry
from app.upload.revisions import manifest_digest
from app.upload.sync import SESSION_HEADER
from tests.test_u2_upload import Harness, _issue, _poll, _sources_count, _upload_md
from tests.test_u2_upload import harness as harness


def _wire(files: dict[str, bytes]) -> tuple[dict, dict, list[tuple[str, dict]]]:
    entries = [(path, hashlib.sha256(data).hexdigest(), len(data)) for path, data in files.items()]
    digest = list(bytes.fromhex(manifest_digest(entries)))
    negotiate = {"manifest_digest": digest,
                 "entries": [[p, list(bytes.fromhex(h)), size] for p, h, size in entries]}
    commit = {"manifest_digest": digest,
              "path_hash_map": {"entries": {p: list(bytes.fromhex(h)) for p, h, _ in entries}}}
    blobs = [(hashlib.sha256(data).hexdigest(), {
        "blob": list(hashlib.sha256(data).digest()), "offset": 0, "len": len(data),
        "chunk_sha256": list(hashlib.sha256(data).digest()), "bytes": list(data),
    }) for data in files.values()]
    return negotiate, commit, blobs


def _request(harness: Harness, token: str, suffix: str, body: dict,
             *, session: str | None = None, method: str = "POST"):
    headers = {"authorization": f"Bearer {token}", "content-type": "application/cbor"}
    if session:
        headers[SESSION_HEADER] = session
    return harness.client.request(method, f"/api/sync/{suffix}", content=cbor2.dumps(body), headers=headers)


def _negotiate(harness: Harness, token: str, body: dict) -> dict:
    response = _request(harness, token, "negotiate", body)
    assert response.status_code == 200, response.text
    return cbor2.loads(response.content)


def _sync(harness: Harness, token: str, files: dict[str, bytes]) -> tuple[str, dict]:
    negotiate, commit, blobs = _wire(files)
    result = _negotiate(harness, token, negotiate)
    session = result["session_id"]
    server_has = {bytes(h).hex() for h in result["server_has"]}
    for raw_hash, frame in blobs:
        if raw_hash in server_has:
            continue
        response = _request(harness, token, f"blob/{raw_hash}/0", frame, session=session, method="PUT")
        assert response.status_code == 200, response.text
    response = _request(harness, token, "commit", commit, session=session)
    assert response.status_code == 200, response.text
    receipt = cbor2.loads(response.content)
    assert receipt == {"committed": True, "server_vault_content_id": bytes(commit["manifest_digest"]).hex()}
    return session, receipt


def test_hooks_revisions_replace_stable_source_and_delete_rename(harness: Harness) -> None:
    token = _issue(harness)["token"]
    _sync(harness, token, {"old.md": b"# old", "delete.md": b"# delete"})
    before = OkcEngineImpl(build_client([])).manifest(harness.engine_root)["sources"][0]
    fingerprint = ProjectRegistry(harness.state.db).source_set_fingerprint("proj_u2")
    for version in range(12):
        _sync(harness, token, {"new.md": f"# Version {version}".encode()})
    after = OkcEngineImpl(build_client([])).manifest(harness.engine_root)["sources"]
    assert len(after) == _sources_count(harness) == 1
    assert after[0]["source_id"] == before["source_id"]
    assert after[0]["path"] != before["path"]
    assert Path(before["path"], "delete.md").is_file()  # history remains immutable
    assert sorted(p.name for p in Path(after[0]["path"]).iterdir() if p.is_file()) == ["new.md"]
    assert Path(after[0]["path"], "new.md").read_bytes() == b"# Version 11"
    assert ProjectRegistry(harness.state.db).source_set_fingerprint("proj_u2") != fingerprint
    with harness.state.db.engine.connect() as conn:
        assert conn.execute(text("SELECT COUNT(*) FROM source_revisions")).scalar_one() == 13


def test_commit_retry_is_durable_and_does_not_restore_old_revision(harness: Harness) -> None:
    token = _issue(harness)["token"]
    first = {"a.md": b"# First"}
    session, receipt = _sync(harness, token, first)
    _sync(harness, token, {"b.md": b"# Newer"})
    replay = _request(harness, token, "commit", _wire(first)[1], session=session)
    assert cbor2.loads(replay.content) == receipt
    current = OkcEngineImpl(build_client([])).manifest(harness.engine_root)["sources"][0]
    assert Path(current["path"], "b.md").exists()


def test_concurrent_snapshot_stale_base_requires_renegotiation(harness: Harness) -> None:
    token = _issue(harness)["token"]
    stale = _wire({"stale.md": b"# stale"})
    session = _negotiate(harness, token, stale[0])["session_id"]
    raw_hash, frame = stale[2][0]
    assert _request(harness, token, f"blob/{raw_hash}/0", frame, session=session, method="PUT").status_code == 200
    _sync(harness, token, {"current.md": b"# current"})
    response = _request(harness, token, "commit", stale[1], session=session)
    assert response.status_code == 409
    assert _negotiate(harness, token, stale[0])["session_id"] != session


def test_resume_offsets_come_from_durable_server_bytes_and_chunk_replay(harness: Harness) -> None:
    token = _issue(harness)["token"]
    negotiate, commit, blobs = _wire({"note.md": b"abcdefgh"})
    session = _negotiate(harness, token, negotiate)["session_id"]
    raw_hash, frame = blobs[0]
    first = dict(frame, bytes=list(b"abcd"), len=4, chunk_sha256=list(hashlib.sha256(b"abcd").digest()))
    assert _request(harness, token, f"blob/{raw_hash}/0", first, session=session, method="PUT").status_code == 200
    assert _request(harness, token, f"blob/{raw_hash}/0", first, session=session, method="PUT").status_code == 200
    # New service instance observes stored sessions and offsets after a restart.
    from app.shared.authz import UploadContext
    from app.upload.sync import SyncService
    from app.upload.tokens import UploadTokenResolver, UploadTokenStore
    ctx = UploadTokenResolver(UploadTokenStore(harness.state.db)).resolve_upload(token)
    assert isinstance(ctx, UploadContext)
    resumed = SyncService(harness.state.db, harness.state.config, harness.state.engine).negotiate(ctx, negotiate)
    assert resumed["session_id"] == session
    assert resumed["resume_offsets"] == [[list(bytes.fromhex(raw_hash)), 4]]
    last = dict(frame, offset=4, bytes=list(b"efgh"), len=4, chunk_sha256=list(hashlib.sha256(b"efgh").digest()))
    assert _request(harness, token, f"blob/{raw_hash}/4", last, session=session, method="PUT").status_code == 200
    assert _request(harness, token, "commit", commit, session=session).status_code == 200


def test_token_rotation_retains_vault_identity_and_multipart_interoperates(harness: Harness) -> None:
    issued = _issue(harness)
    first = _upload_md(harness, issued["token"], "local.md", b"# local")
    assert _poll(harness, issued["token"], first.json()["job_id"])["state"] == "completed"
    source_id = first.json()["source_id"]
    rotated = harness.client.post(f"/api/projects/proj_u2/tokens/{issued['token_id']}/rotate").json()
    assert rotated["sync_endpoint"] == "/api/sync"
    _sync(harness, rotated["token"], {"cloud.md": b"# cloud"})
    sources = OkcEngineImpl(build_client([])).manifest(harness.engine_root)["sources"]
    assert len(sources) == 1 and sources[0]["source_id"] == source_id
    assert not Path(sources[0]["path"], "local.md").exists()
    assert _request(harness, issued["token"], "negotiate", _wire({})[0]).status_code == 401


@pytest.mark.parametrize("corrupt", [False, True])
def test_restart_between_final_chunk_fsync_and_cache_publish_recovers(harness: Harness, corrupt: bool) -> None:
    issued = _issue(harness)
    token = issued["token"]
    negotiate, commit, blobs = _wire({"note.md": b"# crash-window"})
    session = _negotiate(harness, token, negotiate)["session_id"]
    raw_hash, frame = blobs[0]
    assert _request(harness, token, f"blob/{raw_hash}/0", frame, session=session, method="PUT").status_code == 200
    root = Path(harness.state.config.projects_root, "proj_u2", "sync", f"src_{issued['token_id'][4:]}")
    complete = root / "blobs" / raw_hash
    partial = root / "transfers" / session / raw_hash
    complete.replace(partial)  # The durable state immediately before os.replace.
    if corrupt:
        partial.write_bytes(b"x" * frame["len"])
    resumed = _negotiate(harness, token, negotiate)
    if corrupt:
        assert resumed["server_has"] == []
        assert resumed["resume_offsets"] == [[list(bytes.fromhex(raw_hash)), 0]]
        assert partial.stat().st_size == 0
        assert _request(harness, token, f"blob/{raw_hash}/0", frame, session=session, method="PUT").status_code == 200
    else:
        assert resumed["server_has"] == [list(bytes.fromhex(raw_hash))]
        assert resumed["resume_offsets"] == []
        assert complete.read_bytes() == bytes(frame["bytes"])
        assert not partial.exists()
    assert _request(harness, token, "commit", commit, session=session).status_code == 200


def test_bad_hash_missing_blob_and_wrong_token_never_commit(harness: Harness) -> None:
    token = _issue(harness)["token"]
    other = _issue(harness, owner="Other")["token"]
    negotiate, commit, blobs = _wire({"a.md": b"# A"})
    bad = dict(negotiate, manifest_digest=[0] * 32)
    assert _request(harness, token, "negotiate", bad).status_code == 400
    session = _negotiate(harness, token, negotiate)["session_id"]
    assert _request(harness, other, "commit", commit, session=session).status_code == 404
    assert _request(harness, token, "commit", commit, session=session).status_code == 400
    raw_hash, frame = blobs[0]
    assert _request(harness, token, f"blob/{raw_hash}/0", dict(frame, chunk_sha256=[0] * 32),
                    session=session, method="PUT").status_code == 400
    assert _sources_count(harness) == 0


@pytest.mark.parametrize("files", [{"../evil.md": b"bad"}, {"A.md": b"a", "a.md": b"b"},
                                   {"a": b"a", "a/b.md": b"b"}])
def test_manifest_rejects_path_escape_aliases_and_overlap(harness: Harness, files: dict[str, bytes]) -> None:
    token = _issue(harness)["token"]
    assert _request(harness, token, "negotiate", _wire(files)[0]).status_code == 400
    assert _sources_count(harness) == 0


@pytest.mark.parametrize("files", [
    {"Caf\u00e9.md": b"composed", "Cafe\u0301.md": b"decomposed"},
    {"Caf\u00e9": b"file", "Cafe\u0301/note.md": b"ancestor alias"},
])
def test_unicode_alias_negotiation_preserves_current_source(harness: Harness, files: dict[str, bytes]) -> None:
    token = _issue(harness)["token"]
    _sync(harness, token, {"keep.md": b"# Existing knowledge"})
    before = OkcEngineImpl(build_client([])).manifest(harness.engine_root)["sources"]
    with harness.state.db.engine.connect() as conn:
        sessions = conn.execute(text("SELECT COUNT(*) FROM upload_sync_sessions")).scalar_one()
    response = _request(harness, token, "negotiate", _wire(files)[0])
    assert response.status_code == 400, response.text
    assert response.json()["code"] == "VALIDATION_FAILED"
    assert OkcEngineImpl(build_client([])).manifest(harness.engine_root)["sources"] == before
    assert sorted(p.name for p in Path(before[0]["path"]).iterdir() if p.is_file()) == ["keep.md"]
    with harness.state.db.engine.connect() as conn:
        assert conn.execute(text("SELECT COUNT(*) FROM upload_sync_sessions")).scalar_one() == sessions
        assert conn.execute(text("SELECT COUNT(*) FROM source_revisions")).scalar_one() == 1


def test_empty_vault_and_zero_length_blob_are_valid_full_snapshots(harness: Harness) -> None:
    token = _issue(harness)["token"]
    _sync(harness, token, {"empty.md": b""})
    _sync(harness, token, {})
    source = OkcEngineImpl(build_client([])).manifest(harness.engine_root)["sources"][0]
    assert list(Path(source["path"]).rglob("*.md")) == []
    assert Path(source["path"], ".obsidian").is_dir()


def test_core_success_db_failure_retries_without_duplicate_source(harness: Harness) -> None:
    token = _issue(harness)["token"]
    negotiate, commit, blobs = _wire({"note.md": b"# durable"})
    session = _negotiate(harness, token, negotiate)["session_id"]
    raw_hash, frame = blobs[0]
    assert _request(harness, token, f"blob/{raw_hash}/0", frame, session=session, method="PUT").status_code == 200

    def fail_finalization(_conn, _cursor, statement, _parameters, _context, _executemany):
        if statement.startswith("INSERT INTO sources("):
            raise RuntimeError("injected DB finalization failure")

    event.listen(harness.state.db.engine, "before_cursor_execute", fail_finalization)
    try:
        with pytest.raises(RuntimeError, match="injected DB"):
            _request(harness, token, "commit", commit, session=session)
    finally:
        event.remove(harness.state.db.engine, "before_cursor_execute", fail_finalization)
    assert _sources_count(harness) == 0
    assert len(OkcEngineImpl(build_client([])).manifest(harness.engine_root)["sources"]) == 1
    response = _request(harness, token, "commit", commit, session=session)
    assert response.status_code == 200, response.text
    assert _sources_count(harness) == 1
    assert len(OkcEngineImpl(build_client([])).manifest(harness.engine_root)["sources"]) == 1


def test_ten_distinct_sources_still_accept_existing_vault_updates(harness: Harness) -> None:
    tokens = [_issue(harness, owner=f"Contributor {index}")["token"] for index in range(10)]
    for index, token in enumerate(tokens):
        _sync(harness, token, {"note.md": f"# Source {index}".encode()})
    _sync(harness, tokens[0], {"updated.md": b"# replacement at capacity"})
    assert _sources_count(harness) == 10
    assert len(OkcEngineImpl(build_client([])).manifest(harness.engine_root)["sources"]) == 10


def test_snapshot_limit_and_malformed_cbor_are_rejected_before_core(harness: Harness, monkeypatch) -> None:
    token = _issue(harness)["token"]
    monkeypatch.setenv("OKC_WEB_MAX_SYNC_BYTES", "4")
    assert _request(harness, token, "negotiate", _wire({"a.md": b"12345"})[0]).status_code == 400
    response = harness.client.post("/api/sync/negotiate", content=b"\xff",
                                   headers={"authorization": f"Bearer {token}", "content-type": "application/cbor"})
    assert response.status_code == 400
    assert harness.client.post("/api/sync/negotiate", content=b"\xff").status_code == 401
    assert _sources_count(harness) == 0


def test_rust_ciborium_fixture_round_trip(harness: Harness) -> None:
    fixture = Path(__file__).resolve().parents[3] / "okc-hooks/target/debug/examples/protocol-fixture"
    if not fixture.is_file():
        pytest.skip("build okc-hooks upload-client --example protocol-fixture for native wire coverage")
    result = subprocess.run([os.fspath(fixture), "note.md", "# Rust fixture"], check=True,
                            capture_output=True, text=True)
    wire = json.loads(result.stdout)
    token = _issue(harness)["token"]
    negotiate = cbor2.loads(bytes.fromhex(wire["negotiate_hex"]))
    session = _negotiate(harness, token, negotiate)["session_id"]
    for blob in wire["blobs"]:
        frame = cbor2.loads(bytes.fromhex(blob["frame_hex"]))
        response = _request(harness, token, f"blob/{blob['sha256_hex']}/0", frame, session=session, method="PUT")
        assert response.status_code == 200, response.text
    response = _request(harness, token, "commit", cbor2.loads(bytes.fromhex(wire["commit_hex"])), session=session)
    assert cbor2.loads(response.content)["server_vault_content_id"] == wire["manifest_digest_hex"]
