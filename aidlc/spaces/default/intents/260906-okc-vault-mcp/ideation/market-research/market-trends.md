# Market Trends

**Research date:** 2026-09-07 KST  
**Scope:** Trends that materially affect the [`intent-statement`](../intent-capture/intent-statement.md), not a general PKM market forecast

## Executive Summary

Three converging trends shape this product: users expect local ownership of a plain-file Vault; Obsidian itself now offers richer first-party and plugin-hosted automation; and MCP is moving toward a simpler stateless core while deprecating Roots as a central discovery primitive. These trends favor a small local stdio product with an explicit Vault boundary and portable authoring semantics. They also raise the bar for conflict handling: persistent writes alone are common, while safe writes against concurrent app edits are not consistently available.

The opportunity is qualitative and niche. Active open-source projects show sustained interest in connecting agents to Obsidian, but they do not prove the number of OKC users or a commercial market. The correct next validation is whether the integrated OKC authoring experience measurably reduces source-quality problems for real users.

## Local-First Files Are the Product Boundary

Obsidian defines a Vault as a local folder and stores core notes as Markdown. Its supported Vault materials also include Canvas, Bases, and attachments. This makes filesystem-level interoperability attractive: a note created by a local MCP is immediately visible to Obsidian and remains available after an MCP session ends.

Local ownership does not by itself provide safety. A trustworthy product must distinguish four different things:

- editable source Vault content owned by the user;
- MCP configuration and operational state;
- OKC managed project data; and
- OKC sealed or exported outputs.

OKC's current source contract further narrows the useful first target. Its present materialization path is Markdown-focused, while Canvas, Base files, attachments, and complete link rewriting remain broader or open areas. The market implication is to make Markdown authoring excellent first and label other Vault formats honestly rather than implying total Obsidian fidelity.

## Obsidian Automation Is Becoming a Platform Capability

The official Obsidian CLI now exposes note creation, reading, append/prepend, search, outlines, links, properties, history, tags, tasks, and other app-aware operations. It requires a compatible Obsidian installation and a running app, making it a strong adjacent substitute and a possible future interoperability path rather than the foundation of an app-independent product.

The Local REST API plugin now also includes its own MCP server. Its document-map and content-hash `ifMatch` flow is especially important: it demonstrates that duplicate sections can be addressed precisely and that a write can be rejected if content changed after the agent read it. Thin MCP wrappers over that plugin now compete not only with each other but with their upstream plugin's built-in capability.

Product consequence: app-aware commands, live UI state, and rename propagation should be treated as optional future value. The default OKC experience can remain file-based, but it must equal the app-integrated path on transparent write safety where possible.

## MCP Compatibility and Trust Expectations Are Moving

The current MCP release dated 2026-07-28 adopts a stateless core and deprecates Roots, Sampling, and Logging in favor of a smaller protocol surface and model-routed tool requests. This means a new local server should not make MCP Roots its only Vault authorization or discovery mechanism. An explicit path supplied through local server configuration remains easier for a user to inspect and for the server to enforce.

MCP tool annotations can help a host describe read-only or destructive behavior, but they are not a security boundary. Read-only mode, path limits, and destructive-operation policy have to be enforced in the product itself. Compatibility claims should be tied to tested hosts and protocol versions because adoption of a new protocol release will not be simultaneous across clients.

## Persistent Writes Are Table Stakes; Conflict Safety Is Emerging

Every serious alternative supports persisted disk changes. The actual competitive split is how an existing note is changed:

- some tools allow direct whole-file replacement;
- some offer structural patches but do not expose a content-version precondition;
- generic filesystem tooling can make replacement more crash-safe without detecting a stale reader; and
- the Local REST API's built-in MCP exposes an optimistic-concurrency hash for targeted patches.

For an authoring tool used alongside Obsidian, the stale-edit case is ordinary rather than exceptional. The market is moving from “the agent can write” toward “the user can understand and trust what the agent changed.” A product that quietly overwrites a newer edit will fail the local-first promise even if it never sends data off-device.

## Search Sophistication Is Diverse, Not Yet a First-Release Requirement

Current projects range from plugin-delegated search and bounded directory scans to watcher-backed indexes, BM25 full-text retrieval, and optional embeddings. More sophisticated search improves discovery in large Vaults, but it also adds background state, dependencies, startup work, and consistency questions.

For the first OKC audience—an individual starting a new Vault—bounded on-demand search is likely sufficient as a product assumption. A persistent index should be justified by measured Vault sizes and latency, and any index must remain derived state outside the source corpus. Semantic retrieval should not be used as a substitute for deterministic path, metadata, and link-quality checks.

## Obsidian Semantics and OKC Portability Must Coexist

Obsidian properties are YAML frontmatter, and internal links may use wikilink or Markdown syntax with paths, headings, blocks, and aliases. Users expect these files to stay human-editable and to retain useful formatting. Existing tools vary significantly in whether frontmatter edits preserve comments and scalar presentation or reserialize the entire document.

