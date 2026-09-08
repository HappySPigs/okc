# Competitive Analysis

**Research date:** 2026-09-07 KST  
**Scope:** Local, installable authoring MCP for an Obsidian source Vault used with OKC  
**Upstream:** [`intent-statement`](../intent-capture/intent-statement.md)

## Executive Conclusion

The market already supplies persistent Vault CRUD, search, and two broad operating models: direct filesystem access and an Obsidian app/plugin bridge. Therefore, the requirement that a user can keep adding to the Vault throughout one MCP session—and see those files in Obsidian and later sessions—is table stakes, not sufficient differentiation.

The underserved position is an app-independent local authoring product that combines:

- explicit control of one connected source Vault;
- persistent, conflict-aware note creation and modification;
- Obsidian-aware Markdown, frontmatter, heading, block, and link handling;
- OKC-oriented guidance and advisory input-readiness checks; and
- a short, transparent install-to-first-note experience with no hidden product state inside the source corpus.

This supports building a focused OKC MCP while reusing public standards, SDKs, libraries, and proven interaction patterns. It does not support claiming that existing tools are defective or that OKC compilation and approval belong inside this product.

## Evaluation Frame

The comparison follows the confirmed research answers and evaluates alternatives on five user outcomes.

| Outcome | What good looks like |
|---|---|
| Persistent authoring | Create, read, search, append, and revise notes repeatedly in one session; changes persist on disk for Obsidian and later sessions. |
| Change control | Existing-note changes are explicit, collision-aware, recoverable where practical, and do not silently discard external edits. |
| Local boundary | One user-selected Vault is the authority boundary; paths, symlinks, and excluded areas cannot escape it. |
| Knowledge quality | Frontmatter and links remain editable in Obsidian while the product helps users form material that OKC can consume well. |
| Adoption UX | Installation, connection verification, upgrades, troubleshooting, and removal are understandable to an individual user. |

Feature counts are not treated as product value by themselves. Repository stars and recent pushes are directional open-source attention signals only; they are not active-user, revenue, or market-size measurements.

## Competitive Landscape

All directly inspected alternatives are free open source, so price is not a meaningful differentiator for the first release.

| Alternative | Operating model | Strengths to learn from | Gaps relative to this intent |
|---|---|---|---|
| `mcp-obsidian` (Markus Pfundstein, v0.2.2) | Python MCP wrapper over the Obsidian Local REST API plugin | Small tool/service split; plugin-backed search; targeted heading, block, and frontmatter patches; ambiguous headings are rejected. | Obsidian and a configured plugin/API key must be running; whole-file replacement and append lack an exposed expected-version guard; no OKC guidance. |
| `obsidian-mcp-server` (cyanheads, v3.5.1) | TypeScript MCP wrapper over Local REST API | Read/write prefix policy; read-only mode; capability probing; frontmatter editing that better preserves YAML syntax; avoids retrying non-idempotent writes; polished package/deep-link onboarding. | Inherits app, plugin, key, certificate, and upstream API-version dependencies; app command execution expands trust scope; no OKC authoring model; inspected writes do not expose an expected content version. |
| MCPVault (v0.16.0) | Node MCP directly over Vault files | App independence; broad Markdown-aware tools; read-only mode both hides and rejects mutation; lexical and real-path checks; lightweight bounded search. | Vault omission falls back to the current directory; ordinary writes can replace an existing file without an expected-version precondition; malformed YAML may be treated as absent; no OKC boundary or readiness guidance. |
| `obsidian-mcp` (lstpsche, v2.5.0) | Rust MCP directly over Vault files | Watcher-backed metadata index; optional full-text/semantic search; careful canonical path and Unicode ambiguity handling; stdio and HTTP modes. | Larger dependency and operational surface; creates product state under `.obsidian-mcp` in the Vault; frontmatter reserialization may lose presentation details; mutation tools lack an exposed expected-version condition; no OKC guidance. |
| Obsidian Local REST API built-in MCP (plugin v5.1.0) | MCP hosted by the running Obsidian desktop plugin | Rich app-aware operations; document maps identify duplicate sections; content-hash `ifMatch` supports optimistic concurrency; trash deletion, link-aware app behavior, and UI/command access. | Desktop app/plugin/API key remain prerequisites; broader app commands are unnecessary for the first OKC authoring boundary; not an OKC-specific experience. This is now the strongest app-integrated direct substitute. |
| Official MCP filesystem server (v0.6.3) | Generic MCP over explicitly allowed directories | Mature path-boundary checks, symlink handling, safer replacement strategy, and refusal to move over an existing destination. | Generic file and line operations do not understand Obsidian frontmatter, headings, wikilinks, or OKC quality; tool annotations are descriptive hints rather than authorization enforcement. |
| Official Obsidian CLI | Commands handled by a running Obsidian app | First-party access to create/read/append, search, outlines, links, properties, history, tags, and tasks; useful optional interoperability path. | Requires a recent Obsidian installation and running app; it is a command interface, not a packaged OKC MCP or an app-independent authoring boundary. |

