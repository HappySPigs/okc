"""``upload.tokens`` — U2 UploadTokenService + token authentication seam.

Admin-facing token lifecycle (issue/list/revoke/rotate) is **DB-only, pre-core**
(never touches the engine). The token resolver is the authenticate seam U2 supplies
to U0 (``app.state.token_resolver``); U0's ``upload_auth`` authorizes with it.

Token model (§9): a split ``selector.verifier``. The ``selector`` is stored
plaintext (UNIQUE, O(1) lookup); the ``verifier`` is stored only as
``sha256(pepper + salt + verifier)`` with a per-row random salt and compared in
constant time. The plaintext token is revealed EXACTLY once at issue (NFR-SEC-1).
"""

from __future__ import annotations

import hashlib
import hmac
import os
import secrets
from datetime import UTC, datetime, timedelta
from threading import RLock
from typing import cast

from sqlalchemy import text
from ulid import ULID

from app.shared.authz import AuthContext, UploadContext
from app.shared.error import EngineError, EngineErrorCategory, EngineErrorCode
from app.shared.state import StateDb
from app.upload.models import (
    SOURCE_CAP,
    IssuedToken,
    IssueTokenRequest,
    SlotUsage,
    TokenListView,
    TokenSummary,
    UploadTargetView,
)

_OWNER_KINDS = {"department", "individual"}


def _now() -> str:
    return datetime.now(UTC).isoformat()


def _token_pepper() -> str:
    """Server pepper referenced by env-var NAME only (A-2). Empty by default."""
    return os.environ.get("OKC_WEB_TOKEN_PEPPER", "")


class UploadTokenSecret:
    """Split-token value object (CSPRNG selector + verifier)."""

    @staticmethod
    def generate() -> tuple[str, str]:
        # ~256-bit each; url-safe so it rides in a single path segment.
        return secrets.token_urlsafe(32), secrets.token_urlsafe(32)

    @staticmethod
    def verifier_hash(verifier: str, salt: str) -> str:
        digest = hashlib.sha256(f"{_token_pepper()}{salt}{verifier}".encode())
        return digest.hexdigest()

    @staticmethod
    def present(selector: str, verifier: str) -> str:
        return f"{selector}.{verifier}"

    @staticmethod
    def parse(raw: str) -> tuple[str, str] | None:
        """``selector.verifier`` -> parts; ``None`` on any malformed input."""
        if not raw or raw.count(".") != 1:
            return None
        selector, verifier = raw.split(".", 1)
        if not selector or not verifier:
            return None
        return selector, verifier

    @staticmethod
    def verify(verifier: str, salt: str | None, expected_hash: str) -> bool:
        computed = UploadTokenSecret.verifier_hash(verifier, salt or "")
        return hmac.compare_digest(computed, expected_hash)


def _is_expired(expires_at: str | None) -> bool:
    if not expires_at:
        return False
    try:
        return datetime.fromisoformat(expires_at) <= datetime.now(UTC)
    except ValueError:
        return False


def _derive_status(row: dict[str, object]) -> str:
    if row.get("revoked_at"):
        return "revoked"
    if _is_expired(row.get("expires_at")):  # type: ignore[arg-type]
        return "expired"
    return "active"


class UploadTokenStore:
    """Repository over ``upload_tokens`` (U2-owned)."""

    def __init__(self, db: StateDb) -> None:
        self._db = db

    def insert(
        self,
        *,
        token_id: str,
        project_id: str,
        slot_index: int,
        selector: str,
        verifier_hash: str,
        verifier_salt: str,
        owner_display_name: str | None,
        owner_kind: str | None,
        created_by: str | None,
        created_at: str,
        expires_at: str | None,
    ) -> None:
        with self._db.engine.begin() as conn:
            conn.execute(
                text(
                    "INSERT INTO upload_tokens(id, project_id, slot_index, selector, verifier_hash,"
                    " verifier_salt, owner_display_name, owner_kind, created_by, created_at, expires_at)"
                    " VALUES (:id,:pid,:slot,:sel,:vh,:salt,:odn,:ok,:by,:ca,:ea)"
                ),
                {
                    "id": token_id, "pid": project_id, "slot": slot_index, "sel": selector,
                    "vh": verifier_hash, "salt": verifier_salt, "odn": owner_display_name,
                    "ok": owner_kind, "by": created_by, "ca": created_at, "ea": expires_at,
                },
            )

    def find_by_selector(self, selector: str) -> dict[str, object] | None:
        with self._db.engine.connect() as conn:
            row = conn.execute(
                text("SELECT * FROM upload_tokens WHERE selector=:sel"),
                {"sel": selector},
            ).mappings().first()
            return dict(row) if row else None

    def find_by_id(self, token_id: str) -> dict[str, object] | None:
        with self._db.engine.connect() as conn:
            row = conn.execute(
                text("SELECT * FROM upload_tokens WHERE id=:id"),
                {"id": token_id},
            ).mappings().first()
            return dict(row) if row else None

    def list_for_project(self, project_id: str) -> list[dict[str, object]]:
        with self._db.engine.connect() as conn:
            return [
                dict(r)
                for r in conn.execute(
                    text("SELECT * FROM upload_tokens WHERE project_id=:pid ORDER BY created_at"),
                    {"pid": project_id},
                ).mappings()
            ]

    def active_slot_count(self, project_id: str) -> int:
        """Active (non-revoked, non-expired) tokens for the project."""
        return len(self.active_slot_indices(project_id))

    def active_slot_indices(self, project_id: str) -> set[int]:
        """Slots currently held by a non-revoked, non-expired token."""
        now = _now()
        with self._db.engine.connect() as conn:
            rows = conn.execute(
                text(
                    "SELECT slot_index FROM upload_tokens WHERE project_id=:pid"
                    " AND revoked_at IS NULL AND (expires_at IS NULL OR expires_at > :now)"
                ),
                {"pid": project_id, "now": now},
            )
            return {int(row[0]) for row in rows}

    def revoke(self, token_id: str, at: str) -> None:
        with self._db.engine.begin() as conn:
            conn.execute(
                text("UPDATE upload_tokens SET revoked_at=:at WHERE id=:id AND revoked_at IS NULL"),
                {"id": token_id, "at": at},
            )

    def mark_used(self, token_id: str, source_id: str, at: str) -> None:
        with self._db.engine.begin() as conn:
            conn.execute(
                text(
                    "UPDATE upload_tokens SET last_used_at=:at, registered_source_id=:sid WHERE id=:id"
                ),
                {"id": token_id, "at": at, "sid": source_id},
            )


