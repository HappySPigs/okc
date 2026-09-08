# Build and Test Summary

## Scope

- **Execution date**: 2026-09-08
- **Host**: macOS arm64 development checkout
- **Analyzed product**: OKC 0.3.0, Schema 3
- **Change type**: Documentation, one broken relative-link repair, and one documentation-test scan-root extension
- **Application code/dependency/schema change**: None

## Build status

| Check | Result | Notes |
|---|---|---|
| Cargo workspace check | Passed | All seven packages, all targets, all features, locked graph |
| Cargo warnings-as-errors Clippy | Passed | Workspace/all targets/all features |
| Rustfmt check | Passed | Entire Cargo workspace |
| Node native addon build | Passed | Release napi-rs build on current macOS arm64 host |
| VitePress guide build | Passed | `npm ci` then production build |
| Python Rust binding crate | Passed through Cargo | Native package/wheel build and installed API tests were not run |

Build time is not aggregated because commands were independent and some reused
Cargo caches. This run is development evidence, not a reproducible clean build.

## Test execution summary

### Rust unit/integration/doc tests

- **Command**: `cargo test --locked --workspace --all-features --no-fail-fast`
- **Final result**: 130 passed, 0 failed; all doc-test targets passed.
- **Initial baseline result**: 129 application/contract tests passed and the
  one documentation-link target failed because `TRACEABILITY.md` still pointed
  at the pre-relocation `.github` path.
- **Repair**: Changed that link to the current sibling Git-root workflow.
- **Rerun**: Passed, including `documentation_contract` and
  `sdk_output_golden`.

### Node.js

- **Native build**: Passed.
- **Test command**: `npm test --prefix bindings/node`.
- **Sandbox attempt**: 11/13 passed; two loopback server tests failed with
  `listen EPERM 127.0.0.1` due to the sandbox boundary.
- **Approved host rerun**: 13/13 passed, 0 failed.
- **Strict TypeScript**: Passed.
- **Package dry-run**: Passed; root tarball contains the expected six public files.
- **Dependency install**: Locked install completed and reported zero known
  vulnerabilities at that timestamp. The build tool attempted to rewrite its
  own manifest constraint; those unintended generated changes were restored.

### Python

- **Current-run result**: Not executed.
- The Rust `okc-python` package compiled as part of Cargo checks/tests, but no
  Maturin wheel/develop build or Python runtime test was executed.
- Default system Python is 3.9 and has no Maturin/pytest/mypy.
- Homebrew Python 3.12/3.13 exists but has no test tools; temporary `venv`
  bootstrap failed in the host pip trust-store initialization before any
  project build or test ran.
- This is an environment/tooling limitation, not a passing or failing OKC
  Python result. Previously recorded successful wheel/sdist/API/type evidence
  remains historical and is not counted as a fresh pass here.

### Documentation/content validation

- Repository root/docs/guide relative links: Passed through the Rust contract test.
- AI-DLC Markdown relative links: Passed in both the Rust contract test and a
  dedicated read-only validation; fence checks passed in the dedicated validation.
- Mermaid blocks: Passed structural validation; each complex diagram has a text alternative.
- `git diff --check`: Passed after whitespace cleanup.

### Performance

- **Current run**: Not executed.
- **Status**: QG-006 remains not passed.
- The prior 10-Vault/100,000-note/25.6-MB ingestion-only diagnostic is recorded
  in the performance instructions; it is not the required semantic 20 GB workload.

### End-to-end and security

- Rust integration/security/publication/current-schema contracts passed as
  part of the 130-test suite.
- Node full approval/compile/verify/explain workflow passed on the host rerun.
- POSIX TUI PTY and Python E2E were not rerun in this documentation pass;
  their prior evidence remains in `CURRENT_STATE`.
- Coverage-guided fuzzing, process-kill, complete filesystem/Windows, and
  real-provider matrices remain open.

## Exact commands and outcomes

| Command | Outcome |
|---|---|
| `cargo test --locked --workspace --all-features --no-fail-fast` | Passed after link repair; 130/130 |
| `cargo check --locked --workspace --all-targets --all-features` | Passed |
| `cargo clippy --locked --workspace --all-targets --all-features -- -D warnings` | Passed |
| `cargo fmt --all -- --check` | Passed |
| `npm ci --prefix guide` | Passed |
| `npm run docs:build --prefix guide` | Passed |
| `npm ci --ignore-scripts --prefix bindings/node` | Passed outside restricted network sandbox |
| `npm run build --prefix bindings/node` | Passed |
| `npm test --prefix bindings/node` | Passed outside localhost-restricted sandbox; 13/13 |
| `npm run typecheck --prefix bindings/node` | Passed |
| `npm pack --dry-run ./bindings/node --cache /private/tmp/okc-aidlc-npm-cache` | Passed; six files |

The Rust commands required the analyzed host's rustup bin directory in `PATH`.
That environment prefix does not alter command semantics.

## Overall status

| Category | Status |
|---|---|
| AI-DLC documentation through Construction | Complete; reviewed for repository integration on 2026-09-08 |
| Current local Rust checks | Pass |
| Current local Node checks | Pass after host rerun |
| Current guide build | Pass |
| Current Python checks | Not run due host tool environment |
| QG-001 through QG-008 | Not all passed; see `CURRENT_STATE` |
| Ready for stable release/Operations | No |

## Remaining actions

- Resolve the repository remote/archive-tag identity discrepancy.
- Restore a clean supported Python build/test environment for a fresh run.
- Complete all existing product/release blockers; do not infer them closed from
  this documentation and local verification pass.
