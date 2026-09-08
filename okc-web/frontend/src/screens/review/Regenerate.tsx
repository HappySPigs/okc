import { useEffect, useState } from "react";
import { useNavigate, useParams } from "react-router-dom";
import { ArrowRight, CircleCheck, MessageSquare, RefreshCw, ShieldAlert } from "lucide-react";
import { ApiError, jobsApi, projectsApi, reviewApi } from "@/lib/api";
import { useAsync, useJobPoll } from "@/lib/hooks";
import { RUN_PHASES } from "@/lib/format";
import type { PreflightGateView } from "@/lib/types";
import { PageHeader, ErrorState } from "@/components/ui/page";
import { Card } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { Textarea, Label } from "@/components/ui/input";
import { Skeleton } from "@/components/ui/skeleton";
import { Switch } from "@/components/ui/switch";
import { Callout } from "@/components/domain/callout";
import { RunSteps } from "@/components/domain/steps";
import { SeverityBadge } from "@/components/domain/badges";
import { toast } from "@/components/ui/toaster";

// E4-4 regenerate — the ONLY path to resolve blocking findings (base, non-focal).
export function Regenerate() {
  const { id = "", clusterId = "" } = useParams();
  const navigate = useNavigate();
  const before = useAsync(() => reviewApi.cluster(id, clusterId), [id, clusterId]);
  const [feedback, setFeedback] = useState("");
  const [preflight, setPreflight] = useState<PreflightGateView | null>(null);
  const [disclose, setDisclose] = useState(false);
  const [jobId, setJobId] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [err, setErr] = useState<ApiError | null>(null);
  const [after, setAfter] = useState<number | null>(null);

  useEffect(() => { projectsApi.preflight(id).then(setPreflight).catch(() => setPreflight(null)); }, [id]);

  const beforeBlocking = before.data?.findings.filter((f) => f.blocking).length ?? 0;
  const requiresDisclosure = preflight?.requires_disclosure ?? false;

  const { snapshot } = useJobPoll(() => jobsApi.adminJob(id, jobId!), jobId, {
    onTerminal: async (snap) => {
      setBusy(false);
      if (snap.state === "completed") {
        toast.success("재생성 완료");
        try {
          const d = await reviewApi.cluster(id, clusterId);
          setAfter(d.findings.filter((f) => f.blocking).length);
        } catch { /* ignore */ }
      } else {
        toast.error(`재생성 실패: ${snap.error?.code ?? "INTERNAL"}`);
      }
    },
  });

  const run = async () => {
    if (!feedback.trim()) return;
    setBusy(true); setErr(null);
    try {
      const receipt = await reviewApi.regenerate(id, clusterId, {
        feedback: feedback.trim(),
        allow_remote_provider: requiresDisclosure,
        remote_disclosure_confirmed: requiresDisclosure && disclose,
      });
      if (receipt.job_id) setJobId(receipt.job_id);
    } catch (e) {
      setErr(e instanceof ApiError ? e : null);
      setBusy(false);
    }
  };

  const done = snapshot?.state === "completed" && after !== null;
  const resolved = done && after === 0;

  return (
    <div>
      <PageHeader title="Regenerate" description={<span className="font-mono text-[13px]">{clusterId}</span>} />

      {before.loading && <Skeleton className="h-40 w-full" />}
      {before.error && <ErrorState error={before.error} onRetry={before.reload} />}

      {before.data && !jobId && (
        <Card className="space-y-4 p-5">
          <div>
            <div className="mb-1.5 text-sm font-medium">현재 차단 findings</div>
            <div className="flex flex-wrap gap-1.5">
              {before.data.findings.filter((f) => f.blocking).map((f) => <SeverityBadge key={f.finding_id} severity={f.severity} />)}
              {beforeBlocking === 0 && <span className="text-sm text-muted">차단 findings 없음</span>}
            </div>
          </div>
          <div className="space-y-1.5">
            <Label htmlFor="feedback" className="flex items-center gap-1.5"><MessageSquare className="size-4" /> 피드백 (필수)</Label>
            <Textarea id="feedback" value={feedback} onChange={(e) => setFeedback(e.target.value)}
              placeholder="재생성 시 반영할 지침을 입력하세요" data-testid="regenerate-feedback" />
          </div>
          {requiresDisclosure && (
            <label className="flex items-center gap-2 text-sm">
              <Switch checked={disclose} onCheckedChange={setDisclose} /> <ShieldAlert className="size-4" /> 원격 provider 고지 확인
            </label>
          )}
          <Callout>재생성은 현재 synthesis를 폐기하고 하위 승인을 무효화합니다.</Callout>
          {err && <ErrorState error={err} />}
          <Button variant="primary" disabled={!feedback.trim() || (requiresDisclosure && !disclose)} loading={busy} onClick={run} data-testid="regenerate-run">
            <RefreshCw className="size-4" /> 재생성 실행
          </Button>
        </Card>
      )}

      {jobId && (
        <Card className="space-y-4 p-5">
          <RunSteps phases={RUN_PHASES} currentPhase={snapshot?.phase} jobState={snapshot?.state} />
          {done && (
            <>
              <div className="flex items-center gap-3 text-sm">
                <span className="text-muted">차단 findings</span>
                <Badge tone={beforeBlocking > 0 ? "danger" : "neutral"}>{beforeBlocking}</Badge>
                <ArrowRight className="size-4 text-faint" />
                <Badge tone={resolved ? "ok" : "danger"}>{after}</Badge>
              </div>
              {resolved ? (
                <Callout tone="ok" icon={<CircleCheck className="size-4" />}>차단 0 — 승인 가능합니다.</Callout>
              ) : (
                <Callout tone="warn">아직 차단 findings가 남아 있습니다. 추가 피드백으로 다시 재생성하세요.</Callout>
              )}
              <Button variant="primary" onClick={() => navigate(`/projects/${id}/review/clusters/${clusterId}`)}>
                클러스터로 돌아가기 <ArrowRight className="size-4" />
              </Button>
            </>
          )}
        </Card>
      )}
    </div>
  );
}
