#!/usr/bin/env bash
# Bring up a FRESH, isolated okc-web demo instance + the deterministic curation
# provider + the Caddy TLS front. Stops anything on the demo ports first. Demo
# state lives under demo/state and never touches the user's other okc-web state.
set -euo pipefail
source "$(dirname "${BASH_SOURCE[0]}")/lib.sh"

ensure_dirs

log "stopping anything on demo ports (web :${WEB_PORT}, provider :${CURATION_PORT}, tls :${TLS_PORT})..."
free_port "${WEB_PORT}"; free_port "${CURATION_PORT}"; free_port "${TLS_PORT}"

# Deterministic curation provider (all AI roles: embedding + generation). Fully
# local, no external model. It echoes okc-core's identifiers so the strict
# synthesis/critic contracts are satisfied and the merge completes reproducibly.
log "starting curation provider on :${CURATION_PORT} ..."
CURATION_PORT="${CURATION_PORT}" nohup python3 "${CURATION_PY}" >"${LOG_DIR}/curation.log" 2>&1 </dev/null &
echo $! >"${STATE_DIR}/curation.pid"
i=0; until curl -s -o /dev/null "http://127.0.0.1:${CURATION_PORT}/api/embed" 2>/dev/null; do
	i=$((i + 1)); [ "${i}" -ge 20 ] && { warn "curation provider not responding (see ${LOG_DIR}/curation.log)"; break; }; sleep 0.5
done
log "curation provider up on :${CURATION_PORT}"

log "starting okc-web (fresh state DB + projects root under demo/state) ..."
(
	cd "${WEB_DIR}/backend" &&
		OKC_WEB_STATE_DB="${STATE_DIR}/state.db" \
			OKC_WEB_PROJECTS_ROOT="${STATE_DIR}/projects" \
			OKC_WEB_BOOTSTRAP_ADMIN_EMAIL="${ADMIN_EMAIL}" \
			OKC_WEB_BOOTSTRAP_ADMIN_PASSWORD="${ADMIN_PASSWORD}" \
			OKC_WEB_PROVIDERS="$(cat "${CONFIG_DIR}/providers.json")" \
			OKC_WEB_SPA_DIST="${WEB_DIR}/frontend/dist" \
			nohup "${WEB_PY}" -m uvicorn app.main:create_app --factory --workers 1 \
			--host "${WEB_HOST}" --port "${WEB_PORT}" >"${LOG_DIR}/web.log" 2>&1 &
	echo $! >"${STATE_DIR}/web.pid"
)
wait_http "${WEB_BASE}/" 60
admin_login
log "okc-web up at ${WEB_BASE} (admin: ${ADMIN_EMAIL})"

log "starting Caddy TLS front (${TLS_BASE} -> ${WEB_BASE}) ..."
nohup caddy run --config "${CONFIG_DIR}/Caddyfile" >"${LOG_DIR}/caddy.log" 2>&1 </dev/null &
echo $! >"${STATE_DIR}/caddy.pid"
caddy trust >"${LOG_DIR}/caddy-trust.log" 2>&1 || warn "caddy trust failed/needs admin (hooks TLS front; not required for the demo transfer path)"
wait_https "${TLS_BASE}/" 60
log "TLS front up at ${TLS_BASE}"

log "10_up complete."
