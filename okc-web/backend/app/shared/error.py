"""S0.D ErrorTranslationService — the single, table-driven okc-web error contract (§8).

Every error surfaced to a client is an :class:`EngineError` carrying a stable
``code`` + ``category``. HTTP status is derived from the ``code`` ONLY (never by
parsing ``message``). The frontend branches on ``code``/``category`` (§8).

The conversion ``okc.OkcError -> EngineError`` lives in ``adapter`` (ADR-0002):
no okc-core/binding type crosses into ``shared``/U1..U6.
"""

from __future__ import annotations

from enum import Enum
from typing import Any

from fastapi import FastAPI, Request
from fastapi.responses import JSONResponse


class EngineErrorCode(str, Enum):
    """Stable error codes shared across all units. Superset of okc binding codes
    plus okc-web-origin auth/upload codes. SCREAMING_SNAKE_CASE on the wire — this
    matches ``okc.OkcError.code`` exactly, so mapping is near-identity."""

    # --- okc-web auth origin (C-1) ---
    UNAUTHENTICATED = "UNAUTHENTICATED"
    SESSION_EXPIRED = "SESSION_EXPIRED"
    FORBIDDEN = "FORBIDDEN"
    SENSITIVE_REMOTE_FORBIDDEN = "SENSITIVE_REMOTE_FORBIDDEN"
    # --- okc-web upload/domain origin ---
    VALIDATION_FAILED = "VALIDATION_FAILED"
    SOURCE_CAP_EXCEEDED = "SOURCE_CAP_EXCEEDED"
    UPLOAD_TOO_LARGE = "UPLOAD_TOO_LARGE"
    TOKEN_INVALID = "TOKEN_INVALID"
    TOKEN_EXPIRED = "TOKEN_EXPIRED"
    TOKEN_REVOKED = "TOKEN_REVOKED"
    DUPLICATE_SOURCE = "DUPLICATE_SOURCE"
    ACCOUNT_DISABLED = "ACCOUNT_DISABLED"
    NOT_FOUND = "NOT_FOUND"
    METHOD_NOT_ALLOWED = "METHOD_NOT_ALLOWED"
    # --- mirrored from the okc binding ErrorCode ---
    INVALID_ARGUMENT = "INVALID_ARGUMENT"
    PATH_NOT_ABSOLUTE = "PATH_NOT_ABSOLUTE"
    PATH_NOT_FOUND = "PATH_NOT_FOUND"
    PATH_UNSAFE = "PATH_UNSAFE"
    PATH_UNSUPPORTED = "PATH_UNSUPPORTED"
    RESOURCE_LIMIT = "RESOURCE_LIMIT"
    PROJECT_INVALID = "PROJECT_INVALID"
    PROJECT_BUSY = "PROJECT_BUSY"
    PROVIDER_INVALID = "PROVIDER_INVALID"
    PROVIDER_ERROR = "PROVIDER_ERROR"
    PROVIDER_UNAVAILABLE = "PROVIDER_UNAVAILABLE"
    REMOTE_CONSENT_REQUIRED = "REMOTE_CONSENT_REQUIRED"
    APPROVAL_REQUIRED = "APPROVAL_REQUIRED"
    APPROVAL_STALE = "APPROVAL_STALE"
    OUTPUT_EXISTS = "OUTPUT_EXISTS"
    OUTPUT_OVERLAP = "OUTPUT_OVERLAP"
    OUTPUT_DURABILITY_UNCERTAIN = "OUTPUT_DURABILITY_UNCERTAIN"
    ARTIFACT_SCHEMA_UNSUPPORTED = "ARTIFACT_SCHEMA_UNSUPPORTED"
    VERIFICATION_FAILED = "VERIFICATION_FAILED"
    CANCELLED = "CANCELLED"
    SCHEMA_UNSUPPORTED = "SCHEMA_UNSUPPORTED"
    INTERNAL = "INTERNAL"


class EngineErrorCategory(str, Enum):
    AUTH = "auth"
    NOT_FOUND = "not_found"
    CONCURRENCY = "concurrency"
    PROJECT = "project"
    APPROVAL = "approval"
    VERIFICATION = "verification"
    SCHEMA = "schema"
    CONSENT = "consent"
    VALIDATION = "validation"
    LIMIT = "limit"
    PROVIDER = "provider"
    LIFECYCLE = "lifecycle"
    IO = "io"
    INTERNAL = "internal"


