# Four-Module Integration Verification

From repository root after builds:

Run okc-web/backend/.venv/bin/python -m pytest -c okc-web/backend/pyproject.toml scripts/test_integration.py -q.

The test creates disposable local data, starts the real MCP process over stdio, authors a local source note, serializes it with actual hooks Rust/ciborium code, sends it through web's sync receiver, performs actual core integration and curator approval with a deterministic local HTTP provider, compiles/publishes, then runs an actual MCP reader against live uvicorn HTTP with a private read token. It checks fixed revision, content, verification and provenance.

The Rust fixture exercises the real wire encoder; it is not a full installed daemon or native OS watcher test. Hooks unit/integration tests separately verify filesystem/coordinator, retry, target binding and session recovery. The provider is synthetic and deterministic, so this evidence does not certify live AI model quality.

Additional web regressions verify thirteen revisions retain one source, deletion/rename/delete-all, duplicate commits, stale session rejection, chunk resume, token rotation, DB-finalization recovery and storage crash windows.

CI: root web-backend-ci builds both integration participants and runs this same script. hooks-ci defines three OS and two toolchain checks. Remote workflows were configured but not dispatched by this task.
