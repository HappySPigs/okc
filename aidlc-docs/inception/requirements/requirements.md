# Continuous OKC Integration Requirements

The 2026-09-09 requests authorize checking the existing AI-DLC specifications and implementing gaps in the originally described four-module workflow. This continues the completed root review. Complexity: cross-module API/state changes; comprehensive boundary analysis with module-local designs.

| ID | Required behavior | Owner |
|---|---|---|
| INT-01 | Hooks and web speak one authenticated upload protocol with durable final acknowledgement. | hooks + web |
| INT-02 | Repeated changes replace the same source revision; deletions/renames, retry and concurrency do not create duplicate sources or overwrite newer commits. | hooks + web + existing core rebind |
| INT-03 | Configured web is MCP's default read/search source; only absent web configuration selects local automatically. | MCP |
| INT-04 | MCP authors a local source Vault independently of its remote read source; initialize a conventional source Vault safely. | MCP |
| INT-05 | Web publishes verified core output with fixed revision, source/plan/taxonomy identity, provenance and freshness; clients can pin reads. | web + MCP |
| INT-06 | Optional private project reads use revocable read-only Bearer tokens independent of upload credentials; explicit existing public mode remains supported. | web + MCP |
| INT-07 | Core remains authoritative for evidence-preserving review/compile/verify. Existing approval/regenerate/omission/minor-waive implements conflict choice. | core + web |
| INT-08 | Automatic upload is observable and retryable; merge/review/compile/publish remain explicit administrator actions under existing approved policy. | hooks + web |
| INT-09 | Verify module gates and actual cross-module HTTP/CLI contracts; distinguish offline deterministic integration from live AI-provider results. | root |
| INT-10 | Read all module AI-DLC inventories and reconcile current requirements/design/status/test evidence; preserve histories. | all |

Existing product decisions resolve ambiguity: Markdown-focused compilation, <=10 distinct sources, no destructive semantic winner choice, no automatic human approval, no bidirectional source synchronization, no mandatory vector index. The later cross-module request supersedes deferral of web/MCP and hooks/web integration. It does not implicitly require every deferred product roadmap feature.

NFR: bounded hostile-input reads, immutable snapshots, atomic publication pointers, hashed token storage, no credentials in URLs/redirect forwarding for new APIs, recoverable retries, serialized core mutations, idempotent source commits, meaningful regression tests. Existing single-process/local-storage architecture stays authoritative.

Module requirements: [core](../../../okc-core/aidlc-docs/inception/requirements/requirements.md), [hooks](../../../okc-hooks/aidlc-docs/inception/requirements/requirements.md), [MCP](../../../okc-mcp/aidlc-docs/inception/requirements/requirements.md), [web](../../../okc-web/aidlc-docs/inception/requirements/requirements.md).

Extension prompts remain recorded in [requirement-verification-questions.md](requirement-verification-questions.md); no new extension is silently enabled. Existing module opt-ins/opt-outs remain module-local. User's repeated explicit instruction to design and implement provides authority to continue the scoped work without repeating phase permission requests.
