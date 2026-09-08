# Build Instructions — okc-mcp (first Unit)

Current follow-up (2026-09-09): run `npm ci` then `npm run check`; Node >=22.13. The locked development dependencies include fast-check. Build also emits search, setup and web modules. Tests start a local HTTP fixture and require loopback bind permission. See [current verification](web-knowledge-summary.md); dated results below preserve the original first-unit history.

## Prerequisites
- **Build Tool**: npm + TypeScript (`tsc`), per `package.json`.
- **Runtime**: **Node.js ≥ 22.13.0** (declared in `package.json` `engines`). The test runner uses `node --test` and `node --import tsx` (require Node ≥ 18/20); the code targets ES2022/NodeNext.
- **Dependencies**: `@modelcontextprotocol/sdk@1.30.0`, `yaml@2.9.0`, `zod@4.5.4`; dev: `tsx`, `typescript@5.9.3`, `@types/node`.
- **Environment variables**: none. **System**: any OS with Node ≥ 22; no network, no database, no container.

## Build Steps

### 1. Install dependencies
```bash
npm install
```

### 2. Type-check (src + tests)
```bash
npm run typecheck        # tsc --noEmit && tsc -p tsconfig.tests.json
```

### 3. Build
```bash
npm run build            # tsc  -> emits dist/
```

### 4. Verify build success
- **Expected output**: `tsc` exits 0 with no diagnostics.
- **Build artifacts**: `dist/*.js`, `dist/*.d.ts`, `dist/*.js.map` for `authoring, cli, config, guide, notes, rejection, server, vault`. The `bin` entry `okc-mcp` → `dist/cli.js`.
- **Acceptable warnings**: none expected.

### 5. Package (local tarball, D13)
```bash
npm pack                 # runs prepack (npm run build); produces okc-mcp-<version>.tgz
```

## Actual result in this environment (2026-09-08)
- `npm run typecheck` → **PASS**. `npm run build` → **PASS** (`dist/` emitted). Both run under `tsc` and do not need the Node runtime.
- NOTE: the host used here has **Node v16.17.1** (below the required ≥ 22.13), so runtime steps (tests, `serve`) were not executed here.

## Troubleshooting
- **`node: bad option: --import` / `--test` unknown**: your Node is < 18/20. Install Node ≥ 22.13 and re-run.
- **Type errors after edits**: the project uses strict TS (`strict`, `noUncheckedIndexedAccess`, `exactOptionalPropertyTypes`); optional properties must admit `undefined` where a value may be omitted.
- **Dependency errors**: delete `node_modules` and re-run `npm install` on a network-connected host.

## Session capture follow-up (2026-09-09)

Run npm run check from okc-mcp. Existing dependencies are sufficient. The build additionally emits capture.js and capture-guide.js with declarations/maps in dist; the static guide is available through the MCP resource and capture_session prompt. Reconnect the MCP client after rebuilding. No third-party model, server installation or user Vault mutation is needed for the test suite.
