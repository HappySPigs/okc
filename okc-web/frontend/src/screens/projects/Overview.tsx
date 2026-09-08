import { useNavigate, useParams } from "react-router-dom";
import { ArrowRight, Ban, Hammer, Lock, Radio, ScanSearch, ScrollText, Workflow } from "lucide-react";
import { reviewApi } from "@/lib/api";
import { useAsync } from "@/lib/hooks";
import { useProjectContext } from "@/app/project-context";
import { formatDate } from "@/lib/format";
import type { Checkpoint } from "@/lib/types";
import { PageHeader, ErrorState } from "@/components/ui/page";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Skeleton } from "@/components/ui/skeleton";
import { Metric, MetricGrid } from "@/components/domain/metric";
import { PipelineTracker } from "@/components/domain/pipeline-tracker";
import { CheckpointBadge } from "@/components/domain/badges";

export function Overview() {
  const { id = "" } = useParams();
  const navigate = useNavigate();
  const { project, status, loading, error, reload } = useProjectContext();
  const gate = useAsync(() => reviewApi.gate(id), [id]);
  const decisions = useAsync(() => reviewApi.decisions(id), [id]);

  if (loading && !status) return <Skeleton className="h-64 w-full" />;
  if (error) return <ErrorState error={error} onRetry={reload} />;
  if (!project || !status) return null;

  const blocking = gate.data?.blocking_items.length ?? 0;

  return (
    <div>
      <PageHeader
        title={project.name}
        description={<span className="font-mono text-[13px]">{project.curator_id}</span>}
        badge={<CheckpointBadge checkpoint={status.checkpoint} stale={status.stale} blocked={blocking} />}
      />

      <Card className="mb-5 p-5">
        <PipelineTracker
          checkpoint={status.checkpoint}
          stale={status.stale}
          blocked={blocking}
          onSelectStage={(cp) => {
            if (cp === "needs_taxonomy") navigate(`/projects/${id}/review/taxonomy`);
            else if (cp === "needs_clusters") navigate(`/projects/${id}/review`);
          }}
        />
      </Card>

      <MetricGrid className="mb-5">
        <Metric label="소스" value={`${status.source_count}/10`} />
        <Metric label="차단 findings" value={blocking} tone={blocking > 0 ? "danger" : undefined} />
        <Metric label="freeze" value={status.frozen ? "frozen" : "unfrozen"} />
        <Metric label="verify" value={status.checkpoint === "verified" ? "PASS" : "—"} tone={status.checkpoint === "verified" ? "ok" : undefined} />
      </MetricGrid>

      <NextActionCard checkpoint={status.checkpoint} nextAction={status.next_action} blocking={blocking} projectId={id} navigate={navigate} />

      <Card className="mt-5">
        <CardHeader><CardTitle className="flex items-center gap-2 text-sm"><ScrollText className="size-4" /> 최근 감사 로그</CardTitle></CardHeader>
        <CardContent className="space-y-1.5 text-sm">
          {decisions.loading && <Skeleton className="h-16 w-full" />}
          {decisions.data && decisions.data.length === 0 && <div className="text-muted">기록된 결정이 없습니다.</div>}
          {decisions.data?.slice(0, 6).map((d) => (
            <div key={d.id} className="flex items-center justify-between border-b border-line-subtle py-1 last:border-0">
              <span><span className="font-medium">{d.decision_kind}</span> <span className="text-faint font-mono text-[12px]">{d.core_op}</span></span>
              <span className="text-faint text-[12px]">{formatDate(d.created_at)}</span>
            </div>
          ))}
        </CardContent>
      </Card>
    </div>
  );
}

function NextActionCard({
  checkpoint, nextAction, blocking, projectId, navigate,
}: {
  checkpoint: Checkpoint | string;
  nextAction: string;
  blocking: number;
  projectId: string;
  navigate: (p: string) => void;
}) {
  const map: Record<string, { icon: React.ReactNode; to: string; cta: string }> = {
    needs_provider: { icon: <Workflow className="size-5" />, to: `/projects/${projectId}/integration`, cta: "통합으로 이동" },
    needs_sources: { icon: <Lock className="size-5" />, to: `/projects/${projectId}/sources`, cta: "소스 관리" },
    needs_disclosure: { icon: <Workflow className="size-5" />, to: `/projects/${projectId}/integration`, cta: "통합 실행" },
    needs_taxonomy: { icon: <ScanSearch className="size-5" />, to: `/projects/${projectId}/review/taxonomy`, cta: "Taxonomy 승인" },
    needs_clusters: { icon: <ScanSearch className="size-5" />, to: `/projects/${projectId}/review`, cta: "리뷰로 이동" },
    ready_to_compile: { icon: <Hammer className="size-5" />, to: `/projects/${projectId}/compiled`, cta: "컴파일 실행" },
    verified: { icon: <Radio className="size-5" />, to: `/projects/${projectId}/serving`, cta: "서빙 공개" },
  };
  const m = map[checkpoint] ?? map.needs_sources;
  const isBlocked = blocking > 0;
  return (
    <Card className="p-5">
      <div className="flex items-center justify-between gap-4">
        <div className="flex items-center gap-3">
          <div className="text-accent">{isBlocked ? <Ban className="size-5 text-danger" /> : m.icon}</div>
          <div>
            <div className="text-[13px] text-muted">다음 액션</div>
            <div className="font-medium">{isBlocked ? `리뷰 필요 — 차단 ${blocking}건` : nextAction}</div>
          </div>
        </div>
        <Button variant="primary" onClick={() => navigate(isBlocked ? `/projects/${projectId}/review` : m.to)} data-testid="next-action">
          {isBlocked ? "리뷰로 이동" : m.cta} <ArrowRight className="size-4" />
        </Button>
      </div>
    </Card>
  );
}
