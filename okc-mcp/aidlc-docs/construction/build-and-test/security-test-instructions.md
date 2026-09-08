# Security Test Instructions — okc-mcp (first Unit)

The product's own security requirements (REQ-008 bounded authority, REQ-011 trust boundary) are **in scope regardless** of the disabled security-baseline extension (D9). They are verified as behavior tests, not a separate scanner.

## Trust boundary & bounded authority (REQ-008/011, BR-PATH-*/BR-BOUND-*/BR-TRUST-*)
Covered by `tests/vault.test.ts` and `tests/server.test.ts`:
- **Path containment**: reject paths outside the Vault, `..` traversal, absolute/backslash paths, path-depth, NFC/case collisions.
- **Symlink/hardlink rejection**: symlinked files/parents and hard-linked files are refused (reads, writes, and listing skips them); reads use `O_NOFOLLOW`.
- **Hidden/control paths**: `.obsidian`, `.git`, `.okc`, `.trash`, `*.okc-project`, and any dotfile segment are refused/skipped; compiled OKC artifacts are refused as an authoring root.
- **Resource bounds**: file size / file count / scan bytes / response bytes are enforced with refusals, not silent truncation.
- **No dangerous capability**: the tool list contains no shell, arbitrary-HTTP, delete, auto-approval, or AI-invocation tool; no network egress exists in `src/` (verified by inspection — no `child_process`/`http`/`https`/`fetch`/`net` imports).
- **Untrusted content**: note content is treated as data; errors are redacted so untrusted parser/source text is never echoed; TOCTOU re-stat guards and an advisory write lock prevent races.

## Run
```bash
npm test                 # the security behaviors are part of the suite (Node >= 22.13)
```

## Dependency check (recommended, network host)
```bash
npm audit                # review advisories for the 3 runtime deps + dev deps
```

## Notes / deferred
- No auto-approval: all mutating tools default to `dryRun: true`; applying requires an explicit `dryRun: false`.
- Sensitive-content detection in the audit is a **heuristic hint only** and never claims completeness (BR-AUDIT-3); real disclosure preflight belongs to OKC.
- Penetration testing / fuzzing beyond the property tests is deferred to Operations.
