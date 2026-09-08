"""Immutable compilation receipts, publication pointers and scoped read access."""

from __future__ import annotations

import hashlib
import json
import os
import secrets
from datetime import UTC, datetime
from typing import Any

from sqlalchemy import text

from app.shared.error import EngineError
from app.shared.state import StateDb


def digest(value: Any) -> str:
    return hashlib.sha256(json.dumps(value, sort_keys=True, separators=(",", ":")).encode()).hexdigest()


def project_fingerprint(root: str) -> str | None:
    try:
        with open(os.path.join(root, "manifest.json"), "rb") as handle:
            return hashlib.sha256(handle.read(4 * 1024 * 1024)).hexdigest()
    except OSError:
        return None


def compilation_receipt(path: str, manifest: dict[str, Any], source_fingerprint: str,
                        project_root: str) -> dict[str, Any]:
    """Capture hashes only after the core has verified this complete artifact."""
    root = os.path.realpath(path)
    if root != os.path.abspath(path):
        raise EngineError.validation("compiled artifact cannot contain root symlink aliases")
    hashes: dict[str, str] = {}
    for base in ("knowledge", "legacy", ".okc"):
        for directory, dirs, files in os.walk(os.path.join(root, base)):
            for name in dirs + files:
                if os.path.islink(os.path.join(directory, name)):
                    raise EngineError.validation("compiled artifact contains a symlink")
            for name in files:
                full = os.path.join(directory, name)
                hashed = hashlib.sha256()
                with open(full, "rb") as handle:
                    for chunk in iter(lambda: handle.read(1024 * 1024), b""):
                        hashed.update(chunk)
                hashes[os.path.relpath(full, root).replace(os.sep, "/")] = hashed.hexdigest()
    revision = hashes.get(".okc/manifest.json")
    if not revision:
        raise EngineError.validation("compiled manifest missing")
    return {"path": root, "revision": revision, "manifest": manifest, "file_hashes": hashes,
            "source_fingerprint": source_fingerprint, "project_fingerprint": project_fingerprint(project_root)}


