"""Shared immutable source-revision commit for multipart and hooks uploads.

Call ``commit_revision`` only on the engine's single writer. Core source binding
is replayable; the control-plane row, history, and receipt finalize in one DB
transaction. Keeping landed bytes makes a retry safe after DB finalization fails.
"""

from __future__ import annotations

import hashlib
import os
import unicodedata
from typing import Any

from sqlalchemy import text

from app.adapter import dto
from app.adapter.engine import OkcEngineImpl
from app.shared.authz import UploadContext
from app.shared.error import EngineError, EngineErrorCategory, EngineErrorCode
from app.shared.state import StateDb
from app.upload.models import SOURCE_CAP
from app.upload.tokens import _is_expired, _now

Entry = tuple[str, str, int]


def portable_path_key(path: str) -> str:
    """Collision-only key; the exact original path remains part of the manifest."""
    return unicodedata.normalize("NFC", path).casefold()


def safe_path(path: Any) -> str:
    if not isinstance(path, str) or not path or len(path.encode("utf-8")) > 4096:
        raise EngineError.validation("invalid relative file path")
    parts = path.split("/")
    if (path.casefold() == ".obsidian" or any(part in {"", ".", ".."} for part in parts)
            or any(c in path for c in ("\\", ":", "\x00"))
            or any(ord(c) < 32 for c in path)):
        raise EngineError(EngineErrorCode.PATH_UNSAFE, EngineErrorCategory.VALIDATION,
                          "unsafe relative file path")
    return path


def manifest_digest(entries: list[Entry]) -> str:
    digest = hashlib.sha256()
    for path, raw_hash, size in sorted(entries, key=lambda e: (e[0].encode(), e[1], e[2])):
        encoded = path.encode("utf-8")
        digest.update(len(encoded).to_bytes(8, "big"))
        digest.update(encoded)
        digest.update(bytes.fromhex(raw_hash))
        digest.update(size.to_bytes(8, "big"))
    return digest.hexdigest()


def directory_manifest(root: str) -> tuple[str, list[Entry]]:
    entries: list[Entry] = []
    for parent, dirs, names in os.walk(root):
        for name in dirs + names:
            if os.path.islink(os.path.join(parent, name)):
                raise EngineError.validation("source revisions cannot contain symlinks")
        for name in names:
            absolute = os.path.join(parent, name)
            path = safe_path(os.path.relpath(absolute, root).replace(os.sep, "/"))
            with open(absolute, "rb") as handle:
                raw_hash = hashlib.file_digest(handle, "sha256").hexdigest()
            size = os.path.getsize(absolute)
            entries.append((path, raw_hash, size))
    return manifest_digest(entries), entries


def source_identity(db: StateDb, ctx: UploadContext) -> str:
    with db.engine.connect() as conn:
        row = conn.execute(text("SELECT source_identity, registered_source_id FROM upload_tokens WHERE id=:id"),
                           {"id": ctx.token_id}).mappings().first()
    if row is None:
        raise EngineError.unauthenticated("upload capability no longer exists")
    return str(row["source_identity"] or row["registered_source_id"] or f"src_{ctx.token_id[4:]}")


def current_source(db: StateDb, source_id: str) -> dict[str, Any] | None:
    with db.engine.connect() as conn:
        row = conn.execute(text("SELECT * FROM sources WHERE source_id=:id"),
                           {"id": source_id}).mappings().first()
        return dict(row) if row else None


def revision_conflict() -> EngineError:
    return EngineError(EngineErrorCode.PROJECT_BUSY, EngineErrorCategory.CONCURRENCY,
                       "source revision changed; negotiate a new upload session", retryable=True)


