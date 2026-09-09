# Session capture code generation plan

Unit: okc-mcp-session-capture. Workspace root: okc-mcp. Authorization: user's explicit implementation request after the source comparison, plus existing module autopilot; all local implementation and validation are in scope.

Requirements: REQ-023..029. Stories: SC-1..6. Medium-risk local Markdown changes; preserve all existing worktree changes.

- [x] Step 1: Inspect module state/source contracts and record the scoped baseline.
- [x] Step 2: Record requirements, stories, application/functional/NFR design and the execution plan.
- [x] Step 3: Implement local capture discovery, section/marker transformations and preflighted apply in src/capture.ts; extend src/vault.ts and src/authoring.ts only as necessary for the common safe pipeline.
- [x] Step 4: Add src/capture-guide.ts and wire prepare_session_capture, apply_session_capture, capture_session and okc://guide/session-capture in src/server.ts.
- [x] Step 5: Add meaningful domain/PBT and actual stdio scenarios for SC-1..6, updating existing surface assertions.
- [x] Step 6: Update README English/Korean and capture usage/recovery/build-test artifacts; verify packaging includes the guide and executable implementation.
- [x] Step 7: Enforce the user-selected-only trigger in tool inputs, server/prompt/guide and docs; test that unselected sessions are never captured.
- [x] Step 8: Run final capture/full checks; verify package contents, validate module artifact links and record results/state.

Stage sequence: scoped brownfield review -> requirements/stories -> application/functional/NFR design -> code -> build/test.
Skip further unit decomposition and infrastructure design: one local module with no deployment change. Operations/package publication is not requested.
