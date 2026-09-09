# NFR Design — Plan (autopilot, standard) · unit `okc-mcp-first-unit`

> **AUTOPILOT DECISION (2026-09-08)**: Incorporates NFR requirements into design patterns + logical components, and resolves the resiliency-extension design-stage questions (RESILIENCY-04/08/14/15) deferred from Requirements. All auto-answered with MVP defaults appropriate to a local single-process offline tool.

## Artifacts (checklist)
- [x] `nfr-design/nfr-design-patterns.md`
- [x] `nfr-design/logical-components.md`

## Auto-answered questions (incl. deferred resiliency)
- **Resilience patterns**: fail-safe typed rejections + atomic write + single external backup; no retries/circuit-breakers (no remote deps). [Answer]: recommended
- **Scalability patterns**: none — single local process; bounded resources. [Answer]: N/A
- **Performance patterns**: streaming/bounded iteration for list/audit; no caches. [Answer]: bounded, no cache
- **Security patterns**: single path-safety choke point; capability minimization (safe tool surface); untrusted-input handling. [Answer]: recommended
- **RESILIENCY-04 (CI/CD, rollback, deployment)**: local `npm` build/test now, CI deferred; rollback = reinstall previous tarball version (version-pinned); deployment = direct local install. [Answer]: lightweight/version-pinned/direct
- **RESILIENCY-08 (regional topology)**: N/A — no cloud/region; single local machine. [Answer]: N/A
- **RESILIENCY-14 (resiliency testing)**: property-based + behavior tests now; chaos/DR drills deferred to Operations. [Answer]: PBT+behavior now, defer chaos
- **RESILIENCY-15 (incident response)**: lightweight — actionable structured errors + docs; no formal on-call/COE process for a local tool. [Answer]: lightweight
