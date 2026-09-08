"""``adapter.engine`` — the ADR-0002 boundary. The ``OkcEngine`` Protocol is the
sole abstract seam; ``OkcEngineImpl`` is the only class that imports/names ``okc``
binding types. Every binding ``OkcError`` becomes an okc-web ``EngineError`` here;
every payload is schema-v2-guarded here. No binding type crosses into U1..U6.

All methods are synchronous: mutating ops run on the dedicated single-writer
engine worker thread; non-reserving reads run on the read-path pool
(``adapter.queue``). Blocking ``Job.result()`` is therefore never on the event loop.
"""

from __future__ import annotations

import time
from collections.abc import Callable
from typing import Any, Protocol, cast

import okc  # the native binding — named ONLY here and in nothing downstream

from app.adapter import dto, schema_guard
from app.shared.error import EngineError, EngineErrorCategory, EngineErrorCode
from app.shared.jobs import JobProgress

_TERMINAL = {"completed", "failed", "cancelled"}

# okc.OkcError.code (SCREAMING_SNAKE) -> (okc-web code, category). Mirrors the
# Rust `map_error` table. Provider sub-codes collapse to ERROR/UNAVAILABLE.
_ERROR_MAP: dict[str, tuple[EngineErrorCode, EngineErrorCategory]] = {
    "INVALID_ARGUMENT": (EngineErrorCode.INVALID_ARGUMENT, EngineErrorCategory.VALIDATION),
    "PATH_NOT_ABSOLUTE": (EngineErrorCode.PATH_NOT_ABSOLUTE, EngineErrorCategory.VALIDATION),
    "PATH_NOT_FOUND": (EngineErrorCode.PATH_NOT_FOUND, EngineErrorCategory.NOT_FOUND),
    "PATH_UNSAFE": (EngineErrorCode.PATH_UNSAFE, EngineErrorCategory.VALIDATION),
    "PATH_UNSUPPORTED": (EngineErrorCode.PATH_UNSUPPORTED, EngineErrorCategory.VALIDATION),
    "RESOURCE_LIMIT": (EngineErrorCode.RESOURCE_LIMIT, EngineErrorCategory.LIMIT),
    "PROJECT_INVALID": (EngineErrorCode.PROJECT_INVALID, EngineErrorCategory.PROJECT),
    "PROJECT_BUSY": (EngineErrorCode.PROJECT_BUSY, EngineErrorCategory.CONCURRENCY),
    "PROVIDER_INVALID": (EngineErrorCode.PROVIDER_INVALID, EngineErrorCategory.PROVIDER),
    "PROVIDER_AUTHENTICATION": (EngineErrorCode.PROVIDER_ERROR, EngineErrorCategory.PROVIDER),
    "PROVIDER_AUTHORIZATION": (EngineErrorCode.PROVIDER_ERROR, EngineErrorCategory.PROVIDER),
    "PROVIDER_ENV_SECRET_MISSING": (EngineErrorCode.PROVIDER_ERROR, EngineErrorCategory.PROVIDER),
    "PROVIDER_RATE_LIMITED": (EngineErrorCode.PROVIDER_ERROR, EngineErrorCategory.PROVIDER),
    "PROVIDER_UNSUPPORTED": (EngineErrorCode.PROVIDER_ERROR, EngineErrorCategory.PROVIDER),
    "PROVIDER_TIMEOUT": (EngineErrorCode.PROVIDER_UNAVAILABLE, EngineErrorCategory.PROVIDER),
    "PROVIDER_RESPONSE_INVALID": (EngineErrorCode.PROVIDER_UNAVAILABLE, EngineErrorCategory.PROVIDER),
    "PROVIDER_TRANSPORT": (EngineErrorCode.PROVIDER_UNAVAILABLE, EngineErrorCategory.PROVIDER),
    "REMOTE_CONSENT_REQUIRED": (EngineErrorCode.REMOTE_CONSENT_REQUIRED, EngineErrorCategory.CONSENT),
    "SENSITIVE_REMOTE_FORBIDDEN": (EngineErrorCode.SENSITIVE_REMOTE_FORBIDDEN, EngineErrorCategory.AUTH),
    "APPROVAL_REQUIRED": (EngineErrorCode.APPROVAL_REQUIRED, EngineErrorCategory.APPROVAL),
    "APPROVAL_STALE": (EngineErrorCode.APPROVAL_STALE, EngineErrorCategory.APPROVAL),
    "OUTPUT_EXISTS": (EngineErrorCode.OUTPUT_EXISTS, EngineErrorCategory.PROJECT),
    "OUTPUT_OVERLAP": (EngineErrorCode.OUTPUT_OVERLAP, EngineErrorCategory.PROJECT),
    "OUTPUT_DURABILITY_UNCERTAIN": (EngineErrorCode.OUTPUT_DURABILITY_UNCERTAIN, EngineErrorCategory.IO),
    "ARTIFACT_SCHEMA_UNSUPPORTED": (EngineErrorCode.ARTIFACT_SCHEMA_UNSUPPORTED, EngineErrorCategory.SCHEMA),
    "VERIFICATION_FAILED": (EngineErrorCode.VERIFICATION_FAILED, EngineErrorCategory.VERIFICATION),
    "CANCELLED": (EngineErrorCode.CANCELLED, EngineErrorCategory.LIFECYCLE),
    "INTERNAL": (EngineErrorCode.INTERNAL, EngineErrorCategory.INTERNAL),
}


