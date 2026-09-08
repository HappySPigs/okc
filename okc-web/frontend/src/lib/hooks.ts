import { useCallback, useEffect, useRef, useState } from "react";
import { ApiError, TERMINAL_JOB_STATES } from "./api";
import type { JobSnapshot } from "./types";

export interface AsyncState<T> {
  data: T | null;
  error: ApiError | null;
  loading: boolean;
  reload: () => void;
}

/** Run an async fetcher on mount + when `deps` change; expose reload. */
export function useAsync<T>(fetcher: () => Promise<T>, deps: unknown[]): AsyncState<T> {
  const [data, setData] = useState<T | null>(null);
  const [error, setError] = useState<ApiError | null>(null);
  const [loading, setLoading] = useState(true);
  const [nonce, setNonce] = useState(0);
  const fetcherRef = useRef(fetcher);
  fetcherRef.current = fetcher;

  useEffect(() => {
    let live = true;
    setLoading(true);
    setError(null);
    fetcherRef.current()
      .then((d) => { if (live) setData(d); })
      .catch((e) => { if (live) setError(e instanceof ApiError ? e : new ApiError({ code: "INTERNAL", category: "internal", message: String(e), retryable: false }, 0)); })
      .finally(() => { if (live) setLoading(false); });
    return () => { live = false; };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [...deps, nonce]);

  const reload = useCallback(() => setNonce((n) => n + 1), []);
  return { data, error, loading, reload };
}

/** Poll a JobSnapshot until it reaches a terminal state. `key` (typically the
 *  job id) gates + re-inits polling; a null key is idle. `onTerminal` fires once. */
export function useJobPoll(
  fetcher: () => Promise<JobSnapshot>,
  key: string | null,
  opts?: { intervalMs?: number; onTerminal?: (snap: JobSnapshot) => void },
): { snapshot: JobSnapshot | null; error: ApiError | null } {
  const [snapshot, setSnapshot] = useState<JobSnapshot | null>(null);
  const [error, setError] = useState<ApiError | null>(null);
  const fetcherRef = useRef(fetcher);
  fetcherRef.current = fetcher;
  const onTerminalRef = useRef(opts?.onTerminal);
  onTerminalRef.current = opts?.onTerminal;
  const intervalMs = opts?.intervalMs ?? 900;

  useEffect(() => {
    if (!key) { setSnapshot(null); return; }
    let live = true;
    let fired = false;
    let timer: ReturnType<typeof setTimeout>;
    const tick = async () => {
      try {
        const snap = await fetcherRef.current();
        if (!live) return;
        setSnapshot(snap);
        if (!TERMINAL_JOB_STATES.has(snap.state)) {
          timer = setTimeout(tick, intervalMs);
        } else if (!fired) {
          fired = true;
          onTerminalRef.current?.(snap);
        }
      } catch (e) {
        if (!live) return;
        setError(e instanceof ApiError ? e : null);
        timer = setTimeout(tick, intervalMs * 2);
      }
    };
    tick();
    return () => { live = false; clearTimeout(timer); };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [key]);

  return { snapshot, error };
}
