"""``upload.ingest`` — U2 multipart snapshot ingestion (E2-S3..S6).

Capability-authenticated, ordered, fail-fast pipeline. Every step BEFORE
the source binding guarantee okc-core is untouched on rejection (C-1). Native
add/rebind runs on the U0 single writer. Each capability owns one stable source;
new immutable revisions update that source after computing canonical file hashes.
The revision committer is shared with the hooks CBOR receiver.

Supported formats: ``.zip`` (story-canonical vault archive, full hostile-input
defenses) and a single ``.md`` file (one-note vault). ``tar.zst`` is recognized
but rejected (the ``zstandard`` codec is not available in this build).
"""

from __future__ import annotations

import hashlib
import os
import shutil
import tempfile
import zipfile
from dataclasses import dataclass
from threading import Lock

from fastapi import Request, UploadFile
from sqlalchemy import text
from ulid import ULID

from app.adapter.engine import OkcEngineImpl
from app.adapter.queue import EngineWorker
from app.config import AppConfig
from app.shared.authz import UploadContext
from app.shared.error import EngineError, EngineErrorCategory, EngineErrorCode
from app.shared.jobs import JobId
from app.shared.state import StateDb
from app.upload.models import SOURCE_CAP, CheckResult, IngestAccepted, ValidationReport
from app.upload.revisions import (
    commit_revision,
    current_source,
    directory_manifest,
    portable_path_key,
    safe_path,
    source_identity,
)
from app.upload.tokens import UploadTokenStore, _now

_CHUNK = 64 * 1024
_ZIP_MAGIC = b"PK\x03\x04"
_ZSTD_MAGIC = b"\x28\xb5\x2f\xfd"
_COMPRESSION_RATIO_CAP = 200  # per-entry declared:compressed guard (zip-bomb)
_OWNER_KINDS = {"department", "individual"}


def _env_int(name: str, default: int) -> int:
    raw = os.environ.get(name)
    if raw and raw.isdigit():
        return int(raw)
    return default


def _max_upload_bytes() -> int:
    return _env_int("OKC_WEB_MAX_UPLOAD_BYTES", 64 * 1024 * 1024)


def _max_extracted_bytes() -> int:
    return _env_int("OKC_WEB_MAX_EXTRACTED_BYTES", 128 * 1024 * 1024)


def _too_large(detail: str) -> EngineError:
    return EngineError(EngineErrorCode.UPLOAD_TOO_LARGE, EngineErrorCategory.VALIDATION, detail)


def _path_unsafe(detail: str) -> EngineError:
    return EngineError(EngineErrorCode.PATH_UNSAFE, EngineErrorCategory.VALIDATION, detail)


@dataclass
class StagedUpload:
    temp_path: str
    content_hash: str
    size: int
    filename: str
    kind: str  # 'zip' | 'markdown' | 'tar_zst' | 'unknown'


@dataclass
class SourceRow:
    source_id: str
    project_id: str
    owner_display_name: str | None
    owner_kind: str | None
    content_hash: str
    absolute_path: str
    slot_index: int
    upload_token_id: str | None
    document_id: str | None = None


@dataclass
class SlotReservation:
    project_id: str
    slot_index: int
    source_id: str | None = None


