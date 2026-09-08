# NFR Requirements — Plan (autopilot, minimal) · unit `okc-mcp-first-unit`

> **AUTOPILOT DECISION (2026-09-08)**: Minimal depth — NFRs are already enumerated in requirements §5; this stage consolidates them per-unit and records tech-stack decisions. Tech stack is largely fixed by requirements (Node/TS, npm tarball, stdio). Questions auto-answered with recommended MVP defaults.

## Artifacts (checklist)
- [x] `nfr-requirements/nfr-requirements.md`
- [x] `nfr-requirements/tech-stack-decisions.md`

## Auto-answered questions
- **Scalability**: single local user, single Vault, on-demand process — no horizontal scaling. Bound by `maxFileBytes/maxFileCount/maxResponseBytes`. [Answer]: N/A-scaling / bounded
- **Performance**: interactive local latency; no hard SLA; audit/search must stay within bounds and terminate deterministically. [Answer]: best-effort local
- **Availability**: on-demand CLI/stdio; no uptime SLA; N/A DR beyond single external backup (RPO=last save, RTO=manual). [Answer]: N/A-uptime
- **Security**: first-class — REQ-008 bounded authority, REQ-011 trust boundary; treat notes as untrusted; no network. [Answer]: enforce product security
- **Tech stack**: Node.js LTS + TypeScript + MCP stdio SDK + comment-preserving YAML + property-based testing. [Answer]: recommended (see tech-stack-decisions.md)
- **Reliability**: typed fail-safe rejections; atomic writes; single pre-change backup. [Answer]: recommended
- **Maintainability/Testing**: PBT (partial) for pure functions + serialization round-trips; behavior tests for path-safety/conflict/backup. [Answer]: recommended