class SnapshotStore:
    def __init__(self, db: StateDb) -> None:
        self.db = db
        with db.engine.begin() as conn:
            for statement in (
                "CREATE TABLE IF NOT EXISTS serving_snapshots (project_id TEXT NOT NULL REFERENCES projects(id), revision TEXT NOT NULL, payload_json TEXT NOT NULL, created_at TEXT NOT NULL, published_at TEXT, PRIMARY KEY(project_id,revision))",
                "CREATE TABLE IF NOT EXISTS serving_heads (project_id TEXT PRIMARY KEY REFERENCES projects(id), revision TEXT NOT NULL)",
                "CREATE TABLE IF NOT EXISTS serving_access (project_id TEXT PRIMARY KEY REFERENCES projects(id), mode TEXT NOT NULL CHECK(mode IN ('public','private')))",
                "CREATE TABLE IF NOT EXISTS serving_read_tokens (id TEXT PRIMARY KEY, project_id TEXT NOT NULL REFERENCES projects(id), token_hash TEXT NOT NULL UNIQUE, created_at TEXT NOT NULL, revoked_at TEXT)",
            ):
                conn.execute(text(statement))

    def decision_fingerprint(self, project_id: str) -> str:
        with self.db.engine.connect() as conn:
            row = conn.execute(text(
                "SELECT COUNT(*) AS n, MAX(id) AS latest FROM curator_decisions WHERE project_id=:pid"
            ), {"pid": project_id}).mappings().one()
        return digest(dict(row))

    def record(self, project_id: str, payload: dict[str, Any]) -> None:
        with self.db.engine.begin() as conn:
            conn.execute(text(
                "INSERT INTO serving_snapshots(project_id,revision,payload_json,created_at)"
                " VALUES(:pid,:revision,:payload,:now) ON CONFLICT(project_id,revision) DO UPDATE SET"
                " payload_json=excluded.payload_json,created_at=excluded.created_at"
            ), {"pid": project_id, "revision": payload["revision"],
                "payload": json.dumps(payload, sort_keys=True), "now": datetime.now(UTC).isoformat()})
            # Identical manifest bytes name the same immutable revision. A new
            # verified physical copy repairs missing/corrupt storage without
            # changing what a pinned reader sees or reviving an offline project.
            conn.execute(text(
                "UPDATE serving_publications SET compiled_vault_path=:path WHERE project_id=:pid"
                " AND EXISTS (SELECT 1 FROM serving_heads WHERE project_id=:pid AND revision=:rev)"
            ), {"pid": project_id, "rev": payload["revision"], "path": payload["path"]})

    def get(self, project_id: str, revision: str, *, published_only: bool = False) -> dict[str, Any] | None:
        with self.db.engine.connect() as conn:
            row = conn.execute(text(
                "SELECT payload_json,published_at FROM serving_snapshots WHERE project_id=:pid AND revision=:rev"
            ), {"pid": project_id, "rev": revision}).mappings().first()
        if row is None or (published_only and row["published_at"] is None):
            return None
        return dict(json.loads(row["payload_json"]))

    def latest_compiled(self, project_id: str) -> dict[str, Any] | None:
        with self.db.engine.connect() as conn:
            value = conn.execute(text(
                "SELECT payload_json FROM serving_snapshots WHERE project_id=:pid ORDER BY created_at DESC,revision DESC LIMIT 1"
            ), {"pid": project_id}).scalar()
        return dict(json.loads(value)) if value else None

    def current(self, project_id: str) -> dict[str, Any] | None:
        with self.db.engine.connect() as conn:
            value = conn.execute(text(
                "SELECT s.payload_json FROM serving_heads h JOIN serving_snapshots s"
                " ON s.project_id=h.project_id AND s.revision=h.revision"
                " WHERE h.project_id=:pid"
            ), {"pid": project_id}).scalar()
        return dict(json.loads(value)) if value else None

    def activate(self, project_id: str, payload: dict[str, Any], published_by: str | None) -> None:
        now = datetime.now(UTC).isoformat()
        with self.db.engine.begin() as conn:
            conn.execute(text(
                "UPDATE serving_snapshots SET published_at=:at WHERE project_id=:pid AND revision=:rev"
            ), {"pid": project_id, "rev": payload["revision"], "at": now})
            conn.execute(text(
                "INSERT INTO serving_publications(project_id,compiled_vault_path,bound_integration_plan_id,"
                "bound_corpus_hash,bound_taxonomy_hash,status,published_at,published_by)"
                " VALUES(:pid,:path,:plan,:corpus,:tax,'live',:at,:by) ON CONFLICT(project_id) DO UPDATE SET"
                " compiled_vault_path=excluded.compiled_vault_path,bound_integration_plan_id=excluded.bound_integration_plan_id,"
                " bound_corpus_hash=excluded.bound_corpus_hash,bound_taxonomy_hash=excluded.bound_taxonomy_hash,"
                " status='live',published_at=excluded.published_at,published_by=excluded.published_by"
            ), {"pid": project_id, "path": payload["path"], "plan": payload["manifest"]["integration_plan_id"],
                "corpus": payload["manifest"]["corpus_hash"], "tax": payload["manifest"]["taxonomy_hash"],
                "at": now, "by": published_by})
            conn.execute(text(
                "INSERT INTO serving_heads(project_id,revision) VALUES(:pid,:rev)"
                " ON CONFLICT(project_id) DO UPDATE SET revision=excluded.revision"
            ), {"pid": project_id, "rev": payload["revision"]})

    def history(self, project_id: str) -> list[dict[str, Any]]:
        with self.db.engine.connect() as conn:
            rows = conn.execute(text(
                "SELECT revision,created_at,published_at FROM serving_snapshots WHERE project_id=:pid"
                " AND published_at IS NOT NULL ORDER BY published_at DESC LIMIT 100"
            ), {"pid": project_id}).mappings().all()
        return [dict(row) for row in rows]

    def mode(self, project_id: str) -> str:
        with self.db.engine.connect() as conn:
            value = conn.execute(text("SELECT mode FROM serving_access WHERE project_id=:pid"), {"pid": project_id}).scalar()
        return str(value or "public")

    def set_mode(self, project_id: str, mode: str) -> None:
        if mode not in ("public", "private"):
            raise EngineError.validation("access must be public or private")
        with self.db.engine.begin() as conn:
            conn.execute(text(
                "INSERT INTO serving_access(project_id,mode) VALUES(:pid,:mode)"
                " ON CONFLICT(project_id) DO UPDATE SET mode=excluded.mode"
            ), {"pid": project_id, "mode": mode})

    def issue_token(self, project_id: str) -> dict[str, str]:
        token_id = "read_" + secrets.token_hex(12)
        token = secrets.token_urlsafe(32)
        with self.db.engine.begin() as conn:
            conn.execute(text(
                "INSERT INTO serving_read_tokens(id,project_id,token_hash,created_at) VALUES(:id,:pid,:hash,:at)"
            ), {"id": token_id, "pid": project_id, "hash": hashlib.sha256(token.encode()).hexdigest(),
                "at": datetime.now(UTC).isoformat()})
        return {"id": token_id, "token": token}

    def revoke_token(self, project_id: str, token_id: str) -> None:
        with self.db.engine.begin() as conn:
            conn.execute(text(
                "UPDATE serving_read_tokens SET revoked_at=:at WHERE project_id=:pid AND id=:id"
            ), {"pid": project_id, "id": token_id, "at": datetime.now(UTC).isoformat()})

    def authorize(self, project_id: str, authorization: str | None) -> None:
        if self.mode(project_id) == "public":
            return
        scheme, _, token = (authorization or "").partition(" ")
        if scheme.lower() != "bearer" or not token or len(token) > 256:
            raise EngineError.unauthenticated("a project read token is required")
        with self.db.engine.connect() as conn:
            found = conn.execute(text(
                "SELECT id FROM serving_read_tokens WHERE project_id=:pid AND token_hash=:hash AND revoked_at IS NULL"
            ), {"pid": project_id, "hash": hashlib.sha256(token.encode()).hexdigest()}).scalar()
        if found is None:
            raise EngineError.unauthenticated("invalid or revoked project read token")
