# Session capture implementation

Completed the user-selected session workflow in src/capture.ts and src/capture-guide.ts. The MCP server exposes prepare_session_capture, apply_session_capture, capture_session and okc://guide/session-capture. Existing Vault gains read-only prospective-target inspection; mutations reuse authoring.applyMutation.

The user selects which session/content to record. The host supplies explicit userSelected=true per call, extracts topics and chooses relevant local destinations. Preparation is a bounded lexical aid; application handles markers, sections, preflight, hashes, backups and verified receipts. No always-on capture or lifecycle triggers.

Source/test type checks, 103 tests, build, package contents and artifact references passed. [Verification](../../build-and-test/session-capture-summary.md). Existing local-install and web changes were preserved.

