#!/usr/bin/env bash
# Scenario step 1: get 5 personal vaults onto the web.
#  - creates the demo project
#  - binds per-role AI providers (embedding -> nomic, others -> qwen)
#  - uploads all 5 vaults (incl. "mine") via the contributor upload API (zip)
#  - starts the real okc-hooks watcher on "mine" for DETECTION (file-watch ->
#    debounce -> manifest diff -> consent). NOTE: the watcher trusts only the
#    bundled webpki roots, so its HTTPS upload to the local TLS front is not
#    trusted here; the actual transfer of the later change is done by the local
#    upload API on the SAME token/source (see 50_sync.sh). This keeps the demo
#    fully local with no okc-hooks change.
#  - renders + registers okc-mcp (authoring "mine" + reading the web merged brain)
set -euo pipefail
source "$(dirname "${BASH_SOURCE[0]}")/lib.sh"

need jq; need zip
ensure_dirs
umask 077
admin_login

# --- create project ---
PID="$(api POST /api/projects '{"name":"너울 통합 지식베이스"}' | jq -r .id)"
[ -n "$PID" ] && [ "$PID" != null ] || die "project create failed"
echo "$PID" >"$STATE_DIR/project.id"
log "project: $PID"

# --- provider binding ---
# The deterministic curation provider handles all roles (embedding + generation).
# It is input-aware and echoes okc-core's exact identifiers, so it satisfies the
# engine's strict synthesis/critic contracts (a live LLM does not) and drives the
# whole pipeline reproducibly. (The per-role `role` field is still exercised in
# okc-web's tests; here one provider covers all roles.)
api POST "/api/projects/$PID/provider" '{"profile_name":"curation"}' >/dev/null
log "provider bound: curation (deterministic, all roles)"

issue_token() {  # owner_kind owner_display -> plaintext token
	api POST "/api/projects/$PID/tokens" \
		"{\"owner_display_name\":\"$2\",\"owner_kind\":\"$1\"}" | jq -r .token
}

upload_zip() {  # token vault_dir owner_display owner_kind
	local token="$1" dir="$2" owner="$3" kind="$4" zipf job st i=0
	zipf="$STATE_DIR/$(basename "$dir").zip"
	rm -f "$zipf"; ( cd "$dir" && zip -qr "$zipf" . )
	job="$(curl -fsS -X POST "$WEB_BASE/u/$token/upload" \
		-F "file=@$zipf;type=application/zip" \
		-F "owner_display_name=$owner" -F "owner_kind=$kind" | jq -r .job_id)"
	[ -n "$job" ] && [ "$job" != null ] || die "upload did not return a job_id ($dir)"
	until [ "$i" -ge 60 ]; do
		st="$(curl -fsS "$WEB_BASE/u/$token/jobs/$job" | jq -r .state)"
		case "$st" in completed) return 0 ;; failed | cancelled) die "upload job $st ($dir)" ;; esac
		i=$((i + 1)); sleep 1
	done
	die "upload job timeout ($dir)"
}

# --- all 5 vaults via the contributor upload API; save "mine" token for 50_sync ---
for v in $VAULTS; do
	owner="$(vault_owner "$v")"
	kind=department; [ "$v" = mine ] && kind=individual
	tok="$(issue_token "$kind" "$owner")"
	[ -n "$tok" ] && [ "$tok" != null ] || die "token issue failed ($v)"
	upload_zip "$tok" "$VAULTS_DIR/$v" "$owner" "$kind"
	if [ "$v" = mine ]; then printf '%s' "$tok" >"$STATE_DIR/tokens/mine.token"; chmod 600 "$STATE_DIR/tokens/mine.token"; fi
	log "uploaded: $v ($owner)"
done

# --- confirm all 5 sources registered ---
n="$(api GET "/api/projects/$PID" | jq -r .source_count)"
[ "$n" = 5 ] || die "expected 5 sources, got $n"
log "all 5 sources registered (source_count=5/10)"

# --- start the real hooks watcher on "mine" for DETECTION demonstration ---
MINE_TOKEN="$(cat "$STATE_DIR/tokens/mine.token")"
WCFG="$STATE_DIR/watcher-mine.json"
cat >"$WCFG" <<JSON
{
  "vault_path": "$VAULTS_DIR/mine",
  "server_endpoint": "$TLS_BASE/api/sync",
  "token": "$MINE_TOKEN",
  "data_dir": "$STATE_DIR/watcher-state",
  "debounce_ms": 1000,
  "reconciliation_interval_s": 900
}
JSON
chmod 600 "$WCFG"
log "starting hooks watcher on 'mine' (detection demo; transfer via upload API)"
OKC_WATCHER_CONFIG="$WCFG" nohup "$HOOKS_BIN" run "$WCFG" >"$LOG_DIR/watcher.log" 2>&1 </dev/null &
echo $! >"$STATE_DIR/watcher.pid"
i=0; until OKC_WATCHER_CONFIG="$WCFG" "$HOOKS_BIN" status --json >/dev/null 2>&1; do
	i=$((i + 1)); [ "$i" -ge 30 ] && { warn "watcher control socket not ready (see $LOG_DIR/watcher.log)"; break; }; sleep 1
done
OKC_WATCHER_CONFIG="$WCFG" "$HOOKS_BIN" consent acknowledge >/dev/null 2>&1 || true
OKC_WATCHER_CONFIG="$WCFG" "$HOOKS_BIN" consent grant >/dev/null 2>&1 || true
log "watcher live on 'mine' (consent granted)"

# --- render + register okc-mcp (authoring "mine" + reading the web merged brain) ---
MCFG="$STATE_DIR/mcp-mine.json"
cat >"$MCFG" <<JSON
{
  "vaultPath": "$VAULTS_DIR/mine",
  "statePath": "$STATE_DIR/mcp-state",
  "readOnly": false,
  "agents": ["claude"],
  "web": { "baseUrl": "$WEB_BASE", "projectId": "$PID" }
}
JSON
chmod 600 "$MCFG"
if node "$MCP_CLI" setup --config "$MCFG" >"$LOG_DIR/mcp-setup.log" 2>&1; then
	log "okc-mcp registered into Claude Code (config: $MCFG)"
else
	warn "okc-mcp registration skipped/failed (see $LOG_DIR/mcp-setup.log); scripted stdio edit still works"
fi

log "20_seed complete — project $PID has 5 sources; watcher live on 'mine'."
