"""``app.config`` — process configuration from environment (names only, never
secret values; API keys are referenced by env-var NAME and read by the binding
when a job starts). Kept intentionally dependency-light (no pydantic-settings).
"""

from __future__ import annotations

import json
import os
from dataclasses import dataclass, field

from app.adapter.dto import ProviderSpecView


@dataclass
class AppConfig:
    state_db_path: str
    projects_root: str
    providers: list[ProviderSpecView] = field(default_factory=list)
    cors_origins: list[str] = field(default_factory=list)
    cookie_secure: bool = False
    skip_engine: bool = False  # test/CI escape hatch (skip building the okc client)
    spa_dist: str = ""

    @classmethod
    def from_env(cls) -> AppConfig:
        state_db_path = os.environ.get("OKC_WEB_STATE_DB", "okc-web-state.db")
        projects_root = os.environ.get("OKC_WEB_PROJECTS_ROOT", os.path.abspath("./projects"))
        cors = os.environ.get("OKC_WEB_CORS_ORIGINS", "")
        cors_origins = [o.strip() for o in cors.split(",") if o.strip()]
        return cls(
            state_db_path=state_db_path,
            projects_root=projects_root,
            providers=_load_providers(),
            cors_origins=cors_origins,
            cookie_secure=os.environ.get("OKC_WEB_COOKIE_SECURE", "0") == "1",
            skip_engine=os.environ.get("OKC_WEB_SKIP_ENGINE", "0") == "1",
            spa_dist=os.environ.get("OKC_WEB_SPA_DIST", ""),
        )


def _load_providers() -> list[ProviderSpecView]:
    """Providers come from OKC_WEB_PROVIDERS (JSON array) or a single provider
    built from OKC_WEB_PROVIDER_* env vars. Only names/endpoints/model + the
    api-key ENV-VAR NAME are read here — never a raw key."""
    raw = os.environ.get("OKC_WEB_PROVIDERS")
    if raw:
        try:
            data = json.loads(raw)
            return [ProviderSpecView.model_validate(item) for item in data]
        except (json.JSONDecodeError, ValueError):
            return []
    endpoint = os.environ.get("OKC_WEB_PROVIDER_ENDPOINT")
    if endpoint:
        return [
            ProviderSpecView(
                name=os.environ.get("OKC_WEB_PROVIDER_NAME", "default"),
                kind=os.environ.get("OKC_WEB_PROVIDER_KIND", "ollama"),
                endpoint=endpoint,
                model=os.environ.get("OKC_WEB_PROVIDER_MODEL", "llama3"),
                api_key_env=os.environ.get("OKC_WEB_PROVIDER_API_KEY_ENV") or None,
            )
        ]
    return []