class SourceRegistry:
    """Write-side of the U3-owned ``sources`` table (frozen seam). U2 writes at
    ``add_source`` commit; U5 reads for owner-label provenance."""

    def __init__(self, db: StateDb) -> None:
        self._db = db

    def record(self, row: SourceRow) -> None:
        with self._db.engine.begin() as conn:
            conn.execute(
                text(
                    "INSERT INTO sources(source_id, project_id, document_id, owner_display_name,"
                    " owner_kind, content_hash, absolute_path, slot_index, upload_token_id, registered_at)"
                    " VALUES (:sid,:pid,:doc,:odn,:ok,:ch,:path,:slot,:tok,:at)"
                ),
                {
                    "sid": row.source_id, "pid": row.project_id, "doc": row.document_id,
                    "odn": row.owner_display_name, "ok": row.owner_kind, "ch": row.content_hash,
                    "path": row.absolute_path, "slot": row.slot_index, "tok": row.upload_token_id,
                    "at": _now(),
                },
            )

    def count(self, project_id: str) -> int:
        with self._db.engine.connect() as conn:
            return int(
                conn.execute(
                    text("SELECT COUNT(*) FROM sources WHERE project_id=:pid"),
                    {"pid": project_id},
                ).scalar()
                or 0
            )

    def exists_content(self, project_id: str, content_hash: str) -> bool:
        with self._db.engine.connect() as conn:
            row = conn.execute(
                text("SELECT 1 FROM sources WHERE project_id=:pid AND content_hash=:ch LIMIT 1"),
                {"pid": project_id, "ch": content_hash},
            ).first()
            return row is not None

    def used_slot_indices(self, project_id: str) -> set[int]:
        with self._db.engine.connect() as conn:
            rows = conn.execute(
                text("SELECT slot_index FROM sources WHERE project_id=:pid"),
                {"pid": project_id},
            )
            return {int(row[0]) for row in rows}


class SlotAccountant:
    """Owns the ≤10 source cap at the okc-web layer, ahead of core (E2-S5)."""

    def __init__(self, registry: SourceRegistry) -> None:
        self._registry = registry
        self._guard = Lock()
        self._reserved: set[tuple[str, int]] = set()
        self._pending_sources: dict[str, tuple[SlotReservation, int]] = {}

    def reserve(self, project_id: str, source_id: str | None = None,
                existing_slot: int | None = None) -> SlotReservation:
        with self._guard:
            if source_id and source_id in self._pending_sources:
                reservation, count = self._pending_sources[source_id]
                self._pending_sources[source_id] = (reservation, count + 1)
                return reservation
            if existing_slot is not None:
                return SlotReservation(project_id, existing_slot, source_id)
            if self._registry.count(project_id) >= SOURCE_CAP:
                raise self._cap_error()
            used = self._registry.used_slot_indices(project_id)
            for slot_index in range(SOURCE_CAP):
                key = (project_id, slot_index)
                if slot_index not in used and key not in self._reserved:
                    self._reserved.add(key)
                    reservation = SlotReservation(project_id, slot_index, source_id)
                    if source_id:
                        self._pending_sources[source_id] = (reservation, 1)
                    return reservation
        raise self._cap_error()

    def commit(self, reservation: SlotReservation, row: SourceRow) -> None:
        self._registry.record(row)
        self.release(reservation)

    def release(self, reservation: SlotReservation) -> None:
        with self._guard:
            if reservation.source_id in self._pending_sources:
                _, count = self._pending_sources[reservation.source_id]
                if count > 1:
                    self._pending_sources[reservation.source_id] = (reservation, count - 1)
                    return
                del self._pending_sources[reservation.source_id]
            self._reserved.discard((reservation.project_id, reservation.slot_index))

    @staticmethod
    def _cap_error() -> EngineError:
        return EngineError(
            EngineErrorCode.SOURCE_CAP_EXCEEDED,
            EngineErrorCategory.LIMIT,
            f"source cap ({SOURCE_CAP}) reached for project",
        )


class UploadReceiver:
    """Streams the multipart file to a temp staging path with a hard byte cap,
    computing the content hash incrementally (no whole-file buffering)."""

    async def receive(self, file: UploadFile, request: Request) -> StagedUpload:
        cap = _max_upload_bytes()
        declared = request.headers.get("content-length")
        if declared and declared.isdigit() and int(declared) > cap + 4096:
            raise _too_large(f"upload exceeds byte cap ({cap})")
        digest = hashlib.sha256()
        size = 0
        fd, tmp = tempfile.mkstemp(prefix="okcweb-upload-")
        try:
            with os.fdopen(fd, "wb") as out:
                while True:
                    chunk = await file.read(_CHUNK)
                    if not chunk:
                        break
                    size += len(chunk)
                    if size > cap:
                        raise _too_large(f"upload exceeds byte cap ({cap})")
                    digest.update(chunk)
                    out.write(chunk)
        except BaseException:
            _safe_unlink(tmp)
            raise
        return StagedUpload(
            temp_path=tmp, content_hash=digest.hexdigest(), size=size,
            filename=file.filename or "", kind=_detect_kind(tmp, file.filename or ""),
        )