# Table-driven code -> HTTP status (§8). Branch on the code, never prose.
_HTTP_STATUS: dict[EngineErrorCode, int] = {
    EngineErrorCode.UNAUTHENTICATED: 401,
    EngineErrorCode.SESSION_EXPIRED: 401,
    EngineErrorCode.FORBIDDEN: 403,
    EngineErrorCode.SENSITIVE_REMOTE_FORBIDDEN: 403,
    EngineErrorCode.NOT_FOUND: 404,
    EngineErrorCode.PATH_NOT_FOUND: 404,
    EngineErrorCode.METHOD_NOT_ALLOWED: 405,
    EngineErrorCode.PROJECT_BUSY: 409,
    EngineErrorCode.OUTPUT_EXISTS: 409,
    EngineErrorCode.OUTPUT_OVERLAP: 409,
    EngineErrorCode.CANCELLED: 409,
    EngineErrorCode.APPROVAL_REQUIRED: 422,
    EngineErrorCode.APPROVAL_STALE: 422,
    EngineErrorCode.VERIFICATION_FAILED: 422,
    EngineErrorCode.PROJECT_INVALID: 422,
    EngineErrorCode.ARTIFACT_SCHEMA_UNSUPPORTED: 422,
    EngineErrorCode.REMOTE_CONSENT_REQUIRED: 422,
    EngineErrorCode.INVALID_ARGUMENT: 400,
    EngineErrorCode.PATH_NOT_ABSOLUTE: 400,
    EngineErrorCode.PATH_UNSAFE: 400,
    EngineErrorCode.PATH_UNSUPPORTED: 400,
    EngineErrorCode.VALIDATION_FAILED: 400,
    EngineErrorCode.UPLOAD_TOO_LARGE: 400,
    EngineErrorCode.TOKEN_INVALID: 400,
    EngineErrorCode.TOKEN_EXPIRED: 400,
    EngineErrorCode.TOKEN_REVOKED: 400,
    EngineErrorCode.ACCOUNT_DISABLED: 400,
    EngineErrorCode.DUPLICATE_SOURCE: 400,
    EngineErrorCode.SOURCE_CAP_EXCEEDED: 429,
    EngineErrorCode.RESOURCE_LIMIT: 429,
    EngineErrorCode.PROVIDER_UNAVAILABLE: 502,
    EngineErrorCode.PROVIDER_ERROR: 424,
    EngineErrorCode.PROVIDER_INVALID: 424,
    # Safe defaults: never fall through to a crash (§8).
    EngineErrorCode.OUTPUT_DURABILITY_UNCERTAIN: 500,
    EngineErrorCode.SCHEMA_UNSUPPORTED: 500,
    EngineErrorCode.INTERNAL: 500,
}


class EngineError(Exception):
    """The single error type carried across the okc-web layer. Serializes to the
    stable body ``{code, category, message, retryable, retry_after_ms?}``.

    Covers both okc-binding-origin faults (mapped in ``adapter``) and okc-web
    auth/validation-origin faults, so it doubles as the app's ``ApiError``.
    """

    def __init__(
        self,
        code: EngineErrorCode,
        category: EngineErrorCategory,
        message: str,
        retryable: bool = False,
        retry_after_ms: int | None = None,
    ) -> None:
        super().__init__(f"{code.value}: {message}")
        self.code = code
        self.category = category
        self.message = message
        self.retryable = retryable
        self.retry_after_ms = retry_after_ms

    # --- Common okc-web-origin constructors ---
    @classmethod
    def unauthenticated(cls, message: str = "session absent or expired") -> EngineError:
        return cls(EngineErrorCode.UNAUTHENTICATED, EngineErrorCategory.AUTH, message)

    @classmethod
    def session_expired(cls) -> EngineError:
        return cls(EngineErrorCode.SESSION_EXPIRED, EngineErrorCategory.AUTH, "session absent or expired")

    @classmethod
    def forbidden(cls, message: str = "insufficient role") -> EngineError:
        return cls(EngineErrorCode.FORBIDDEN, EngineErrorCategory.AUTH, message)

    @classmethod
    def validation(cls, message: str) -> EngineError:
        return cls(EngineErrorCode.VALIDATION_FAILED, EngineErrorCategory.VALIDATION, message)

    @classmethod
    def not_found(cls, message: str = "not found") -> EngineError:
        return cls(EngineErrorCode.NOT_FOUND, EngineErrorCategory.NOT_FOUND, message)

    @classmethod
    def internal(cls, message: str) -> EngineError:
        return cls(EngineErrorCode.INTERNAL, EngineErrorCategory.INTERNAL, message)

    def as_retryable(self, retry_after_ms: int | None = None) -> EngineError:
        self.retryable = True
        self.retry_after_ms = retry_after_ms
        return self

    def http_status(self) -> int:
        return _HTTP_STATUS.get(self.code, 500)

    def retry_after_seconds(self) -> int | None:
        """Retryable by CODE (the binding's ``retryable`` flag is unreliable for
        reservation-collision PROJECT_BUSY — §4/§8)."""
        if self.code == EngineErrorCode.PROJECT_BUSY:
            return 2
        if self.code == EngineErrorCode.RESOURCE_LIMIT:
            return 5
        if self.retry_after_ms is not None:
            return max(1, -(-self.retry_after_ms // 1000))  # ceil-div
        return None

    def to_body(self) -> dict[str, Any]:
        body: dict[str, Any] = {
            "code": self.code.value,
            "category": self.category.value,
            "message": self.message,
            "retryable": self.retryable,
        }
        if self.retry_after_ms is not None:
            body["retry_after_ms"] = self.retry_after_ms
        return body


def register_error_handlers(app: FastAPI) -> None:
    """Register the single EngineError -> HTTP response mapping on the app."""

    @app.exception_handler(EngineError)
    async def _handle_engine_error(_request: Request, exc: EngineError) -> JSONResponse:
        headers: dict[str, str] = {}
        secs = exc.retry_after_seconds()
        if secs is not None:
            headers["Retry-After"] = str(secs)
        return JSONResponse(status_code=exc.http_status(), content=exc.to_body(), headers=headers)
