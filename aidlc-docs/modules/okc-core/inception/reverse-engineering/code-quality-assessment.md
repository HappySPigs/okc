# Reverse Engineering — Code Quality Assessment

## Assessment scope

The assessment uses current source, package manifests, normative quality gates,
direct test enumeration, and a local macOS arm64 verification run. It does not
claim unsupported-host or release qualification.

## Test inventory

| Suite | Static inventory | Main coverage |
|---|---:|---|
| Rust `#[test]` functions | 130 | Core, AI, app, interop, CLI/TUI, integration contracts |
| Python public test functions | 9 | Runtime expands parametrized cases; API, errors, complete workflow |
| Node test declarations | 10 | Runtime expands subtests; CJS/ESM, types, complete workflow |
| POSIX PTY harness | 1 workflow | Interactive provider/review/compile/verify/restart |

The final current-run outcome is recorded in
[`build-and-test-summary.md`](../../construction/build-and-test/build-and-test-summary.md).

## Quality indicators

| Indicator | Assessment | Evidence |
|---|---|---|
| Type safety | Strong | Rust ownership/types; strict Serde DTOs; Python/TypeScript declarations |
| Unsafe isolation | Strong with one scoped exception | Workspace forbids unsafe; napi macro adapter permits generated unsafe only |
| Determinism discipline | Strong current slice | Canonical JSON, sorted collections, domain hashes, shared golden |
| Hostile-input posture | Strong but incomplete | Bounds, strict paths/JSON, no-follow reads, fail-closed verification; fuzz/TOCTOU gaps remain |
| Test traceability | Strong | `docs/TRACEABILITY.md` maps every REQ to implementation/evidence/status |
| Documentation | Extensive | Specs, stable algorithms, ADRs, guides, current state, history |
| Release readiness | Not ready | QG-006, platform matrices, signing, non-Markdown/Pack and other blockers |

## Positive patterns

- One-way package dependencies and one compiler policy owner.
- Provider-neutral proposal interfaces and deterministic replay/compile boundary.
- Exact hash-bound approval invalidation rather than mutable authorization.
- Plan-derived verification inventory rather than attacker-supplied manifests.
- Bounded queues with independent terminal result retention.
- Explicit negative API tests that keep retired commands/schemas absent.
- Stable cross-language artifact golden.

## Technical debt and open risk

| Area | Current state | Consequence |
|---|---|---|
| Semantic scale | Development path lacks complete chunk/HNSW/union/hierarchy implementation and full workload proof | REQ-PERF-001/QG-006 open |
| Source memory | Accepted entries are buffered | 20 GB target not qualified |
| Materialization | Markdown notes and redirects only | Attachment/Canvas/Base/full-link/Pack release work open |
| Filesystem | macOS/Linux source-content no-follow is partial system coverage | Enumeration, output ancestors, Windows reparse and crash races remain |
| Cancellation/recovery | Bounded cooperative path exists | Fine-grained compile cancellation, process-kill recovery, all-host proof remain |
| Provider matrix | Synthetic/local contract tests | Real-provider conformance and provider-backed host PTY remain |
| Supply chain | Local packaging evidence and workflows exist | Remote two-pass native matrix, protected publication, signing/notarization absent |
| Repository metadata | Actual remote/tag hashes differ from accepted release metadata | Must reconcile before treating archive/repository claims as verified |

## Anti-pattern review

No competing compiler policy package, command-provider adapter, runtime schema
compatibility layer, Pack facade, or binding-owned project state was found.
Several large single-file modules (`integration.rs`, `project_state.rs`,
`okc-interop/src/lib.rs`, `okc-ai/src/lib.rs`, `tui.rs`) increase review cost,
but splitting them must preserve schema identity, public APIs, and test seams.

## Assessment conclusion

The current Markdown-directory vertical slice is coherent and heavily tested,
but the repository must continue to describe itself as a development product.
Passing local tests cannot close the named release gates.