class ArchiveValidator:
    """Inspects the staged artifact BEFORE landing (FR-UP-3). Blocking failures
    (format / traversal / symlink / zip-bomb) raise; non-blocking findings
    (non-markdown content) are returned as warnings."""

    def inspect(self, staged: StagedUpload) -> ValidationReport:
        if staged.kind == "markdown":
            return ValidationReport(ok=True, markdown_files=1, total_files=1)
        if staged.kind == "tar_zst":
            raise EngineError(
                EngineErrorCode.VALIDATION_FAILED,
                EngineErrorCategory.VALIDATION,
                "tar.zst is not supported in this build (zstandard codec unavailable)",
            )
        if staged.kind != "zip":
            raise EngineError(
                EngineErrorCode.VALIDATION_FAILED,
                EngineErrorCategory.VALIDATION,
                "unsupported upload format (expected .zip or .md)",
            )
        return self._inspect_zip(staged)

    def _inspect_zip(self, staged: StagedUpload) -> ValidationReport:
        cap = _max_extracted_bytes()
        total_uncompressed = 0
        total_files = 0
        markdown_files = 0
        file_keys: set[str] = set()
        directory_keys: set[str] = set()
        try:
            with zipfile.ZipFile(staged.temp_path) as zf:
                for info in zf.infolist():
                    name = info.filename
                    if info.is_dir():
                        _assert_safe_relpath(name)
                        directory_keys.add(portable_path_key(name.rstrip("/")))
                        continue
                    _assert_safe_relpath(name)
                    key = portable_path_key(safe_path(name))
                    if key in file_keys:
                        raise _path_unsafe("duplicate or platform-equivalent archive file paths")
                    file_keys.add(key)
                    if _is_symlink(info):
                        raise _path_unsafe(f"symlink entry rejected: {name}")
                    total_files += 1
                    total_uncompressed += info.file_size
                    if total_uncompressed > cap:
                        raise _too_large(f"extracted size exceeds cap ({cap}) — possible zip bomb")
                    if (
                        info.compress_size > 0
                        and info.file_size > 1024 * 1024
                        and info.file_size / info.compress_size > _COMPRESSION_RATIO_CAP
                    ):
                        raise _too_large(f"compression ratio too high for {name} — possible zip bomb")
                    if name.lower().endswith(".md"):
                        markdown_files += 1
                if file_keys & directory_keys:
                    raise _path_unsafe("archive file path overlaps a directory")
                for key in file_keys | directory_keys:
                    parts = key.split("/")
                    if any("/".join(parts[:i]) in file_keys for i in range(1, len(parts))):
                        raise _path_unsafe("archive file path overlaps a directory ancestor")
        except zipfile.BadZipFile as exc:
            raise EngineError(
                EngineErrorCode.VALIDATION_FAILED,
                EngineErrorCategory.VALIDATION,
                "corrupt or invalid zip archive",
            ) from exc

        report = ValidationReport(ok=True, markdown_files=markdown_files, total_files=total_files)
        if total_files == 0:
            raise EngineError(
                EngineErrorCode.VALIDATION_FAILED, EngineErrorCategory.VALIDATION, "empty archive"
            )
        if markdown_files == 0:
            report.warnings.append(
                CheckResult(
                    name="markdown_content", status="warn", code="CONTENT_NOT_MARKDOWN",
                    detail="no .md files — non-markdown content is excluded from the merged output",
                )
            )
        return report


