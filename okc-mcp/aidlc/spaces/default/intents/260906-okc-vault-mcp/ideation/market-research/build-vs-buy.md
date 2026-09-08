# Build vs Buy vs Partner Assessment

**Decision date:** 2026-09-07 KST  
**Decision scope:** Product strategy derived from the [`intent-statement`](../intent-capture/intent-statement.md); not implementation authorization

## Recommendation

**Build a focused, free, open-source OKC authoring MCP; reuse the official MCP SDK and proven open-source patterns and libraries; keep Obsidian app/CLI/plugin integration optional.**

This is a build-and-compose decision, not a ground-up reinvention. No inspected product supplies the full combination of OKC-oriented source guidance, app-independent operation, explicit Vault boundaries, conflict-aware persistent editing, and a clean separation between source notes and product state. Conversely, mature alternatives already demonstrate reusable patterns for path containment, structural patches, YAML preservation, bounded search, read-only policy, optimistic concurrency, and onboarding.

The recommendation authorizes product discovery and later design. It does not authorize Construction, select specific dependencies, or turn this MCP into an OKC compiler or approval service.

## Options Compared

| Option | Time to initial capability | Fit with intent | Principal risk | Decision |
|---|---|---|---|---|
| Use a generic filesystem MCP plus prompts | Fastest prototype | Low to medium | Cross-tool safety and OKC rules remain advisory and fragmented; user gets no single recovery or readiness experience. | Reject as product; useful only as a learning baseline. |
| Use an existing Obsidian MCP unchanged | Fast | Low | Persistent CRUD exists, but OKC guidance, source/output separation, and consistent conflict semantics do not. | Reject. |
| Fork a direct-filesystem Obsidian MCP | Medium | Medium | Inherits assumptions about overwrite behavior, Vault-resident state, indexing, frontmatter, or packaging; upstream merge burden obscures the product boundary. | Do not choose by default. Reconsider only if later design shows overwhelming code alignment. |
| Fork a Local REST API wrapper | Medium | Medium for app users, low for default | Requires Obsidian/plugin/key/certificate lifecycle and inherits upstream API compatibility pressure; no longer clearly differentiated from the plugin's built-in MCP. | Reject for default path. |
| Build focused MCP and compose proven parts | Medium | High | Requires disciplined scope and independent safety validation. | **Recommend.** |
| Partner with an Obsidian plugin or CLI project | Variable | Medium as an extension | Coordination and app dependency can slow the core local experience. | Keep as optional future route; no partnership required now. |
| Buy a commercial product | Unknown | Low | No inspected paid offering matches the narrow local OKC authoring boundary; introduces cost and control without closing the fit gap. | Reject based on current evidence. |

## What to Reuse Without Copying a Product Boundary

The build should benefit from public work while preserving license obligations and independently validating behavior.

| Reusable lesson | Evidence in the landscape | Product-level adoption |
|---|---|---|
| Enforce one filesystem boundary at a shared service layer | Official filesystem MCP; cyanheads path policy | Every read, search hit, and write is checked against the same explicit Vault authority. |
| Refuse ambiguous structural targets | Markus wrapper; Local REST document map | Never guess which repeated heading or block the user meant. |
| Preserve YAML syntax where possible and expose malformed input | cyanheads and MCPVault use document-oriented YAML handling; OKC retains malformed frontmatter with diagnostics | Do not erase comments/style unnecessarily and never treat parse failure as clean metadata. |
| Use optimistic concurrency for existing-note changes | Local REST built-in MCP exposes document version plus `ifMatch` | A stale read becomes an actionable conflict instead of a quiet overwrite. |
| Avoid retrying non-idempotent operations blindly | cyanheads service retry policy | Do not duplicate appended content after an uncertain transport result. |
| Make read-only mode enforceable and visible | MCPVault | Hide unavailable write affordances and reject direct mutation attempts. |
| Bound search before adding persistent infrastructure | MCPVault's lightweight search compared with lstpsche's indexed approach | Meet the first user need with predictable limits, then use measurements to justify an index. |
| Make installation paths concrete | package runners, MCP bundles, and deep links across existing projects | Publish copyable, version-aware host examples plus a connection check. |

