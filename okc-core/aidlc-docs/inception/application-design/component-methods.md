# Application Design — Component Methods

Signatures are summarized from current public source. Detailed validation rules
are in the Construction functional design and normative specifications.

## Compiler core

| Component | Method | Input | Output/purpose |
|---|---|---|---|
| `SourceId` | `new(value)` | bounded identifier text | validated source identity |
| `SourceSpec` | `directory(id, path)` | source ID and directory | typed directory descriptor |
| `SourceSpec` | `archive(id, path)` | source ID and supported archive | ZIP/tar.zst descriptor |
| `CorpusBuilder` | `workspace(path)` | optional SQLite path | builder with private persistence |
| `CorpusBuilder` | `build(sources)` | iterable source descriptors | `PreparedCorpus` |
| `IntegrationCorpus` | `seal(policy_hash, documents)` | source-derived records | sorted validated corpus/hash |
| `TaxonomyProposal` | `seal(corpus, clusters, recording_hash)` | complete assignments | validated taxonomy/hash |
| `SynthesisProposal` | `seal(...)` | sections, evidence, dispositions, contradictions | proposal/hash |
| `CriticReport` | `seal(...)` | findings and critic recording | report/hash |
| `ClusterApproval` | `approve(...)` | proposal, critic, curator, omissions/waivers | hash-bound approval |
| `ApprovedIntegrationPlan` | `seal(...)` | complete approved integration | only compile authority |
| Integration module | `compile(plan, output)` | approved plan and absent path | `CompiledArtifact` |
| Integration module | `verify(root)` | current artifact directory | `CompiledVaultManifest` |
| Integration module | `explain(root, output_path)` | current artifact and one safe path | `ProvenanceRecord` |

## Provider layer

| Component | Method | Purpose |
|---|---|---|
| `StructuredGenerator` | `capabilities()` | Declare identity, limits, schema support, and data boundary |
| `StructuredGenerator` | `generate_structured(request, cancellation)` | Return untrusted structured proposal response |
| `Embedder` | `embed(request, cancellation)` | Return bounded validated vectors in request order |
| `ProviderProfile` | `validate()` | Reject invalid endpoints, limits, secret-bearing options, or credential conflicts |
| `ProviderClient` | `new/with_api_key/with_transport` | Construct a provider adapter with isolated transport/secret seams |

## Application layer

| Component | Method group | Purpose |
|---|---|---|
| `ProjectStore` | `create`, `open` | Establish/validate current project layout |
| `ProjectStore` | `add_source_explicit`, `rebind_source_explicit`, `replace_sources_explicit` | Validate absolute binding mutations and invalidate stale authority |
| `ProjectStore` | `put_object`, `read_object` | No-clobber content-addressed object persistence |
| `ProjectStore` | `begin_or_resume_run`, `register_task`, `append_task_status`, `record_exchange` | Append resumable task history |
| `ProjectStore` | `append_approval`, `seal_latest_integration_plan`, `store_approved_integration_plan` | Build immutable review/compile authority |
| `ProjectStore` | `compile`, `record_verified_output` | Hold writer authority through publication/result recording |
| `IntegrationService` | `checkpoint`, `preflight`, `execute` | Determine and advance the semantic workflow |
| `IntegrationService` | `latest_taxonomy`, `approve_taxonomy` | Review/approve complete taxonomy |
| `IntegrationService` | `completed_clusters`, `request_cluster_regeneration`, `approve_cluster` | Review/revise/approve cluster closure |
| `IntegrationService` | `compile_latest`, `verify` | Provider-free compilation and independent verification |
| `WorkspaceBootstrap` | `discover`, `discover_projects`, `discover_vaults`, `manual_vault` | Bounded cwd discovery |
| `WorkspaceBootstrap` | `validate_source_selection`, `suggested_project_path`, `suggested_output_path` | Safe source/project/output placement |
| `ProviderService` | `load_config`, `profile`, `upsert_profile`, `remove_profile`, `client`, `test_profile` | Profile and credential lifecycle |
| `ArtifactService` | `verify`, `explain` | Current directory artifact boundary |
| `Worker` | `submit`, `try_recv`, `cancel` | One-operation TUI worker and progress/cancellation |

## Interop and bindings

| Object | Methods |
|---|---|
| `OkcClient` | API info, project create/open, provider test, artifact verify/explain |
| `Project` | manifest/status, source/language/route mutation, preflight/integrate/review/regenerate/compile |
| `Job<T>` | state, bounded events, cancel, terminal result |
| Python adapter | snake_case equivalents with keyword-only `output_path` for explanation |
| Node adapter | camelCase equivalents with mandatory `{outputPath}` and `RemoteConsent` objects |

## CLI/TUI methods

- `main` parses only the current command grammar and maps error families to
  exits 2/3/4/5/6/7/70.
- `commands.rs` delegates project/provider/integration/review/compile operations.
- `tui::reduce(Model, AppEvent) -> (Model, Vec<Effect>)` owns pure state
  transitions; `tui::run` owns terminal setup/restoration and effect execution.
