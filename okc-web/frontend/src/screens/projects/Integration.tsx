import { useEffect, useMemo, useRef, useState } from "react";
import { useNavigate, useParams, useSearchParams } from "react-router-dom";
import {
  Activity, ArrowRight, Ban, Info, Play, Server, ShieldAlert, TerminalSquare, Workflow,
} from "lucide-react";
import { ApiError, jobsApi, projectsApi } from "@/lib/api";
import { useAsync, useJobPoll } from "@/lib/hooks";
import { useProjectContext } from "@/app/project-context";
import { formatTime, phaseLabel, RUN_PHASES } from "@/lib/format";
import type { PreflightGateView } from "@/lib/types";
import { PageHeader, ErrorState } from "@/components/ui/page";
import { Card } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { Skeleton } from "@/components/ui/skeleton";
import { Switch } from "@/components/ui/switch";
import { Tooltip } from "@/components/ui/tooltip";
import { Callout } from "@/components/domain/callout";
import { PipelineTracker } from "@/components/domain/pipeline-tracker";
import { RunSteps } from "@/components/domain/steps";
import { ProgressBar } from "@/components/domain/progress";
import { Metric } from "@/components/domain/metric";
import { toast } from "@/components/ui/toaster";

// ★FOCAL #1 — Integration orchestration & live progress monitor (E3-5).
export function Integration() {
  const { id = "" } = useParams();
  const navigate = useNavigate();
  const [, setSearchParams] = useSearchParams();
  const { status, reload, loading } = useProjectContext();
  const providers = useAsync(() => projectsApi.providers(id), [id]);

  const [preflight, setPreflight] = useState<PreflightGateView | null>(null);
  const [discloseConfirmed, setDiscloseConfirmed] = useState(false);
  const [selectedProvider, setSelectedProvider] = useState("");
  const [jobId, setJobId] = useState<string | null>(null);
  const [running, setRunning] = useState(false);
  const [err, setErr] = useState<ApiError | null>(null);
  const prevPhase = useRef<string | null>(null);

  const frozen = status?.frozen ?? false;
  const checkpoint = status?.checkpoint ?? "";
  const needsProvider = checkpoint === "needs_provider";

  // Preflight is only valid once frozen (best-effort; surfaces the disclosure gate).
  useEffect(() => {
    if (frozen) projectsApi.preflight(id).then(setPreflight).catch(() => setPreflight(null));
  }, [frozen, id]);

  const { snapshot } = useJobPoll(
    () => jobsApi.adminJob(id, jobId!),
    jobId,
    {
      onTerminal: (snap) => {
        setRunning(false);
        if (snap.state === "completed") toast.success("통합 완료 — 리뷰로 진행하세요");
        else if (snap.state === "failed") toast.error(`통합 실패: ${snap.error?.code ?? "INTERNAL"}`);
        else toast("통합이 취소되었습니다");
        reload();
      },
    },
  );

  useEffect(() => {
    if (snapshot?.phase && snapshot.phase !== prevPhase.current) {
      prevPhase.current = snapshot.phase;
      toast(`단계: ${phaseLabel(snapshot.phase)}`);
    }
  }, [snapshot?.phase]);

  const requiresDisclosure = preflight?.requires_disclosure ?? false;
  const canRun = frozen && !running && (!requiresDisclosure || discloseConfirmed);

  const bind = async () => {
    if (!selectedProvider) return;
    try { await projectsApi.bindProvider(id, selectedProvider); reload(); toast.success("provider가 바인딩되었습니다"); }
    catch (e) { if (e instanceof ApiError) toast.error(e.message); }
  };

  const run = async () => {
    setRunning(true); setErr(null); prevPhase.current = null;
    try {
      const res = await projectsApi.integrate(id, requiresDisclosure, requiresDisclosure && discloseConfirmed);
      setJobId(res.job_id);
      setSearchParams({ job: "live" });
    } catch (e) {
      setErr(e instanceof ApiError ? e : null);
      setRunning(false);
    }
  };

  const elapsed = useMemo(() => {
    if (!snapshot) return "—";
    const start = new Date(snapshot.created_at).getTime();
    const end = new Date(snapshot.finished_at ?? snapshot.updated_at).getTime();
    return `${Math.max(0, Math.round((end - start) / 1000))}s`;
  }, [snapshot]);

  if (loading && !status) return <Skeleton className="h-64 w-full" />;

  return (
    <div>
      <PageHeader title="통합 오케스트레이션" description="freeze 후 통합을 실행하고 장기 Job을 라이브로 관측합니다. (엔진 Job의 관측 창 — 채팅/쿼리 아님)" />

      {status && <Card className="mb-5 p-5"><PipelineTracker checkpoint={status.checkpoint} stale={status.stale} /></Card>}

      {/* Provider & Disclosure gates (design-system §13.1) */}
      <div className="mb-5 grid gap-4 lg:grid-cols-2">
        <Card className="p-5">
          <div className="mb-2 flex items-center gap-2 text-sm font-medium"><Server className="size-4" /> Provider</div>
          {needsProvider ? (
            <div className="space-y-2">
              <select
                className="w-full rounded-lg border border-line bg-surface px-3 py-2 text-sm"
                value={selectedProvider} onChange={(e) => setSelectedProvider(e.target.value)} data-testid="provider-select"
              >
                <option value="">provider 프로파일 선택…</option>
                {providers.data?.map((p) => <option key={p.name} value={p.name}>{p.name} · {p.model}</option>)}
              </select>
              <Button variant="primary" size="sm" disabled={!selectedProvider} onClick={bind} data-testid="provider-bind">바인딩</Button>
            </div>
          ) : (
            <Badge tone="ok"><Server className="size-3" /> 서버 env-var로 사전 프로비저닝됨 — 승인 불필요</Badge>
          )}
        </Card>

        <Card className="p-5">
          <div className="mb-2 flex items-center gap-2 text-sm font-medium"><ShieldAlert className="size-4" /> Disclosure</div>
          {!frozen ? (
            <Callout icon={<Info className="size-4" />}>preflight는 소스 고정 후에 가능합니다.</Callout>
          ) : requiresDisclosure ? (
            <label className="flex items-center gap-2 text-sm">
              <Switch checked={discloseConfirmed} onCheckedChange={setDiscloseConfirmed} data-testid="disclosure-switch" />
              원격 provider 사용 고지를 확인합니다 (per-run 동의)
            </label>
          ) : (
            <Badge tone="ok">로컬 provider — 고지 불필요</Badge>
          )}
        </Card>
      </div>

      {/* Run control */}
      <Card className="mb-5 flex items-center justify-between p-5">
        <div className="text-sm">
          {frozen ? <span className="text-muted">소스가 고정되었습니다. 통합을 실행할 수 있습니다.</span>
            : <span className="text-warn-text">먼저 소스를 고정하세요.</span>}
        </div>
        <div className="flex items-center gap-2">
          {!frozen && <Button variant="secondary" onClick={() => navigate(`/projects/${id}/sources`)}>소스로 이동</Button>}
          <Tooltip content={!canRun && requiresDisclosure && !discloseConfirmed ? "원격 provider 고지 확인이 필요합니다" : undefined}>
            <Button variant="primary" disabled={!canRun} loading={running} onClick={run} data-testid="run-integration">
              <Play className="size-4" /> 통합 실행
            </Button>
          </Tooltip>
        </div>
      </Card>

      {err && <div className="mb-5"><ErrorState error={err} /></div>}

      {/* Progress monitor (FOCAL) */}
      {jobId && snapshot && (
        <Card className="p-5">
          <div className="mb-4 flex items-center justify-between">
            <div className="flex items-center gap-2 font-medium"><Activity className="size-4 text-accent" /> 실행 진행</div>
            <Tooltip content="취소 API는 아직 노출되지 않았습니다(publish barrier 이전만 취소 가능하도록 설계).">
              <span><Button variant="outline" size="sm" disabled><Ban className="size-4" /> 취소</Button></span>
            </Tooltip>
          </div>

          <div className="grid gap-6 lg:grid-cols-[220px_1fr]">
            <RunSteps phases={RUN_PHASES} currentPhase={snapshot.phase} jobState={snapshot.state} />

            <div className="space-y-4">
              <div className="grid grid-cols-3 gap-3">
                <Metric label="경과 시간" value={elapsed} />
                <Metric label="진행" value={`${snapshot.progress_completed}${snapshot.progress_total ? `/${snapshot.progress_total}` : ""}`} />
                <Metric label="현재 phase" value={phaseLabel(snapshot.phase)} />
              </div>
              <ProgressBar value={snapshot.progress_completed} total={snapshot.progress_total}
                tone={snapshot.state === "completed" ? "ok" : "accent"} />

              <div className="rounded-lg border border-line bg-[var(--slate-1)]">
                <div className="flex items-center gap-2 border-b border-line px-3 py-2 text-[12px] text-faint"><TerminalSquare className="size-3.5" /> 이벤트 로그</div>
                <div className="max-h-56 overflow-y-auto p-3 font-mono text-[12px]" data-testid="event-log">
                  {snapshot.events.length === 0 && <div className="text-faint">이벤트 대기 중…</div>}
                  {snapshot.events.map((e) => (
                    <div key={e.sequence} className="flex items-center gap-2 py-0.5">
                      <span className="text-faint">{formatTime(e.ts)}</span>
                      <Badge tone="accent" className="px-1 py-0 text-[11px]">{e.phase ?? e.state ?? "evt"}</Badge>
                      <span className="text-muted">{e.state}{e.total ? ` ${e.completed}/${e.total}` : ""}{e.current_item ? ` · ${e.current_item}` : ""}</span>
                    </div>
                  ))}
                  {snapshot.error && (
                    <div className="py-0.5 text-danger-text">✖ {snapshot.error.code} · {snapshot.error.category}</div>
                  )}
                </div>
              </div>

              {snapshot.state === "completed" && (
                <Button variant="primary" onClick={() => navigate(`/projects/${id}/review`)} data-testid="to-review">
                  리뷰로 이동 <ArrowRight className="size-4" />
                </Button>
              )}
            </div>
          </div>
        </Card>
      )}

      {!jobId && frozen && (
        <Callout icon={<Workflow className="size-4" />}>통합을 실행하면 여기에서 파이프라인 phase(preflight→embedding→candidate→synthesis→critic)를 라이브로 관측합니다.</Callout>
      )}
    </div>
  );
}
