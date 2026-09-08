# AI-DLC State Tracking

## Project Information
- **Project Type**: Brownfield
- **Project**: Installable MCP for published OKC knowledge and local Obsidian source authoring
- **Start Date**: 2026-09-06 (original); v1 workflow resumed 2026-09-08
- **Current Phase**: CONSTRUCTION
- **Current Stage**: CONSTRUCTION COMPLETE — user-selected session capture verified (2026-09-09 KST): source/test type checks, 103 tests, build and package inclusion passed. See construction/build-and-test/session-capture-summary.md. Previous web/local-install work is preserved.
- **Unit 2 `okc-mcp-retrieval-unit` (NEW, 2026-09-09)**: Requirements delta + Functional Design DRAFTED, then a **REDUCED scope IMPLEMENTED & VERIFIED GREEN** (2026-09-09) per the user's recommended choice: **REQ-015 `fold` + REQ-017 `outline_note` (ATX-only) + BR-VISIBLE-1 latent-bug fix** shipped in `src/`; then per an explicit go/no-go (2026-09-09): **REQ-016 `list_backlinks` IMPLEMENTED** and **REQ-014 BM25 DROPPED** (untraced to success criteria + muted IDF). Adversarial verification found a latent defect in ALREADY-SHIPPED code (`visibleMarkdown` astral masking → BR-VISIBLE-1), now fixed. Artifacts: `inception/requirements/requirements-retrieval-unit.md`, `construction/okc-mcp-retrieval-unit/functional-design/{business-rules,business-logic-model}.md`. 7 open questions pending (BM25 go/no-go [untraced], rank+fold composition, folding default, backlinks subset, setext, cross-Unit edits, concrete params).

## Framework
- **AI-DLC v1.0.1** (classic, rules-based; `CLAUDE.md` + `.aidlc-rule-details/`).
- Switched from the 2.7.1 harness on 2026-09-08 per the user's standing instruction. The removed 2.7.1 state is archived at git checkpoint `5c89366` (switch commit `42a9641`).

## Workspace State
- **Existing Code**: Yes — TypeScript (`src/cli.ts`, `config.ts`, `guide.ts`, `notes.ts`, `server.ts`, `vault.ts`), `tests/`, `package.json`.
- **Build System**: npm (package.json), TypeScript.
- **Workspace Root**: /Users/sihun/workspace/projects/okc/okc-mcp
- **Reverse Engineering**: Deferred (not run). Prior research drafts exist (`aidlc-docs/inception/existing-mcp-research.md`, `okc-vault-design.md`, `repository-ux.md`). No formal artifacts under `aidlc-docs/inception/reverse-engineering/`. The review proposal explicitly states existing code must NOT be used to infer or fix product scope retroactively, so deriving requirements from the draft code is intentionally avoided. Can be run on request.

## Product Gate (LIFTED 2026-09-08 by explicit user instruction)
- **Construction Code Generation is APPROVED** — lifted by the user's explicit "aidlc 확인해서 진행해 … 구현 … autopilot 모드로 너가 계속 진행해" (2026-09-08), NOT auto-lifted by autopilot. Rationale: requirements explicitly approved + all design stages complete + git-reversible/local/unpublished work. See audit.md.
- The pre-existing `src/`/`tests/` were **unapproved drafts**; they are being regenerated to conform to the approved design (drafts are reference-only, not authoritative).
- No stage approval or execution history may be created retroactively (this lift is recorded prospectively).
- Active scope: the 2026-09-09 web knowledge delta supersedes the former remote-HTTP/compiled-reader exclusion for configured fixed-endpoint publication reads only. No move/rename/merge, multi-source authoring, semantic search, delete, automatic review approval or internal AI invocation.

## Code Location Rules
- **Application Code**: Workspace root (NEVER in aidlc-docs/)
- **Documentation**: aidlc-docs/ only

