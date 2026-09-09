#!/usr/bin/env bash
# One-command teardown for okc-hooks + okc-mcp (macOS / Linux). Mirrors install.sh.
# Deregisters the hooks daemon (+ removes its installed config) and unregisters
# okc-mcp from the coding agents. Vaults are never touched.
#   ./uninstall.sh [/path/to/okc-install.config.json]
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
CONFIG="${1:-$ROOT/okc-install.config.json}"

log()  { printf '\033[1;34m[okc-uninstall]\033[0m %s\n' "$*"; }
warn() { printf '\033[1;33m[okc-uninstall]\033[0m %s\n' "$*" >&2; }

# okc-hooks teardown (uses the built binary if present).
BIN="$ROOT/okc-hooks/target/release/watcher-bin"
if [ -x "$BIN" ]; then
  log "okc-hooks 제거 중 (서비스 해제 + 설치된 config 삭제)…"
  "$BIN" uninstall --purge-token || warn "okc-hooks uninstall 이 비정상 종료했습니다(이미 제거됐을 수 있음)."
else
  warn "watcher-bin 이 없어 okc-hooks 제거를 건너뜁니다."
fi

# okc-mcp teardown (needs the config to know which agents; --purge removes the generated config).
CLI="$ROOT/okc-mcp/dist/cli.js"
if [ -f "$CLI" ]; then
  umask 077
  TMP="$(mktemp -d "${TMPDIR:-/tmp}/okc-uninstall.XXXXXX")"
  trap 'rm -rf "$TMP"' EXIT
  if [ -f "$CONFIG" ] && command -v node >/dev/null 2>&1; then
    node "$ROOT/scripts/okc-install-render.mjs" "$CONFIG" "$TMP" >/dev/null 2>&1 || true
  fi
  log "okc-mcp 등록 해제 중…"
  if [ -f "$TMP/mcp.json" ]; then
    node "$CLI" unregister --config "$TMP/mcp.json" --purge || warn "okc-mcp unregister 가 비정상 종료했습니다."
  else
    node "$CLI" unregister --purge || warn "okc-mcp unregister 가 비정상 종료했습니다(설정 없이 모든 agent 시도)."
  fi
else
  warn "dist/cli.js 가 없어 okc-mcp 제거를 건너뜁니다."
fi

log "완료."