OKC adds portability constraints that ordinary Obsidian use may not surface immediately: strict UTF-8 and Unicode normalization, path and case-collision detection, no symlinked input, and exclusions such as `.obsidian/**` and `.git/**`. Its parser retains malformed frontmatter as opaque source and reports a diagnostic. This favors fail-visible guidance over silent repair and makes Korean or mixed-script path testing a core usability concern, not a localization afterthought.

## Category Activity and Audience Limits

A 2026-09-07 GitHub API snapshot showed active maintenance across all inspected projects. Directional attention ranged from 27 stars for the newer Rust server to 4,373 for the established Python wrapper; MCPVault, Local REST API, and the cyanheads server also had hundreds to thousands of stars. These signals support the existence of an active developer/user category around Obsidian automation.

They do not answer:

- how many users need OKC-specific authoring;
- how frequently source-structure problems block their work;
- which MCP hosts they use; or
- whether they value an app-independent workflow over plugin integration.

Accordingly, no TAM, SAM, SOM, active-user count, or revenue forecast is asserted. The initial market is the explicitly defined OKC user niche, and validation should focus on successful onboarding and fewer observable source-quality issues.

## Product Implications

| Trend | First-product implication | What to defer |
|---|---|---|
| Local-first plain files | Persist normal editable notes in one explicit source Vault. | Hosted sync or proprietary storage. |
| App automation maturity | Keep app/CLI/plugin integration compatible and optional. | Requiring the app for the default path. |
| MCP protocol evolution | Use explicit local configuration and publish a tested compatibility window. | Treating Roots or annotations as authorization. |
| Trustworthy agent writes | Make existing-note edits conflict-aware and explain recovery. | Silent overwrite convenience. |
| Search diversity | Start with bounded, predictable discovery and measure. | Embeddings or a background daemon without evidence. |
| OKC portability | Advise on path, metadata, and link issues while preserving source. | Acting as compiler or approval authority. |

## Assumptions & Open Questions

- Assumption: the first cohort works mainly with Markdown notes rather than Canvas, Bases, or large binary collections.
- Assumption: a local stdio launch remains supported by the target MCP hosts despite broader protocol changes.
- Open question: which hosts and protocol revisions must the first documented compatibility matrix include?
- Open question: at what Vault scale do bounded scans stop meeting an acceptable interaction time?
- Open question: do users prefer to resolve a stale-write conflict manually, review a generated proposal, or retry after an automatic reread?
- Open question: which optional Obsidian CLI or plugin features create enough value to justify their additional app-running requirement?

## Sources

- [Obsidian Vault documentation](https://obsidian.md/help/vault), [file formats](https://obsidian.md/help/file-formats), [properties](https://obsidian.md/help/properties), and [internal links](https://obsidian.md/help/links).
- [Official Obsidian CLI documentation](https://obsidian.md/help/cli).
- [Obsidian Local REST API built-in MCP documentation](https://github.com/coddingtonbear/obsidian-local-rest-api/blob/209eff08154374bbec02142ab8e763e68fb0d13b/README.md) and [MCP implementation](https://github.com/coddingtonbear/obsidian-local-rest-api/blob/209eff08154374bbec02142ab8e763e68fb0d13b/src/mcpHandler.ts).
- [MCP 2026-07-28 release announcement](https://blog.modelcontextprotocol.io/posts/2026-07-28/) and [SEP-2577: deprecate Roots, Sampling, and Logging](https://modelcontextprotocol.io/seps/2577-deprecate-roots-sampling-and-logging).
- [Official MCP filesystem server](https://github.com/modelcontextprotocol/servers/blob/d73f99efbfd40c3aa1b61e88728b3d49fb52608f/src/filesystem/README.md).
- [OKC source parser](https://github.com/dolgogae/ObsidianKnowlegeComplication/blob/7f87f7c81a72701710d8668b2569e51478dccc75/crates/okc-core/src/parse.rs), [source I/O](https://github.com/dolgogae/ObsidianKnowlegeComplication/blob/7f87f7c81a72701710d8668b2569e51478dccc75/crates/okc-core/src/source_io.rs), and [security boundaries](https://github.com/dolgogae/ObsidianKnowlegeComplication/blob/7f87f7c81a72701710d8668b2569e51478dccc75/docs/specs/security-and-trust-boundaries.md).
- GitHub repository/API snapshots for [mcp-obsidian](https://github.com/MarkusPfundstein/mcp-obsidian), [obsidian-mcp-server](https://github.com/cyanheads/obsidian-mcp-server), [MCPVault](https://github.com/bitbonsai/mcpvault), [obsidian-mcp](https://github.com/lstpsche/obsidian-mcp), and [Local REST API](https://github.com/coddingtonbear/obsidian-local-rest-api), captured 2026-09-07.

