# NFR Design — Patterns

## Determinism patterns

| Pattern | Implementation | Protects |
|---|---|---|
| Canonical encoding | Recursively sorted generic JSON; typed maps use deterministic order | hashes, plans, recordings, artifacts |
| Domain-separated identity | Distinct SHA-256 domains for content/entity/plan/provenance roles | type confusion and silent reuse |
| Explicit ordering | Sort sources, files, documents, blocks, metadata, clusters, evidence, files | host/input/thread-order independence |
| Record then replay | Provider response becomes immutable hashed build input | offline reproducibility despite AI nondeterminism |
| Plan-owned bytes | Materializer uses only validated approved plan | no live inference during compile |
| Cross-surface golden | Rust/Python/Node compare exact inventory digest | adapter parity |

## Security and privacy patterns

| Pattern | Implementation | Failure behavior |
|---|---|---|
| Hostile-input boundary | strict UTF-8/paths/JSON/YAML, archive bounds, no source symlink following | reject before semantic use |
| Least authority | provider receives data only; adapters call typed application/core operations | no filesystem/tool/approval authority |
| Pre-disclosure gate | scan blocks and metadata; authorize each role/cache miss | sensitive remote route or missing consent fails |
| Secret reference | env/keychain account only, zeroizing value at call | no plaintext fallback or persisted secret |
| Fail-closed artifact detection | schema/family before full decode; plan-derived inventory | mixed/unknown/corrupt/symlinked content rejected |
| Terminal sanitization | visible safe representation of controls; no OSC authority | hostile display text cannot control terminal |

## Reliability patterns

| Pattern | Implementation | Known limit |
|---|---|---|
| Immutable authority chain | content-addressed proposals/critics/approvals/plans | requires explicit new revision for edits |
| Append-only state machine | runs/tasks/events/exchanges/revisions/approvals in SQLite | no complete crash-intent recovery yet |
| Conservative invalidation | journal fresh run before manifest replacement | may require reintegration after partial update |
| Bounded scheduler | sync channels, 1–64 workers, 64 queued/events | overload returns typed resource error |
| Separate completion slot | terminal result does not compete with progress | progress may coalesce by design |
| Cooperative cancellation | token checks and explicit publishing barrier | current core compile is not finely cancellable |
| Explicit post-commit state | durability-uncertain error preserves visible output | physical power-loss guarantees platform-specific |

## Publication patterns

1. Validate plan and destination.
2. Create a restrictive sibling stage.
3. Write with create-new semantics.
4. Synchronize every file and directories bottom-up where supported.
5. Independently regenerate/verify the stage.
6. Use one atomic no-replace namespace commit.
7. Synchronize parent or return the distinct post-commit error.
8. Keep/disarm the stage guard at the precise ownership boundary.

This is one directory publication. It is not an atomic transaction with a Pack.

## Portability patterns

- Portable paths use strict relative components, NFC and pinned full case-fold
  collision keys, and file/directory prefix collision checks.
- Linux/macOS source content uses descriptor-relative no-follow opens.
- Final publication uses target-specific safe primitives; unsupported targets fail closed.
- SDK callers compare path identity because Windows canonical spelling may use
  extended-length syntax.
- LF fixture bytes avoid host text-mode translation in cross-language goldens.

## Maintainability patterns

- Ports/adapters keep core independent from provider/UI/runtime details.
- Public core surface is small; private parser/analysis machinery stays hidden.
- Feature gates remove keychain/updater from binding dependency behavior.
- Strict typed DTOs reject unknown fields and nested duplicate keys.
- Negative contract tests ensure retired commands/schemas do not reappear.
- Specs, algorithms, ADRs, traceability, current state, guide, and AI-DLC docs
  each have distinct authority roles.

## Scalability patterns and gaps

Implemented bounds, SQLite persistence, Rayon collection, MinHash candidate
limits, provider batch limits, and bounded queues prevent unbounded individual
operations. They do not yet satisfy the complete semantic-scale contract.
Streaming accepted content, fixed HNSW/chunk/spill profiles, hierarchical
synthesis, and full workload evidence remain proposed/open.
