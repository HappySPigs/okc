# Cross-Module API Inventory

| Producer → consumer | Current contract | Assessment |
|---|---|---|
| Hooks → server | CBOR negotiate/blob/commit; Bearer header; committed server content ID | No matching web receiver. |
| Upload client → web | POST /u/{token}/upload, multipart file; job_id/source_id/content_hash | Different payload, routes, auth interpretation, acknowledgement. |
| Web → core | Python okc client/project/jobs; interop DTO schema 2 | Real adapter present; only add_source wired for upload. |
| Web → MCP | GET /api/serving/{project_id}/contract, /files, /file, /verify, /explain | Read-only, currently unauthenticated; no MCP consumer. |
| Agent → MCP | stdio MCP read/search/authoring tools over local Vault | Local implementation present; no remote source selection. |

Keep artifact schema 3, interop DTO schema 2, and private journal schema 4 distinct. Upload/serving protocol version and snapshot identity are independent contract concerns.

Current web accepts ZIP and Markdown; tar.zst is explicitly rejected in this build. Hooks does not currently emit either as its real wire contract. API existence does not establish producer/consumer compatibility.

Evidence: [hooks protocol](../../../okc-hooks/crates/upload-client/src/protocol.rs), [web upload](../../../okc-web/backend/app/upload/router.py), [web validation](../../../okc-web/backend/app/upload/ingest.py), [web serving](../../../okc-web/backend/app/serving/router.py), [core API authority](../../../okc-core/aidlc-docs/inception/reverse-engineering/api-documentation.md), [MCP server](../../../okc-mcp/src/server.ts).
