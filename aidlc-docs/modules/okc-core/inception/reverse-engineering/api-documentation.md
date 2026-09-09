# Reverse Engineering — API Documentation

## API shape

OKC exposes native library and command interfaces. It has no current REST API,
web server, MCP server, or public C ABI.

## Rust core API

### Corpus construction

| Item | Contract |
|---|---|
| `SourceId::new(value)` | Validate a 1–128 byte bounded ASCII source identifier; reject `.` and `..` |
| `SourceSpec::directory(id, path)` | Describe a directory source |
| `SourceSpec::archive(id, path)` | Describe a ZIP, `.tar.zst`, or `.tzst` source |
| `CorpusBuilder::new().workspace(path).build(sources)` | Run the stable hostile-input pipeline and return `PreparedCorpus` |
| `PreparedCorpus` | Exposes sealed `IntegrationCorpus`, deterministic `BlockTextMap`, and source count |

### Current integration API

`okc_core::integration` exposes generation-neutral Schema 3 types. The main
record chain is:

```text
IntegrationCorpus
TaxonomyProposal and ApprovalBinding
SynthesisProposal and CriticReport
ClusterApproval and ApprovedClusterRevision
ApprovedIntegrationPlan
CompiledVaultManifest and ProvenanceRecord
```

| Operation | Signature summary | Preconditions |
|---|---|---|
| `ApprovedIntegrationPlan::seal(...)` | Seal corpus, taxonomy approval, cluster revisions, and recordings | Exact complete validation succeeds |
| `compile(plan, output)` | Materialize a new Schema 3 directory | Output absent and plan valid; the current core call has no fine-grained cancellation input |
| `verify(root)` | Return `CompiledVaultManifest` | Entire directory, plan-derived inventory, bytes, provenance, and checksums valid |
| `explain(root, output_path)` | Return one `ProvenanceRecord` | Full artifact verifies; relative output path is safe and uniquely recorded |

Compilation and artifact operations never call a provider.

## Provider API

| Interface/type | Responsibility |
|---|---|
| `StructuredGenerator` | Advertise capabilities and return one structured response |
| `Embedder` | Advertise capabilities and return a validated embedding batch |
| `ProviderProfile` | Exact kind, endpoint, model, secret reference, limits, and options |
| `ProviderClient` | Synchronous bounded adapter for OpenAI, Anthropic, Gemini, Ollama, or OpenAI-compatible HTTP |
| `validate_portable_schema` | Enforce the common strict JSON Schema subset |
| `validate_json_instance` | Independently validate provider JSON output |

Current roles are `embedding`, `organizer`, `synthesis`, and `critic`.
Anthropic does not claim a native embedding capability. The `command` provider
kind is absent.

## Application services

| Service | Principal operations |
|---|---|
| `ProjectStore` | `create`, `open`, source add/rebind/replace, route/language mutation, object access, plan sealing/compile, writer lock |
| `WorkspaceBootstrap` | bounded project/Vault discovery, manual selection, safe project/output suggestion |
| `ProviderService` | load/upsert/remove profiles, resolve credentials, construct/test clients |
| `IntegrationService` | checkpoint, preflight, execute, taxonomy review, cluster review/regeneration, compile latest, verify |
| `ArtifactService` | `verify(path)`, `explain(path, output_path)` for current Schema 3 directories |
| `Worker` | one bounded TUI operation, progress retrieval, cancellation outcome |

Project roots end in `.okc-project` and contain `manifest.json`,
`state.sqlite3`, `objects/`, `workspace/build.sqlite3`, and transient
`project.lock`.

## CLI contract

```text
okc [--project PATH]
okc tui [--project PATH]
okc project create PATH --name NAME --curator ID [--policy-version VERSION] [--language TAG]
okc --project PATH project source add SOURCE_ID SOURCE_PATH [--owner NAME]
okc --project PATH project ai-route set [ROLE] PROFILE
okc provider add|list|show|test|remove
okc --project PATH integrate [--allow-remote-provider --yes]
okc --project PATH integration status [--format human|json]
okc --project PATH review taxonomy show|export|approve
okc --project PATH review cluster list|show|export|approve|regenerate
okc [--project PATH] compile [--integration-plan FILE] --output DIRECTORY [--format human|json]
okc verify DIRECTORY [--format human|json]
okc explain DIRECTORY OUTPUT_PATH [--format human|json]
okc doctor
okc update [stable|latest|VERSION] [--yes]
```

No subcommand on a TTY starts the TUI; no subcommand on a non-TTY prints help
and exits 2. Exit classes are 0, 2, 3, 4, 5, 6, 7, and 70 as defined in the
public SDK/CLI specification.

## Runtime-neutral interop API

`okc-interop` owns `INTEROP_SCHEMA_VERSION = 2`, `OkcClient`, `Project`,
`Job<T>`, `ProviderProfile`, `SourceInput`, progress DTOs, and structured
errors. All operation paths must be explicit absolute lexical paths.

| Client operation group | Methods |
|---|---|
| Client/global | `api_info`, `create_project`, `open_project`, `test_provider`, `verify_artifact`, `explain_artifact` |
| Project state | `manifest`, `status`, `add_source`, `rebind_source`, `replace_sources`, `set_language`, `set_ai_route` |
| Integration | `preflight`, `integrate`, `taxonomy`, `approve_taxonomy`, `clusters`, `approve_cluster`, `regenerate_cluster`, `compile` |
| Job control | `state`, `events`, `cancel`, blocking Rust `result` / runtime-specific result projection |

Jobs use states `queued`, `running`, `cancelling`, `publishing`, `completed`,
`failed`, and `cancelled`. Cancel outcomes are `requested`, `too_late`, and
`already_finished`.

## Python and Node projections

| Concept | Python | Node.js |
|---|---|---|
| Distribution/import | `okc-compiler` / `okc` | `okc-compiler` |
| Client | `OkcClient(...)` | `new OkcClient({...})` |
| Naming | snake_case | camelCase, except opaque source metadata/provider option keys |
| Explanation | `explain_artifact(path, *, output_path=...)` | `explainArtifact(path, {outputPath})` |
| Result wait | `job.result()` | `await job.result()` |
| Type contract | `.pyi` and `py.typed` | `index.d.ts` |

Verification returns schema version, validity, canonical artifact path, and
manifest. Explanation returns schema version, artifact path, and one record.
Neither surface returns a generic artifact-family payload.

## Structured errors

Language errors contain `code`, `category`, `message`, `retryable`, and
`details`. Callers branch on code/category, never parse human text. Recognized
retired markers map to `ARTIFACT_SCHEMA_UNSUPPORTED`; malformed, mixed,
symlinked, unknown, or corrupt inputs remain fail-closed verification errors.

## Schema/version registry

| Boundary | Version |
|---|---:|
| Product/artifact/integration/provider | 3 |
| Public interop DTO | 2 |
| Private project journal | 4 |
| Product version | 0.3.0 |
