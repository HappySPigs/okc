"""``adapter.schema_guard`` — the single okc-binding schema-version guard point.

Asserts ``okc.INTEROP_SCHEMA_VERSION == 2`` at client init and at every DTO
decode; a mismatch yields ``SCHEMA_UNSUPPORTED`` (never a silent mis-decode).
``okc`` is imported lazily so the rest of ``shared``/``adapter.dto`` can be
unit-tested without the built native extension.
"""

from __future__ import annotations

from app.shared.error import EngineError, EngineErrorCategory, EngineErrorCode

# The okc-binding schema version okc-web is built against.
EXPECTED_SCHEMA_VERSION = 2


def _mismatch(found: object) -> EngineError:
    return EngineError(
        EngineErrorCode.SCHEMA_UNSUPPORTED,
        EngineErrorCategory.SCHEMA,
        f"okc binding schema version {found} is unsupported "
        f"(okc-web requires {EXPECTED_SCHEMA_VERSION})",
    )


def assert_startup() -> None:
    """Startup assertion — call once before serving. Aborts if incompatible."""
    import okc  # lazy: native extension

    check(okc.INTEROP_SCHEMA_VERSION)


def check(version: int | None) -> None:
    """Per-decode assertion — the ``interop_schema_version`` each payload carries.

    ``None`` (field absent on a given payload) is tolerated: startup already
    proved the client-wide version, so absence here is not a mis-decode.
    """
    if version is None:
        return
    if version != EXPECTED_SCHEMA_VERSION:
        raise _mismatch(version)
