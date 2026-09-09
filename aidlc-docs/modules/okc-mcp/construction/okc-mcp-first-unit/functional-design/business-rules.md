# Business Rules — okc-mcp-first-unit

**Status**: Functional Design output (autopilot). Technology-agnostic rules. These are the correctness/safety spec the implementation must satisfy; they map directly to REQ-* and are the basis for tests.

## Path safety (REQ-008)
- **BR-PATH-1**: Every path is resolved to a real absolute path and MUST be a descendant of `Vault.root`. Reject `path-denied` otherwise (includes `..` traversal escaping the root).
- **BR-PATH-2**: Reject any path that traverses or targets a **symlink** or **hardlink**. Resolution must not follow links out of the Vault.
- **BR-PATH-3**: Reject hidden control paths — dotfiles/dot-directories used for tool/VCS control (e.g. `.obsidian`, `.git`, `.trash`) and any hidden segment — for both reads and writes.
- **BR-PATH-4**: Backup targets MUST be *outside* `Vault.root` (`path-denied` if inside).

## Resource bounds (REQ-008)
- **BR-BOUND-1**: Reject `bounds-exceeded` when a single file exceeds `maxFileBytes`.
- **BR-BOUND-2**: Listing/audit/search cap results at `maxFileCount`; responses cap at `maxResponseBytes`; on exceed, refuse rather than truncate silently (report the bound).

## Content hashing & conflict (REQ-004, REQ-006)
- **BR-HASH-1**: The hash is deterministic over exact on-disk bytes; equal bytes → equal hash; any byte change → different hash.
- **BR-HASH-2**: Every `read`/`readNote` returns the current hash.
- **BR-HASH-3**: An update to an existing file REQUIRES a matching `expectedHash`. Mismatch → `hash-mismatch` (response includes the current hash); the file is left unchanged.
- **BR-HASH-4**: An update that omits `expectedHash` for an existing file is rejected (`hash-mismatch`) — never a blind overwrite.

## Create vs update (REQ-003)
- **BR-CREATE-1**: `create` refuses to overwrite an existing file (`overwrite-refused`).
- **BR-CREATE-2**: A created note has valid frontmatter and the given title/body (+ optional aliases/tags/source).
- **BR-CREATE-3**: `create` performs no backup (nothing prior to protect).

## Structure preservation & partial update (REQ-005, REQ-013)
- **BR-STRUCT-1**: A partial frontmatter update changes ONLY the specified keys. All unknown keys, the body, and YAML comments are preserved verbatim.
- **BR-STRUCT-2**: Malformed YAML (input or would-be result) is rejected `malformed-yaml`; the tool never guesses or auto-repairs beyond the requested, valid change; the file is left unchanged.
- **BR-STRUCT-3**: In-note edits (standardize/fix-yaml/reinforce) MUST NOT move, rename, or merge files. Only the target file's content changes.
- **BR-STRUCT-4**: Link/source reinforcement writes literal text only — no link-graph resolution, rename-time relinking, or full Obsidian link interpretation.

## Backup & recovery (REQ-004, REQ-009)
- **BR-BACKUP-1**: Before any mutating write to an existing file, write exactly one pre-change backup of prior content outside the Vault, traceable to its source note.
- **BR-BACKUP-2**: No backup is written for a rejected mutation (fail-safe: no misleading backup).
- **BR-BACKUP-3**: Only the single latest pre-change backup is guaranteed per note; no generational retention or auto-cleanup.
- **BR-BACKUP-4**: Recovery is manual and documented; a restore is re-authored through the normal conflict-aware, backup-generating write path (no special rollback tool). RPO = last save point; RTO = manual.

## Atomic write (REQ-003/004/005)
- **BR-ATOMIC-1**: An accepted write is atomic — the note file is never left partially written. (Implementation via temp file + same-directory swap; this is an internal write mechanism, NOT a user-visible move/rename of the note path.)
- **BR-ATOMIC-2**: On any failure mid-write, the original file and its content hash are unchanged.

## Heuristic audit (REQ-007)
- **BR-AUDIT-1**: Audit is strictly read-only; it mutates nothing and moves nothing.
- **BR-AUDIT-2**: Issues are grouped into: `yaml`, `path`, `link`, `duplicate`, `operational-noise`, `unsupported-format`, each with the offending path.
- **BR-AUDIT-3**: Output is explicitly labeled heuristic and MUST NOT claim OKC compiler validation passed.
- **BR-AUDIT-4**: Audit skips out-of-bounds/symlink/hardlink/hidden paths and respects resource bounds.

## Discovery (REQ-006)
- **BR-DISC-1**: Path listing is deterministic (stable ordering across identical runs).
- **BR-DISC-2**: Search is literal substring and Unicode-correct, including Korean; matching is byte/codepoint-faithful (no locale-dependent folding that would drop Korean matches).

## Trust boundary (REQ-011)
- **BR-TRUST-1**: The tool surface exposes NO shell execution, arbitrary HTTP, delete, auto-approval, or internal-AI-invocation tool.
- **BR-TRUST-2**: Note content is untrusted data — never executed, shell-interpolated, or used to construct outbound requests. No network egress at runtime.

## Setup (REQ-001/002/009/010)
- **BR-SETUP-1**: Register exactly one Vault by absolute path; a second registration in a session is rejected.
- **BR-SETUP-2**: Registration performs no migration and no folder relocation.
- **BR-SETUP-3**: Config, backups, and templates live outside the Vault.
- **BR-SETUP-4**: Diagnostics is a shallow local self-check (runtime, filesystem, Vault reachability) with no network calls; failures are actionable.
- **BR-SETUP-5**: Emitted client config uses absolute paths, no relative paths/unresolved variables, and embeds no secrets or network endpoints.

## Rejection contract (cross-cutting)
- **BR-REJECT-1**: All refusals use the typed `Rejection.kind` set; failures are fail-safe (no partial write, no side effect, no backup).
- **BR-REJECT-2**: The surface serializes rejections into structured MCP tool errors preserving `kind`.

## Requirement → rules map
REQ-003 → BR-CREATE-*, BR-ATOMIC · REQ-004 → BR-HASH-*, BR-BACKUP-*, BR-ATOMIC · REQ-005 → BR-STRUCT-1/2, BR-ATOMIC · REQ-006 → BR-HASH-2, BR-DISC-* · REQ-007 → BR-AUDIT-* · REQ-008 → BR-PATH-*, BR-BOUND-* · REQ-009 → BR-BACKUP-1/4, BR-SETUP-3 · REQ-011 → BR-TRUST-* · REQ-013 → BR-STRUCT-1/3/4.
