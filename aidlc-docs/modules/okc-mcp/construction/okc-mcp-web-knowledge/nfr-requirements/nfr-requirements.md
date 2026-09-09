# NFR requirements — web knowledge

Preserve all first-unit source-integrity safeguards. New HTTP dependency must have explicit timeouts, cancellation, no credential forwarding, bounded stream/file/list/scan/response sizes and safe errors. Service availability impact is medium: an unavailable remote corpus prevents requested retrieval but cannot mutate local sources or switch their meaning.

Recovery/backup and local tarball rollback decisions remain inherited. Cache is transient and needs no backup. No availability SLA, distributed deployment, embedding provider or production infrastructure is introduced. Search remains bounded lexical retrieval; no claim of semantic ranking or production-scale performance.

Actual fast-check restores compliance with previously deferred PBT-08/09: structured generators, enabled shrinking, seeded reproduction, existing npm test integration. An initial shrinking counterexample exposed a test expectation issue (analyzeNote deduplicates tags); round-trip now verifies the serialization boundary directly.
