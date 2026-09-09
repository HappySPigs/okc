# Application Design — Services

## Service model

These are in-process application services, not network services.

### WorkspaceBootstrap

- Resolves an explicit project first.
- Otherwise checks the bounded hidden default, safe sibling, and direct-child projects.
- Discovers only cwd/direct-child Vault directories and direct supported archives.
- Rejects symlinks, managed directories, nested/overlapping sources, and unsafe placement.

### ProviderService

- Loads strict schema-3 profile configuration.
- Stores only environment-variable or opaque keychain references.
- Resolves a secret immediately before use and passes a zeroizing value to the client.
- Never falls back to plaintext when native credential storage is unavailable.
- Produces a capability-tested `ProviderClient` for `IntegrationService`.

### IntegrationExecution

- Builds/reuses deterministic task/cache identities.
- Runs embedding, candidate, organizer, synthesis, and critic work.
- Persists complete validated exchanges before they can be reused.
- Converts provider JSON into locally sealed domain records.
- Does not grant approval or publish output.

### IntegrationService

- Reads current project configuration and returns the next checkpoint.
- Builds the corpus and scans block/metadata content before disclosure.
- Authorizes every remote cache miss separately.
- Coordinates taxonomy and cluster review, regeneration, and invalidation.
- Seals a plan only after complete approval closure.
- Compiles and verifies through `okc-core` while preserving project locking.

### ArtifactService

- Detects artifact family/schema before full decoding.
- Accepts only a current Schema 3 directory for verify/explain.
- Returns a structured unsupported error for recognizable retired markers.
- Fails closed for symlinked, mixed, malformed, oversized, corrupt, or unknown inputs.

### Worker and interop scheduler

- TUI `Worker` permits one active operation with one command slot and 64
  progress events; completion is stored separately.
- Interop accepts 1–64 worker threads and at most 64 queued jobs per client.
- A process-wide canonical project reservation prevents two clients from
  mutating the same project while the on-disk lock protects across processes.
- Cancellation is cooperative before publication; at/after the barrier it is too late.

## Service orchestration

| Workflow | Orchestrator | Collaborators | Durable result |
|---|---|---|---|
| Project setup | CLI/TUI/interop | WorkspaceBootstrap, ProjectStore | manifest, journal, object/workspace dirs |
| Provider setup | CLI/TUI | ProviderService, CredentialStore | non-secret global profile config and optional keychain value |
| Integration | IntegrationService | ProjectStore, IntegrationExecution, ProviderService, core validators | tasks, recordings, proposals, approvals, sealed plan |
| Compilation | IntegrationService or direct core caller | ProjectStore, core materializer | verified new directory and optional project output pointer |
| Artifact inspection | ArtifactService | core verifier/explainer | typed manifest or provenance record |
| Language automation | `OkcClient`/`Project` | scheduler, application services | typed interop-schema-2 result/error |

## Transaction and failure boundaries

- Project object bytes are synchronized before journal pointers commit.
- Source/config invalidation commits before manifest replacement; a later
  failure may require reintegration but cannot resurrect old approval.
- SQLite and filesystem manifest update are not one atomic transaction; this
  limitation remains explicit.
- Compilation has an atomic final namespace commit, not a multi-resource
  transaction with any future Pack.
- Publication-success/parent-sync-failure returns a distinct durability-uncertain state.
