"""``app.main`` — FastAPI app factory + startup wiring (U0 §11).

Startup order (fail fast): SchemaGuard → config → StateDb migrate → JobStore /
AuditStore → EngineWorker (single writer + read path) → routers → error handlers.
Served by uvicorn with a SINGLE worker (``--workers 1``): the process owns the
one write ``OkcClient`` (NFR-CONC-1); multiple workers would break single-writer.

Later waves attach themselves via ``UNIT_MODULES`` discovery — each unit exposes
``register(app, state)`` in ``app.<unit>.router``. W0 mounts health + the
canonical job-status routes (Q7) + the SPA static assets.
"""

from __future__ import annotations

import logging
import os
from collections.abc import AsyncIterator
from contextlib import asynccontextmanager

from fastapi import Depends, FastAPI
from fastapi.middleware.cors import CORSMiddleware
from fastapi.staticfiles import StaticFiles

from app.adapter import schema_guard
from app.adapter.queue import EngineWorker
from app.config import AppConfig
from app.shared.audit import AuditStore
from app.shared.authz import UploadContext, admin_auth, upload_auth
from app.shared.error import EngineError, register_error_handlers
from app.shared.jobs import JobSnapshot, JobStore
from app.shared.state import StateDb

logging.basicConfig(level=os.environ.get("OKC_WEB_LOG_LEVEL", "INFO"))
_log = logging.getLogger("okc_web")


class AppState:
    """Container for the shared singletons, stashed on ``app.state``."""

    def __init__(self, config: AppConfig, db: StateDb, jobs: JobStore,
                 audit: AuditStore, engine: EngineWorker | None) -> None:
        self.config = config
        self.db = db
        self.jobs = jobs
        self.audit = audit
        self.engine = engine


def create_app(config: AppConfig | None = None) -> FastAPI:
    cfg = config or AppConfig.from_env()

    if not cfg.skip_engine:
        schema_guard.assert_startup()  # abort before serving on a version mismatch

    db = StateDb.open(cfg.state_db_path)
    jobs = JobStore(db)
    audit = AuditStore(db)
    engine: EngineWorker | None = None
    if not cfg.skip_engine:
        engine = EngineWorker.create(cfg.providers, jobs)
    state = AppState(cfg, db, jobs, audit, engine)

    @asynccontextmanager
    async def lifespan(_app: FastAPI) -> AsyncIterator[None]:
        _log.info("okc-web starting (engine=%s)", "on" if engine else "skipped")
        try:
            yield
        finally:
            if engine is not None:
                engine.shutdown()

    app = FastAPI(title="okc-web", version="0.1.0", lifespan=lifespan)
    app.state.app_state = state
    # Resolvers default to deny-all until U1/U2 install real ones.
    app.state.session_resolver = None
    app.state.token_resolver = None

    register_error_handlers(app)

    if cfg.cors_origins:
        app.add_middleware(
            CORSMiddleware,
            allow_origins=cfg.cors_origins,
            allow_credentials=True,
            allow_methods=["*"],
            allow_headers=["*"],
        )

    _mount_core_routes(app)
    _register_units(app, state)
    _mount_spa(app, cfg)
    return app


def _register_units(app: FastAPI, state: AppState) -> None:
    """Discover and register each unit module (U1..U5). Each unit exposes
    ``register(app, state)`` in ``app.<unit>.router`` — it builds its services,
    installs any auth resolver on ``app.state``, and includes its router. Units
    are discovered dynamically so parallel wave code-gen never edits main.py.
    """
    import importlib

    for mod_name in UNIT_MODULES:
        try:
            mod = importlib.import_module(mod_name)
        except ModuleNotFoundError:
            continue  # unit not generated yet (earlier wave)
        register = getattr(mod, "register", None)
        if callable(register):
            register(app, state)
            _log.info("registered unit %s", mod_name)


def _mount_core_routes(app: FastAPI) -> None:
    @app.get("/api/health")
    def health() -> dict[str, str]:
        return {"status": "ok", "service": "okc-web"}

    # Canonical job-status polling (Q7). Two authenticated mounts, one snapshot shape.
    @app.get("/api/projects/{project_id}/jobs/{job_id}")
    def admin_job_status(
        project_id: str, job_id: str, _admin=Depends(admin_auth)  # noqa: ANN001
    ) -> JobSnapshot:
        return _get_job_or_404(app, job_id, expect_project=project_id)

    @app.get("/u/{token}/jobs/{job_id}")
    def upload_job_status(
        job_id: str, ctx: UploadContext = Depends(upload_auth)
    ) -> JobSnapshot:
        return _get_job_or_404(app, job_id, expect_project=ctx.project_id)


def _get_job_or_404(app: FastAPI, job_id: str, expect_project: str | None) -> JobSnapshot:
    state: AppState = app.state.app_state
    snap = state.jobs.get(job_id)
    if snap is None:
        raise EngineError.not_found("job not found")
    # Scope guard: a token/admin may only read jobs for its own project.
    if expect_project is not None and snap.project_id not in (None, expect_project):
        raise EngineError.not_found("job not found")
    return snap


def _mount_spa(app: FastAPI, cfg: AppConfig) -> None:
    dist = cfg.spa_dist or _default_spa_dist()
    if dist and os.path.isdir(dist):
        app.mount("/", StaticFiles(directory=dist, html=True), name="spa")


def _default_spa_dist() -> str:
    here = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))  # backend/
    candidate = os.path.abspath(os.path.join(here, "..", "frontend", "dist"))
    return candidate


# Unit modules discovered at startup (each exposes `register(app, state)`).
# Present-or-absent tolerant, so the app runs at any wave of construction.
UNIT_MODULES: list[str] = [
    "app.auth.router",          # U1
    "app.upload.router",        # U2
    "app.orchestration.router",  # U3
    "app.review.router",        # U4
    "app.serving.router",       # U5
]
