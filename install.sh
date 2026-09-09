#!/usr/bin/env bash
# One-command local installer for okc-hooks + okc-mcp (macOS / Linux).
#
# Fill okc-install.config.json (copy from okc-install.config.example.json), then run:
#   ./install.sh                         # uses ./okc-install.config.json
#   ./install.sh /path/to/config.json    # or an explicit path
#   SKIP_BUILD=1 ./install.sh            # skip the cargo/npm build step (already built)
#
# It renders your one combined config into per-module configs, builds each module,
# and runs each module's `setup` — installing the hooks auto-start daemon and
# registering okc-mcp into your coding agents (Claude Code / Codex). Tokens are kept
# in 0600 temp files that are deleted on exit; they are never printed.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
CONFIG="${1:-$ROOT/okc-install.config.json}"
EXAMPLE="$ROOT/okc-install.config.example.json"

log()  { printf '\033[1;34m[okc-install]\033[0m %s\n' "$*"; }
fail() { printf '\033[1;31m[okc-install]\033[0m %s\n' "$*" >&2; exit 1; }

[ -f "$CONFIG" ] || fail "설정 파일이 없습니다: $CONFIG
  먼저 예제를 복사해 값을 채우세요:
    cp \"$EXAMPLE\" \"$ROOT/okc-install.config.json\"
  그런 다음 다시 ./install.sh 를 실행하세요."

command -v node >/dev/null 2>&1 || fail "node(>=22) 가 필요합니다."

# 토큰이 담기므로 0700 임시 디렉터리 + 종료 시 정리(정상/오류 모두).
umask 077
TMP="$(mktemp -d "${TMPDIR:-/tmp}/okc-install.XXXXXX")"
trap 'rm -rf "$TMP"' EXIT

log "설정을 모듈별 config 로 변환 중…"
node "$ROOT/scripts/okc-install-render.mjs" "$CONFIG" "$TMP"

INSTALLED=0

if [ -f "$TMP/hooks.json" ]; then
  command -v cargo >/dev/null 2>&1 || fail "okc-hooks 설치에는 cargo(Rust) 가 필요합니다."
  if [ "${SKIP_BUILD:-0}" != "1" ]; then
    log "okc-hooks 빌드 중 (cargo build --release)…"
    ( cd "$ROOT/okc-hooks" && cargo build --release )
  fi
  BIN="$ROOT/okc-hooks/target/release/watcher-bin"
  [ -x "$BIN" ] || fail "watcher-bin 이 없습니다: $BIN (SKIP_BUILD 를 해제하고 다시 실행하세요)"
  log "okc-hooks 설치 중 (자동시작 데몬 등록)…"
  "$BIN" setup --config "$TMP/hooks.json"
  INSTALLED=$((INSTALLED + 1))
fi

if [ -f "$TMP/mcp.json" ]; then
  command -v npm >/dev/null 2>&1 || fail "okc-mcp 설치에는 npm 이 필요합니다."
  if [ "${SKIP_BUILD:-0}" != "1" ]; then
    log "okc-mcp 빌드 중 (npm ci && npm run build)…"
    ( cd "$ROOT/okc-mcp" && npm ci && npm run build )
  fi
  CLI="$ROOT/okc-mcp/dist/cli.js"
  [ -f "$CLI" ] || fail "dist/cli.js 가 없습니다: $CLI (SKIP_BUILD 를 해제하고 다시 실행하세요)"
  log "okc-mcp 설치 중 (coding agent 등록)…"
  node "$CLI" setup --config "$TMP/mcp.json"
  INSTALLED=$((INSTALLED + 1))
fi

[ "$INSTALLED" -gt 0 ] || fail "설치할 모듈이 없습니다 (둘 다 enabled=false 입니까?)."
log "완료! $INSTALLED 개 모듈 설치가 끝났습니다. 제거는 ./uninstall.sh 를 사용하세요."
