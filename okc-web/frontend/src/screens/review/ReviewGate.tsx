import { useNavigate, useParams } from "react-router-dom";
import { ArrowRight, Ban, CircleAlert, OctagonAlert, ScanSearch, ShieldCheck, TriangleAlert } from "lucide-react";
import { reviewApi } from "@/lib/api";
import { useAsync } from "@/lib/hooks";
import { useProjectContext } from "@/app/project-context";
import { PageHeader, ErrorState } from "@/components/ui/page";
import { Button } from "@/components/ui/button";
import { Alert } from "@/components/ui/alert";
import { Badge } from "@/components/ui/badge";
import { SkeletonRows } from "@/components/ui/skeleton";
import { EmptyState } from "@/components/ui/empty-state";
import { Tooltip } from "@/components/ui/tooltip";
import { Table, TBody, TD, TH, THead, TR } from "@/components/ui/table";
import { Metric, MetricGrid } from "@/components/domain/metric";

// E4-1 review overview & compile gate (base — non-focal).
export function ReviewGate() {
  const { id = "" } = useParams();
  const navigate = useNavigate();
  const { status } = useProjectContext();
  const gate = useAsync(() => reviewApi.gate(id), [id]);
  const clusters = useAsync(() => reviewApi.clusters(id), [id]);

  const needsTaxonomy = status?.checkpoint === "needs_taxonomy";
  const blocked = gate.data?.state === "Blocked";
  const ready = gate.data?.state === "Ready";
  const critical = gate.data?.blocking_items.filter((b) => b.severity === "critical").length ?? 0;
  const major = gate.data?.blocking_items.filter((b) => b.severity === "major").length ?? 0;

  return (
    <div>
      <PageHeader title="리뷰 & 컴파일 게이트" description="스코어보드 + 컴파일 가능 여부. 컴파일 실행 트리거는 Compiled Vault 한 곳에만 있습니다." />

      {gate.loading && <SkeletonRows rows={2} />}
      {gate.error && <ErrorState error={gate.error} onRetry={gate.reload} />}

      {gate.data && (
        <>
          {/* Hero gate banner — 3 branches */}
          {blocked ? (
            <Alert variant="destructive" className="mb-5" icon={<Ban className="size-5" />}
              title={`컴파일 거부 — 차단 findings ${gate.data.blocking_items.length}건`}>
              Critical {critical} · Major {major}. Major/Critical은 waive 불가 — regenerate로만 해소됩니다.
            </Alert>
          ) : ready ? (
            <Alert variant="success" className="mb-5" icon={<ShieldCheck className="size-5" />} title="컴파일 준비 완료">
              모든 승인이 완료되고 차단 findings가 0입니다.
            </Alert>
          ) : (
            <Alert variant="warning" className="mb-5" icon={<TriangleAlert className="size-5" />} title="승인 대기">
              {needsTaxonomy ? "Taxonomy 승인이 필요합니다." : "미승인 클러스터가 있습니다."}
            </Alert>
          )}

          <MetricGrid className="mb-5">
            <Metric label="클러스터" value={clusters.data?.length ?? "—"} />
            <Metric label="차단 findings" value={gate.data.blocking_items.length} tone={gate.data.blocking_items.length > 0 ? "danger" : "ok"} />
            <Metric label="Critical" value={critical} tone={critical > 0 ? "danger" : undefined} />
            <Metric label="Major" value={major} tone={major > 0 ? "danger" : undefined} />
          </MetricGrid>
        </>
      )}

      {needsTaxonomy ? (
        <EmptyState icon={ScanSearch} title="Taxonomy 승인이 먼저 필요합니다" description="클러스터 리뷰는 taxonomy 승인 후 잠금 해제됩니다."
          action={<Button variant="primary" onClick={() => navigate(`/projects/${id}/review/taxonomy`)}>Taxonomy 승인으로 이동</Button>} />
      ) : (
        <>
          {clusters.loading && <SkeletonRows rows={3} />}
          {clusters.error && <ErrorState error={clusters.error} onRetry={clusters.reload} />}
          {clusters.data && clusters.data.length === 0 && (
            <EmptyState icon={ScanSearch} title="클러스터가 없습니다" description="통합을 먼저 실행하세요."
              action={<Button variant="primary" onClick={() => navigate(`/projects/${id}/integration`)}>통합으로 이동</Button>} />
          )}
          {clusters.data && clusters.data.length > 0 && (
            <Table>
              <THead><TR><TH>클러스터</TH><TH>findings</TH><TH>severity</TH><TH /></TR></THead>
              <TBody>
                {clusters.data.map((c) => (
                  <TR key={c.cluster_id}>
                    <TD className="font-mono text-[13px]">{c.cluster_id}</TD>
                    <TD className="tabular">{c.finding_count}</TD>
                    <TD>
                      <div className="flex items-center gap-1.5">
                        {c.blocking_count > 0 ? <Badge tone="danger"><OctagonAlert className="size-3" /> 차단 {c.blocking_count}</Badge> : <Badge tone="ok">차단 0</Badge>}
                        {c.minor_count > 0 && <Badge tone="warn"><CircleAlert className="size-3" /> Minor {c.minor_count}</Badge>}
                      </div>
                    </TD>
                    <TD className="text-right">
                      <Button size="sm" variant="secondary" onClick={() => navigate(`/projects/${id}/review/clusters/${c.cluster_id}`)} data-testid={`review-cluster-${c.cluster_id}`}>리뷰</Button>
                    </TD>
                  </TR>
                ))}
              </TBody>
            </Table>
          )}
        </>
      )}

      <div className="mt-6 flex justify-end">
        <Tooltip content={blocked ? "미해소 Major/Critical 때문에 컴파일이 거부됩니다." : undefined}>
          <span>
            <Button variant="primary" disabled={!ready} onClick={() => navigate(`/projects/${id}/compiled`)} data-testid="to-compiled">
              Compiled Vault로 이동 <ArrowRight className="size-4" />
            </Button>
          </span>
        </Tooltip>
      </div>
    </div>
  );
}
