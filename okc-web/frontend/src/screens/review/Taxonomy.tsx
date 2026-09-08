import { useState } from "react";
import { useNavigate, useParams } from "react-router-dom";
import { ArrowRight, Check, FolderTree, Info } from "lucide-react";
import { ApiError, jobsApi, reviewApi } from "@/lib/api";
import { useAsync, useJobPoll } from "@/lib/hooks";
import { RUN_PHASES } from "@/lib/format";
import { PageHeader, ErrorState } from "@/components/ui/page";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Textarea, Label } from "@/components/ui/input";
import { Skeleton } from "@/components/ui/skeleton";
import { Callout } from "@/components/domain/callout";
import { RunSteps } from "@/components/domain/steps";
import { toast } from "@/components/ui/toaster";

// E4-2 taxonomy review + approval (base). The proposal payload is opaque
// (okc-core taxonomy shape) — rendered read-only; MVP approves without edits.
export function Taxonomy() {
  const { id = "" } = useParams();
  const navigate = useNavigate();
  const { data, error, loading, reload } = useAsync(() => reviewApi.taxonomy(id), [id]);
  const [rationale, setRationale] = useState("");
  const [busy, setBusy] = useState(false);
  const [jobId, setJobId] = useState<string | null>(null);
  const [approveErr, setApproveErr] = useState<ApiError | null>(null);

  const { snapshot } = useJobPoll(() => jobsApi.adminJob(id, jobId!), jobId, {
    onTerminal: (snap) => {
      if (snap.state === "completed") toast.success("Synthesis 생성 완료");
      else toast.error(`실패: ${snap.error?.code ?? "INTERNAL"}`);
    },
  });

  const approve = async () => {
    if (!rationale.trim()) return;
    setBusy(true); setApproveErr(null);
    try {
      const receipt = await reviewApi.approveTaxonomy(id, { rationale: rationale.trim() });
      toast.success("Taxonomy 승인됨");
      if (receipt.job_id) setJobId(receipt.job_id);
      reload();
    } catch (e) {
      setApproveErr(e instanceof ApiError ? e : null);
    } finally {
      setBusy(false);
    }
  };

  const clustersReady = snapshot?.state === "completed";

  return (
    <div>
      <PageHeader title="Taxonomy 편집·승인" description="여러 부서 Vault가 하나의 taxonomy로 통합됩니다. 승인하면 클러스터별 synthesis가 생성됩니다." />

      {loading && <Skeleton className="h-64 w-full" />}
      {error && <ErrorState error={error} onRetry={reload} />}

      {data && (
        <div className="space-y-4">
          <Card>
            <CardHeader><CardTitle className="flex items-center gap-2 text-sm"><FolderTree className="size-4" /> 제안된 Taxonomy</CardTitle></CardHeader>
            <CardContent>
              <TaxonomyPreview payload={data.payload} />
            </CardContent>
          </Card>

          <Callout icon={<Info className="size-4" />}>
            provenance는 파일 단위 보증입니다(스팬 단위 아님, NFR-DET-1). 소스 변경 시 이 승인은 무효화됩니다(freeze-then-run).
          </Callout>

          {jobId ? (
            <Card className="p-5">
              <div className="mb-3 text-sm font-medium">Synthesis 생성</div>
              <RunSteps phases={RUN_PHASES} currentPhase={snapshot?.phase} jobState={snapshot?.state} />
              {clustersReady && (
                <Button className="mt-4" variant="primary" onClick={() => navigate(`/projects/${id}/review`)}>
                  클러스터 리뷰로 이동 <ArrowRight className="size-4" />
                </Button>
              )}
            </Card>
          ) : (
            <Card className="p-5">
              <div className="space-y-1.5">
                <Label htmlFor="rationale">승인 사유 (필수)</Label>
                <Textarea id="rationale" value={rationale} onChange={(e) => setRationale(e.target.value)}
                  placeholder="이 taxonomy 승인 근거를 입력하세요" data-testid="taxonomy-rationale" />
              </div>
              {approveErr && <div className="mt-3"><ErrorState error={approveErr} /></div>}
              <Button className="mt-4" variant="primary" disabled={!rationale.trim()} loading={busy} onClick={approve} data-testid="approve-taxonomy">
                <Check className="size-4" /> Taxonomy 승인 & Synthesis 생성
              </Button>
            </Card>
          )}
        </div>
      )}
    </div>
  );
}

function TaxonomyPreview({ payload }: { payload: unknown }) {
  // Best-effort render of the opaque taxonomy shape: list cluster-like entries,
  // else pretty-print. Never assumes an exact schema.
  const clusters = extractClusters(payload);
  if (clusters.length > 0) {
    return (
      <ul className="space-y-1.5">
        {clusters.map((c, i) => (
          <li key={i} className="flex items-center gap-2 rounded-lg border border-line-subtle px-3 py-2 text-sm">
            <FolderTree className="size-4 text-faint" />
            <span className="font-medium">{c.name}</span>
            {c.count != null && <span className="text-faint text-[12px]">· {c.count} notes</span>}
          </li>
        ))}
      </ul>
    );
  }
  return (
    <pre className="max-h-72 overflow-auto rounded-lg border border-line bg-[var(--slate-1)] p-3 font-mono text-[12px] text-muted">
      {JSON.stringify(payload, null, 2)}
    </pre>
  );
}

function extractClusters(payload: unknown): { name: string; count?: number }[] {
  const arr =
    Array.isArray(payload) ? payload :
    payload && typeof payload === "object" && Array.isArray((payload as Record<string, unknown>).clusters)
      ? ((payload as Record<string, unknown>).clusters as unknown[])
      : [];
  const out: { name: string; count?: number }[] = [];
  for (const el of arr) {
    if (el && typeof el === "object") {
      const o = el as Record<string, unknown>;
      const name = String(o.name ?? o.cluster_id ?? o.id ?? "");
      if (name) {
        const notes = o.notes ?? o.members ?? o.documents;
        out.push({ name, count: Array.isArray(notes) ? notes.length : undefined });
      }
    }
  }
  return out;
}
