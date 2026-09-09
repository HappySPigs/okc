# Unit Test Execution — okc-mcp (first Unit)

## Run unit tests
```bash
npm test                 # node --import tsx --test tests/*.test.ts
```
Requires **Node ≥ 22.13** (the runner uses `node --test` + `--import tsx`).

## Test files and what they cover
| File | Focus | Key business rules |
|---|---|---|
| `tests/config.test.ts` | Config bounds; config kept outside the Vault | BR-SETUP-3/5 |
| `tests/vault.test.ts` | Path safety, bounds, hash, atomic create/update, single external backup, locks, ROOT_CHANGED, `locate` + backup traceability | BR-PATH-*, BR-BOUND-*, BR-HASH-*, BR-CREATE-1, BR-ATOMIC-*, BR-BACKUP-* |
| `tests/notes.test.ts` | Frontmatter parse/serialize/patch preservation, malformed rejection, audit categories, `fixYamlContent`/`reinforceContent`/`replaceBody` | BR-STRUCT-1/2/4, BR-AUDIT-2/3 |
| `tests/authoring.test.ts` | Centralized `applyMutation` invariants: create preview/apply/refuse-overwrite; update requires matching hash; exactly one backup on accept, none on reject | BR-HASH-3/4, BR-BACKUP-1/2, nfr P2 |
| `tests/properties.pbt.test.ts` | Property-based: hash determinism, partial-merge only-named-keys, empty-patch byte-exact no-op, literal Korean search | NFR-TEST-1, D11 (self-contained generator; see note) |

> **PBT update (2026-09-09)**: fast-check is installed and all property tests use structured generators, automatic shrinking and fixed seeds. Failures print replay seed/path and the shrunk counterexample. `npm test` includes these tests and `tests/web.test.ts`; the previous offline framework gap is resolved.

## Review test results
- **Expected**: all tests pass, 0 failures. Node's TAP summary prints `# pass`/`# fail` totals.
- **Coverage**: no numeric threshold gate (MVP); every REQ/BR and all 21 first-Unit stories are exercised by at least one behavior or property test (see `../okc-mcp-first-unit/code/code-summary.md`).

## Fix failing tests
1. Read the failing assertion + file:line from the TAP output.
2. Fix the code (not the test, unless the test encodes a wrong expectation).
3. Re-run `npm test` until green, then `npm run check`.

## Actual result in this environment (2026-09-08)
**NOT EXECUTED** — host Node is v16.17.1 (< 22.13; lacks `node --test`/`--import`). Pure-logic functions were separately runtime-verified against compiled `dist/` (rejection map, hash, frontmatter preservation, `fixYaml`/`reinforce`/`replaceBody`, audit category projection — all passed). Run `npm test` on a Node ≥ 22.13 host to execute the full suite.

## Session capture follow-up

Run node --import tsx --test tests/capture.test.ts. The suite covers local discovery, section placement, CRLF/BOM preservation, marker identity, re-capture, preflight, partial recovery, readonly/path/response bounds. Two domain-shaped fast-check properties run with seed 20260909 and shrinking enabled (200 and 100 cases).

User-selection regression: capture calls require userSelected=true for the current selected session. Tests cover omitted/false selection, no scanning/writing before selection, ordinary reads without capture, and no standing permission after a completed save. Final full suite: 103/103 pass; see session-capture-summary.md.
