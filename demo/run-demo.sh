#!/usr/bin/env bash
# One-command okc full-flow demo.
#
#   ./run-demo.sh            # run against existing state (idempotent bring-up)
#   ./run-demo.sh --fresh    # wipe demo state first, then run from scratch
#   OKC_DEMO_HEADED=1 ./run-demo.sh --fresh   # watch the browser live
#
# Phases:
#   00 preflight → 10 up (fresh web + Caddy + warm models) → 20 seed (5 vaults)
#   → BASELINE merge via Playwright (revision 1)
#   → 40 MCP edits "mine" → 50 watcher detects + upload → project STALE
#   → RE-MERGE via Playwright (revision 2)
set -euo pipefail
DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$DIR"
source scripts/lib.sh

if [ "${1:-}" = "--fresh" ]; then
	log "== resetting demo state =="
	bash scripts/reset.sh || true
fi

log "== 00 preflight =="; bash scripts/00_preflight.sh
log "== 10 bring-up =="; bash scripts/10_up.sh
log "== 20 seed (5 vaults) =="; bash scripts/20_seed.sh

log "== BASELINE merge (Playwright) → revision 1 =="
( cd e2e && npx playwright test baseline.spec.ts )

log "== scenario step 2: MCP edits 'mine' =="; node scripts/40_mcp_edit.mjs
log "== scenario steps 3-4: watcher detects + upload → STALE =="; bash scripts/50_sync.sh

log "== RE-MERGE (Playwright) → revision 2 =="
( cd e2e && npx playwright test remerge.spec.ts )

log "== DEMO COMPLETE =="
log "screenshots: $SCREENSHOT_DIR"
log "publication history now has revision 1 (baseline) + revision 2 (after the MCP edit)."
