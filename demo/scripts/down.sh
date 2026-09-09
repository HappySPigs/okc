#!/usr/bin/env bash
# Stop all demo services (okc-web, Caddy, hooks watcher) and unregister okc-mcp.
# Leaves demo/state in place (use reset.sh to wipe it).
set -euo pipefail
source "$(dirname "${BASH_SOURCE[0]}")/lib.sh"

if [ -f "$STATE_DIR/watcher-mine.json" ]; then
	OKC_WATCHER_CONFIG="$STATE_DIR/watcher-mine.json" "$HOOKS_BIN" stop >/dev/null 2>&1 || true
fi
pkill -f "watcher-bin run" 2>/dev/null || true

if [ -f "$STATE_DIR/mcp-mine.json" ]; then
	node "$MCP_CLI" unregister --config "$STATE_DIR/mcp-mine.json" >/dev/null 2>&1 || true
fi

for svc in web caddy watcher curation; do
	pf="$STATE_DIR/$svc.pid"
	[ -f "$pf" ] && { kill "$(cat "$pf")" 2>/dev/null || true; rm -f "$pf"; }
done
pkill -f "curation_provider.py" 2>/dev/null || true
free_port "$WEB_PORT"; free_port "$TLS_PORT"; free_port "$CURATION_PORT"
log "demo services stopped (state preserved at $STATE_DIR)"