class UploadTokenService:
    """Admin-facing token lifecycle. Guarded upstream by ``admin_context`` +
    ``require(ctx, (Role.ADMIN,))`` (RBAC-before-core, C-1)."""

    def __init__(self, store: UploadTokenStore, db: StateDb) -> None:
        self._store = store
        self._db = db
        # FastAPI executes sync handlers in a thread pool. Keep slot selection +
        # insert atomic inside this process so concurrent admins cannot receive
        # the same active slot.
        self._issue_lock = RLock()

    def issue(self, actor: AuthContext, project_id: str, req: IssueTokenRequest) -> IssuedToken:
        with self._issue_lock:
            return self._issue(actor, project_id, req)

    def _issue(
        self,
        actor: AuthContext,
        project_id: str,
        req: IssueTokenRequest,
        *,
        preferred_slot: int | None = None,
    ) -> IssuedToken:
        self._require_project(project_id)
        owner_kind = self._validate_owner_kind(req.owner_kind)
        used_slots = self._store.active_slot_indices(project_id)
        if len(used_slots) >= SOURCE_CAP:
            raise EngineError(
                EngineErrorCode.SOURCE_CAP_EXCEEDED,
                EngineErrorCategory.LIMIT,
                f"active upload-token slots at capacity ({SOURCE_CAP})",
            )
        slot_index = self._available_slot(used_slots, preferred_slot)
        selector, verifier = UploadTokenSecret.generate()
        salt = secrets.token_hex(16)
        verifier_hash = UploadTokenSecret.verifier_hash(verifier, salt)
        token_id = f"tok_{ULID()}"
        created_at = _now()
        expires_at = self._expiry(req.ttl_seconds, created_at)
        created_by = self._actor_account_id(actor)
        self._store.insert(
            token_id=token_id, project_id=project_id, slot_index=slot_index, selector=selector,
            verifier_hash=verifier_hash, verifier_salt=salt,
            owner_display_name=req.owner_display_name, owner_kind=owner_kind,
            created_by=created_by, created_at=created_at, expires_at=expires_at,
        )
        plaintext = UploadTokenSecret.present(selector, verifier)
        return IssuedToken(
            token_id=token_id, token=plaintext, upload_url=f"/u/{plaintext}",
            project_id=project_id, slot_index=slot_index,
            owner_display_name=req.owner_display_name, owner_kind=owner_kind,
            created_at=created_at, expires_at=expires_at,
        )

    def list(self, actor: AuthContext, project_id: str) -> TokenListView:
        rows = self._store.list_for_project(project_id)
        tokens = [
            TokenSummary(
                token_id=str(r["id"]), project_id=str(r["project_id"]),
                slot_index=cast(int, r["slot_index"]), selector=str(r["selector"]),
                status=_derive_status(r),
                owner_display_name=_opt_str(r.get("owner_display_name")),
                owner_kind=_opt_str(r.get("owner_kind")),
                created_at=str(r["created_at"]), expires_at=_opt_str(r.get("expires_at")),
                last_used_at=_opt_str(r.get("last_used_at")),
                registered_source_id=_opt_str(r.get("registered_source_id")),
            )
            for r in rows
        ]
        used = self._registered_source_count(project_id)
        return TokenListView(tokens=tokens, slot_usage=SlotUsage(used=used))

    def revoke(self, actor: AuthContext, project_id: str, token_id: str) -> None:
        row = self._store.find_by_id(token_id)
        if row is None or str(row["project_id"]) != project_id:
            raise EngineError.not_found("upload token not found")
        # Idempotent: revoking an already-revoked/expired token is a no-op success.
        self._store.revoke(token_id, _now())

    def rotate(self, actor: AuthContext, project_id: str, token_id: str) -> IssuedToken:
        with self._issue_lock:
            row = self._store.find_by_id(token_id)
            if row is None or str(row["project_id"]) != project_id:
                raise EngineError.not_found("upload token not found")
            self._store.revoke(token_id, _now())
            # Reissue on the same decorative slot/label (revoke + reissue, same slot).
            req = IssueTokenRequest(
                owner_display_name=_opt_str(row.get("owner_display_name")),
                owner_kind=_opt_str(row.get("owner_kind")),
            )
            return self._issue(
                actor,
                project_id,
                req,
                preferred_slot=cast(int, row["slot_index"]),
            )

    def slot_usage(self, project_id: str) -> SlotUsage:
        return SlotUsage(used=self._registered_source_count(project_id))

    def target_view(self, ctx: UploadContext) -> UploadTargetView:
        name = self._project_name(ctx.project_id)
        return UploadTargetView(
            project_id=ctx.project_id, project_name=name, slot_index=ctx.slot_index,
            owner_display_name=ctx.owner_display_name, owner_kind=ctx.owner_kind,
        )

    # --- helpers ---
    def _available_slot(self, used_slots: set[int], preferred: int | None) -> int:
        if preferred is not None and 0 <= preferred < SOURCE_CAP and preferred not in used_slots:
            return preferred
        for slot_index in range(SOURCE_CAP):
            if slot_index not in used_slots:
                return slot_index
        raise EngineError(
            EngineErrorCode.SOURCE_CAP_EXCEEDED,
            EngineErrorCategory.LIMIT,
            f"active upload-token slots at capacity ({SOURCE_CAP})",
        )

    def _validate_owner_kind(self, owner_kind: str | None) -> str | None:
        if owner_kind is None:
            return None
        if owner_kind not in _OWNER_KINDS:
            raise EngineError.validation(f"owner_kind must be one of {sorted(_OWNER_KINDS)}")
        return owner_kind

    def _expiry(self, ttl_seconds: int | None, created_at: str) -> str | None:
        if ttl_seconds is None:
            return None
        if ttl_seconds <= 0:
            raise EngineError.validation("ttl_seconds must be positive")
        return (datetime.fromisoformat(created_at) + timedelta(seconds=ttl_seconds)).isoformat()

    def _actor_account_id(self, actor: AuthContext) -> str | None:
        principal = actor.principal
        return getattr(principal, "account_id", None)

    def _require_project(self, project_id: str) -> None:
        if self._project_row(project_id) is None:
            raise EngineError.not_found("project not found")

    def _project_row(self, project_id: str) -> dict[str, object] | None:
        with self._db.engine.connect() as conn:
            row = conn.execute(
                text("SELECT id, name FROM projects WHERE id=:id"),
                {"id": project_id},
            ).mappings().first()
            return dict(row) if row else None

    def _project_name(self, project_id: str) -> str:
        row = self._project_row(project_id)
        if row is None:
            raise EngineError.not_found("project not found")
        return str(row["name"])

    def _registered_source_count(self, project_id: str) -> int:
        with self._db.engine.connect() as conn:
            return int(
                conn.execute(
                    text("SELECT COUNT(*) FROM sources WHERE project_id=:pid"),
                    {"pid": project_id},
                ).scalar()
                or 0
            )