class SourceLander:
    """Materializes validated bytes to a deterministic ABSOLUTE directory under
    the project sources root (C-6: disk THEN register). The landing path is
    built from server-generated ids only — no user input, no traversal."""

    def __init__(self, config: AppConfig) -> None:
        self._config = config

    def land(self, project_id: str, source_id: str, staged: StagedUpload) -> str:
        landed = os.path.abspath(
            os.path.join(self._config.projects_root, project_id, "sources", source_id)
        )
        try:
            os.makedirs(landed, exist_ok=True)
            if staged.kind == "markdown":
                name = _safe_markdown_name(staged.filename)
                with open(staged.temp_path, "rb") as src, open(
                    os.path.join(landed, name), "wb"
                ) as dst:
                    _copy(src, dst)
            else:  # zip (already validated safe)
                self._extract_zip(staged.temp_path, landed)
        except BaseException:
            _safe_rmtree(landed)
            raise
        return landed

    def _extract_zip(self, zip_path: str, landed: str) -> None:
        with zipfile.ZipFile(zip_path) as zf:
            for info in zf.infolist():
                if info.is_dir():
                    continue
                _assert_safe_relpath(info.filename)  # defense in depth
                target = os.path.join(landed, info.filename)
                real_target = os.path.realpath(target)
                if os.path.commonpath([real_target, os.path.realpath(landed)]) != os.path.realpath(landed):
                    raise _path_unsafe(f"entry escapes landing root: {info.filename}")
                os.makedirs(os.path.dirname(target), exist_ok=True)
                with zf.open(info) as src, open(target, "wb") as dst:
                    _copy(src, dst)


class UploadIngestService:
    """Orchestrates the ordered fail-fast pipeline (E2-S3..S6)."""

    def __init__(
        self,
        *,
        db: StateDb,
        engine: EngineWorker | None,
        config: AppConfig,
        tokens: UploadTokenStore,
        registry: SourceRegistry,
        slots: SlotAccountant,
        receiver: UploadReceiver,
        validator: ArchiveValidator,
        lander: SourceLander,
    ) -> None:
        self._db = db
        self._engine = engine
        self._config = config
        self._tokens = tokens
        self._registry = registry
        self._slots = slots
        self._receiver = receiver
        self._validator = validator
        self._lander = lander

    async def ingest(
        self,
        ctx: UploadContext,
        file: UploadFile,
        owner_display_name: str | None,
        owner_kind: str | None,
        request: Request,
    ) -> IngestAccepted:
        if self._engine is None:
            raise EngineError.internal("engine worker unavailable")
        root = self._project_root(ctx.project_id)  # 404 fast if project row missing
        owner_label = owner_display_name if owner_display_name is not None else ctx.owner_display_name
        okind = _validate_owner_kind(owner_kind if owner_kind is not None else ctx.owner_kind)

        # 2. atomically reserve an in-flight slot (source ≤10) — fast 429.
        source_id = source_identity(self._db, ctx)
        current = current_source(self._db, source_id)
        reservation = self._slots.reserve(ctx.project_id, source_id,
                                          current["slot_index"] if current else None)
        staged: StagedUpload | None = None
        landed: str | None = None
        try:
            # 3. receive: stream to temp with a hard byte cap + incremental hash.
            staged = await self._receiver.receive(file, request)
            # 4b. hostile-input validation (blocking failures raise here).
            report = self._validator.inspect(staged)

            # 5. land to an absolute directory under the sources root (disk THEN register).
            landed = self._lander.land(ctx.project_id, f"{source_id}/revisions/{ULID()}", staged)
            staged.content_hash, _entries = directory_manifest(landed)
            if self._registry.exists_content(ctx.project_id, staged.content_hash):
                raise EngineError(EngineErrorCode.DUPLICATE_SOURCE, EngineErrorCategory.VALIDATION,
                                  "identical vault content is already registered for this project")
            # 6+7. add_source on the single-writer worker; commit/record/mark_used AFTER.
            job_id = self._enqueue_add_source(
                ctx=ctx, root=root, source_id=source_id, landed=landed,
                owner_display_name=owner_label, owner_kind=okind,
                content_hash=staged.content_hash, reservation=reservation,
            )
        except BaseException:
            self._slots.release(reservation)
            if landed is not None:
                _safe_rmtree(landed)
            raise
        finally:
            if staged is not None:
                _safe_unlink(staged.temp_path)
        return IngestAccepted(
            job_id=job_id, source_id=source_id, project_id=ctx.project_id,
            slot_index=reservation.slot_index, content_hash=staged.content_hash,
            owner_display_name=owner_label, warnings=report.warnings,
        )

    def _enqueue_add_source(
        self,
        *,
        ctx: UploadContext,
        root: str,
        source_id: str,
        landed: str,
        owner_display_name: str | None,
        owner_kind: str | None,
        content_hash: str,
        reservation: SlotReservation,
    ) -> JobId:
        assert self._engine is not None  # guarded by caller

        def run(engine: OkcEngineImpl, _progress) -> None:  # noqa: ANN001 - progress sink typed in queue
            try:
                if self._registry.exists_content(ctx.project_id, content_hash):
                    raise EngineError(EngineErrorCode.DUPLICATE_SOURCE, EngineErrorCategory.VALIDATION,
                                      "identical vault content is already registered for this project")
                commit_revision(
                    self._db, engine, ctx, source_id=source_id, root=root, landed=landed,
                    content_hash=content_hash, slot_index=reservation.slot_index,
                    owner_display_name=owner_display_name, owner_kind=owner_kind,
                )
            finally:
                # Immutable revisions are retained even on uncertain core/DB outcomes.
                self._slots.release(reservation)

        return self._engine.enqueue("add_source", ctx.project_id, ctx.token_id, run)

    def _project_root(self, project_id: str) -> str:
        with self._db.engine.connect() as conn:
            row = conn.execute(
                text("SELECT engine_root_abs_path FROM projects WHERE id=:id"),
                {"id": project_id},
            ).mappings().first()
        if row is None:
            raise EngineError.not_found("project not found")
        return str(row["engine_root_abs_path"])


