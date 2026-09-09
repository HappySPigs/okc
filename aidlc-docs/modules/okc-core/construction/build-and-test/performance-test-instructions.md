# Performance Test Instructions

## Status

Performance qualification is required and not passed. Do not infer a budget
from safety ceilings or historical V1 targets.

## Normative workload

- 10 Vaults
- 100,000 notes
- 20 GB accepted source data
- Complete semantic candidate/integration path
- Report wall time, peak RSS, accepted files/bytes, candidate counts, provider
  tokens/cost evidence, and failure slices on an accepted reference machine

The current specification does not yet accept the numerical time/RSS budget or
reference environment. Resolve that documentation decision before a pass/fail claim.

## Ingestion diagnostic

Build the disposable probe:

```bash
cargo build --locked --release -p okc-core --example corpus_probe
```

Run with a platform RSS measurement tool:

```bash
target/release/examples/corpus_probe SOURCES NOTES BYTES_PER_NOTE
```

The probe must report source counts/bytes, corpus hash, build duration, and
`qg_006_pass: false`. It performs no provider calls and is not the semantic gate.

## Historical diagnostic result

The recorded 2026-09-06 run used 10 Vaults, 100,000 notes, and 25,600,000 bytes:

- Corpus construction: 174.569 seconds
- Maximum RSS: 5,360,336,896 bytes
- Provider calls: zero
- Semantic candidates: not measured
- QG-006: not passed

## Full test prerequisites

1. Accept the reference machine and numerical time/RSS/provider budget.
2. Accept/freeze chunk, batching, HNSW, candidate union, spill, and hierarchy parameters.
3. Implement bounded streaming storage and complete semantic execution.
4. Use representative source distributions and record exact versions/configuration.
5. Run repetitions and report confidence/failure slices, not only a best result.

## Failure response

Do not relax bounds or silently skip semantic work. Identify memory/candidate/
provider bottlenecks, preserve determinism, update the relevant proposed ADR,
and rerun after review.
