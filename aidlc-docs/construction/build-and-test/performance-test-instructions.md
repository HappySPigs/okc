# Performance and Bounds

No new throughput/latency SLO or production load benchmark was requested. Performance assessment is limited to bounded work and configured limits.

MCP enforces request timeout, response bytes, file count, note bytes, per-operation scan bytes and cancellation. Web sync bounds raw snapshot bytes, entries, path length and chunks; negotiated transfer offsets and temporary blobs avoid replaying completed content. Hooks spools wanted file bytes once before transmission and coalesces events until retry deadlines.

Tests exercise these relevant bounds and retries. Full semantic-scale core benchmarks, multi-tenant load and cross-OS native service behavior remain outside the accepted current product scope; do not infer results from fast unit tests.
