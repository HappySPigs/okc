# U5 Versioned Serving Continuation Plan

Authority: 2026-09-09 user requests to verify every module against their four-module requirements and design/implement gaps. Relevant requirements: root INT-05/06, existing E5-S1..S5; original no-winner-select and explicit curator publication remain.

Functional design: record compiled path and verified manifest; immutable snapshot identified by SHA-256 of manifest bytes; publish/restore only verified artifacts; old revisions available while current project is published. Source-set and project-manifest fingerprints determine stale without confusing them with compiled corpus hash. Read access is explicit public/private plus hashed revocable project tokens. Admin authorization precedes all changes.

NFR design: reuse SQLite transactions, single-writer core queue, bounded metadata, strict relative paths and no symlink escape; verify output before registration; reject missing revision instead of switching; no plaintext credential persistence; preserve original files.

- [x] Read current requirements, serving implementation/design/test evidence and root gap analysis.
- [x] Coordinate revision/auth response schema with MCP and source-state ownership with upload unit.
- [x] Add serving snapshot/access/token persistence without modifying upload-owned state migration file.
- [x] Record actual successful compile path and publication identity from verified artifact.
- [x] Add fixed-revision read/verify/explain, history/restore, admin access/token endpoints.
- [x] Add meaningful tests for actual artifact publication, revision consistency, token isolation/revocation and freshness.
- [x] Run serving regressions plus web aggregate gates, document final contracts and evidence.

No new cloud infrastructure; optional read authorization extends the existing single-process application. Current user direction authorizes implementation through verification; routine phase gates do not require repeated permission requests.

Final evidence: 114 full backend tests passed, including 4 real-core serving/recovery tests; 8 frontend tests/build passed. Root four-module bridge passed. Ruff clean and mypy clean across 55 source/test files. Final serving contract: [versioned-serving.md](../u5-serving/code/versioned-serving.md).