Reuse must be assessed during design for license compatibility, dependency health, platform behavior, and current MCP support. A concept seen in an open-source project is evidence for a pattern, not automatic permission to copy its implementation.

## Why the Default Should Be Direct Filesystem Access

The chosen user promise is a local MCP that works on an explicitly connected source Vault even when the Obsidian app is not running. Direct file access best supports that promise and preserves the central benefit: changes are ordinary editable files immediately available to both Obsidian and later MCP sessions.

The choice carries obligations:

- app-level rename and live metadata behavior cannot be assumed;
- external edits require explicit conflict detection;
- Unicode, path collisions, symlinks, and exclusions must align with OKC's accepted source boundary; and
- derived search/configuration state must remain outside the source Vault.

Obsidian CLI or the Local REST API can later offer app-aware extensions such as UI opening, history, command execution, or link-aware rename. They should be separately enabled and should not silently change the default trust boundary.

## Product and Repository Management Model

One public repository should be the authoritative home for product code, installation documentation, release notes, security policy, and compatibility evidence. The user journey should be optimized around an individual OKC user rather than repository internals.

### Required First-Run Experience

1. A single recommended installation command or package-manager path.
2. A host configuration example that requires an explicit absolute Vault path; never silently use the current directory.
3. A connection diagnostic that shows the resolved Vault and whether writes are enabled.
4. A synthetic first-note flow that proves persistence by opening the normal `.md` file in Obsidian.
5. A concise explanation of safe existing-note changes and what happens on a conflict.
6. An advisory OKC-readiness view with a clear statement that no compilation or approval occurred.

### Ongoing Repository UX

- Use semantic versions and immutable release notes; publish supported OS, runtime, MCP-host, Obsidian, and OKC compatibility evidence.
- Keep a short quick start in the README and move concepts, security boundaries, troubleshooting, upgrades, and migration details into focused docs.
- Ship only synthetic example Vaults; exclude personal notes, local host configuration, keys, indexes, caches, managed OKC projects, and sealed outputs.
- Document update and uninstall behavior, including which files remain because they are user-authored notes.
- Separate stable core capabilities from experimental app/CLI integrations and mark both their prerequisites and trust impact.
- Accept bug reports with reproducible synthetic Vaults and redact note contents by default.
- Treat source notes as untrusted input; repository contribution instructions must not imply that instructions embedded in a note are executable authority.

## Decision Boundaries and Revisit Triggers

The following are intentionally deferred rather than silently decided in market research.

| Topic | Initial stance | Revisit when |
|---|---|---|
| Persistent full-text or semantic index | Not assumed | Representative Vault measurements show bounded search misses latency or relevance needs. |
| Required Obsidian plugin/CLI integration | Optional extension | Interviews show app-aware history, rename, or UI behavior is essential to the primary journey. |
| Remote/HTTP server | Out of first-product position | A real multi-device or shared-service use case exists with an explicit security model. |
| Canvas, Bases, and binary attachments | Do not claim complete support | OKC's source/materialization contract and user demand define usable semantics. |
| Compiler invocation or approval | Outside this authoring product | Only a separately authorized OKC integration scope may reconsider it. |
| Commercial pricing | Free/open source | Sustained maintenance needs and user willingness-to-pay are evidenced rather than assumed. |

## Risks and Mitigations to Carry Forward

- **Safety differentiation is easy to claim and hard to prove.** Later requirements must specify collision, overwrite, external-edit, and recovery behavior as observable acceptance criteria.
- **Protocol churn can make setup examples stale.** The repository needs tested version ranges and an upgrade policy rather than “latest” assumptions.
- **Domain guidance can become accidental compiler policy.** Label templates and readiness advice separately from verified OKC constraints and cite the OKC version used.
- **A clean Vault conflicts with convenient hidden indexes.** Keep configuration and derived state externally and make their location inspectable.
- **A broad tool catalog can weaken onboarding and trust.** Prioritize the authoring journey; add app commands, embeddings, remote transport, and destructive operations only through explicit later scope decisions.