# --- module helpers ---


def _detect_kind(temp_path: str, filename: str) -> str:
    with open(temp_path, "rb") as f:
        head = f.read(8)
    if head.startswith(_ZIP_MAGIC):
        return "zip"
    if head.startswith(_ZSTD_MAGIC):
        return "tar_zst"
    lower = filename.lower()
    if lower.endswith(".md") or lower.endswith(".markdown"):
        return "markdown"
    if lower.endswith(".zip"):
        return "zip"
    if lower.endswith(".tar.zst") or lower.endswith(".tzst"):
        return "tar_zst"
    return "unknown"


def _assert_safe_relpath(name: str) -> None:
    if not name:
        raise _path_unsafe("empty archive entry name")
    if name.startswith("/") or name.startswith("\\") or (len(name) > 1 and name[1] == ":"):
        raise _path_unsafe(f"absolute path entry rejected: {name}")
    normalized = os.path.normpath(name)
    if normalized.startswith("..") or f"{os.sep}.." in normalized or normalized == "..":
        raise _path_unsafe(f"path traversal entry rejected: {name}")


def _is_symlink(info: zipfile.ZipInfo) -> bool:
    mode = info.external_attr >> 16
    return (mode & 0o170000) == 0o120000


def _safe_markdown_name(filename: str) -> str:
    base = os.path.basename(filename) or "note.md"
    if not base.lower().endswith((".md", ".markdown")):
        base = f"{base}.md"
    return base


def _copy(src, dst) -> None:  # noqa: ANN001 - binary file objects
    while True:
        chunk = src.read(_CHUNK)
        if not chunk:
            break
        dst.write(chunk)


def _safe_unlink(path: str) -> None:
    try:
        if path and os.path.exists(path):
            os.remove(path)
    except OSError:
        pass


def _safe_rmtree(path: str) -> None:
    try:
        if path and os.path.isdir(path):
            shutil.rmtree(path)
    except OSError:
        pass


def _validate_owner_kind(value: str | None) -> str | None:
    if value is not None and value not in _OWNER_KINDS:
        raise EngineError.validation(f"owner_kind must be one of {sorted(_OWNER_KINDS)}")
    return value
