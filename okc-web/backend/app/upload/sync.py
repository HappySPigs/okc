"""Authenticated, resumable CBOR receiver for the okc-hooks wire protocol."""

from __future__ import annotations

import hashlib
import io
import json
import os
import shutil
import tempfile
from threading import RLock
from typing import Any

import cbor2
from fastapi import APIRouter, Depends, FastAPI, Request, Response
from sqlalchemy import text
from starlette.concurrency import run_in_threadpool
from ulid import ULID

from app.adapter.engine import OkcEngineImpl
from app.adapter.queue import EngineWorker
from app.config import AppConfig
from app.shared.authz import UploadContext, upload_auth
from app.shared.error import EngineError, EngineErrorCategory, EngineErrorCode
from app.shared.state import StateDb
from app.upload.revisions import (
    Entry,
    commit_revision,
    current_source,
    directory_manifest,
    manifest_digest,
    portable_path_key,
    revision_conflict,
    safe_path,
    source_identity,
)
from app.upload.tokens import _now

MAX_CHUNK_BYTES = 8 * 1024 * 1024
MAX_CBOR_BYTES = 17 * 1024 * 1024
MAX_FILES = 100_000
MAX_PENDING = 16
SESSION_HEADER = "X-OKC-Upload-Session"


def _limit(name: str, default: int) -> int:
    value = os.environ.get(name, "")
    return int(value) if value.isdigit() and int(value) > 0 else default


def _digest(value: Any) -> str:
    if not isinstance(value, list) or len(value) != 32 or any(type(b) is not int or not 0 <= b <= 255 for b in value):
        raise EngineError.validation("digest must be an array of 32 bytes")
    return bytes(value).hex()


def _uint(value: Any) -> int:
    if type(value) is not int or not 0 <= value < 2**64:
        raise EngineError.validation("expected an unsigned 64-bit integer")
    return int(value)


async def _body(request: Request) -> dict[str, Any]:
    if request.headers.get("content-type", "").split(";")[0].strip() != "application/cbor":
        raise EngineError.validation("Content-Type must be application/cbor")
    raw = bytearray()
    async for chunk in request.stream():
        if len(raw) + len(chunk) > MAX_CBOR_BYTES:
            raise EngineError(EngineErrorCode.UPLOAD_TOO_LARGE, EngineErrorCategory.LIMIT,
                              "CBOR request exceeds the bounded transfer size")
        raw.extend(chunk)
    try:
        stream = io.BytesIO(raw)
        decoded = cbor2.CBORDecoder(stream).decode()
        if stream.read(1) or not isinstance(decoded, dict) or any(not isinstance(k, str) for k in decoded):
            raise ValueError("expected one CBOR object")
        return decoded
    except (ValueError, TypeError, cbor2.CBORDecodeError, RecursionError) as exc:
        raise EngineError.validation("malformed CBOR request") from exc


def _response(body: dict[str, Any]) -> Response:
    return Response(cbor2.dumps(body), media_type="application/cbor")


def bearer_upload_auth(request: Request) -> UploadContext:
    scheme, _, token = request.headers.get("authorization", "").partition(" ")
    if scheme.lower() != "bearer" or not token.strip():
        raise EngineError.unauthenticated("an upload Bearer token is required")
    try:
        return upload_auth(request, token.strip())
    except EngineError as exc:
        if exc.code in {EngineErrorCode.TOKEN_INVALID, EngineErrorCode.TOKEN_EXPIRED, EngineErrorCode.TOKEN_REVOKED}:
            raise EngineError.unauthenticated("invalid, expired, or revoked upload capability") from exc
        raise