def map_error(e: okc.OkcError) -> EngineError:
    """The single binding -> okc-web error conversion (ADR-0002). Branch on the
    code string; preserve retryable + retry_after_ms; never leak binding types."""
    code, category = _ERROR_MAP.get(
        str(getattr(e, "code", "")), (EngineErrorCode.INTERNAL, EngineErrorCategory.INTERNAL)
    )
    retry_after_ms: int | None = None
    details = getattr(e, "details", None)
    if isinstance(details, dict):
        raw = details.get("retry_after_ms")
        if isinstance(raw, int):
            retry_after_ms = raw
    return EngineError(
        code=code,
        category=category,
        message=str(getattr(e, "message", "") or e),
        retryable=bool(getattr(e, "retryable", False)),
        retry_after_ms=retry_after_ms,
    )


def _map_progress(ev: dict[str, Any]) -> JobProgress:
    return JobProgress(
        sequence=int(ev.get("sequence", 0) or 0),
        state=str(ev.get("state", "running")),
        phase=str(ev.get("phase", "") or ""),
        completed=int(ev.get("completed", 0) or 0),
        total=ev.get("total"),
        current_item=ev.get("current_item"),
    )


def _result(job: okc.Job) -> Any:
    """Block for a job's terminal result, mapping any binding error."""
    try:
        return job.result()
    except okc.OkcError as e:  # noqa: PERF203
        raise map_error(e) from e


def _drive(job: okc.Job, progress: Callable[[JobProgress], None]) -> Any:
    """Drive a long op to terminal, pumping progress events into the sink."""
    try:
        while True:
            for ev in job.events():
                progress(_map_progress(ev))
            if job.state in _TERMINAL:
                break
            time.sleep(0.05)
        for ev in job.events():
            progress(_map_progress(ev))
        return job.result()
    except okc.OkcError as e:
        raise map_error(e) from e


class OkcEngine(Protocol):
    """The single abstract engine seam (ADR-0002). Read ops return a ``*View``;
    long ops accept a progress sink; all errors are okc-web ``EngineError``."""

    def create_project(self, spec: dto.CreateProjectSpec) -> dto.ProjectRefView: ...
    def open_project(self, root: str) -> dto.ProjectRefView: ...
    def status(self, root: str) -> dto.StatusView: ...
    def add_source(self, root: str, cmd: dto.AddSourceCmd) -> None: ...
    def set_ai_route(self, root: str, profile_name: str, role: str | None = None) -> None: ...
    def preflight(self, root: str) -> dto.PreflightView: ...
    def integrate(self, root: str, disclosure: dto.DisclosureCmd,
                  progress: Callable[[JobProgress], None]) -> dto.IntegrationView: ...
    def taxonomy(self, root: str) -> Any: ...
    def approve_taxonomy(self, root: str, cmd: dto.ApproveTaxonomyCmd) -> None: ...
    def clusters(self, root: str) -> Any: ...
    def approve_cluster(self, root: str, cmd: dto.ApproveClusterCmd) -> None: ...
    def regenerate_cluster(self, root: str, cmd: dto.RegenerateClusterCmd,
                           progress: Callable[[JobProgress], None]) -> None: ...
    def compile(self, root: str, output: str) -> dto.CompileView: ...
    def verify(self, artifact_path: str) -> dto.VerificationView: ...
    def explain(self, artifact_path: str, output_path: str) -> dto.ProvenanceView: ...
    def manifest(self, root: str) -> Any: ...


