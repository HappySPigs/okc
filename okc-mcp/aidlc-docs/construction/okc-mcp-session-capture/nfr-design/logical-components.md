# Session capture logical components

CaptureGuide -> host agent -> MCP adapter -> CaptureService -> existing Vault/authoring.
CaptureService reads local notes and calls pure candidate/marker/section helpers.
All physical mutations remain behind authoring.applyMutation and Vault.
No infrastructure allocation, database migration, additional worker or provider is introduced.

