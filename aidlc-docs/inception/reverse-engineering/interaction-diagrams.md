# Cross-Module Interaction Sequences

Text sequences are used for portable rendering.

## Source update

1. Local author or MCP changes a source note.
2. Hooks startup scan, filesystem trigger, or reconciliation requests a scan.
3. Hooks compares against committed state and prepares negotiate/blob/commit.
4. Current web expects multipart upload at a different route: the live sequence stops at this boundary.
5. Independently, web multipart upload can enqueue core add_source and return a job receipt.
6. Changed uploads become independent sources; repeated revision replacement is not wired.

## Review and publication

1. Admin freezes the source set and configures provider/preflight.
2. Web calls core integration and exposes taxonomy/cluster review checkpoints.
3. Curator approves or regenerates using current evidence-preservation semantics.
4. Admin compiles and publishes.
5. Web exposes the selected artifact through read APIs.
6. MCP currently has no web reader: the agent-consumption sequence stops at this boundary.

## Proposed authored-knowledge feedback path

MCP writes a local source → hooks uploads its revision → web/core review and compile → web publishes a fixed revision → MCP refreshes that revision for reading.

This is an intended integration path for review, not an implemented or approved new behavior. See [gap evidence](integration-gap-review.md).