## Stage Progress
### 🔵 INCEPTION PHASE
- [x] Workspace Detection
- [ ] Reverse Engineering (deferred — see Workspace State)
- [x] Requirements Analysis (approved 2026-09-08; `requirements/requirements.md`)
- [x] User Stories (autopilot-approved 2026-09-08; 3 personas, 21 first-Unit stories, 6 follow-up stubs; `user-stories/personas.md`, `user-stories/stories.md`)
- [x] Workflow Planning (autopilot 2026-09-08; `plans/execution-plan.md`)
- [x] Application Design — autopilot 2026-09-08; `application-design/` (components, component-methods, services, component-dependency, application-design)
- [x] Units Generation — SKIP (single cohesive first Unit; follow-ups deferred per requirements §7)

### 🟢 CONSTRUCTION PHASE — single unit `okc-mcp-first-unit`
- [x] Functional Design — autopilot 2026-09-08; `construction/okc-mcp-first-unit/functional-design/` (domain-entities, business-rules, business-logic-model)
- [x] NFR Requirements — autopilot 2026-09-08; `.../nfr-requirements/` (nfr-requirements, tech-stack-decisions)
- [x] NFR Design — autopilot 2026-09-08; `.../nfr-design/` (nfr-design-patterns, logical-components; resiliency design-stage Qs resolved, no blocking findings)
- [x] Infrastructure Design — SKIP (no cloud/deploy infra; local tarball + stdio)
- [x] Code Generation — DONE (autopilot 2026-09-08). Part 1 (plan) + Part 2 (generation) complete via **refactor-in-place** of the hardened draft to the approved design (canonical rejection kinds, categorized audit, centralized `applyMutation`, design tool surface, backup traceability + `locate`, per-check `doctor`). `npm run typecheck` + `npm run build` PASS; pure-logic runtime-verified. Plan: `construction/plans/okc-mcp-first-unit-code-generation-plan.md`; summary: `construction/okc-mcp-first-unit/code/code-summary.md`.
- [x] Build and Test — **COMPLETE & VERIFIED GREEN** (2026-09-09, macOS Node v24.13.1). Instruction artifacts authored (autopilot 2026-09-08): `construction/build-and-test/{build-instructions,unit-test-instructions,integration-test-instructions,performance-test-instructions,security-test-instructions,build-and-test-summary}.md`. Full `npm run check` **executed → exit 0**: `npm run typecheck` PASS · `npm test` **48/48 pass, 0 fail** · `npm run build` PASS (`dist/` emitted). Two defects found+fixed on first real run: test-side wrong assertion (body-only update preserves frontmatter) + code-side synchronous throw from Promise-returning `updateNote`/`standardizeFrontmatter` guard (made `async` → rejects uniformly). build-and-test-summary.md updated with actual tallies.