## Assumptions & Open Questions

- Assumption: free open source is sustainable for the initial niche and improves trust through inspectability.
- Assumption: no existing commercial or open-source product has an undocumented OKC-specific mode that closes the identified fit gap.
- Open question: which MCP SDK version and host compatibility range will remain supportable when design begins?
- Open question: which reusable libraries satisfy licensing, Unicode/path behavior, and frontmatter-preservation needs after technical evaluation?
- Open question: what telemetry-free validation process can measure onboarding success and source-quality improvement without collecting personal Vault content?
- Open question: what user-facing recovery workflow should follow a detected external-edit conflict?

## Sources

- [MCP TypeScript SDK documentation](https://ts.sdk.modelcontextprotocol.io/v2/api/%40modelcontextprotocol/client/) and [MCP 2026-07-28 release](https://blog.modelcontextprotocol.io/posts/2026-07-28/).
- [Official MCP filesystem server](https://github.com/modelcontextprotocol/servers/blob/d73f99efbfd40c3aa1b61e88728b3d49fb52608f/src/filesystem/README.md) and [filesystem implementation](https://github.com/modelcontextprotocol/servers/blob/d73f99efbfd40c3aa1b61e88728b3d49fb52608f/src/filesystem/index.ts).
- [Obsidian Local REST API MCP handler](https://github.com/coddingtonbear/obsidian-local-rest-api/blob/209eff08154374bbec02142ab8e763e68fb0d13b/src/mcpHandler.ts) and [Vault operations](https://github.com/coddingtonbear/obsidian-local-rest-api/blob/209eff08154374bbec02142ab8e763e68fb0d13b/src/vaultOperations.ts).
- [cyanheads path policy](https://github.com/cyanheads/obsidian-mcp-server/blob/2d4e8d1114d650b45568caa9a163e6e853423266/src/services/obsidian/path-policy.ts) and [Obsidian service](https://github.com/cyanheads/obsidian-mcp-server/blob/2d4e8d1114d650b45568caa9a163e6e853423266/src/services/obsidian/obsidian-service.ts).
- [MCPVault server](https://github.com/bitbonsai/mcpvault/blob/c5abeda9bed11864079f70ae7f33d134e294aad2/server.ts), [filesystem handling](https://github.com/bitbonsai/mcpvault/blob/c5abeda9bed11864079f70ae7f33d134e294aad2/src/filesystem.ts), and [search](https://github.com/bitbonsai/mcpvault/blob/c5abeda9bed11864079f70ae7f33d134e294aad2/src/search.ts).
- [lstpsche Vault implementation](https://github.com/lstpsche/obsidian-mcp/blob/fea2e1f50a8a76d5232b40d07b654ae8037985c7/src/vault/mod.rs) and [watcher](https://github.com/lstpsche/obsidian-mcp/blob/fea2e1f50a8a76d5232b40d07b654ae8037985c7/src/vault/watcher.rs).
- [Official Obsidian CLI documentation](https://obsidian.md/help/cli).
- [OKC product and scope](https://github.com/dolgogae/ObsidianKnowlegeComplication/blob/7f87f7c81a72701710d8668b2569e51478dccc75/docs/specs/product-and-scope.md), [source I/O](https://github.com/dolgogae/ObsidianKnowlegeComplication/blob/7f87f7c81a72701710d8668b2569e51478dccc75/crates/okc-core/src/source_io.rs), and [security boundaries](https://github.com/dolgogae/ObsidianKnowlegeComplication/blob/7f87f7c81a72701710d8668b2569e51478dccc75/docs/specs/security-and-trust-boundaries.md).

