#!/usr/bin/env bash
# Preflight: verify every dependency the demo needs is present and healthy.
# Does NOT install models (that is a separate long-running pull); it checks them.
set -euo pipefail
source "$(dirname "${BASH_SOURCE[0]}")/lib.sh"

log "preflight: tools"
need curl; need jq; need node; need lsof; need caddy; need python3

log "preflight: modules built"
[ -x "$WEB_PY" ]     || die "okc-web venv python missing: $WEB_PY (build the binding + venv first)"
[ -x "$HOOKS_BIN" ]  || die "okc-hooks watcher-bin missing: $HOOKS_BIN (cargo build --release --workspace)"
[ -f "$MCP_CLI" ]    || die "okc-mcp dist/cli.js missing: $MCP_CLI (npm ci && npm run build)"

log "preflight: deterministic curation provider"
[ -f "$CURATION_PY" ] || die "curation provider missing: $CURATION_PY"

log "preflight: frontend SPA build"
if [ ! -f "$WEB_DIR/frontend/dist/index.html" ]; then
	warn "frontend/dist missing — building (npm install && npm run build)…"
	( cd "$WEB_DIR/frontend" && npm install >/dev/null 2>&1 && npm run build >/dev/null 2>&1 ) \
		|| die "frontend build failed; build it manually in okc-web/frontend"
fi

log "preflight OK"
