# Application Design — Consolidated View

## Design objective

Preserve one deterministic, auditable compilation policy while allowing
operator and language surfaces to coordinate provider-assisted semantic work.
The design separates proposal generation, human authority, canonical state,
and artifact publication.

## Architecture summary

1. `okc-core` converts immutable hostile sources into a sealed corpus and owns
   every validation/materialization/verification invariant.
2. `okc-ai` provides capability-driven provider adapters and returns proposal data only.
3. `okc-app` owns long-lived project state, disclosure, review, resume, and orchestration.
4. `okc-interop` turns the same application behavior into bounded jobs and stable DTO/errors.
5. `okc`, Python, and Node are thin interaction adapters.

## Key design records

- [Components](components.md)
- [Component methods](component-methods.md)
- [Services](services.md)
- [Dependencies and data flow](component-dependency.md)
- [Unit definition](unit-of-work.md)
- [Unit dependencies](unit-of-work-dependency.md)
- [Requirement/story map](unit-of-work-story-map.md)

## Principal workflows

| Workflow | Entry | Authority transition | Terminal result |
|---|---|---|---|
| Corpus construction | Rust/app source descriptors | none; produces immutable facts | `PreparedCorpus` |
| Semantic integration | app/interop/CLI/TUI | recorded proposals become eligible for explicit review | proposal/critic/task records |
| Review | curator through app surface | exact taxonomy/cluster hashes receive authority | `ApprovedIntegrationPlan` |
| Compilation | approved plan | no new authority; deterministic execution only | published Schema 3 directory |
| Verification/explanation | artifact path | no mutation | typed manifest or provenance record |

## Data and schema boundaries

- Schema 3: corpus/integration/provider/project/artifact records.
- Schema 4: private append-only project journal.
- Interop schema 2: Python/Node result DTOs.
- API v1: public language operation family.
- Schema 1/2 artifacts: recognized only for structured unsupported errors.

## Design non-goals

- No current Pack, non-Markdown materialization, command provider, MCP server,
  Obsidian plugin, cloud runtime, or registry.
- No AI-generated approval, silent remote consent, majority-vote truth, or
  source mutation.
- No adoption of proposed ADR-0028 through ADR-0031 by documentation.

## Validation

The package graph matches `Cargo.toml`; public operations match current source
and the SDK/CLI specification; limits and gaps match `CURRENT_STATE` and
`TRACEABILITY`. Detailed business/NFR rules continue in the Construction unit.