class OkcEngineImpl:
    """The one and only concrete engine, wrapping a single ``okc.OkcClient``.

    Because the binding has no synchronous handle accessor, a ``Project`` is
    obtained via ``open_project(root).result()`` and cached by canonical path
    (design C0.5). One impl backs the single-writer worker; a second impl (its
    own client) backs the read path.
    """

    def __init__(self, client: okc.OkcClient) -> None:
        self._client = client
        self._handles: dict[str, okc.Project] = {}

    def _handle(self, root: str) -> okc.Project:
        key = str(root)
        proj = self._handles.get(key)
        if proj is None:
            proj = _result(self._client.open_project(key))
            self._handles[key] = proj
        return proj

    def create_project(self, spec: dto.CreateProjectSpec) -> dto.ProjectRefView:
        job = self._client.create_project(
            spec.root_abs_path,
            name=spec.name,
            curator_id=spec.curator_id,
            policy_version=spec.policy_version,
            language=spec.language,
        )
        project = _result(job)
        self._handles[str(spec.root_abs_path)] = project
        return dto.ProjectRefView(root_abs_path=str(project.path))

    def open_project(self, root: str) -> dto.ProjectRefView:
        project = self._handle(root)
        return dto.ProjectRefView(root_abs_path=str(project.path))

    def status(self, root: str) -> dto.StatusView:
        payload = _result(self._handle(root).status())
        schema_guard.check(payload.get("interop_schema_version"))
        return dto.StatusView.from_native(payload)

    def add_source(self, root: str, cmd: dto.AddSourceCmd) -> None:
        source = okc.SourceInput(
            cmd.source_id, cmd.absolute_path,
            owner_display_name=cmd.owner_display_name, snapshot_id=cmd.snapshot_id,
        )
        _result(self._handle(root).add_source(source))

    def set_ai_route(self, root: str, profile_name: str, role: str | None = None) -> None:
        # `role` is a validated okc AiRole literal at the seam; cast for the stub.
        _result(self._handle(root).set_ai_route(profile_name, role=cast(Any, role)))

    def preflight(self, root: str) -> dto.PreflightView:
        return dto.PreflightView.from_native(_result(self._handle(root).preflight()))

    def integrate(self, root: str, disclosure: dto.DisclosureCmd,
                  progress: Callable[[JobProgress], None]) -> dto.IntegrationView:
        job = self._handle(root).integrate(
            allow_remote_provider=disclosure.allow_remote_provider,
            remote_disclosure_confirmed=disclosure.remote_disclosure_confirmed,
        )
        payload = _drive(job, progress)
        schema_guard.check(payload.get("interop_schema_version"))
        return dto.IntegrationView.from_native(payload)

    def taxonomy(self, root: str) -> Any:
        payload = _result(self._handle(root).taxonomy())
        schema_guard.check(payload.get("interop_schema_version"))
        return payload

    def approve_taxonomy(self, root: str, cmd: dto.ApproveTaxonomyCmd) -> None:
        _result(self._handle(root).approve_taxonomy(
            edited_clusters=cmd.edited_clusters, rationale=cmd.rationale
        ))

    def clusters(self, root: str) -> Any:
        payload = _result(self._handle(root).clusters())
        schema_guard.check(payload.get("interop_schema_version"))
        return payload.get("payload", payload)

    def approve_cluster(self, root: str, cmd: dto.ApproveClusterCmd) -> None:
        _result(self._handle(root).approve_cluster(
            cmd.cluster_id,
            omission_rationales=cmd.omission_rationales or None,
            minor_waivers=cmd.minor_waivers or None,
        ))

    def regenerate_cluster(self, root: str, cmd: dto.RegenerateClusterCmd,
                           progress: Callable[[JobProgress], None]) -> None:
        job = self._handle(root).regenerate_cluster(
            cmd.cluster_id, cmd.feedback,
            allow_remote_provider=cmd.disclosure.allow_remote_provider,
            remote_disclosure_confirmed=cmd.disclosure.remote_disclosure_confirmed,
        )
        _drive(job, progress)

    def compile(self, root: str, output: str) -> dto.CompileView:
        payload = _result(self._handle(root).compile(output))
        schema_guard.check(payload.get("interop_schema_version"))
        return dto.CompileView.from_native(payload)

    def verify(self, artifact_path: str) -> dto.VerificationView:
        payload = _result(self._client.verify_artifact(artifact_path))
        schema_guard.check(payload.get("interop_schema_version"))
        return dto.VerificationView.from_native(payload)

    def explain(self, artifact_path: str, output_path: str) -> dto.ProvenanceView:
        payload = _result(self._client.explain_artifact(artifact_path, output_path=output_path))
        schema_guard.check(payload.get("interop_schema_version"))
        return dto.ProvenanceView.from_native(payload)

    def manifest(self, root: str) -> Any:
        payload = _result(self._handle(root).manifest())
        schema_guard.check(payload.get("interop_schema_version"))
        return payload.get("payload", payload)


_PROVIDER_KINDS = {
    "openai": "open_ai", "open_ai": "open_ai", "anthropic": "anthropic",
    "gemini": "gemini", "ollama": "ollama",
    "openai_compatible": "open_ai_compatible", "openai-compatible": "open_ai_compatible",
    "open_ai_compatible": "open_ai_compatible",
}


def build_client(specs: list[dto.ProviderSpecView]) -> okc.OkcClient:
    """Build a process ``OkcClient`` from okc-web provider specs (env-var names
    only, never secret values)."""
    profiles = []
    for s in specs:
        kind = _PROVIDER_KINDS.get(s.kind.lower())
        if kind is None:
            raise EngineError.validation(f"unknown provider kind '{s.kind}'")
        profiles.append(okc.ProviderProfile(
            name=s.name, kind=cast(Any, kind), endpoint=s.endpoint,
            model=s.model, api_key_env=s.api_key_env,
        ))
    try:
        return okc.OkcClient(profiles)
    except okc.OkcError as e:
        raise map_error(e) from e
