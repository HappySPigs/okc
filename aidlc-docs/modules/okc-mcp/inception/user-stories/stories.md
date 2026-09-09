# User Stories — Obsidian MCP for authoring an OKC input Vault (First Unit)

**Status**: User Stories output — awaiting explicit user approval. Traceable to the approved [`../requirements/requirements.md`](../requirements/requirements.md) (REQ-001..013) and its 5 success criteria (SC1..SC5). Personas: [`personas.md`](personas.md).

**Method**: generated per persona, then adversarially verified (INVEST + REQ traceability + MVP scope-creep), synthesized, checked by a completeness critic, and repaired once. Result: **21 active first-Unit stories**, **6 follow-up stubs**. Scope-creep found: none. Coverage gaps: none (see §Coverage; REQ-012 is an intentional non-story).

**Conventions**
- Format: `As a <persona>, I want <capability>, so that <benefit>`.
- Acceptance criteria: Given / When / Then, testable.
- Priority: **MoSCoW** (Must / Should / Could / Won't). Follow-up stubs are **Won't** (this Unit).
- Each story tags its `REQ` ids and the `SC` success criteria it serves.

---

## P1 — 신규 설치자 (New Installer)

### US-IN-01 · Install the MCP from a local tarball using docs alone — **Must**
_As a New Installer, I want to install the MCP locally from a documented npm tarball, so that I can get a working local server without needing the Obsidian app, a REST plugin, an API key, or an OKC binary._
- Given the published documentation and the local tarball, When I follow the install steps exactly as written, Then the MCP installs and its executable is runnable with no undocumented manual step.
- Given a machine with only Node/npm and a local filesystem, When I complete installation, Then no network access, Obsidian app, REST plugin, API key, or OKC binary is required for the install to succeed.
- Given a completed install, When I invoke the server over stdio, Then it starts and reports its version string.
- **REQ**: REQ-001, REQ-010 · **SC**: SC1

### US-IN-02 · Register exactly one existing Vault without migration — **Must**
_As a New Installer, I want to connect exactly one existing Vault by its absolute path, so that I can author against my real notes without relocating folders or migrating to custom frontmatter._
- Given the absolute path to an existing Vault, When I register it, Then the Vault is accepted with no folder relocation and no forced frontmatter migration.
- Given an already-registered Vault, When I attempt to register a second Vault in the same session, Then the request is rejected because the first Unit binds a single Vault.
- Given a registered Vault, When I inspect the configuration, Then exactly one Vault root is bound as an editable input Vault.
- **REQ**: REQ-001, REQ-002, REQ-009 · **SC**: SC1, SC2

### US-IN-03 · Generate reviewable configuration stored outside the Vault — **Must**
_As a New Installer, I want the tool to generate its configuration into a location outside the input Vault, so that I can review the setup and keep tool state from polluting my knowledge files._
- Given a registered Vault, When I run configuration generation, Then a human-readable config file is written outside the Vault and never inside it.
- Given generated configuration, When I open it, Then I can review the Vault binding and settings before using the tool.
- Given configuration, backups, or templates are created, When I inspect the Vault directory, Then none of that tool state appears inside it.
- **REQ**: REQ-009, REQ-010 · **SC**: SC1

### US-IN-04 · Run diagnostics/self-check before authoring — **Must**
_As a New Installer, I want to run a diagnostics/self-check command, so that I can confirm the environment and Vault binding are valid before I trust the tool with my notes._
- Given an installed MCP and registered Vault, When I run the diagnostics command, Then it validates the Node runtime, filesystem access, and Vault reachability, and reports pass/fail per check.
- Given a Vault path that is missing, unreadable, or resolves outside allowed bounds, When I run diagnostics, Then the check fails with an actionable message rather than silently proceeding.
- Given diagnostics run, When it completes, Then it performs a shallow local self-check only and makes no network calls.
- **REQ**: REQ-010, REQ-008 · **SC**: SC1

### US-IN-05 · Emit absolute-path MCP client configuration — **Must**
_As a New Installer, I want the tool to output ready-to-paste MCP client configuration using absolute paths, so that my MCP client connects to the correct local server on the first try._
- Given a completed install and registered Vault, When I request client-config output, Then it contains the server command and Vault reference as absolute paths with no relative paths or unresolved variables.
- Given the emitted client config, When I paste it into a supported MCP client, Then the client launches and connects to the server over stdio.
- Given the emitted client config, When I review it, Then it embeds no secrets, API keys, or network endpoints.
- **REQ**: REQ-010, REQ-001 · **SC**: SC1
- _Note: AC2 is end-to-end verifiable only with an external MCP client present; the server-side outputs (AC1, AC3) are independently verifiable._

### US-IN-06 · See the safe tool surface (no dangerous capabilities) at setup — **Must**
_As a New Installer, I want to confirm the installed tool exposes no shell, arbitrary HTTP, delete, auto-approval, or AI-invocation capability, so that I can trust the local setup cannot take destructive or unbounded actions on my notes._
- Given the installed MCP, When I list its available tools, Then no shell-execution, arbitrary-HTTP, delete, auto-approval, or internal-AI-invocation tool is present.
- Given the tool surface, When I review each exposed tool, Then every tool operates on note content as untrusted data and offers no destructive or network-egress action.
- **REQ**: REQ-011 · **SC**: SC1, SC2

### US-IN-09 · Confirm access is bounded to the registered Vault — **Must**
_As a New Installer, I want the tool to reject access outside the registered Vault and to bound file size, file count, and response size, so that I can trust it cannot reach unintended files or exhaust resources._
- Given a request that targets a path outside the registered Vault, or a symlink, hardlink, or hidden control path, When it is attempted, Then the tool rejects it.
- Given any read or listing operation, When the file size, file count, or response size would exceed configured bounds, Then the operation refuses to exceed the limits and reports the bound.
- Given a rejected out-of-bounds request, When the rejection occurs, Then it fails safely without partial access or side effects.
- **REQ**: REQ-008 · **SC**: SC2

### US-IN-07 · Locate documented backup and manual recovery path at setup — **Should**
_As a New Installer, I want setup to tell me where pre-change backups are stored outside the Vault and how manual recovery works, so that I understand my data-protection posture before I start editing._
- Given a completed install, When I review the configuration or docs, Then the external backup location outside the Vault is stated with an absolute or clearly resolvable path.
- Given the backup location, When I read the recovery instructions, Then they describe a manual restore and state that there is no generational retention or auto-cleanup.
- **REQ**: REQ-004, REQ-009 · **SC**: SC5

### US-IN-08 · Author a first note to confirm setup works end-to-end — **Should**
_As a New Installer, I want to create one first Markdown note in the connected Vault right after setup, so that I can confirm the installation works before relying on it._
- Given a registered Vault, When I create a note with a title and body, Then a Markdown file is written into the Vault with valid frontmatter.
- Given a target path that already exists, When I attempt to create a note there, Then the tool refuses to overwrite the existing file.
- Given the note is created, When I read it back, Then the read result includes a content hash for later conflict-aware updates.
- **REQ**: REQ-003, REQ-006 · **SC**: SC1

---

## P2 — 기존 Vault 저작자 (Existing-Vault Author) — PRIMARY

> Onboarding for this persona is served by US-IN-01 / US-IN-04 / US-IN-05 / US-IN-06 (not duplicated here).

### US-AU-07 · List and literal-search notes, including Korean text — **Should**
_As an Existing-Vault Author, I want deterministic path listing and literal search that works for Korean text, with reads returning a content hash, so that I can locate the notes I need to tidy and safely feed those reads into conflict-aware updates._
- Given a registered Vault, When I list paths, Then the ordering is deterministic across repeated runs.
- Given notes containing Korean text, When I run a literal search for a Korean substring, Then matching files are returned.
- Given I read a note, When the read returns, Then it includes a content hash usable as the `expectedHash` for a later update.
- Given a large result set, When listing or searching, Then response size and file count stay within configured bounds.
- **REQ**: REQ-006, REQ-008 · **SC**: SC2, SC5

### US-AU-01 · Run a heuristic quality audit on my existing Vault — **Must**
_As an Existing-Vault Author, I want to run a read-only heuristic quality audit over my existing notes and receive a categorized list of issues, so that I can understand what makes my Vault a weak OKC input without being forced to migrate or restructure anything._
- Given a registered existing Vault, When I request an audit, Then I receive issues grouped by category (YAML, path, link, duplicate, operational-noise, unsupported-format), each with the offending file path.
- Given the audit reports results, When it presents findings, Then it explicitly states they are heuristic and does NOT claim OKC compiler validation passed.
- Given my existing folder layout and custom frontmatter, When the audit completes, Then no file is moved, renamed, migrated, or modified — the operation is strictly read-only.
- Given a large Vault, When the audit runs, Then file count and response size stay within bounds, paths outside the Vault (incl. symlink/hardlink/hidden control paths) are skipped, and the run terminates deterministically.
- **REQ**: REQ-007, REQ-002, REQ-008 · **SC**: SC2, SC4

### US-AU-02 · Standardize frontmatter (title/aliases/tags) within an existing note — **Must**
_As an Existing-Vault Author, I want to standardize the title, aliases, and tags in the frontmatter of an existing note in place, so that my notes present consistent metadata to OKC without me relocating or recreating files._
- Given an existing note, When I apply a partial frontmatter update for title/aliases/tags, Then only those keys change and all unknown frontmatter keys, body content, and YAML comments are preserved.
- Given the update targets an existing file, When it is applied, Then the file is modified in place with no move, rename, or merge.
- Given a supplied `expectedHash`, When it does not match the file's current content hash, Then the write is rejected and the file is left unchanged.
- Given a mutation is accepted, When it is applied, Then exactly one pre-change backup is first written outside the Vault.
- **REQ**: REQ-013, REQ-005, REQ-004 · **SC**: SC3, SC4

### US-AU-03 · Fix invalid YAML frontmatter in place — **Must**
_As an Existing-Vault Author, I want the tool to help me correct invalid YAML frontmatter in an existing note while preserving everything else, so that the note parses cleanly for OKC without silently losing content._
- Given a note flagged for invalid YAML, When I request a fix, Then the corrected frontmatter is written in place while unknown keys, body content, and YAML comments are preserved.
- Given YAML that cannot be parsed or safely corrected, When a fix is attempted, Then the malformed YAML is rejected and the file is left unchanged rather than guessed at.
- Given a fix is accepted, When it is applied, Then a single pre-change backup is written outside the Vault first and no move/rename occurs.
- **REQ**: REQ-005, REQ-013, REQ-004 · **SC**: SC3, SC4

### US-AU-04 · Reinforce sources and links within an existing note — **Must**
_As an Existing-Vault Author, I want to add or strengthen source references and link text inside an existing note in place, so that claims stay evidence-backed and connections are retained when the Vault is handed to OKC._
- Given an existing note, When I add or update a source/link within its frontmatter or body, Then the change is applied in place, preserving surrounding body content, unknown keys, and YAML comments.
- Given the note already has sources or links, When I apply an update, Then existing sources and links are not dropped or overwritten unless I explicitly change them.
- Given this is in-note reinforcement, When I edit links, Then only literal link text is written — no link-graph resolution, rename-time relinking, or full Obsidian link interpretation (link validity remains the audit's concern per REQ-007).
- Given an accepted change with a supplied `expectedHash`, When the hash mismatches the write is rejected; otherwise a single pre-change backup is written outside the Vault before the edit.
- **REQ**: REQ-013, REQ-003, REQ-005, REQ-004 · **SC**: SC3, SC4

### US-AU-05 · Update a note safely with conflict detection and backup — **Must**
_As an Existing-Vault Author, I want every update to be conflict-aware and backed up before it writes, so that I never silently overwrite a concurrently changed note and can recover the prior content._
- Given a note read that returned a content hash, When I submit an update with that hash as `expectedHash` and the file is unchanged, Then the update succeeds.
- Given the file changed since I read it, When I submit the update with the stale `expectedHash`, Then the write is rejected with a conflict indication and the file is untouched.
- Given any mutation is about to occur, When it is applied, Then exactly one pre-change backup is written outside the Vault (no generational retention, no auto-cleanup).
- Given a backup exists, When I follow the documented manual restore, Then I can recover the pre-change content of that note.
- **REQ**: REQ-004, REQ-005, REQ-009 · **SC**: SC3, SC5

### US-AU-06 · Author a new evidence-backed note — **Should**
_As an Existing-Vault Author, I want to create a new Markdown note with a title and body plus optional aliases, tags, and source, so that I can add new knowledge to my Vault in an OKC-ready shape._
- Given a target path inside the Vault, When I create a note with title and body (and optional aliases/tags/source), Then a well-formed Markdown file with valid frontmatter is written.
- Given a file already exists at the target path, When I attempt to create a note there, Then the operation is refused and the existing file is not overwritten.
- Given the target path resolves outside the registered Vault or through a symlink/hardlink/hidden control path, When creation is attempted, Then it is rejected.
- **REQ**: REQ-003, REQ-008, REQ-002 · **SC**: SC1, SC3

---

## P3 — 변경 검토·복구자 (Change Reviewer / Recoverer)

### US-RV-01 · Read a note with a content hash before editing — **Must**
_As a Change Reviewer / Recoverer, I want each read of a note to return its current content hash, so that I can capture a known baseline to review against and pass back as `expectedHash` on my next update._
- Given an existing note in the registered Vault, When I read it, Then the response includes the note body/frontmatter and a deterministic content hash of the current on-disk content.
- Given I read the same unchanged note twice, When I compare the two responses, Then the returned hashes are identical.
- Given a file is changed on disk between two reads, When I read it again, Then the returned hash differs from the earlier one.
- Given a path that resolves outside the registered Vault or through a symlink/hardlink, When I attempt to read it, Then the read is rejected without returning content.
- **REQ**: REQ-006, REQ-008 · **SC**: SC5

### US-RV-02 · Block updates when the file changed since my baseline — **Must**
_As a Change Reviewer / Recoverer, I want an update to be rejected when the `expectedHash` I supply no longer matches the file on disk, so that a concurrent change is never silently overwritten._
- Given I submit an update with an `expectedHash` that matches the current on-disk content, When the update runs, Then it is applied and the file is written.
- Given the file changed on disk after my last read, When I submit an update with the now-stale `expectedHash`, Then the update is rejected and the file is left unchanged.
- Given a rejected update due to hash mismatch, When I inspect the response, Then it clearly reports a hash-mismatch/conflict and includes the current content hash so I can re-read and reconcile.
- Given an update request omits `expectedHash` for an existing file, When it runs, Then it is rejected rather than overwriting blindly.
- **REQ**: REQ-004 · **SC**: SC3, SC5

### US-RV-03 · Automatic pre-change backup outside the Vault before any mutation — **Must**
_As a Change Reviewer / Recoverer, I want a single pre-change backup of the prior content written outside the Vault before any mutating write, so that I always have the earlier version to recover from if an edit went wrong._
- Given an update or partial-frontmatter change is about to modify an existing note, When the mutation proceeds, Then the prior file content is first written to a backup location outside the registered Vault.
- Given a mutation is rejected (e.g. hash mismatch or malformed YAML), When no write occurs, Then no misleading backup implying a change is left behind.
- Given the backup is written, When I inspect it, Then it identifies the source note path so I can tell which note and pre-change state it represents.
- Given I perform a second mutation on the same note, When the new pre-change backup is written, Then only the single latest pre-change backup is guaranteed (no generational retention or auto-cleanup).
- **REQ**: REQ-004, REQ-009 · **SC**: SC5

### US-RV-04 · Manually recover earlier content from the external backup — **Must**
_As a Change Reviewer / Recoverer, I want documented manual recovery of earlier content from the external pre-change backup, so that I can restore a note after an edit went wrong._
- Given a pre-change backup exists outside the Vault, When I follow the documented recovery procedure, Then I can obtain the exact prior content of the affected note.
- Given I want to restore that content, When I re-author the note using standard update tools with a matching `expectedHash`, Then the restore is applied through the same conflict-aware, backup-generating path as any other write.
- Given recovery is manual by design, When I look for automatic rollback or generational restore, Then the tool exposes none and the docs describe restore as a manual step (RTO = manual).
- **REQ**: REQ-004, REQ-009 · **SC**: SC5

### US-RV-05 · Malformed-YAML and structure-preserving guardrails on updates — **Should**
_As a Change Reviewer / Recoverer, I want partial updates to reject malformed YAML and preserve unknown keys, body, and comments, so that reviewing a change never risks corrupting or dropping content I did not intend to touch._
- Given a partial frontmatter update that would produce malformed YAML, When I submit it, Then it is rejected and the file is left unchanged.
- Given a partial frontmatter update touching only specific keys, When it succeeds, Then unknown frontmatter keys, YAML comments, and the note body are preserved verbatim.
- Given a successful partial update, When I re-read the note, Then the returned content hash reflects exactly the intended change and nothing else.
- **REQ**: REQ-005 · **SC**: SC3, SC5

---

## Follow-up Units — deferred stubs (Won't, this Unit)

Kept as stubs for traceability only; no acceptance criteria, not built in the first Unit.

| ID | Title | Deferred to | Note |
|---|---|---|---|
| US-AU-08 | Author onboarding (retired) | N/A — folded into US-IN-01/04/05/06 | Was an oversized bundle re-asserting the atomic installer stories; retired per INVEST 'Small'. Author onboarding = cross-mapped US-IN stories. No capability lost. |
| US-AU-09 | File move/rename/merge with link-preserving recovery UX | Follow-up Unit 1 | First Unit does in-note tidying only; no move/rename/merge (REQ-013 / D4 / D7). |
| US-AU-10 | Multi-Vault authoring and comparison | Follow-up Unit 2 | First Unit binds exactly one Vault (D1). |
| US-IN-FU-01 | Publish/update distribution beyond local tarball | Follow-up Unit 5 | First Unit installs from a local npm tarball only. |
| US-IN-FU-02 | Connect and compare multiple Vaults | Follow-up Unit 2 | Single-Vault only (D1); installer-side duplicate of US-AU-10. |
| US-RV-07 | Link-preserving recovery UX for file move/rename/merge | Follow-up Unit 1 | No file move/rename/merge in the first Unit, so no link-preserving recovery UX. |

Other excluded (per requirements §8): semantic search, Obsidian UI/Dataview, remote HTTP, provider execution, OKC review approval, attachment/Canvas/Base conversion, full link interpretation, deletion, auto-approval.

---

## Coverage & traceability

**Requirements → stories (first Unit)**
- REQ-001 installable local stdio MCP — US-IN-01, US-IN-02, US-IN-05
- REQ-002 respect existing Vaults (no forced migration) — US-IN-02, US-AU-01, US-AU-06
- REQ-003 knowledge authoring (refuse overwrite) — US-IN-08, US-AU-04, US-AU-06
- REQ-004 conflict-aware updates + single external pre-change backup, manual recovery — US-AU-02/03/04/05, US-IN-07, US-RV-02/03/04
- REQ-005 preserve structure; reject malformed YAML — US-AU-02/03/04/05, US-RV-05
- REQ-006 discovery (deterministic listing, literal Korean search, read-with-hash) — US-IN-08, US-AU-07, US-RV-01
- REQ-007 heuristic quality audit (no compiler-pass claim) — US-AU-01
- REQ-008 bounded authority — US-IN-04, US-IN-09, US-AU-01, US-AU-06, US-AU-07, US-RV-01
- REQ-009 separate knowledge & tool state — US-IN-02/03/07, US-AU-05, US-RV-03/04
- REQ-010 reviewable installation — US-IN-01/03/04/05
- REQ-011 trust boundary — US-IN-06
- REQ-012 lifecycle records — **no user story (intentional)**: a project meta/documentation requirement satisfied by the AI-DLC artifact trail, not an author-facing capability.
- REQ-013 in-note organization (first-Unit primary journey; no move/rename/merge) — US-AU-02, US-AU-03, US-AU-04

**Success criteria → stories**
- SC1 install from docs alone + author a first note — US-IN-01/02/03/04/05/06/08, US-AU-06
- SC2 understand problems + improvements without forced migration — US-IN-02/06/09, US-AU-01/07
- SC3 add/update without losing sources, conflicting claims, or links — US-AU-02/03/04/05/06, US-RV-02/05
- SC4 input-structure problems + collection noise reduced for OKC — US-AU-01/02/03/04
- SC5 review changes, identify conflicts, recover earlier content — US-IN-07, US-AU-05/07, US-RV-01/02/03/04/05

**Verification summary (adversarial)**
- MVP scope-creep check: **passed** — no active first-Unit story implies file move/rename/merge, multi-Vault, real OKC ingestion/compiler validation, semantic search, Obsidian UI/Dataview, remote HTTP, deletion, or auto-approval. Single-Vault rejection (US-IN-02 AC2) and dangerous-tool-absence (US-IN-06) are trust-boundary *enforcement*, not deferred-capability *implementations*.
- INVEST repair applied: the oversized `US-AU-08` bundle was retired to a stub; `US-IN-06` was split into US-IN-06 (REQ-011 tool surface) + US-IN-09 (REQ-008 bounded authority). A baseless `preview/diff` story (no REQ) was dropped.
- No genuine first-Unit capability gap; REQ-001..011 and REQ-013 each map to ≥1 active story, and SC1..SC5 are each served by ≥1 story.
