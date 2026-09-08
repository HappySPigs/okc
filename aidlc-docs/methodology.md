# Applying AI-DLC to the OKC MCP

> **Official workflow installation (2026-09-06):** Following the user's later installation request, the Codex distribution of version 2.7.1 was installed. Follow the [setup guide](../docs/aidlc-setup.md) for current usage, provenance, and diagnostics. The abbreviated method below is preserved as a historical, unapproved proposal. Installation does not create approval or completion for earlier stages.

> Process correction: the abbreviated proposal below was written without user review of the development process. Construction stopped after the user challenged that sequence, and the project returned to Inception. Follow [state.md](state.md) for current status and the [Inception review proposal](inception/review.md) for the next review. The Units, technology choices, and autonomous-progress language below do not constitute an approved workflow.

Research baseline: 2026-09-06. This document is an unapproved proposal that attempted to adapt the AWS method to a small local MCP product. Even after the official installation, it must not be treated as official execution history or evidence that an approval gate passed.

## Distinguishing the original method from the current implementation

AWS's 2025-07-31 introduction and Method Definition describe **Inception → Construction → Operations**. AI refines intent into requirements and work units, people review important decisions, and design, code, tests, and operations material remain in the repository as durable context. A Unit, which defines a work boundary, is distinct from a Bolt, which is a short implementation-and-validation iteration. See the [AWS introduction](https://aws.amazon.com/blogs/devops/ai-driven-development-life-cycle/) and [Method Definition, especially II.9–10 and III](https://prod.d13rzhkk8cj2z0.amplifyapp.com/aidlc.pdf).

