# Web knowledge and local authoring requirements

Status: implementation authorized by the user's 2026-09-09 request, under the existing autopilot authorization. This module owns MCP changes; umbrella coordination remains in the [root integration workspace](../../../../aidlc-docs/README.md).

## Scope and prior decisions

The current user request supersedes the first-unit exclusions on configured HTTP consumption and querying compiled artifacts. Existing source Vault protection, no arbitrary HTTP tool, no internal AI invocation, no automatic approval, no destructive note operation, and source backup requirements remain. Semantic/embedding search is still excluded: agent knowledge consumption uses bounded literal search, outline, backlinks, provenance, and revision identity.

| Requirement | Acceptance | Story |
|---|---|---|
| REQ-018 Web first | Configured `web.baseUrl/projectId` selects published web knowledge by default. Only absent web configuration selects local. Remote failures never silently fall back. | As a knowledge user, I see the intended integrated corpus. |
| REQ-019 Separate write target | Authoring always uses explicit local `vaultPath`; reads allow explicit `source=local` to inspect and edit the original. Web-only read-only setup requires no local directory. | As an author, I can add source knowledge while querying the published corpus. |
| REQ-020 Revision and provenance | Every remote tool response carries project, immutable revision, stale status, source URLs; multi-file operations pin one revision. An optional revision input lets callers keep pages consistent. | As a reviewer, I can identify and reproduce the evidence used. |
| REQ-021 Bounded web adapter | Fixed serving endpoints, optional bearer token, http(s) only, no embedded URL credentials, no redirects, streaming byte limits, timeout and cancellation, safe relative paths, validated metadata, no secret/body echo in errors. | As an operator, remote failures remain bounded and diagnosable. |
| REQ-022 Source initialization | Explicit CLI initializes a new source directory with optional conventional subfolders, refuses existing targets, validates parent and immutable-artifact boundaries. Authoring guidance/templates remain outside the corpus. | As a new author, I can start a valid empty source Vault without fabricated knowledge. |

## Validation and constraints

Existing local config remains compatible. Tool responses retain their prior fields. A `source` envelope makes local/web origin explicit; mutation responses identify local target. A fresh contract is fetched for each remote operation. Within-operation note caching is bounded by `maxScanBytes`, released after that operation, and is never used to serve data while offline or after authorization failure. Remote trust is server integrity, not factual or publisher authenticity.

No infrastructure, provider, embedding model, deployment, external publication, or source migration is introduced. Standard requirements/design depth; unit decomposition is one cohesive MCP follow-up. Existing security extension remains disabled; product security constraints apply. Resiliency and PBT partial status are inherited; remote failure tests extend existing behavior tests.
