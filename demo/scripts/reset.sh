#!/usr/bin/env bash
# Full reset: stop services and WIPE demo state (DB, projects, tokens, watcher/mcp state).
set -euo pipefail
source "$(dirname "${BASH_SOURCE[0]}")/lib.sh"

bash "$DEMO_DIR/scripts/down.sh" || true
rm -rf "$STATE_DIR"
# Remove the note the MCP step authors into "mine", so re-runs start clean.
rm -f "$VAULTS_DIR"/mine/세션-메모-*.md 2>/dev/null || true
log "demo state wiped: $STATE_DIR"
