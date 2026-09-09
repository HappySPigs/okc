#!/usr/bin/env bash
# Scenario steps 3–4: the hooks watcher DETECTS the MCP edit; the changed "mine"
# is uploaded (transfer via the local upload API on the same token/source, since
# the watcher's HTTPS upload can't trust the local TLS front without an okc-hooks
# change). The web then reports the project as STALE (prior approvals invalidated).
set -euo pipefail
source "$(dirname "${BASH_SOURCE[0]}")/lib.sh"

need jq; need zip
WCFG="$STATE_DIR/watcher-mine.json"
[ -f "$WCFG" ] || die "watcher config missing; run 20_seed.sh first"
PID="$(cat "$STATE_DIR/project.id" 2>/dev/null || true)"
MINE_TOKEN="$(cat "$STATE_DIR/tokens/mine.token" 2>/dev/null || true)"
[ -n "$PID" ] || die "project.id missing; run 20_seed.sh first"
[ -n "$MINE_TOKEN" ] || die "mine token missing; run 20_seed.sh first"
admin_login

# --- (step 3a) show the watcher DETECTING the change ---
log "watcher: forcing a detection cycle (sync-now)…"
OKC_WATCHER_CONFIG="$WCFG" "$HOOKS_BIN" sync-now >/dev/null 2>&1 || true
sleep 2
WLOG="$STATE_DIR/watcher-state/watcher.log"
echo "----- watcher DETECTED the change (from $WLOG) -----"
grep -E '"event":"cycle.start"' "$WLOG" 2>/dev/null | tail -2
echo "  → watcher's own HTTPS upload is untrusted here (local self-signed front):"
grep -E 'UnknownIssuer' "$WLOG" 2>/dev/null | tail -1
echo "  → so the transfer is done via the upload API on the same token/source:"
echo "----------------------------------------------------"

# --- (step 3b) transfer the changed "mine" via the upload API (same token/source) ---
log "uploading changed 'mine' via upload API (same token → new source revision)…"
zipf="$STATE_DIR/mine.zip"; rm -f "$zipf"; ( cd "$VAULTS_DIR/mine" && zip -qr "$zipf" . )
job="$(curl -fsS -X POST "$WEB_BASE/u/$MINE_TOKEN/upload" \
	-F "file=@$zipf;type=application/zip" -F "owner_kind=individual" | jq -r .job_id)"
[ -n "$job" ] && [ "$job" != null ] || die "re-upload did not return a job_id"
i=0; until [ "$i" -ge 60 ]; do
	st="$(curl -fsS "$WEB_BASE/u/$MINE_TOKEN/jobs/$job" | jq -r .state)"
	case "$st" in completed) break ;; failed | cancelled) die "re-upload job $st" ;; esac
	i=$((i + 1)); sleep 1
done
log "changed 'mine' uploaded (new revision of the same source)"

# --- (step 4) the web now reports STALE ---
log "waiting for the web to report the changed source as STALE…"
i=0; until [ "$i" -ge 60 ]; do
	stale="$(api GET "/api/projects/$PID/status" | jq -r .stale)"
	if [ "$stale" = true ]; then
		log "project $PID is STALE — the change is visible on the web (re-merge required)."
		exit 0
	fi
	i=$((i + 1)); sleep 2
done
die "project did not go stale after upload"
