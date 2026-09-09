#!/usr/bin/env bash
# Launch okc-web with Claude-via-Bedrock generation.
#
# okc-core has no native Bedrock transport, so this starts a localhost shim
# (scripts/bedrock_claude_shim.py) that speaks okc-core's `anthropic` wire
# format and forwards to Bedrock's Converse API, then starts okc-web pointed
# at the shim. The Bedrock key lives ONLY in the shim's process env; okc-web
# talks to the shim over localhost with no auth.
#
# NOTE: this key/policy is generation-only (no Bedrock embeddings), so the AI
# integrate step will report a typed "embedding capability missing" error.
# Upload / freeze / serve / review and Claude generation all work.
#
# Required in your environment before running:
#   export AWS_BEARER_TOKEN_BEDROCK=ABSK...        # your Bedrock API key
# Optional overrides:
#   export AWS_REGION=ap-northeast-2
#   export BEDROCK_MODEL=global.anthropic.claude-sonnet-5   # or claude-haiku-4-5 / claude-opus-4-8
#   export SHIM_PORT=8799
#   export OKC_WEB_PORT=8000
#   export OKC_WEB_BOOTSTRAP_ADMIN_EMAIL=admin@example.com
#   export OKC_WEB_BOOTSTRAP_ADMIN_PASSWORD=change-me
set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"   # okc-web/
py="$here/backend/.venv/bin/python"

: "${AWS_BEARER_TOKEN_BEDROCK:?set AWS_BEARER_TOKEN_BEDROCK to your Bedrock API key}"
export AWS_REGION="${AWS_REGION:-ap-northeast-2}"
export BEDROCK_MODEL="${BEDROCK_MODEL:-global.anthropic.claude-sonnet-5}"
export SHIM_PORT="${SHIM_PORT:-8799}"
OKC_WEB_PORT="${OKC_WEB_PORT:-8000}"

echo "starting Bedrock shim on :$SHIM_PORT (model=$BEDROCK_MODEL, region=$AWS_REGION)"
"$py" "$here/scripts/bedrock_claude_shim.py" &
shim_pid=$!
trap 'kill "$shim_pid" 2>/dev/null || true' EXIT
sleep 1.5

export OKC_WEB_BOOTSTRAP_ADMIN_EMAIL="${OKC_WEB_BOOTSTRAP_ADMIN_EMAIL:-admin@example.com}"
export OKC_WEB_BOOTSTRAP_ADMIN_PASSWORD="${OKC_WEB_BOOTSTRAP_ADMIN_PASSWORD:-okc-admin-123}"
export OKC_WEB_PROVIDER_NAME=bedrock-claude
export OKC_WEB_PROVIDER_KIND=anthropic
export OKC_WEB_PROVIDER_ENDPOINT="http://127.0.0.1:$SHIM_PORT"
export OKC_WEB_PROVIDER_MODEL="$BEDROCK_MODEL"

echo "starting okc-web on http://127.0.0.1:$OKC_WEB_PORT"
cd "$here/backend"
exec "$py" -m uvicorn app.main:create_app --factory --workers 1 \
  --host 127.0.0.1 --port "$OKC_WEB_PORT"