class UploadTokenResolver:
    """The token authenticate seam installed at ``app.state.token_resolver``.

    Returns ``None`` for malformed / unknown / bad-verifier tokens (U0 maps to
    ``TOKEN_INVALID``, revealing nothing). For a verifier-valid token that is
    revoked/expired, raises the distinct ``TOKEN_REVOKED``/``TOKEN_EXPIRED``
    (propagates through ``upload_auth`` → global handler). Core is never touched.
    """

    def __init__(self, store: UploadTokenStore) -> None:
        self._store = store

    def resolve_upload(self, token_plaintext: str) -> UploadContext | None:
        parsed = UploadTokenSecret.parse(token_plaintext)
        if parsed is None:
            return None
        selector, verifier = parsed
        row = self._store.find_by_selector(selector)
        if row is None:
            return None
        if not UploadTokenSecret.verify(verifier, _opt_str(row.get("verifier_salt")), str(row["verifier_hash"])):
            return None
        if row.get("revoked_at"):
            raise EngineError(
                EngineErrorCode.TOKEN_REVOKED, EngineErrorCategory.AUTH, "upload token revoked"
            )
        if _is_expired(_opt_str(row.get("expires_at"))):
            raise EngineError(
                EngineErrorCode.TOKEN_EXPIRED, EngineErrorCategory.AUTH, "upload token expired"
            )
        return UploadContext(
            token_id=str(row["id"]), project_id=str(row["project_id"]),
            slot_index=cast(int, row["slot_index"]),
            owner_display_name=_opt_str(row.get("owner_display_name")),
            owner_kind=_opt_str(row.get("owner_kind")),
        )


def _opt_str(value: object) -> str | None:
    return None if value is None else str(value)
