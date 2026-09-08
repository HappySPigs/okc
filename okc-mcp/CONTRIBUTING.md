# Development and review

The product scope is **authoring an Obsidian source Vault suitable for OKC**. Read the [requirements](aidlc-docs/inception/requirements.md) and [current status](aidlc-docs/state.md) first. Do not reproduce OKC compiler or approval policy in this repository.

```sh
npm ci
npm run check
npm pack
```

Keep changes small and include the related requirement, user behavior, and test evidence in the same pull request. Update the README and traceability table for new CLI commands, tools, or configuration. Mark breaking changes in the changelog. If a design alternative changes the product boundary, record it in the AI-DLC Inception requirements first.

Use only synthetic data and temporary Vaults in tests. Do not commit or attach personal Vaults, private source text, configuration, backups, or tokens. Review licensing and attribution separately before copying code from another project.

For write changes, verify stale-hash handling, create-without-replacement, external backups, path and file-type boundaries, and real stdio calls. Do not hide safety limitations or add supported operating systems without test evidence. For documentation changes, check links and executable examples. A successful local package build is separate from public release.