### Verified Source Snapshot

Repository behavior was inspected at these revisions rather than inferred from names or README claims alone.

| Project | Revision inspected | Latest activity at research time |
|---|---|---|
| MarkusPfundstein/mcp-obsidian | `5ee0b84fa8319fd2fdf0db0ee1febb065e712a15` | 2026-08-31 |
| cyanheads/obsidian-mcp-server | `2d4e8d1114d650b45568caa9a163e6e853423266` | 2026-09-04 |
| bitbonsai/mcpvault | `c5abeda9bed11864079f70ae7f33d134e294aad2` | 2026-08-31 |
| lstpsche/obsidian-mcp | `fea2e1f50a8a76d5232b40d07b654ae8037985c7` | 2026-08-21 |
| coddingtonbear/obsidian-local-rest-api | `209eff08154374bbec02142ab8e763e68fb0d13b` | 2026-08-30 |
| modelcontextprotocol/servers | `d73f99efbfd40c3aa1b61e88728b3d49fb52608f` | 2026-09-02 |

## Table Stakes and Differentiators

### Table Stakes

- Connect an explicit Vault and prove which Vault is connected before writing.
- Persist repeated create, read, search, append, and controlled update operations during one MCP session and across later sessions.
- Preserve usable Obsidian Markdown, YAML frontmatter, headings, blocks, wikilinks, and ordinary links.
- Bound search and returned content so a large note does not unexpectedly consume the client context.
- Provide read-only and write-scope controls that are enforced by the server, not inferred from model intent or tool annotations.
- Refuse ambiguous targets and explain how the user can retry safely.
- Work locally without requiring a hosted knowledge service.
- Offer deterministic setup, connection diagnostics, update guidance, and removal guidance.

### Defensible Differentiators

- Guide the user toward OKC-useful evidence, status, decision, open-item, link, and navigation patterns without pretending those suggestions are compiler mandates.
- Surface advisory OKC readiness issues such as malformed frontmatter, non-portable paths, case/Unicode collisions, ambiguous links, or unsupported source material before compilation.
- Require a current version/hash or another explicit safe condition when modifying existing content so external Obsidian edits are not quietly lost.
- Keep source notes, local MCP configuration/state, OKC managed projects, and sealed outputs visibly separate.
- Treat malformed metadata as a visible problem rather than silently normalizing or discarding it.
- Make Korean and mixed-script Vaults a first-class usability case while respecting OKC's portable path and Unicode constraints.

The product must remain an authoring assistant. Readiness checks are advisory unless an actual OKC compiler call is made, and compiler approval remains outside this product's authority.

## Positioning and Repository UX Implications

The clearest position is: **“Safely author an editable Obsidian source Vault that is easier to use with OKC.”** It should not be positioned as another generic filesystem agent, an Obsidian replacement, or an OKC compiler UI.

For the first individual user, repository and package UX should form one short journey:

1. install the free local package;
2. choose an explicit absolute source-Vault path;
3. verify the connected path and effective write policy;
4. create a first structured note and inspect it in Obsidian;
5. run an advisory readiness view; and
6. know how to update, diagnose, or fully remove the server without touching notes.

The public repository should keep a versioned quick start, changelog, compatibility notes, security/trust-boundary documentation, and synthetic example Vault. Personal notes and machine-local configuration must never be examples committed to the repository. Optional app or CLI integrations should be labeled separately so the default path stays app-independent.

## Audience and Demand Signal

The initial audience is deliberately narrow: an individual OKC user starting or improving a source Vault. Public repository attention confirms that local Obsidian automation is an active category—at the research snapshot, the inspected projects ranged from tens to several thousand GitHub stars and all had recent 2026 activity—but it does not establish OKC-specific demand.

No defensible TAM, active-user count, or willingness-to-pay figure was found. The free/open-source assumption is consistent with the inspected alternatives. Product fit should therefore be validated through observed onboarding and authoring outcomes, not a fabricated market-size estimate.

