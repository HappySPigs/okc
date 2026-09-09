# NFR Requirements — Technology Decisions

## Decision table

| Choice | NFR rationale | Constraint/trade-off | Authority |
|---|---|---|---|
| Rust 2024 core | Strong types, predictable native behavior, cross-platform binaries | Contributor complexity and native matrices | ADR-0001 |
| Framework-first package graph | One policy/provenance implementation | Adapters cannot optimize by duplicating policy | ADR-0002 |
| SQLite local state | Self-contained bounded resume and deterministic queries | Not a distributed registry; cross-file transaction limits | ADR-0005/0019 |
| SHA-256 domain identities + canonical JSON | Determinism, staleness, audit closure | Schema/encoder changes require migration/ADR/goldens | current specs/algorithms |
| Comrak plus Obsidian-aware parser | CommonMark structure plus exact proprietary spans | Parser complexity and upgrade golden review | ADR-0008/0012 |
| BTree/order-before-reduction | Host/order-independent bytes | Potential memory/sort cost at scale | QG-002 |
| Synchronous ureq/rustls providers | Bounded direct I/O without async runtime in app path | Thread-based concurrency and explicit deadlines | ADR-0023/0025 |
| `std::thread` bounded workers | Responsive TUI/runtime-neutral jobs | Cooperative cancellation only; queue sizing fixed | ADR-0019/0025/0026 |
| Content-addressed objects + append-only journal | Resume, immutable evidence, stale authority | More storage and explicit garbage/recovery policy needed | ADR-0024/0025 |
| Sibling stage + native no-replace | Preserve existing/race-winner destinations | Platform/filesystem qualification required | ADR-0014/current output spec |
| PyO3 ABI3 | CPython 3.11+ with one stable native ABI target family | Wheel matrix/host build maintenance | ADR-0026 |
| napi-rs Node-API 9 | Node 22.13+ ESM/CJS native API | Platform addon packaging and scoped macro unsafe | ADR-0026 |
| Typed interop DTO schema 2 | Equivalent language behavior and stable errors | Breaking schema change needs coordinated bindings | ADR-0026/0027 |
| Clap + Ratatui/Crossterm | One native CLI/TUI binary | Terminal-specific accessibility/testing burden | ADR-0019 |
| Feature-gated keyring/updater | Keep bindings side-effect free | CLI build has more platform dependencies | ADR-0021/0026 |
| cargo-dist/Maturin/npm/VitePress | Native release/package/docs pipelines | Supply-chain/reproducibility/signing gates remain | ADR-0020 and release spec |

## Rejected or absent choices

- Vendor model SDK in `okc-core`: rejected to preserve provider neutrality.
- Tokio application worker/runtime: rejected; only the pinned updater owns a
  scoped transitive current-thread runtime exception.
- PostgreSQL/search/graph services for current local compiler: not required.
- Binding-specific business logic or state: rejected in favor of `okc-interop`.
- Current-schema command provider, Pack, MCP server, or Obsidian plugin: absent
  pending separate accepted decisions and evidence.
- Experimental memory/retrieval algorithms in default compilation: prohibited.

## Version discipline

Exact stored/protocol literals, identities, output paths, and public DTO fields
are compatibility constraints. Dependency upgrades that could change Unicode,
parser, serializer, archive, TLS, ABI, or package bytes require their relevant
golden and matrix reviews.
