# Tech stack decisions — web knowledge

Retain Node >=22.13, TypeScript strict, MCP SDK, zod and yaml. Use built-in fetch/AbortSignal/TextDecoder; no HTTP SDK or network provider dependency. Add fast-check as a development dependency to implement the already-selected shrinking PBT framework. Package lock pins the installed version.

Local serialization/authoring types stay compatible. Empty local paths are an internal absence representation permitted only for web-only read-only configuration; JSON configuration round-trips are tested. Tokens remain only in external user configuration and are sent as Bearer to the fixed configured endpoint. Use HTTPS for nonlocal credential-bearing deployments.
