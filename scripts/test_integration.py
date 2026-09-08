"""Four-module contract test: MCP authoring → Rust hooks frames → web/core → MCP.

Run from repository root with the backend venv and backend pytest configuration.
Requires built MCP dist and the hooks protocol-fixture example. Uses a synthetic
provider; filesystem watching/retry behavior is covered by hooks' own tests.
"""

from __future__ import annotations

import json
import socket
import subprocess
import threading
import time
from pathlib import Path

import cbor2
import uvicorn

from tests.test_serving_revisions import _compile_publish, _ok
from tests.test_serving_revisions import published_client as published_client

ROOT = Path(__file__).resolve().parents[1]


def _mcp(config: Path, operation: str, note: str, expected: str) -> dict:
    result = subprocess.run([
        "node", str(ROOT / "scripts/mcp-integration-client.mjs"),
        "--config", str(config), "--operation", operation,
        "--note-path", note, "--expected", expected,
    ], cwd=ROOT, capture_output=True, text=True, timeout=30)
    assert result.returncode == 0, result.stderr
    data = json.loads(result.stdout)
    assert data["ok"], data
    return data


def test_mcp_author_hooks_upload_core_publish_mcp_read(published_client, tmp_path):
    client, app, pid, token, _first = published_client
    source = tmp_path / "source"
    source.mkdir()
    local_config = tmp_path / "author.json"
    local_config.write_text(json.dumps({"vaultPath": str(source), "statePath": str(tmp_path / "mcp-state")}), encoding="utf-8")
    phrase = "Knowledge shared through all four OKC modules."
    authored = _mcp(local_config, "author", "notes/from-agent.md", phrase)
    assert authored["operation"] == "author"
    content = (source / "notes/from-agent.md").read_text(encoding="utf-8")
    binary = ROOT / "okc-hooks/target/debug/examples/protocol-fixture"
    assert binary.is_file(), "build: cargo build -p upload-client --example protocol-fixture"
    wire = subprocess.run([str(binary), "notes/from-agent.md", content],
                          capture_output=True, text=True, check=True, timeout=10)
    frames = json.loads(wire.stdout)
    headers = {"Authorization": f"Bearer {token}", "Content-Type": "application/cbor"}
    negotiated = client.post("/api/sync/negotiate", content=bytes.fromhex(frames["negotiate_hex"]), headers=headers)
    assert negotiated.status_code == 200, negotiated.text
    offer = cbor2.loads(negotiated.content)
    headers["X-OKC-Upload-Session"] = offer["session_id"]
    for blob in frames["blobs"]:
        uploaded = client.put(f"/api/sync/blob/{blob['sha256_hex']}/0",
                              content=bytes.fromhex(blob["frame_hex"]), headers=headers)
        assert uploaded.status_code == 200, uploaded.text
    committed = client.post("/api/sync/commit", content=bytes.fromhex(frames["commit_hex"]), headers=headers)
    assert committed.status_code == 200, committed.text
    assert cbor2.loads(committed.content)["committed"] is True
    assert _ok(client.get(f"/api/projects/{pid}"))["source_count"] == 1
    published = _compile_publish(client, pid)

    read_token = _ok(client.post(f"/api/projects/{pid}/serving/tokens"))["token"]
    _ok(client.put(f"/api/projects/{pid}/serving/access", json={"mode": "private"}))
    sock = socket.socket()
    sock.bind(("127.0.0.1", 0))
    port = sock.getsockname()[1]
    server = uvicorn.Server(uvicorn.Config(app, lifespan="off", log_level="error"))
    thread = threading.Thread(target=server.run, kwargs={"sockets": [sock]}, daemon=True)
    thread.start()
    try:
        deadline = time.monotonic() + 10
        while not server.started and thread.is_alive() and time.monotonic() < deadline:
            time.sleep(0.01)
        assert server.started
        web_config = tmp_path / "reader.json"
        web_config.write_text(json.dumps({"readOnly": True, "web": {
            "baseUrl": f"http://127.0.0.1:{port}", "projectId": pid, "token": read_token,
        }}), encoding="utf-8")
        result = _mcp(web_config, "read", "knowledge/sdk/fixture.md", phrase)
        assert result["source"]["revision"] == published["revision"]
        assert result["source"]["kind"] == "web"
        assert result["verified"] and result["hasProvenance"]
    finally:
        server.should_exit = True
        thread.join(timeout=10)
        sock.close()
        assert not thread.is_alive()