class SyncService:
    def __init__(self, db: StateDb, config: AppConfig, engine: EngineWorker | None) -> None:
        self.db, self.config, self.engine = db, config, engine
        self._lock = RLock()

    def _session(self, ctx: UploadContext, session_id: str | None) -> dict[str, Any]:
        with self.db.engine.connect() as conn:
            row = conn.execute(text("SELECT * FROM upload_sync_sessions WHERE id=:id AND token_id=:token"),
                               {"id": session_id, "token": ctx.token_id}).mappings().first()
        if row is None:
            raise EngineError.not_found("upload session not found")
        return dict(row)

    def _root(self, session: dict[str, Any]) -> str:
        return os.path.join(self.config.projects_root, session["project_id"], "sync", session["source_id"])

    def _blob(self, session: dict[str, Any], raw_hash: str, partial: bool = False) -> str:
        root = self._root(session)
        if partial:
            return os.path.join(root, "transfers", session["id"], raw_hash)
        return os.path.join(root, "blobs", raw_hash)

    @staticmethod
    def _entries(session: dict[str, Any]) -> list[Entry]:
        return [tuple(entry) for entry in json.loads(session["entries_json"])]  # type: ignore[misc]

    @staticmethod
    def _matches(path: str, raw_hash: str, size: int) -> bool:
        if os.path.islink(path) or not os.path.isfile(path) or os.path.getsize(path) != size:
            return False
        with open(path, "rb") as handle:
            return hashlib.file_digest(handle, "sha256").hexdigest() == raw_hash

    def _parse_entries(self, body: dict[str, Any]) -> tuple[str, list[Entry]]:
        digest = _digest(body.get("manifest_digest"))
        rows = body.get("entries")
        if not isinstance(rows, list) or len(rows) > MAX_FILES:
            raise EngineError.validation("manifest exceeds the file-count limit")
        entries: list[Entry] = []
        seen: set[str] = set()
        sizes: dict[str, int] = {}
        total = 0
        for row in rows:
            if not isinstance(row, list) or len(row) != 3:
                raise EngineError.validation("manifest entries must be path/hash/size triples")
            path, raw_hash, size = safe_path(row[0]), _digest(row[1]), _uint(row[2])
            # Portable snapshots cannot contain aliases on case-insensitive hosts.
            key = portable_path_key(path)
            if key in seen or (raw_hash in sizes and sizes[raw_hash] != size):
                raise EngineError.validation("duplicate path or inconsistent blob size")
            seen.add(key)
            sizes[raw_hash] = size
            total += size
            entries.append((path, raw_hash, size))
        for path in seen:
            parts = path.split("/")
            if any("/".join(parts[:i]) in seen for i in range(1, len(parts))):
                raise EngineError.validation("file paths overlap directory paths")
        if total > _limit("OKC_WEB_MAX_SYNC_BYTES", 128 * 1024 * 1024):
            raise EngineError(EngineErrorCode.UPLOAD_TOO_LARGE, EngineErrorCategory.LIMIT,
                              "snapshot exceeds OKC_WEB_MAX_SYNC_BYTES")
        if manifest_digest(entries) != digest:
            raise EngineError.validation("manifest digest does not match its entries")
        return digest, sorted(entries)

    def negotiate(self, ctx: UploadContext, body: dict[str, Any]) -> dict[str, Any]:
        digest, entries = self._parse_entries(body)
        with self._lock:
            source_id = source_identity(self.db, ctx)
            current = current_source(self.db, source_id)
            base = current["content_hash"] if current else None
            with self.db.engine.begin() as conn:
                row = conn.execute(text("SELECT * FROM upload_sync_sessions WHERE token_id=:token"
                                        " AND manifest_digest=:digest AND (base_revision IS :base"
                                        " OR (status='committed' AND manifest_digest IS :base))"
                                        " ORDER BY created_at DESC LIMIT 1"),
                                   {"token": ctx.token_id, "digest": digest, "base": base}).mappings().first()
                if row is None:
                    # Superseded pending sessions remain replay-safe but no longer count
                    # against the current-base concurrency bound.
                    pending = conn.execute(text("SELECT COUNT(*) FROM upload_sync_sessions WHERE token_id=:token"
                                                " AND status='pending' AND base_revision IS :base"),
                                           {"token": ctx.token_id, "base": base}).scalar_one()
                    if pending >= MAX_PENDING:
                        raise EngineError(EngineErrorCode.RESOURCE_LIMIT, EngineErrorCategory.LIMIT,
                                          "too many pending upload sessions for this revision")
                    session_id = f"sync_{ULID()}"
                    conn.execute(text("INSERT INTO upload_sync_sessions(id,token_id,project_id,source_id,"
                                      "base_revision,manifest_digest,entries_json,created_at)"
                                      " VALUES(:id,:token,:pid,:sid,:base,:digest,:entries,:at)"),
                                 {"id": session_id, "token": ctx.token_id, "pid": ctx.project_id,
                                  "sid": source_id, "base": base, "digest": digest,
                                  "entries": json.dumps(entries, ensure_ascii=False), "at": _now()})
                else:
                    session_id = str(row["id"])
            session = self._session(ctx, session_id)
            server_has: list[list[int]] = []
            offsets: list[list[Any]] = []
            for raw_hash, size in sorted({(h, size) for _, h, size in entries}):
                complete = self._blob(session, raw_hash)
                partial = self._blob(session, raw_hash, partial=True)
                if self._matches(complete, raw_hash, size):
                    server_has.append(list(bytes.fromhex(raw_hash)))
                else:
                    offset = os.path.getsize(partial) if os.path.isfile(partial) else 0
                    if offset > size:
                        raise EngineError.validation("stored blob exceeds its negotiated size")
                    if os.path.isfile(partial) and offset == size:
                        # Recovery after chunk fsync but before cache publication:
                        # never advertise size as resumable unless the complete
                        # bytes have been verified and are available to commit.
                        if self._matches(partial, raw_hash, size):
                            os.makedirs(os.path.dirname(complete), exist_ok=True)
                            os.replace(partial, complete)
                            server_has.append(list(bytes.fromhex(raw_hash)))
                            continue
                        with open(partial, "wb") as handle:
                            handle.flush()
                            os.fsync(handle.fileno())
                        offset = 0
                    offsets.append([list(bytes.fromhex(raw_hash)), offset])
            return {"server_has": server_has, "session_id": session_id, "resume_offsets": offsets}

    def chunk(self, ctx: UploadContext, session_id: str | None, raw_hash: str,
              offset: int, body: dict[str, Any]) -> dict[str, Any]:
        if len(raw_hash) != 64 or any(c not in "0123456789abcdef" for c in raw_hash):
            raise EngineError.validation("invalid blob hash")
        offset = _uint(offset)
        data = body.get("bytes")
        if not isinstance(data, list) or len(data) > MAX_CHUNK_BYTES or any(type(b) is not int or not 0 <= b <= 255 for b in data):
            raise EngineError.validation("invalid or oversized chunk bytes")
        payload = bytes(data)
        if (_digest(body.get("blob")) != raw_hash or _uint(body.get("offset")) != offset
                or _uint(body.get("len")) != len(payload)
                or _digest(body.get("chunk_sha256")) != hashlib.sha256(payload).hexdigest()):
            raise EngineError.validation("chunk identity, length, or checksum mismatch")
        with self._lock:
            session = self._session(ctx, session_id)
            sizes = {h: size for _, h, size in self._entries(session)}
            if raw_hash not in sizes or offset + len(payload) > sizes[raw_hash]:
                raise EngineError.validation("chunk is outside the negotiated manifest")
            complete = self._blob(session, raw_hash)
            if self._matches(complete, raw_hash, sizes[raw_hash]):
                with open(complete, "rb") as handle:
                    handle.seek(offset)
                    if handle.read(len(payload)) != payload:
                        raise EngineError.validation("chunk replay differs from committed bytes")
                return {"received": sizes[raw_hash]}
            partial = self._blob(session, raw_hash, partial=True)
            os.makedirs(os.path.dirname(partial), exist_ok=True)
            size = os.path.getsize(partial) if os.path.isfile(partial) else 0
            if offset < size:
                with open(partial, "rb") as handle:
                    handle.seek(offset)
                    if offset + len(payload) > size or handle.read(len(payload)) != payload:
                        raise revision_conflict()
            elif offset == size:
                with open(partial, "ab") as handle:
                    handle.write(payload)
                    handle.flush()
                    os.fsync(handle.fileno())
                size += len(payload)
            else:
                raise revision_conflict()
            if size == sizes[raw_hash]:
                if not self._matches(partial, raw_hash, size):
                    # The uncommitted corrupt transfer can be restarted at offset 0.
                    with open(partial, "wb"):
                        pass
                    raise EngineError.validation("assembled blob checksum mismatch")
                os.makedirs(os.path.dirname(complete), exist_ok=True)
                os.replace(partial, complete)
            return {"received": size}

    async def commit(self, ctx: UploadContext, session_id: str | None,
                     body: dict[str, Any]) -> dict[str, Any]:
        digest = _digest(body.get("manifest_digest"))
        path_map = body.get("path_hash_map")
        if not isinstance(path_map, dict) or not isinstance(path_map.get("entries"), dict):
            raise EngineError.validation("invalid commit path/hash map")
        entries = {safe_path(path): _digest(value) for path, value in path_map["entries"].items()}
        if self.engine is None:
            raise EngineError.internal("engine worker unavailable")

        def run(engine: OkcEngineImpl) -> dict[str, Any]:
            with self._lock:
                session = self._session(ctx, session_id)
                expected = self._entries(session)
                if digest != session["manifest_digest"] or entries != {path: h for path, h, _ in expected}:
                    raise EngineError.validation("commit differs from the negotiated manifest")
                receipt = {"server_vault_content_id": digest, "committed": True}
                if session["status"] == "committed":
                    return receipt
                current = current_source(self.db, session["source_id"])
                if session["base_revision"] != (current["content_hash"] if current else None):
                    raise revision_conflict()
                for _, raw_hash, size in expected:
                    if not self._matches(self._blob(session, raw_hash), raw_hash, size):
                        raise EngineError.validation("manifest has missing or corrupt blobs")
                landed = os.path.abspath(os.path.join(self._root(session), "revisions", digest))
                if not os.path.exists(landed):
                    os.makedirs(os.path.dirname(landed), exist_ok=True)
                    stage = tempfile.mkdtemp(prefix=".revision-", dir=os.path.dirname(landed))
                    try:
                        # Core recognizes an empty Obsidian vault by this empty
                        # metadata directory. No extra file enters the manifest.
                        os.makedirs(os.path.join(stage, ".obsidian"), exist_ok=True)
                        for path, raw_hash, _ in expected:
                            target = os.path.join(stage, path)
                            os.makedirs(os.path.dirname(target), exist_ok=True)
                            with open(self._blob(session, raw_hash), "rb") as src, open(target, "xb") as dst:
                                shutil.copyfileobj(src, dst)
                                dst.flush()
                                os.fsync(dst.fileno())
                        os.rename(stage, landed)
                    finally:
                        if os.path.isdir(stage):
                            shutil.rmtree(stage)
                if directory_manifest(landed)[0] != digest:
                    raise EngineError.validation("landed source revision checksum mismatch")
                with self.db.engine.connect() as conn:
                    root = conn.execute(text("SELECT engine_root_abs_path FROM projects WHERE id=:id"),
                                        {"id": ctx.project_id}).scalar_one()
                commit_revision(self.db, engine, ctx, source_id=session["source_id"], root=root,
                                landed=landed, content_hash=digest, slot_index=ctx.slot_index,
                                owner_display_name=ctx.owner_display_name, owner_kind=ctx.owner_kind,
                                session_id=session["id"])
                return receipt

        return await self.engine.call(run)


def register_sync(app: FastAPI, db: StateDb, config: AppConfig, engine: EngineWorker | None) -> None:
    service = SyncService(db, config, engine)
    router = APIRouter(prefix="/api/sync")

    @router.post("/negotiate")
    async def negotiate(request: Request, ctx: UploadContext = Depends(bearer_upload_auth)) -> Response:
        body = await _body(request)
        return _response(await run_in_threadpool(service.negotiate, ctx, body))

    @router.put("/blob/{raw_hash}/{offset}")
    async def chunk(raw_hash: str, offset: int, request: Request,
                    ctx: UploadContext = Depends(bearer_upload_auth)) -> Response:
        body = await _body(request)
        return _response(await run_in_threadpool(service.chunk, ctx, request.headers.get(SESSION_HEADER),
                                                 raw_hash, offset, body))

    @router.post("/commit")
    async def commit(request: Request, ctx: UploadContext = Depends(bearer_upload_auth)) -> Response:
        return _response(await service.commit(ctx, request.headers.get(SESSION_HEADER), await _body(request)))

    app.include_router(router)
