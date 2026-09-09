#!/usr/bin/env bash
# Shared config + helpers for the okc full-flow demo. Sourced by every script.
# No secrets are hard-coded except the LOCAL demo admin bootstrap password.

set -euo pipefail

# --- paths ---
DEMO_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"   # demo/
REPO_ROOT="$(cd "$DEMO_DIR/.." && pwd)"                       # okc/
STATE_DIR="${OKC_DEMO_STATE:-$DEMO_DIR/state}"
VAULTS_DIR="$DEMO_DIR/vaults"
CONFIG_DIR="$DEMO_DIR/config"
LOG_DIR="$STATE_DIR/logs"
SCREENSHOT_DIR="$STATE_DIR/screenshots"
COOKIE_JAR="$STATE_DIR/cookies.txt"

# --- modules ---
WEB_DIR="$REPO_ROOT/okc-web"
WEB_PY="$WEB_DIR/backend/.venv/bin/python"
HOOKS_BIN="$REPO_ROOT/okc-hooks/target/release/watcher-bin"
MCP_CLI="$REPO_ROOT/okc-mcp/dist/cli.js"

# --- endpoints ---
WEB_HOST="127.0.0.1"
WEB_PORT="${OKC_DEMO_WEB_PORT:-8000}"
WEB_BASE="http://$WEB_HOST:$WEB_PORT"
TLS_PORT="${OKC_DEMO_TLS_PORT:-8443}"
TLS_BASE="https://localhost:$TLS_PORT"
OLLAMA_BASE="${OLLAMA_HOST:-http://localhost:11434}"
CURATION_PORT="${CURATION_PORT:-8797}"                           # deterministic curation provider (all roles)
CURATION_PY="$DEMO_DIR/scripts/curation_provider.py"

# --- demo admin (LOCAL bootstrap only; not a real secret) ---
ADMIN_EMAIL="${OKC_DEMO_ADMIN_EMAIL:-admin@okc.demo}"
ADMIN_PASSWORD="${OKC_DEMO_ADMIN_PASSWORD:-okc-demo-1234}"

# --- models ---
GEN_MODEL="${OKC_DEMO_GEN_MODEL:-qwen2.5:7b}"
EMBED_MODEL="${OKC_DEMO_EMBED_MODEL:-nomic-embed-text}"

# --- the 5 sources ("mine" is the live/MCP one) + owner labels (bash 3.2 safe) ---
VAULTS="eng product sales support mine"
vault_owner() {
	case "$1" in
	eng) printf '%s' "엔지니어링팀" ;;
	product) printf '%s' "프로덕트팀" ;;
	sales) printf '%s' "세일즈팀" ;;
	support) printf '%s' "고객지원팀" ;;
	mine) printf '%s' "내 개인 볼트" ;;
	*) printf '%s' "$1" ;;
	esac
}

log()  { printf '\033[1;34m[demo]\033[0m %s\n' "$*"; }
warn() { printf '\033[1;33m[demo]\033[0m %s\n' "$*" >&2; }
die()  { printf '\033[1;31m[demo]\033[0m %s\n' "$*" >&2; exit 1; }

need() { command -v "$1" >/dev/null 2>&1 || die "required tool not found on PATH: $1"; }

ensure_dirs() { mkdir -p "$STATE_DIR" "$LOG_DIR" "$SCREENSHOT_DIR" "$STATE_DIR/tokens" \
	"$STATE_DIR/watcher-state" "$STATE_DIR/mcp-state" "$STATE_DIR/projects"; }

# wait_http URL [timeout_s] [curl_flags...]
wait_http() {
	local url="$1" t="${2:-90}"; shift 2 || true
	local i=0
	until curl -fsS "$@" -o /dev/null "$url" 2>/dev/null; do
		i=$((i + 1)); [ "$i" -ge "$t" ] && die "timeout waiting for $url"; sleep 1
	done
}
wait_https() { wait_http "$1" "${2:-90}" -k; }   # -k: local Caddy internal cert

# free a TCP port by killing whatever listens on it (demo ports only)
free_port() {
	local p="$1" pids
	pids="$(lsof -ti "tcp:$p" -sTCP:LISTEN 2>/dev/null || true)"
	[ -n "$pids" ] && { warn "stopping process(es) on :$p ($pids)"; kill $pids 2>/dev/null || true; sleep 1; }
	return 0
}

admin_login() {
	curl -fsS -c "$COOKIE_JAR" -b "$COOKIE_JAR" -H 'content-type: application/json' \
		-d "{\"email\":\"$ADMIN_EMAIL\",\"password\":\"$ADMIN_PASSWORD\"}" \
		"$WEB_BASE/api/auth/login" >/dev/null || die "admin login failed (is okc-web up?)"
}

# api METHOD PATH [JSON_BODY] -> stdout JSON (uses the admin cookie jar)
api() {
	local method="$1" path="$2" body="${3:-}"
	if [ -n "$body" ]; then
		curl -fsS -c "$COOKIE_JAR" -b "$COOKIE_JAR" -X "$method" \
			-H 'content-type: application/json' -d "$body" "$WEB_BASE$path"
	else
		curl -fsS -c "$COOKIE_JAR" -b "$COOKIE_JAR" -X "$method" "$WEB_BASE$path"
	fi
}
