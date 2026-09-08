"""``shared.state`` — the single SQLite WAL state file (§9).

One long-lived process, one state DB. All okc-web control-plane tables live
here (accounts/sessions/upload_tokens/projects/**sources**/jobs/job_events/
curator_decisions/serving_publications). The ``sources`` (SourceRegistry)
write-read seam is frozen in W0 so U2 (write) and U5 (read) build against it
before U3 lands (open question resolved: physical schema = U0, semantics = U3).

Substrate = SQLAlchemy 2.0 Core ``Engine`` over stdlib ``sqlite3`` in WAL mode
(ADR-0025). ``Migrations`` is an ordered DDL runner (no Alembic — MVP).
"""

from __future__ import annotations

import sqlite3
from datetime import UTC, datetime

from sqlalchemy import Engine, create_engine, event, text

from app.shared.error import EngineError

# The initial schema. Idempotent (`IF NOT EXISTS`); the whole control-plane data
# model is created up front so later waves need no migration churn.
MIGRATION_0001_INIT = """
CREATE TABLE IF NOT EXISTS schema_migrations (
    version     INTEGER PRIMARY KEY,
    applied_at  TEXT NOT NULL
);

-- U1: accounts (password_hash hashed@rest: argon2id PHC)
CREATE TABLE IF NOT EXISTS accounts (
    id            TEXT PRIMARY KEY,
    email         TEXT NOT NULL UNIQUE,
    display_name  TEXT NOT NULL,
    role          TEXT NOT NULL CHECK (role IN ('admin','contributor')),
    password_hash TEXT NOT NULL,
    status        TEXT NOT NULL CHECK (status IN ('active','disabled')) DEFAULT 'active',
    created_at    TEXT NOT NULL,
    updated_at    TEXT NOT NULL,
    last_login_at TEXT
);

-- U1: sessions (id_hash = SHA-256 of opaque id; plaintext lives only in cookie)
CREATE TABLE IF NOT EXISTS sessions (
    id_hash      TEXT PRIMARY KEY,
    account_id   TEXT NOT NULL REFERENCES accounts(id),
    created_at   TEXT NOT NULL,
    last_seen_at TEXT NOT NULL,
    expires_at   TEXT NOT NULL,
    revoked_at   TEXT,
    user_agent   TEXT,
    ip           TEXT
);
CREATE INDEX IF NOT EXISTS idx_sessions_account ON sessions(account_id);

-- U3: projects
CREATE TABLE IF NOT EXISTS projects (
    id                     TEXT PRIMARY KEY,
    name                   TEXT NOT NULL,
    engine_root_abs_path   TEXT NOT NULL,
    curator_id             TEXT NOT NULL,
    created_by             TEXT REFERENCES accounts(id),
    source_set_fingerprint TEXT,
    freeze_state           TEXT NOT NULL CHECK (freeze_state IN ('unfrozen','frozen')) DEFAULT 'unfrozen',
    frozen_at              TEXT,
    created_at             TEXT NOT NULL,
    updated_at             TEXT NOT NULL
);

-- U2: upload_tokens (verifier_hash hashed@rest; selector public)
CREATE TABLE IF NOT EXISTS upload_tokens (
    id                   TEXT PRIMARY KEY,
    project_id           TEXT NOT NULL REFERENCES projects(id),
    slot_index           INTEGER NOT NULL,
    selector             TEXT NOT NULL UNIQUE,
    verifier_hash        TEXT NOT NULL,
    verifier_salt        TEXT,
    owner_display_name   TEXT,
    owner_kind           TEXT CHECK (owner_kind IN ('department','individual')),
    created_by           TEXT REFERENCES accounts(id),
    created_at           TEXT NOT NULL,
    expires_at           TEXT,
    revoked_at           TEXT,
    last_used_at         TEXT,
    registered_source_id TEXT,
    source_identity      TEXT
);
CREATE INDEX IF NOT EXISTS idx_tokens_project ON upload_tokens(project_id, revoked_at);

-- U3-owned SourceRegistry (written by U2 at commit, read by U5). Seam frozen in W0.
CREATE TABLE IF NOT EXISTS sources (
    source_id          TEXT PRIMARY KEY,
    project_id         TEXT NOT NULL REFERENCES projects(id),
    document_id        TEXT,
    owner_display_name TEXT,
    owner_kind         TEXT,
    content_hash       TEXT NOT NULL,
    absolute_path      TEXT NOT NULL,
    slot_index         INTEGER NOT NULL,
    upload_token_id    TEXT REFERENCES upload_tokens(id),
    registered_at      TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_sources_project ON sources(project_id);

-- U2: immutable revision history plus resumable hooks transfer sessions.
CREATE TABLE IF NOT EXISTS source_revisions (
    source_id     TEXT NOT NULL,
    revision_hash TEXT NOT NULL,
    absolute_path TEXT NOT NULL,
    registered_at TEXT NOT NULL,
    PRIMARY KEY (source_id, revision_hash)
);
CREATE TABLE IF NOT EXISTS upload_sync_sessions (
    id              TEXT PRIMARY KEY,
    token_id        TEXT NOT NULL REFERENCES upload_tokens(id),
    project_id      TEXT NOT NULL REFERENCES projects(id),
    source_id       TEXT NOT NULL,
    base_revision   TEXT,
    manifest_digest TEXT NOT NULL,
    entries_json    TEXT NOT NULL,
    status          TEXT NOT NULL CHECK (status IN ('pending','committed')) DEFAULT 'pending',
    landed_path     TEXT,
    created_at      TEXT NOT NULL,
    committed_at    TEXT
);
CREATE INDEX IF NOT EXISTS idx_sync_token ON upload_sync_sessions(token_id, created_at);

-- U0: jobs (the id clients poll — Q7) + companion job_events append log
CREATE TABLE IF NOT EXISTS jobs (
    id                 TEXT PRIMARY KEY,
    project_id         TEXT,
    kind               TEXT NOT NULL,
    state              TEXT NOT NULL,
    phase              TEXT,
    progress_completed INTEGER NOT NULL DEFAULT 0,
    progress_total     INTEGER,
    error_code         TEXT,
    error_category     TEXT,
    error_retryable    INTEGER,
    requested_by       TEXT,
    created_at         TEXT NOT NULL,
    updated_at         TEXT NOT NULL,
    started_at         TEXT,
    finished_at        TEXT
);
CREATE INDEX IF NOT EXISTS idx_jobs_project ON jobs(project_id);

CREATE TABLE IF NOT EXISTS job_events (
    job_id       TEXT NOT NULL REFERENCES jobs(id),
    sequence     INTEGER NOT NULL,
    phase        TEXT,
    state        TEXT,
    completed    INTEGER,
    total        INTEGER,
    current_item TEXT,
    ts           TEXT NOT NULL,
    PRIMARY KEY (job_id, sequence)
);

-- U0 audit (produced by U4/U3) — append-only. decision_kind is enum-constrained
-- so NO winner-select row is representable (C3).
CREATE TABLE IF NOT EXISTS curator_decisions (
    id            TEXT PRIMARY KEY,
    project_id    TEXT NOT NULL,
    account_id    TEXT REFERENCES accounts(id),
    curator_id    TEXT NOT NULL,
    decision_kind TEXT NOT NULL CHECK (decision_kind IN ('approve_cluster','regenerate_cluster','approve_taxonomy')),
    target_ref    TEXT,
    proposal_hash TEXT,
    critic_hash   TEXT,
    taxonomy_hash TEXT,
    payload_json  TEXT NOT NULL,
    core_op       TEXT NOT NULL,
    core_job_id   TEXT REFERENCES jobs(id),
    created_at    TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_decisions_project ON curator_decisions(project_id);

-- U5: serving_publications
CREATE TABLE IF NOT EXISTS serving_publications (
    project_id                TEXT PRIMARY KEY REFERENCES projects(id),
    compiled_vault_path       TEXT NOT NULL,
    bound_integration_plan_id TEXT,
    bound_corpus_hash         TEXT,
    bound_taxonomy_hash       TEXT,
    status                    TEXT NOT NULL CHECK (status IN ('offline','live','stale')) DEFAULT 'offline',
    published_at              TEXT,
    published_by              TEXT
);
"""