## Assumptions & Open Questions

- Assumption: first users prefer an explicit source-Vault connection over the convenience and risk of defaulting to the current directory.
- Assumption: an app-independent default is more valuable to OKC users than first-release access to Obsidian UI commands and live app metadata.
- Open question: how often do target users edit the same note concurrently through Obsidian and MCP, and what recovery language do they understand?
- Open question: which Vault sizes and link densities make a persistent index materially better than bounded on-demand scanning?
- Open question: will target MCP hosts support the current protocol release uniformly, or will a documented compatibility window be required?
- Open question: the success baseline, target reduction in input issues, measurement window, and observation method remain to be established in requirements work.

## Sources

- [MarkusPfundstein/mcp-obsidian README](https://github.com/MarkusPfundstein/mcp-obsidian/blob/5ee0b84fa8319fd2fdf0db0ee1febb065e712a15/README.md), [server](https://github.com/MarkusPfundstein/mcp-obsidian/blob/5ee0b84fa8319fd2fdf0db0ee1febb065e712a15/src/mcp_obsidian/server.py), and [client service](https://github.com/MarkusPfundstein/mcp-obsidian/blob/5ee0b84fa8319fd2fdf0db0ee1febb065e712a15/src/mcp_obsidian/obsidian.py).
- [cyanheads/obsidian-mcp-server README](https://github.com/cyanheads/obsidian-mcp-server/blob/2d4e8d1114d650b45568caa9a163e6e853423266/README.md), [path policy](https://github.com/cyanheads/obsidian-mcp-server/blob/2d4e8d1114d650b45568caa9a163e6e853423266/src/services/obsidian/path-policy.ts), and [frontmatter operations](https://github.com/cyanheads/obsidian-mcp-server/blob/2d4e8d1114d650b45568caa9a163e6e853423266/src/services/obsidian/frontmatter-ops.ts).
- [MCPVault README](https://github.com/bitbonsai/mcpvault/blob/c5abeda9bed11864079f70ae7f33d134e294aad2/README.md), [filesystem handling](https://github.com/bitbonsai/mcpvault/blob/c5abeda9bed11864079f70ae7f33d134e294aad2/src/filesystem.ts), and [frontmatter handling](https://github.com/bitbonsai/mcpvault/blob/c5abeda9bed11864079f70ae7f33d134e294aad2/src/frontmatter.ts).
- [lstpsche/obsidian-mcp README](https://github.com/lstpsche/obsidian-mcp/blob/fea2e1f50a8a76d5232b40d07b654ae8037985c7/README.md), [Vault module](https://github.com/lstpsche/obsidian-mcp/blob/fea2e1f50a8a76d5232b40d07b654ae8037985c7/src/vault/mod.rs), and [path handling](https://github.com/lstpsche/obsidian-mcp/blob/fea2e1f50a8a76d5232b40d07b654ae8037985c7/src/vault/path.rs).
- [Obsidian Local REST API README](https://github.com/coddingtonbear/obsidian-local-rest-api/blob/209eff08154374bbec02142ab8e763e68fb0d13b/README.md), [MCP handler](https://github.com/coddingtonbear/obsidian-local-rest-api/blob/209eff08154374bbec02142ab8e763e68fb0d13b/src/mcpHandler.ts), and [Vault operations](https://github.com/coddingtonbear/obsidian-local-rest-api/blob/209eff08154374bbec02142ab8e763e68fb0d13b/src/vaultOperations.ts).
- [Official MCP filesystem server](https://github.com/modelcontextprotocol/servers/blob/d73f99efbfd40c3aa1b61e88728b3d49fb52608f/src/filesystem/README.md) and [implementation](https://github.com/modelcontextprotocol/servers/blob/d73f99efbfd40c3aa1b61e88728b3d49fb52608f/src/filesystem/index.ts).
- [Official Obsidian CLI documentation](https://obsidian.md/help/cli).
- [OKC product scope](https://github.com/dolgogae/ObsidianKnowlegeComplication/blob/7f87f7c81a72701710d8668b2569e51478dccc75/docs/specs/product-and-scope.md), [canonical knowledge IR](https://github.com/dolgogae/ObsidianKnowlegeComplication/blob/7f87f7c81a72701710d8668b2569e51478dccc75/docs/specs/canonical-knowledge-ir.md), and [security boundaries](https://github.com/dolgogae/ObsidianKnowlegeComplication/blob/7f87f7c81a72701710d8668b2569e51478dccc75/docs/specs/security-and-trust-boundaries.md).

