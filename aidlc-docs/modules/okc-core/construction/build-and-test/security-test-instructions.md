# Security Test Instructions

## Automated security regressions

```bash
cargo test --locked -p okc-core --test adversarial_contract
cargo test --locked -p okc-core --test corpus_builder_contract
cargo test --locked -p okc-core integration::tests
cargo test --locked -p okc-ai
cargo test --locked -p okc-app
cargo test --locked -p okc-interop
```

## Required threat scenarios

| Boundary | Scenarios |
|---|---|
| Source | traversal, absolute/reserved/lossy/duplicate paths, symlinks, hardlinks, mutation, archive bombs, ambient ignores |
| Parser/JSON | duplicate keys, malformed YAML/Markdown/Canvas, Unicode/control ambiguity, bounded depth/size |
| Provider | credential URL/options, redirect, spoofed loopback, malformed/duplicate JSON, vector drift/non-finite values, retry deadline, secret reflection |
| Disclosure | metadata-only secrets, exact UTF-8 ranges, local routing, missing per-call remote consent |
| Project | root/object/database/sidecar aliases, lock contention, stale approvals, journal/manifest failure ordering |
| Artifact | root/marker/manifest/content symlinks, mixed/unknown/old schema, oversized plans/manifests, added/tampered files |
| Publication | existing leaf, live/dangling symlink winner, deterministic barrier race, cleanup failure, post-commit sync error |
| Runtime | full queues, cancellation boundary, terminal controls, binding path/credential restrictions |

## Dependency checks

```bash
cargo tree --locked --workspace
npm audit --audit-level=high --prefix bindings/node
npm audit --audit-level=high --prefix guide
```

Network-backed audits are time-sensitive. Record the timestamp, registry, and
network/sandbox status; do not report an unexecuted audit as passing.

## Manual review

- Confirm no raw secret/value/length enters logs, errors, state, recordings,
  progress events, manifests, provenance, or TUI.
- Confirm source/provider text cannot trigger shell, tool, HTTP, deletion,
  approval, terminal OSC, or policy behavior.
- Confirm no new dependency edge bypasses core/app ownership.
- Confirm future/proposed features are not exposed by help, SDK types, or artifacts.

## Remaining qualification

Coverage-guided fuzzing, complete TOCTOU/ancestor/reparse/hardlink matrices,
process-kill/power-loss injection, malware scanning, broader sensitive corpora,
and all supported filesystems/platforms remain release work.