AWS's public workflow description from 2025-11-29 adapts the kinds and depth of stages to each task. It addresses both the burden of applying a large-organization process unchanged to a small change and the risk of removing human review. See [AWS's adaptive workflows article](https://aws.amazon.com/blogs/devops/open-sourcing-adaptive-workflows-for-ai-driven-development-life-cycle-ai-dlc/).

At the research date, `awslabs/aidlc-workflows` described Workflows 2.0 GA on `main`, with README version `2.7.1`. The reproducible reference is:

```text
repository: https://github.com/awslabs/aidlc-workflows
commit: 22ed2d101f4f01196b76d5725cf8d9aabe5fef9e
commit time: 2026-09-06T03:22:39Z
```

The current implementation extends the original method to five phases and 33 stages: **Initialization → Ideation → Inception → Construction → Operation**. The default `classic` scope omits Ideation, and scope and depth are adjusted separately. Do not combine the three-phase 2025 introduction, older `aidlc-docs` conventions, and the current 2.x engine structure as though they were the same version. See the [pinned README](https://github.com/awslabs/aidlc-workflows/blob/22ed2d101f4f01196b76d5725cf8d9aabe5fef9e/README.md) and [pinned scope guide](https://github.com/awslabs/aidlc-workflows/blob/22ed2d101f4f01196b76d5725cf8d9aabe5fef9e/docs/guide/05-scopes-and-depth.md).

The 2.x guide's default Construction flow is stage-major. After reviewing the first Construction execution stage, the user chooses whether later stages may proceed autonomously. It also states that the Bolt plan and the engine's actual execution groupings are not identical. Collaboration among several AI workers in this project must not be recorded as official gate or audit events from that engine. See the [pinned phases and stages guide](https://github.com/awslabs/aidlc-workflows/blob/22ed2d101f4f01196b76d5725cf8d9aabe5fef9e/docs/guide/04-phases-and-stages.md).

## Application scope for this project

`okc-mcp` is **a local MCP for authoring and organizing an Obsidian Vault so it provides good input to OKC**. The work combines new-product development with investigation of an existing consumer system's input contract. It distinguishes the live Vault edited by the user from the immutable snapshot collected by OKC. The MCP assists with the former and does not modify compiler-owned snapshots, canonical state, or approval policy.

The referenced OKC tree is Schema 3 and development version `0.3.0` at commit `7f87f7c81a72701710d8668b2569e51478dccc75`. Actual collection behavior for Markdown, frontmatter, and links informs the authoring design. OKC's `normative-future` MCP specification describes a compiler-control adapter and must not be copied wholesale into this source-authoring product's feature list. See [OKC current state](https://github.com/dolgogae/okc/blob/7f87f7c81a72701710d8668b2569e51478dccc75/docs/CURRENT_STATE.md) and the [Vault compilation pipeline](https://github.com/dolgogae/okc/blob/7f87f7c81a72701710d8668b2569e51478dccc75/docs/specs/vault-compilation-pipeline.md).

The initial work uses the following depth. This is a project-specific application proposal, not a separate scope name defined by AWS.

| Stage | Work | Reviewable result |
| --- | --- | --- |
| Initialization and intent | Confirm the user goal, local-installation constraints, repository state, and existing OKC contract | Execution state that distinguishes facts, user requirements, and design assumptions |
| Inception | Compare existing Obsidian MCPs; define Vault boundaries, user flows, a minimal tool contract, and work units | Research evidence, requirements, Vault design, and repository UX |
| Construction | Build the stdio server, safe note creation and updates, input-quality audit, preview/apply flow, installation, and diagnostics | Executable local package, validation results, and remaining limitations |
| Operations preparation | Verify local tarball installation, diagnostics, release compatibility, and recovery procedures | Records that distinguish a distributable candidate from actual public release |

Remote servers, AWS accounts, cloud infrastructure as code, multi-user authentication, and a web UI are not required for the current local stdio goal. Installation and updates are this product's operational process, so Operations preparation should not be omitted.

## Units and Bolts proposed for the first implementation

A Unit is a user-facing capability boundary. The following table proposes initial Units; final implementation and test names should be linked from Construction records.

| Unit | User outcome | Completion criterion |
| --- | --- | --- |
| U1 — Installation and connection | Connect an MCP client with one configuration after local installation | Validate `serve`, `doctor`, configuration generation, and a real stdio handshake |
| U2 — Vault authoring | Create, read, search, and update notes in the registered Vault | Path boundaries, create-without-overwrite, content-hash conflict protection, and finite results |
| U3 — OKC input preparation | Understand metadata, link, and duplicate problems and apply planned cleanup | Read-only auditing, change preview, rejection of stale plans, and explicit change scope |
| U4 — Distribution and maintenance | Install, diagnose, and remove the package outside the source repository | Tarball-install smoke test, synthetic-Vault validation, user documentation, and recorded release blockers |

The first Bolt would validate the smallest end-to-end installation and authoring path across U1 and U2. The next Bolt would complete U3 auditing and safe preparation changes, followed by U4 installation validation. Each Unit should document its boundaries before implementation and link test evidence afterward. Keep each Bolt's code small enough to review.

Treat follow-up Units as separate intents. Link-preserving note moves, merging semantic duplicates, attachment cleanup, and team concurrent editing each have different data-change risks. The first product does not require native OKC installation or compiler control. Never describe an input-quality audit as a guarantee that the full OKC compilation succeeds.

## Human judgment and audit records

The user's original request supported research, design, and local implementation. Under the historical proposal, it was interpreted as permitting reversible file authoring, tests, and installation-candidate packaging. It must not be recorded as retroactive approval of a specific design.

Records distinguish:

- **User request:** What the user actually asked for in the conversation and its source.
- **Verified fact:** A claim supported by a specific commit, API, document, or execution result.
- **Design assumption:** A default chosen by AI to proceed and the impact of changing it.
- **Implementation and validation:** Files actually created, checks actually run, and their pass or fail results.
- **User decision:** Only a choice confirmed in an actual later response.

Present decisions that broaden product direction or mutation authority only when the concrete difference and impact can be reviewed. Implementation of a source-authoring tool was included in the interpreted historical scope. Development uses synthetic Vaults to validate writes and failures and never modifies a user's personal Vault as example data. Canonical approval records, remote provider calls, and public package publication are not automatic follow-up actions. This describes the historical scope choice; it is not a rule requiring a formal approval question for every document.

Initially, official external workflow files were read only as reference material. The official 2.7.1 Codex distribution and hooks were later connected under the user's explicit installation request. The relationship among that commit, local adaptations, and existing project records is documented in the [setup guide](../docs/aidlc-setup.md). Scope and product-stage approval remain separate decisions made through the actual workflow.

## Minimum records needed to resume work

Version documentation alongside implementation. `aidlc-docs` is this project's chosen location for preserved pre-install history; it neither creates nor imitates the `aidlc/spaces/<space>/intents/...` engine state used by official 2.x.

| Record | Contents |
| --- | --- |
| Status and decision record | Current stage, actual user decisions, assumptions, and unresolved items |
| Inception documents | Research, requirements, Vault design, installation, and maintenance UX |
| Construction documents | Tool contract, implementation boundaries, test commands, and results |
| Operations documents | Local installation, supported environments, updates, recovery, and public-release status |

At the end of validation, check requirements against code and tests. Keep passing tests distinct from product approval and local package creation distinct from registry publication. A later session should read these records and continue from the remaining work.
