# Performance Test Instructions (W5)

Performance is **not a scored dimension** for this hackathon MVP, and the design is deliberately single-process/single-writer. This doc records the few properties worth a smoke check and the rationale for not building a load harness.

## Properties that matter (and how they're already structured)
- **Read path never blocks behind a multi-minute engine job.** Reserving/mutating ops run on the single-writer `ThreadPoolExecutor(max_workers=1)`; status/serving reads use a **separate** read pool + a second read-path `OkcClient` (`app/adapter/queue.py`). So `GET /status`, job polling, and serving reads stay responsive while an `integrate` runs.
- **Single active mutation per project.** U3 rejects a duplicate reserving op pre-enqueue with retryable `PROJECT_BUSY` (409 + `Retry-After`); the single writer is the backstop. No thundering herd.
- **Long ops are async + polled.** `integrate`/`regenerate` return a `JobId` immediately; clients poll `GET /api/projects/{id}/jobs/{jobId}` (fast SQLite WAL read). No request is held open for minutes.

## Optional smoke checks (manual)
```bash
# read latency under a simulated in-flight job (should stay fast):
#   1) start the app; insert/allow a running job; hammer GET /status + /jobs/{id}
# app startup fail-fast:
cd backend && .venv/bin/python -c "from app.main import create_app; create_app()"   # SchemaGuard + migrate + worker wiring
```

## Explicitly out of scope (MVP)
Concurrency load tests, throughput/latency SLAs, multi-process scaling, and provider-latency benchmarks. The design constraints (single org, single process, ≤10 sources/project, HTTP polling not SSE) make a formal load harness low-value for the demo. Revisit if productionized.

Continuous upload verification now checks 13 revisions within one source and updates
at the ten-source cap, plus bounded CBOR requests and snapshot size. See
[continuous-sync verification](continuous-sync-verification.md). No throughput SLA is claimed.
