# Application Design — Components

## Design premise

The current system is framework-first with thin adapters. Components below are
logical boundaries inside one local native product, not independently deployed
services.

| Component | Package/file boundary | Purpose | Principal interface |
|---|---|---|---|
| Source descriptor | `okc-core/source.rs` | Validate stable source identity and typed directory/archive locator | `SourceId`, `SourceSpec` |
| Corpus builder | `okc-core/corpus.rs` plus private snapshot/parser modules | Convert hostile immutable sources into sealed Schema 3 corpus and block projection | `CorpusBuilder::build -> PreparedCorpus` |
| Integration domain | `okc-core/integration.rs` | Define and validate taxonomy, evidence, dispositions, critic, approvals, and plan closure | Schema 3 DTOs and `ApprovedIntegrationPlan::seal/validate` |
| Materializer/verifier | `okc-core/integration.rs` | Deterministically render, verify, atomically publish, and explain current directories | `compile`, `verify`, `explain` |
| Provider capability adapter | `okc-ai` | Normalize five provider APIs into generation/embedding proposal contracts | `StructuredGenerator`, `Embedder`, `ProviderClient` |
| Project store | `okc-app/lib.rs`, `project_state.rs` | Own project manifest, immutable objects, schema-4 journal, lock, invalidation, task/approval state | `ProjectStore` |
| Provider service | `okc-app/provider_service.rs` | Persist non-secret profiles and resolve environment/keychain credentials just in time | `ProviderService`, `CredentialStore` |
| Integration execution | `okc-app/integration_execution.rs` | Execute/reuse provider tasks and locally convert responses to current proposal records | `IntegrationExecution::execute` |
| Integration review service | `okc-app/integration_service.rs` | Expose checkpoints, preflight, review, regeneration, plan seal, compile, verify | `IntegrationService` |
| Workspace bootstrap | `okc-app/workspace_bootstrap.rs` | Bounded deterministic cwd project/Vault discovery and safe destination suggestion | `WorkspaceBootstrap` |
| Artifact service | `okc-app/artifact_service.rs` | Detect current/retired markers safely and expose typed directory verification/explanation | `ArtifactService` |
| Application worker | `okc-app/worker.rs` | Keep TUI responsive with one bounded operation and publication-aware cancellation | `Worker` |
| Interop facade/scheduler | `okc-interop` | Provide API-v1 DTOs, structured errors, jobs, queue limits, and same-project exclusion | `OkcClient`, `Project`, `Job<T>` |
| CLI adapter | `okc/main.rs`, `commands.rs` | Parse current command grammar and map service outcomes to stable exits/output | `okc` command tree |
| TUI adapter | `okc/tui.rs` | Render the full workflow through a pure reducer/effect boundary | `Model`, `AppEvent`, `Effect`, `reduce`, `run` |
| Python adapter | `bindings/python` | Project interop into Python names/classes/types | `okc.OkcClient`, `Project`, `Job` |
| Node adapter | `bindings/node` | Project interop into ESM/CJS/camelCase/TypeScript | `OkcClient`, `Project`, `Job` |

## Responsibility constraints

- Only `okc-core` decides corpus validity, approval closure, artifact bytes,
  verification, and publication safety.
- Only `okc-ai` performs live provider transport.
- Only `okc-app` owns canonical project workflow/state.
- Only `okc-interop` owns cross-runtime job/error/DTO behavior.
- CLI/TUI/Python/Node adapters translate interaction; they cannot bypass review.
- Future MCP/Obsidian adapters must call the same application boundary.
