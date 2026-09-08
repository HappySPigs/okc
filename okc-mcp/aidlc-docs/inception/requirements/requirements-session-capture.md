# Session capture requirements

Status: implementation authorized by “그래서 이제 우리도 기능 만들어서 넣자” on 2026-09-09 KST. Module scope: okc-mcp. Existing autopilot authorization applies to this local follow-up; no new stage approval is invented.

## Intent and scope
The user explicitly selects which session/content to record. Capture never starts from session lifecycle, task progress, a new message, or a previous successful capture. The selection applies only to this request. The user asks the coding agent to record that selected session. The agent chooses relevant existing local notes/sections or new note locations, without asking the user to choose paths or create/update operations. The agent owns semantic decisions; the MCP provides bounded local discovery and verified persistence. [Source research](../../../../aidlc-docs/inception/reverse-engineering/obsidian-mcp-placement-research-2026-09-09.md).

| ID | Requirement | Acceptance |
|---|---|---|
| REQ-023 | User-selected session workflow | Server instructions, a capture_session prompt and a session guide instruct the host to extract topics, search/read local candidates, choose destinations and apply changes after a preview. A user request to save authorizes ordinary local writes without path-selection questions. |
| REQ-024 | Local capture context | prepare_session_capture returns ranked lexical candidates, heading paths, folder patterns, existing records for the session, and explicitly untrusted organization hints. Web configuration never changes this local lookup. |
| REQ-025 | Repeatable records | Stable sessionId/itemId markers locate earlier records after restart. Repeating identical content is a no-op; revisions replace only that item's block in the same note. Duplicate or malformed markers fail closed. |
| REQ-026 | In-place topic placement | apply_session_capture adds records under an existing ATX heading or creates a titled note at the agent-selected path. Multiple topics/notes are supported, unrelated bytes and YAML remain unchanged, and existing decisions/contradictions are preserved. |
| REQ-027 | Review and recovery | Preview writes nothing. Existing-note changes require current hashes and retain external backups. Validate the entire batch before writes; report per-file receipts and any runtime partial result accurately. No claim of a multi-file transaction. |
| REQ-028 | Compatibility and bounds | Preserve existing tools, readonly and immutable-target restrictions, scan/note/response limits and web-first knowledge reads. No internal provider, embedding download, delete, move, source merge or automatic publication. |
| REQ-029 | Evidence | Unit/PBT and actual stdio tests cover existing-section placement, new-topic placement, multiple topics, recapture, conflicts, local/web isolation and readonly behavior. No hosted-model accuracy claim. |

The narrow candidate score is a local authoring aid, not a revival of the dropped general BM25 requirement or a semantic classifier. The host supplies topic queries, including synonyms when needed, reads candidates and makes the final relevance decision. Automatic session-end hooks and raw transcript collection are outside this follow-up.

Extensions inherited: security baseline disabled (product safeguards active); resiliency enabled; PBT partial. No opt-in question is repeated.


## REQ-030: User selection controls capture

User correction: “아니야 항상 하면안되고 세션은 유저가 선택해서 넣는거야”. Both capture tools require userSelected=true, defaulting false. Missing/false declarations reject before scanning or writing. The host obtains that scope from the current user request and captures only the selected session/content, not other histories. Once complete, another capture/re-capture requires another user selection. The server does not claim it can independently authenticate the human behind the declaration. No global always-capture mode or lifecycle hook is introduced. Negative domain/stdio regressions verify rejection and unchanged Vault/state.