### 🟢 CONSTRUCTION PHASE — Unit 2 `okc-mcp-retrieval-unit` (NEW — awaiting approval)
- [x] Requirements delta — DRAFTED 2026-09-09 (`inception/requirements/requirements-retrieval-unit.md`): REQ-014..017, decision log D14..D20, §7/§8 clarifying notes, honesty-stance edits (D20 gate), cross-Unit-edit disclosure, 7 open questions. **Awaiting explicit user approval + BM25/backlinks product go/no-go.**
- [x] Functional Design — DRAFTED 2026-09-09 (`construction/okc-mcp-retrieval-unit/functional-design/{business-rules,business-logic-model}.md`): BR-RANK/FOLD/LINK/OUTLINE/VISIBLE rules; per-capability tool surface/algorithm/edge-cases/tests; shared BR-VISIBLE-1 latent-bug fix; unresolved combined rank+fold semantics + missing-test gaps flagged. **Awaiting approval.**
- [ ] NFR Requirements / NFR Design — deferred; first-Unit NFRs (security/data-protection/PBT-partial) largely inherited; retrieval-unit adds only read-path bounds already covered by REQ-008 (SCAN_LIMIT/RESPONSE_LIMIT). Revisit only if approved scope requires.
- [x] Code Generation (PARTIAL — reduced scope) — DONE & VERIFIED GREEN 2026-09-09. Implemented `src/search.ts` (`foldText`), `src/notes.ts` (`outlineHeadings` + the BR-VISIBLE-1 `visibleMarkdown` length-preserving fix), `src/server.ts` (`search_notes` `fold` param + honest description per D20; new read-only `outline_note` tool). TDD RED→GREEN; `npm run check` **exit 0 → typecheck PASS · tests 58/58 (was 48; +10) · build PASS (dist emitted incl. dist/search.js)**. Setext headings deferred (ATX-only — the safest minimal cut). Then per the go/no-go: **REQ-016 `list_backlinks` IMPLEMENTED** (refactored `auditNotes`' link resolver into shared exported `resolveWikiLink`/`wikiLinkOccurrences` — audit output byte-preserved; added `backlinksOf` + read-only `list_backlinks` tool) and **REQ-014 BM25 DROPPED**. `npm run check` re-run **exit 0 → typecheck PASS · tests 62/62 (was 48; +14) · build PASS**. Deferred/documented for later: setext headings, combined rank+fold (moot — BM25 dropped).
- Produced via ultracode Workflow (14 agents, 0 errors, ~1.25M subagent tokens): per-capability design → 2-lens adversarial verify → synthesis → completeness critic. Run `wf_701c07b0-32d`.

## Extension Configuration
Recorded from `requirement-verification-questions.md` answers (2026-09-08):
- **security-baseline** — **Disabled** (Q9=B). Full rules file NOT loaded. NOTE: product's own security requirements REQ-008/REQ-011 remain in scope regardless (pending confirmation via clarification Q2).
- **resiliency-baseline** — **Enabled** (Q10=A) as directional design-time guidance. Load `.aidlc-rule-details/extensions/resiliency/baseline/resiliency-baseline.md` when generating requirements/NFR/design.
- **property-based-testing** — **Enabled: Partial** (Q11=B) — PBT for pure functions + serialization round-trips only. Load `.aidlc-rule-details/extensions/testing/property-based/property-based-testing.md` at testing/design stages.

## Historical Session Resume Points (superseded by the active web follow-up)
- **Last Completed Stage**: Workspace Detection
- **Next Action**: User answers the questions in `aidlc-docs/inception/plans/story-generation-plan.md`. Then analyze answers for ambiguity (Step 9-10), get plan approval, then Part 2 generation (`user-stories/stories.md` + `user-stories/personas.md`). Assessment recorded in `aidlc-docs/inception/plans/user-stories-assessment.md`.
- **Resiliency answers on file** (`requirements-resiliency-questions.md`): Q1=A (single pre-change external backup + manual recovery), Q2=A (lightweight release governance: tag + CHANGELOG + release notes).
- **Autopilot mode active** (2026-09-08): make recommended-but-MVP-scoped stage choices, proceed through gates without blocking, keep full artifacts + audit. HARD STOP before Code Generation (real src/) — product gate NOT auto-lifted. Ultracode on (use Workflow for substantive stages).
- **Next Action**: AWAIT explicit user approval to begin Code Generation (writes real src/). All inception + construction design docs complete. On approval: Code Generation Part 1 (plan) → Part 2 (generate code + PBT/behavior tests for unit `okc-mcp-first-unit`) → Build and Test. NOTE: existing `src/`/`tests/` are unapproved drafts; decide with the user whether to build fresh or reconcile.
- **Do NOT use the Workflow tool** — it triggers a permission prompt the user reads as "asking" (rejected 2026-09-08). Author artifacts directly.
- **Verification answers on file** (`requirement-verification-questions.md`): Q1=C, Q2=A, Q3=A, Q4=B, Q5=A, Q6=A, Q7=A, Q8=A, Q9=B, Q10=A, Q11=B.
- **Clarification answers on file** (`requirements-clarification-questions.md`): Q1=A (first-Unit "정리·구조화" = in-note tidying only, no file move/rename/merge), Q2=A (security extension off = extra ruleset only; REQ-008/011 remain in scope). Final contradiction check: no remaining contradictions.

## Active Web Knowledge Follow-up — 2026-09-09

- [x] Workspace/state/prior requirement/design/code verification; preceding root integration review identified the gaps.
- [x] Requirements/stories and scoped application/functional/NFR design: `inception/requirements/requirements-web-knowledge.md`, `construction/okc-mcp-web-knowledge/`.
- [x] Workflow/code generation plan executed: `construction/plans/okc-mcp-web-knowledge-code-generation-plan.md`.
- [x] Configured web-first read tools, explicit local reads, local-only authoring, web-only readonly startup, revision/provenance/stale metadata, bounded HTTP, new source initialization/template resource.
- [x] Existing missing fast-check framework repaired; shrinking/fixed-seed PBT active in npm test.
- [x] Full verification: typecheck/build pass; 76 tests pass, no skip/fail. Synthetic real HTTP plus real stdio/doctor included. Scope-specific regression covers core original legacy paths.
- [x] README English/Korean, examples, build/test summary and audit updated.
- Infrastructure design / additional decomposition: skipped (one local client; no deployment change). Operations: placeholder; no external publication performed.
- Root coordinator reported actual integration PASS: MCP authoring → Rust hooks CBOR → real core/web → private revision-pinned MCP read/verify/explain (`scripts/test_integration.py`). Any newly reported defects remain part of the root initiative; aggregate evidence stays in root AI-DLC.

## Active Session Capture Follow-up — 2026-09-09

- [x] Module workspace detection, scoped baseline and prior-contract review.
- [x] Requirements/stories, application/functional/NFR design and code-generation plan.
- [x] Local discovery, marker/section persistence and MCP workflow.
- [x] Domain/PBT/stdio verification, documentation and final checks (103 tests, build/type checks and package inclusion passed).
- Plan: [session capture](construction/plans/okc-mcp-session-capture-code-generation-plan.md).
- Existing tool and source/web contracts remain; new local placement helpers are authorized. No new provider or infrastructure.

## Active Local Install Follow-up (Root Unit U2) — 2026-09-09

- [x] Construction (Functional Design + Code Generation) for root-initiative **Unit U2** (`../../aidlc-docs/inception/plans/local-install-unit-of-work.md`), requirements LIR-M1..M5 (`../../aidlc-docs/inception/requirements/local-install-requirements.md`). Design: [functional-design](construction/local-install/functional-design/functional-design.md).
- [x] Added optional `agents` enum (`claude`|`codex`, deduped) to the strict zod config. New credential-isolated `src/install.ts`: `runSetup` (0600 config at `${XDG_CONFIG_HOME|~/.config}/okc-mcp/config.json`, best-effort non-blocking web validation via reused `WebVault`, official-CLI registration with print-only fallback, injectable `CommandRunner`), `runUnregister` (`--purge` removes only the generated config; never the Vault), shared `clientConfigSnippet`. Wired `setup`/`unregister` in `cli.ts`. Examples + README/README_KOR updated.
- [x] Tests: `agents` config round-trip PBT (NFR-4) + `tests/install.test.ts` (argv, CLI-absent fallback, web-warn, 0600, token-never-logged, idempotent, unregister+purge). Verification: `npm run typecheck` PASS · `npm run build` PASS · 19/19 U2-relevant tests green · full suite 82/83 (the 1 failure is pre-existing/unrelated — `server.test.ts` US-IN-06 session-capture tool-surface drift, not touched by U2). Real-CLI smoke confirmed 0600 write, web warning, token redaction, `client-config` regression.
- Security Baseline applicable rules enforced (SECURITY-03/05/06/09/10/11/12/13/15); no new dependency; `package-lock.json` unchanged. No commit/push/publish performed. Aggregate cross-module verification remains root-owned.

### Session capture trigger clarification

User explicitly selects the session/content to record. No always-on recording or lifecycle triggers. Both capture tools require a per-call userSelected=true declaration (default false); prior captures never enable later ones. Relevance/placement remains automatic within the selected content. REQ-030 and negative domain/stdio tests passed; final re-verification complete. See [session capture verification](construction/build-and-test/session-capture-summary.md).
