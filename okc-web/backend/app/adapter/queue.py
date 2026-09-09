"""``adapter.queue`` — S0.A single-writer engine serialization (Q6 / NFR-CONC-1).

The Rust design used a dedicated blocking ``EngineActor`` OS thread owning the
sole ``OkcClient`` behind a bounded mpsc queue, because ``Job.result()`` blocks.
The Python port is faithful: a ``ThreadPoolExecutor(max_workers=1)`` IS the single
writer — every reserving/mutating op runs on that one worker, one at a time,
serialized on top of the binding's process-global reservation set. Non-reserving
reads run on a separate small pool backed by a SECOND ``OkcClient`` (the read
path), so a status/serving read never stalls behind a multi-minute integrate.

Async FastAPI handlers bridge to the worker via ``loop.run_in_executor`` (await
terminal for fast ops) or fire-and-forget submit (long ops → return ``JobId``
immediately; the worker pumps progress into the ``JobStore``).
"""

from __future__ import annotations

import asyncio
import logging
from collections.abc import Callable
from concurrent.futures import ThreadPoolExecutor
from typing import Any, TypeVar

from app.adapter import dto
from app.adapter.engine import OkcEngineImpl, build_client
from app.shared.error import EngineError
from app.shared.jobs import JobId, JobProgress, JobStore

_log = logging.getLogger("okc_web.engine")
T = TypeVar("T")

# Ops that reserve/mutate the project (go through the single writer). Everything
# else (taxonomy/clusters/manifest/verify/explain) is a non-reserving read.
RESERVING_OPS = frozenset({
    "status", "add_source", "rebind_source", "replace_sources", "set_ai_route", "preflight",
    "integrate", "approve_taxonomy", "approve_cluster", "regenerate_cluster", "compile",
})


class EngineWorker:
    def __init__(self, engine: OkcEngineImpl, reader: OkcEngineImpl, jobs: JobStore) -> None:
        self._engine = engine
        self._reader = reader
        self._jobs = jobs
        # max_workers=1 => exactly one reserving/mutating op at a time (single writer).
        self._engine_pool = ThreadPoolExecutor(max_workers=1, thread_name_prefix="okc-engine")
        self._read_pool = ThreadPoolExecutor(max_workers=4, thread_name_prefix="okc-read")

    @classmethod
    def create(cls, specs: list[dto.ProviderSpecView], jobs: JobStore) -> EngineWorker:
        # Two independent clients: one owned by the single writer, one for reads.
        engine = OkcEngineImpl(build_client(specs))
        reader = OkcEngineImpl(build_client(specs))
        return cls(engine, reader, jobs)

    # --- fast reserving/mutating op: submit to the single writer, await terminal ---
    async def call(self, fn: Callable[[OkcEngineImpl], T]) -> T:
        loop = asyncio.get_running_loop()
        return await loop.run_in_executor(self._engine_pool, lambda: fn(self._engine))

    # --- non-reserving read: separate pool + read-path client (queue bypass) ---
    async def read(self, fn: Callable[[OkcEngineImpl], T]) -> T:
        loop = asyncio.get_running_loop()
        return await loop.run_in_executor(self._read_pool, lambda: fn(self._reader))

    # --- long reserving op: create the job, return its id immediately, run async ---
    def enqueue(
        self,
        kind: str,
        project_id: str | None,
        requested_by: str | None,
        run: Callable[[OkcEngineImpl, Callable[[JobProgress], None]], Any],
    ) -> JobId:
        job_id = self._jobs.create(kind, project_id, requested_by)

        def worker() -> None:
            try:
                self._jobs.mark_running(job_id)
                _log.info("engine job %s (%s) started project=%s by=%s", job_id, kind, project_id, requested_by)
                run(self._engine, lambda p: self._jobs.record_progress(job_id, p))
                self._jobs.record_terminal(job_id, None)
                _log.info("engine job %s (%s) ok", job_id, kind)
            except EngineError as e:
                _log.warning("engine job %s (%s) failed: %s/%s", job_id, kind, e.code.value, e.category.value)
                self._jobs.record_terminal(job_id, e)
            except Exception as exc:  # noqa: BLE001 - never lose a job to an unexpected error
                _log.exception("engine job %s crashed", job_id)
                self._jobs.record_terminal(job_id, EngineError.internal(str(exc)))

        self._engine_pool.submit(worker)
        return job_id

    def shutdown(self) -> None:
        self._engine_pool.shutdown(wait=False, cancel_futures=True)
        self._read_pool.shutdown(wait=False, cancel_futures=True)
