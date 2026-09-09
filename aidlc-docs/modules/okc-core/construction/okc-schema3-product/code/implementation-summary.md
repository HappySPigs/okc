# Code Generation — As-Built Implementation Summary

## Outcome

The current Schema 3 product is implemented as seven Cargo packages under one
workspace. This stage changed no runtime implementation. It added one line to
the Rust documentation contract so CI scans `aidlc-docs/`, and changed no
Python, JavaScript, TypeScript, schema, dependency, migration, artifact byte,
or public API.

## Implemented source boundary

### `okc-core`

- Public `CorpusBuilder::build` accepts typed source descriptors and returns
  a sealed `PreparedCorpus`.
- Private source/snapshot/parser/normalization/dedup/planning modules enforce
  immutable hostile-input policy.
- `integration.rs` owns current Schema 3 records, evidence/approval closure,
  Markdown-directory materialization, provenance, verification, explanation,
  and atomic no-replace publication.
- Optional `archives` and `sqlite` features compile separately; provider and
  presentation concepts are absent.

### `okc-ai`

- Defines four semantic roles and generation/embedding capability traits.
- Supports OpenAI, Anthropic, Gemini, Ollama, and OpenAI-compatible HTTP shapes.
- Validates endpoints, profile limits, portable JSON Schema, returned JSON,
  vector count/dimension/finite values, retry policy, and total deadlines.
- Keeps API keys in redacted zeroizing wrappers and rejects credential-bearing options.

### `okc-app`

- Creates/opens current projects with manifest schema 3 and private SQLite journal schema 4.
- Persists content-addressed immutable objects and append-only run/task/exchange/
  approval/plan/output state under a writer lock.
- Provides shared workspace, provider, disclosure, integration/review,
  artifact, worker, credential, and optional updater services.
- Enforces current-plan invalidation, per-call consent, safe output placement,
  and verified-output binding.

### `okc-interop`

- Provides public API v1 with interop DTO schema 2.
- Validates absolute paths and immutable provider profiles without keychain support.
- Runs operations in a bounded scheduler with same-project reservation,
  publication-aware cancellation, 64-event retention, and separate terminal results.
- Maps app/core/provider failures to stable structured error codes/categories.

### `okc` CLI/TUI

- Exposes one current command tree and one ten-screen terminal application.
- Uses shared `okc-app` services for provider, project, integration, review,
  compilation, and verification behavior.
- Uses a pure reducer/effects split, bounded worker, hostile terminal rendering,
  English/Korean labels, ASCII/high-contrast options, and terminal restoration.
- Keeps retired commands/options and Pack surfaces absent through parse tests.

### Python and Node.js

- Python ships `okc-compiler`, imported as `okc`, targeting CPython 3.11 ABI3.
- Node ships `okc-compiler` with ESM, CommonJS, TypeScript, and Node-API 9.
- Both bind only `okc-interop`, accept explicit paths, expose equivalent jobs/
  errors/results, and do not prompt/print/discover cwd/access keychain/update.
- Node converts contract keys to camelCase while preserving arbitrary source
  metadata and provider option keys.

## Persistence layout

```text
Name.okc-project/
  manifest.json
  state.sqlite3
  objects/
  workspace/build.sqlite3
  project.lock                 transient

CompiledVault/
  knowledge/*.md
  legacy/<source-id>/**/*.md
  .okc/integration-plan.json
  .okc/provenance.jsonl
  .okc/manifest.json
  .okc/checksums.txt
```

The tree above is a path inventory, not an ASCII relationship diagram.
`legacy/` is a current redirect namespace, not an old-schema reader.

## Schema and identity compatibility

| Contract | Current value |
|---|---|
| Product | 0.3.0 |
| Artifact/project/provider/integration schema | 3 |
| Private application journal | 4 |
| Interop DTO | 2 |
| Pack profile literal | `okc-tar-zstd-deterministic-v3` reserved in manifest only |
| Cross-language inventory golden | `452ca0671e806a93b4f36f218cf9e62da899f6404c74705c2cf0ca14e413c7e5` |

## Correctly absent implementation

- Schema 1/2 readers, writers, migration, upgrade, aliases, and deprecated facade.
- Current Pack writer/reader and CLI/SDK Pack options.
- Attachment, Canvas, Base carry-through and complete link rewriting.
- Command-provider, MCP server, Obsidian plugin, web service, and registry.
- Experimental ALG-MEM behavior in the default path.
- Registry publication, complete signing/notarization, or stable-release state.

## Partial implementation and blockers

- Complete deterministic chunk/HNSW/candidate-union/hierarchical semantic scale.
- Manual section amendment and persisted sensitive-finding review exceptions.
- Fine-grained cancellation inside the core compile and process-kill recovery.
- Complete descriptor-relative enumeration/managed-output ancestor/Windows
  reparse/hardlink/fuzz/filesystem evidence.
- Native four-target SDK matrices, independent reproducibility, and protected signing/publication.
- QG-006 full 20 GB semantic benchmark under an accepted reference budget.

## Files changed by this Code Generation stage

- **Created/updated**: Markdown under `aidlc-docs/`, documentation-state/
  traceability/history records, and the documentation link test's scan roots.
- **Application code**: none.
- **Build/dependency/configuration**: none.
- **Database migrations**: none.
- **Deployment artifacts**: none.

## Verification handoff

Exact commands and outcomes are in
[`build-and-test-summary.md`](../../build-and-test/build-and-test-summary.md).