def commit_revision(
    db: StateDb, engine: OkcEngineImpl, ctx: UploadContext, *, source_id: str,
    root: str, landed: str, content_hash: str, slot_index: int,
    owner_display_name: str | None, owner_kind: str | None,
    session_id: str | None = None,
) -> None:
    with db.engine.connect() as conn:
        token = conn.execute(text("SELECT * FROM upload_tokens WHERE id=:id"),
                             {"id": ctx.token_id}).mappings().first()
        if token is None or token["revoked_at"] or _is_expired(token["expires_at"]):
            raise EngineError.unauthenticated("upload capability expired or was revoked before commit")
        previous = conn.execute(text("SELECT * FROM sources WHERE source_id=:id"),
                                {"id": source_id}).mappings().first()
        if session_id:
            session = conn.execute(text("SELECT * FROM upload_sync_sessions WHERE id=:id AND token_id=:token"),
                                   {"id": session_id, "token": ctx.token_id}).mappings().first()
            if session is None:
                raise EngineError.not_found("upload session not found")
            if session["status"] == "committed":
                return
            if session["base_revision"] != (previous["content_hash"] if previous else None):
                raise revision_conflict()
        count = conn.execute(text("SELECT COUNT(*) FROM sources WHERE project_id=:id"),
                             {"id": ctx.project_id}).scalar_one()
        if previous is None and count >= SOURCE_CAP:
            raise EngineError(EngineErrorCode.SOURCE_CAP_EXCEEDED, EngineErrorCategory.LIMIT,
                              f"source cap ({SOURCE_CAP}) reached for project")
        duplicate = conn.execute(text("SELECT 1 FROM sources WHERE project_id=:pid AND content_hash=:hash"
                                      " AND source_id != :sid"),
                                 {"pid": ctx.project_id, "hash": content_hash, "sid": source_id}).first()
        if duplicate:
            raise EngineError(EngineErrorCode.DUPLICATE_SOURCE, EngineErrorCategory.VALIDATION,
                              "identical vault content already belongs to another source")

    # Fresh native manifest handles recovery after core success but DB failure.
    bound = next((s for s in engine.manifest(root)["sources"] if s["source_id"] == source_id), None)
    cmd = dto.AddSourceCmd(source_id=source_id, absolute_path=landed,
                           owner_display_name=owner_display_name, snapshot_id=content_hash)
    if bound is None:
        engine.add_source(root, cmd)
    elif bound["path"] != landed or bound.get("snapshot_id") != content_hash:
        engine.rebind_source(root, cmd)

    now = _now()
    with db.engine.begin() as conn:
        if previous:
            conn.execute(text("INSERT OR IGNORE INTO source_revisions VALUES(:id,:hash,:path,:at)"),
                         {"id": source_id, "hash": previous["content_hash"],
                          "path": previous["absolute_path"], "at": previous["registered_at"]})
        conn.execute(text("INSERT INTO sources(source_id, project_id, owner_display_name, owner_kind,"
                          " content_hash, absolute_path, slot_index, upload_token_id, registered_at)"
                          " VALUES(:id,:pid,:owner,:kind,:hash,:path,:slot,:token,:at)"
                          " ON CONFLICT(source_id) DO UPDATE SET content_hash=excluded.content_hash,"
                          " absolute_path=excluded.absolute_path, owner_display_name=excluded.owner_display_name,"
                          " owner_kind=excluded.owner_kind, upload_token_id=excluded.upload_token_id,"
                          " registered_at=excluded.registered_at"),
                     {"id": source_id, "pid": ctx.project_id, "owner": owner_display_name,
                      "kind": owner_kind, "hash": content_hash, "path": landed,
                      "slot": previous["slot_index"] if previous else slot_index,
                      "token": ctx.token_id, "at": now})
        conn.execute(text("INSERT OR IGNORE INTO source_revisions VALUES(:id,:hash,:path,:at)"),
                     {"id": source_id, "hash": content_hash, "path": landed, "at": now})
        conn.execute(text("UPDATE upload_tokens SET last_used_at=:at, registered_source_id=:sid"
                          " WHERE id=:id"), {"at": now, "sid": source_id, "id": ctx.token_id})
        if session_id:
            conn.execute(text("UPDATE upload_sync_sessions SET status='committed', landed_path=:path,"
                              " committed_at=:at WHERE id=:id"),
                         {"path": landed, "at": now, "id": session_id})
