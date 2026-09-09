# Web knowledge workflows

Read: resolve explicit source or configured default. Local delegates to existing Vault. Web validates contract identity, creates one snapshot, requests data with revision, enforces bounds, and attaches source metadata. Multi-file search/audit/backlinks reuse this snapshot. A later call can request the same revision; absence requests current publication. Any remote error returns a typed MCP error without local fallback.

Author: read_note with source=local provides current original SHA-256; existing authoring service validates requested changes, writes backup for an existing note, and applies the atomic update. Response identifies local and pending upload/integration. Hooks/web own later transmission, review and publication.

Initialize: validate absent absolute destination and existing real parent; reject immutable ancestors; exclusively create root and conventional empty folders; emit outside-Vault configuration and optional template resource URI. Existing Vaults continue through config/serve without initialization.

Component dependencies: CLI uses config/setup/Vault/WebVault; server uses existing authoring, notes, Vault and WebSnapshot reader interface; WebVault uses native fetch plus runtime schema validation; it never calls core provider APIs. Root owns the inter-module endpoint contract.
