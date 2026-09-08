# Official AI-DLC setup for this repository

The Codex distribution of `awslabs/aidlc-workflows` **2.7.1** is installed in this project. It is pinned to commit `22ed2d101f4f01196b76d5725cf8d9aabe5fef9e`.

- [Official installation guide](https://github.com/awslabs/aidlc-workflows/blob/22ed2d101f4f01196b76d5725cf8d9aabe5fef9e/docs/guide/harnesses/codex-cli.md)
- [Installation provenance and file hashes](aidlc-upstream/installation.json)
- [Upstream license](aidlc-upstream/LICENSE)

This development-workflow setup was authorized by the user's request on 2026-09-06: “이거 여기서 사용가능하게 세팅해” (“Set this up so it can be used here”). The product is still **awaiting Inception review**. Installing the workflow must not be recorded as requirements approval or authorization to enter Construction. Existing files under `src/`, `tests/`, and the package configuration remain unapproved drafts.

## Usage

Open a **new Codex conversation** with this project as the working directory, then enter:

```text
$aidlc --doctor
```

After confirming the setup, begin the product review with a request like:

```text
$aidlc Use aidlc-docs/state.md and aidlc-docs/inception/review.md to formally review the requirements for a local Obsidian MCP that authors an OKC input Vault. The existing code and documents are unapproved drafts. Do not implement, test, or package the product until I have reviewed the requirements and authorized the transition to Construction.
```

Use `$aidlc --status` to inspect status and `$aidlc --help` for help. If `$aidlc` is not listed, reopen Codex and check both the project path and the `aidlc` entry under `/skills`. Setup work does not create a workflow intent or record completed stages or approvals. The first actual product workflow creates official records from initialization and may use the existing drafts only as reference material.

You can also inspect the installation from a shell without invoking a model:

```sh
bun .codex/tools/aidlc-utility.ts version
bun .codex/tools/aidlc-utility.ts doctor
bun .codex/tools/aidlc-utility.ts status
```

## Installed components and existing records

| Path | Purpose |
|---|---|
| `.agents/skills/` | The official `$aidlc` skill plus 42 stage and supporting skills |
| `.codex/tools/`, `.codex/hooks/`, `.codex/hooks.json` | The official engine and Codex hooks |
| `.codex/agents/` | Markdown descriptions and TOML configuration for 14 official roles |
| `.codex/config.toml` | This project's AI-DLC rule path and gate configuration |
| `aidlc/spaces/default/memory/` | Official method rules and project constraints derived from actual user instructions |
| `aidlc/spaces/<space>/intents/` | Intents, state, audit records, and artifacts created by future official engine runs |
| `aidlc-docs/` | Research, drafts, and process-correction history written before installation |
| `docs/aidlc-upstream/` | Pinned installation guidance, original configuration example, license, provenance, and hashes |

Keep the product approval state in `aidlc-docs/state.md` distinct from future official engine state files. Never copy existing documents into official stages as if those stages had completed, and never backfill approval events. Preserve the official engine TypeScript, skills, stage graph, and shared hook implementations as distributed. Record local Codex adapter changes separately, as below, without modifying upstream provenance hashes to imply that those files are unmodified official artifacts.

## Codex-specific adaptations

The installation uses the **existing Codex model, provider, and login** instead of the distribution's default AWS Bedrock connection. Project configuration omits model names, context sizes, reasoning defaults, and Bedrock provider blocks. The five official roles configured for `openai.gpt-5.6-terra` use the OpenAI model name `gpt-5.6-terra`; their per-role reasoning settings remain unchanged. Other roles inherit the current session model. No AWS account or new API key was added.

Existing sandbox, network, and TUI settings are inherited. The official rule path, delegation-depth limit, and question-tool settings remain in place. `default_mode_request_user_input` was marked as under development in the Codex version used for installation. Where that capability is unavailable, question rendering follows the official workflow fallback and host tool constraints.

Codex runs only trusted hooks. Exactly 15 entries for this project's `.codex/hooks.json` were registered under `[hooks.state]` in the user's `~/.codex/config.toml`. The previous user configuration was backed up beside it as `config.toml.before-okc-mcp-aidlc-*.bak`. Existing model, login, MCP, and other project settings were preserved. Neither the user configuration nor its backup was copied into this repository.

For **six entries with matchers**, the hashes printed by the official `package.ts codex trust` command differed from the hashes calculated by Codex CLI 0.153.4. The hook file was not changed. Instead, all 15 entries were registered with the `currentHash` returned for the installed official hooks by Codex's read-only `hooks/list`. A fresh app-server process then reported every entry as `trusted` and `enabled: true`. Do not assume trust is complete merely because the official generator produced output.

According to the [OpenAI Hooks documentation](https://learn.chatgpt.com/docs/hooks.md), Codex CLI 0.153.4 requires successful `Stop` and `SubagentStop` hooks to return JSON on stdout. The shared AI-DLC hooks returned exit code 0 with empty stdout on permitted paths, which Codex displayed as `Hook failed`. The local `.codex/hooks/aidlc-codex-adapter.ts` normalizes only those empty successful responses to `{}`. Blocking responses and exit codes pass through unchanged; shared hook implementations and workflow state transitions are not modified.

The same documentation requires `permissionDecision: "allow"` whenever a `PreToolUse` response includes `updatedInput`. To prevent `Hook failed` on session binding and stage-rule delivery, the Codex adapter supplies the missing allow decision. Denials and exit-code-2 blocking responses are unchanged.

Codex reads project configuration only for trusted projects. See the [OpenAI configuration reference](https://learn.chatgpt.com/docs/config-file/config-reference#configtoml). Hook registration and trust checks verify installation; they are not evidence that an actual product-workflow hook ran in the current conversation. Real hook history begins with the first run in a new Codex conversation.

## Validation record

Environment on 2026-09-06: macOS arm64, Codex CLI `0.153.4`, Bun `1.4.0`.

- Official `doctor` against a temporary installation: **44 passed, 0 failed**.
- Official `doctor` in the actual project: **44 passed, 0 failed**; version `aidlc 2.7.1`.
- Syntax checks completed for 15 TOML entries and 15 official trusted-hook entries.
- Strict Codex app-server configuration loading succeeded against the actual project. It reported the existing `gpt-6-astra` model, AI-DLC rule path, and question-feature setting.
- Codex `skills/list`: 42 skills, `aidlc` enabled, and zero project-skill errors.
- Codex `hooks/list`: all 15 hooks trusted and enabled, with zero errors or warnings.
- Installation hashes matched for 331 distributed files. Hashes for 15 product source, test, and package files were unchanged before and after installation. The check also confirmed that personal runtime data is excluded while shared workflow records are retained by `.gitignore`.
- Official `status`: `No active AI-DLC workflow found.` No real intent was initialized.
- The product's `npm run check`, tests, build, and packaging were not run.

Additional validation on 2026-09-07 with Codex CLI `0.153.4`:

- Reproduced the original `exit 0` plus empty stdout from a real `Stop` input with no active intent.
- Confirmed that the Codex adapter returns `exit 0` plus `{}` for the same input.
- Applied the same contract to the empty successful `SubagentStop` path.
- Confirmed that `PreToolUse` session-binding output includes both `updatedInput` and `permissionDecision: "allow"`.
- Applied the same allow-decision boundary to `updatedInput` from the shared stage-rule hook.
- Reran the official `doctor` to check hook heartbeat, drop behavior, locking, and installation state.
- This local compatibility change intentionally makes one Codex adapter file differ from its upstream installation hash. The pinned upstream commit and provenance record remain unchanged.

The official diagnostic also reported 47 advisory items from existing scopes, one keyword overlap between project and organization rules, and eight method files that were not yet committed. The overlap is between the instruction to pause unapproved product work and organization-level trunk-development wording; they are different constraints. The absence of a hook heartbeat was consistent with not having started a product workflow. Do not present this setup diagnostic as completed stage execution or completed product validation.

## Reinstallation and updates

1. Verify the commit and official installation guide in the [installation provenance](aidlc-upstream/installation.json).
2. Compare the new distribution's `.codex/`, `.agents/`, and `aidlc/` trees. Do not overwrite existing intents, audit records, user knowledge, or customized `memory/` wholesale.
3. Preserve project-specific model adaptations and the approval boundary in the root `AGENTS.md`.
4. Merge only the `# AI-DLC` portion of upstream `.gitignore` into the existing project rules. Share state, audit records, and artifacts while excluding user cursors and local runtime data.
5. From an official source checkout, run `bun install --frozen-lockfile`, then generate trust entries with `bun scripts/package.ts codex trust --project <absolute-project-path>`. Replace existing entries for that hook file and do not create duplicate TOML tables. Also inspect the hook hashes and trust state reported by the new Codex version. If an entry appears as `modified`, review the installed hook command and trust it again through Codex's hook-trust interface. See the validation record above for the generator discrepancy observed in 0.153.4.
6. Run the official `doctor`, verify Codex skill and hook discovery, and update provenance, hashes, and actual validation records.

Because this repository contains installation provenance and distributed files, other machines do not need to install the framework's development dependencies. Each machine must provide Codex CLI 0.145.0 or later, Bun, and trust for this project and its hooks.