_CONTROL_PLANE_TABLES = (
    "accounts", "sessions", "projects", "upload_tokens", "sources",
    "jobs", "job_events", "curator_decisions", "serving_publications",
    "source_revisions", "upload_sync_sessions",
)


def _utc_now_iso() -> str:
    return datetime.now(UTC).isoformat()


class StateDb:
    """Clone-able handle (thin wrapper) around the single SQLite state Engine.

    SQLAlchemy manages a per-thread connection pool, so the single-writer engine
    worker thread, the read-path threads, and the FastAPI request threads each
    get their own connection over the one WAL file — concurrent readers + one
    writer, exactly like the Rust design.
    """

    def __init__(self, engine: Engine) -> None:
        self.engine = engine

    @classmethod
    def open(cls, path: str) -> StateDb:
        engine = create_engine(
            f"sqlite:///{path}",
            connect_args={"check_same_thread": False, "timeout": 5.0},
            future=True,
        )
        _install_pragmas(engine)
        db = cls(engine)
        db.migrate()
        return db

    @classmethod
    def open_in_memory(cls) -> StateDb:
        # A shared-cache in-memory DB kept alive by a static pool (tests).
        from sqlalchemy.pool import StaticPool

        engine = create_engine(
            "sqlite://",
            connect_args={"check_same_thread": False},
            poolclass=StaticPool,
            future=True,
        )
        _install_pragmas(engine, wal=False)
        db = cls(engine)
        db.migrate()
        return db

    def migrate(self) -> None:
        # DDL uses sqlite3.executescript (multi-statement) on the raw driver
        # connection, kept OUT of a SQLAlchemy-managed transaction because
        # executescript issues its own COMMIT.
        try:
            raw = self.engine.raw_connection()
            try:
                driver: sqlite3.Connection = raw.driver_connection  # type: ignore[assignment]
                driver.executescript(MIGRATION_0001_INIT)
                token_columns = {row[1] for row in driver.execute("PRAGMA table_info(upload_tokens)")}
                if "source_identity" not in token_columns:
                    driver.execute("ALTER TABLE upload_tokens ADD COLUMN source_identity TEXT")
                driver.execute(
                    "UPDATE upload_tokens SET source_identity = COALESCE(registered_source_id, 'src_' || substr(id, 5))"
                    " WHERE source_identity IS NULL"
                )
                driver.execute(
                    "INSERT OR IGNORE INTO schema_migrations(version, applied_at) VALUES (1, ?)",
                    (_utc_now_iso(),),
                )
                driver.execute(
                    "INSERT OR IGNORE INTO schema_migrations(version, applied_at) VALUES (2, ?)",
                    (_utc_now_iso(),),
                )
                raw.commit()
            finally:
                raw.close()
        except Exception as exc:  # pragma: no cover - defensive
            raise EngineError.internal(f"migrate: {exc}") from exc

    def table_names(self) -> set[str]:
        with self.engine.connect() as conn:
            rows = conn.execute(
                text("SELECT name FROM sqlite_master WHERE type='table'")
            ).scalars()
            return set(rows)


def _install_pragmas(engine: Engine, wal: bool = True) -> None:
    @event.listens_for(engine, "connect")
    def _set_pragmas(dbapi_conn: sqlite3.Connection, _rec: object) -> None:  # noqa: ANN401
        cur = dbapi_conn.cursor()
        try:
            if wal:
                cur.execute("PRAGMA journal_mode=WAL")
            cur.execute("PRAGMA foreign_keys=ON")
            cur.execute("PRAGMA busy_timeout=5000")
        finally:
            cur.close()
