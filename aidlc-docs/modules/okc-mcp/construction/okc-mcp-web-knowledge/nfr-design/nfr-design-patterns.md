# NFR design — web knowledge

Use fixed-origin adapter, typed refusal, immutable snapshot reads and bounded per-operation cache. Revalidate publication and authorization on every new operation. Explicit local selection provides an intentional alternative, not automatic failover. Streaming response accumulation enforces bytes before JSON/text parsing; metadata schemas prevent identity drift. File/provenance URLs are built with URLSearchParams so note paths cannot become endpoints.

Keep the hardened local mutation pipeline and immutable-artifact authoring guard. Remote reader has a separate path policy matching published core paths, since original legacy spelling can be NFD/uppercase/deep. Framework-backed property testing checks encoding and authoring invariants; real HTTP/stdio tests check operational semantics.
