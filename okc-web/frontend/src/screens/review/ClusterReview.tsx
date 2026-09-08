import { useState } from "react";
import { useNavigate, useParams } from "react-router-dom";
import Markdown from "react-markdown";
import {
  Ban, CircleCheck, EyeOff, Filter, RefreshCw, Scale, Undo2,
} from "lucide-react";
import { ApiError, reviewApi } from "@/lib/api";
import { cn } from "@/lib/cn";
import { useAsync } from "@/lib/hooks";
import type { ClusterDetailView, CriticFindingView } from "@/lib/types";
import { PageHeader, ErrorState } from "@/components/ui/page";
import { Card } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { Skeleton } from "@/components/ui/skeleton";
import { Tooltip } from "@/components/ui/tooltip";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { Dialog, DialogContent, DialogFooter, DialogHeader } from "@/components/ui/dialog";
import { Textarea, Label, Input } from "@/components/ui/input";
import { Callout } from "@/components/domain/callout";
import { SeverityBadge, WaivedBadge, ApprovedBadge } from "@/components/domain/badges";
import { toast } from "@/components/ui/toaster";

const WAIVE_FORBIDDEN = "Major/Critical은 waive 불가 — regenerate로만 해소됩니다.";

// ★FOCAL #2 (main hero) — Cluster review workbench (E4-3). Honestly surfaces the
// okc-core constraints: no winner-select, Major/Critical waive-forbidden + compile-block.
export function ClusterReview() {
  const { id = "", clusterId = "" } = useParams();
  const navigate = useNavigate();
  const list = useAsync(() => reviewApi.clusters(id), [id]);
  const detail = useAsync(() => reviewApi.cluster(id, clusterId), [id, clusterId]);

  const [minorWaivers, setMinorWaivers] = useState<Record<string, string>>({});
  const [omissions, setOmissions] = useState<Record<string, string>>({});
  const [waiveTarget, setWaiveTarget] = useState<CriticFindingView | null>(null);
  const [omissionOpen, setOmissionOpen] = useState(false);
  const [approveOpen, setApproveOpen] = useState(false);
  const [approving, setApproving] = useState(false);
  const [approved, setApproved] = useState(false);
  const [err, setErr] = useState<ApiError | null>(null);

  const d = detail.data;
  const blockingFindings = d?.findings.filter((f) => f.blocking) ?? [];
  const canApprove = !!d && blockingFindings.length === 0 && !approved;

  const doApprove = async () => {
    setApproving(true); setErr(null);
    try {
      await reviewApi.approveCluster(id, clusterId, { minor_waivers: minorWaivers, omission_rationales: omissions });
      toast.success("클러스터가 승인되었습니다");
      setApproved(true);
      setApproveOpen(false);
      list.reload();
    } catch (e) {
      setErr(e instanceof ApiError ? e : null);
    } finally {
      setApproving(false);
    }
  };

  return (
    <div>
      <PageHeader
        title="클러스터 리뷰"
        badge={approved ? <ApprovedBadge /> : blockingFindings.length > 0 ? <Badge tone="danger"><Ban className="size-3" /> 차단 {blockingFindings.length}</Badge> : undefined}
        description={<span className="font-mono text-[13px]">{clusterId}</span>}
      />

      {blockingFindings.length > 0 && (
        <Alert2 />
      )}

      <div className="grid gap-4 lg:grid-cols-[240px_1fr_300px]">
        {/* Left — cluster list */}
        <Card className="p-2">
          <div className="mb-1 flex items-center gap-1.5 px-2 py-1 text-[12px] text-faint"><Filter className="size-3.5" /> 클러스터</div>
          {list.loading && <Skeleton className="h-40 w-full" />}
          <div className="space-y-0.5" data-testid="cluster-list">
            {list.data?.map((c) => (
              <button key={c.cluster_id} onClick={() => navigate(`/projects/${id}/review/clusters/${c.cluster_id}`)}
                className={cn("flex w-full items-center justify-between gap-2 rounded-lg px-2 py-1.5 text-left text-[13px] hover:bg-subtle",
                  c.cluster_id === clusterId && "bg-subtle font-medium")}>
                <span className="truncate font-mono">{c.cluster_id}</span>
                {c.blocking_count > 0 ? <Badge tone="danger" className="px-1">{c.blocking_count}</Badge>
                  : <CircleCheck className="size-3.5 text-ok" />}
              </button>
            ))}
          </div>
        </Card>

        {/* Center — detail tabs */}
        <div>
          {detail.loading && <Skeleton className="h-96 w-full" />}
          {detail.error && <ErrorState error={detail.error} onRetry={detail.reload} />}
          {d && <CenterTabs detail={d} />}
        </div>

        {/* Right — findings inspector rail */}
        <Card className="p-3">
          <div className="mb-2 text-[12px] font-medium text-faint">FINDINGS</div>
          <div className="space-y-2.5">
            {d?.findings.length === 0 && <div className="text-sm text-muted">findings 없음</div>}
            {d?.findings.map((f) => (
              <FindingCard key={f.finding_id} finding={f} waived={!!minorWaivers[f.finding_id]}
                onWaive={() => setWaiveTarget(f)} />
            ))}
          </div>
        </Card>
      </div>

      {/* Bottom sticky decision action bar */}
      <div className="sticky bottom-0 mt-6 flex items-center justify-between gap-3 rounded-xl border border-line bg-surface/95 p-3 backdrop-blur">
        <div className="text-[13px]">
          {blockingFindings.length > 0
            ? <span className="text-danger-text">차단 findings 존재 → 컴파일 거부</span>
            : <span className="text-muted">차단 0 — 승인 가능</span>}
        </div>
        <div className="flex items-center gap-2">
          <Button variant="ghost" onClick={() => setOmissionOpen(true)} data-testid="omission-btn"><EyeOff className="size-4" /> Omission 제안</Button>
          <Button variant="secondary" onClick={() => navigate(`/projects/${id}/review/clusters/${clusterId}/regenerate`)} data-testid="regenerate-btn">
            <RefreshCw className="size-4" /> Regenerate
          </Button>
          <Tooltip content={!canApprove && blockingFindings.length > 0 ? WAIVE_FORBIDDEN : undefined}>
            <span>
              <Button variant="primary" disabled={!canApprove} onClick={() => setApproveOpen(true)} data-testid="approve-cluster">
                <CircleCheck className="size-4" /> 승인
              </Button>
            </span>
          </Tooltip>
        </div>
      </div>

      {err && <div className="mt-3"><ErrorState error={err} /></div>}

      {/* Minor waive dialog */}
      <RationaleDialog
        open={!!waiveTarget}
        onOpenChange={(v) => { if (!v) setWaiveTarget(null); }}
        title="Minor Waive"
        description={waiveTarget?.message}
        label="Waive 사유 (필수)"
        onSubmit={(reason) => {
          if (waiveTarget) setMinorWaivers((m) => ({ ...m, [waiveTarget.finding_id]: reason }));
          setWaiveTarget(null);
        }}
      />

      {/* Omission dialog */}
      <OmissionDialog open={omissionOpen} onOpenChange={setOmissionOpen}
        onSubmit={(key, reason) => { setOmissions((o) => ({ ...o, [key]: reason })); setOmissionOpen(false); }} />

      {/* Approve confirm */}
      <Dialog open={approveOpen} onOpenChange={setApproveOpen}>
        <DialogContent>
          <DialogHeader title="클러스터 승인" description="아래 waiver/omission 결정이 함께 기록됩니다." />
          <div className="space-y-2 text-sm">
            <div className="flex justify-between"><span className="text-muted">Minor waivers</span><span className="tabular">{Object.keys(minorWaivers).length}</span></div>
            <div className="flex justify-between"><span className="text-muted">Omission 제안</span><span className="tabular">{Object.keys(omissions).length}</span></div>
          </div>
          <DialogFooter>
            <Button variant="ghost" onClick={() => setApproveOpen(false)}>취소</Button>
            <Button variant="primary" loading={approving} onClick={doApprove} data-testid="approve-confirm"><CircleCheck className="size-4" /> 승인</Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </div>
  );
}

function Alert2() {
  return (
    <div className="mb-4 flex items-center gap-2 rounded-lg border border-danger-line bg-danger-soft px-3.5 py-2.5 text-sm text-danger-text okc-pulse">
      <Ban className="size-4" /> 미해소 Major/Critical findings 때문에 컴파일이 거부됩니다. regenerate로만 해소할 수 있습니다.
    </div>
  );
}

function CenterTabs({ detail }: { detail: ClusterDetailView }) {
  const [tab, setTab] = useState("synthesis");
  const markdown = extractSynthesisMarkdown(detail.proposal);
  const contradictions = extractContradictions(detail.contradictions);
  return (
    <Card className="p-0">
      <Tabs value={tab} onValueChange={setTab}>
        <TabsList className="px-3 pt-1">
          <TabsTrigger value="synthesis" data-testid="tab-synthesis">Synthesis</TabsTrigger>
          <TabsTrigger value="contradictions" data-testid="tab-contradictions">모순 {contradictions.length > 0 && `(${contradictions.length})`}</TabsTrigger>
          <TabsTrigger value="omission">Omission 제안</TabsTrigger>
          <TabsTrigger value="taxonomy">Taxonomy</TabsTrigger>
        </TabsList>

        <TabsContent value="synthesis" className="p-5">
          <article className="prose-sm max-w-none space-y-2 text-sm leading-6 [&_h1]:text-lg [&_h1]:font-semibold [&_h2]:mt-4 [&_h2]:font-semibold [&_code]:font-mono [&_ul]:list-disc [&_ul]:pl-5">
            <Markdown>{markdown}</Markdown>
          </article>
          <Callout className="mt-5">provenance는 파일 단위 보증입니다(스팬 단위 아님, NFR-DET-1).</Callout>
        </TabsContent>

        <TabsContent value="contradictions" className="p-5">
          <div className="mb-3 flex items-center justify-center gap-2 text-sm font-medium text-contra-text">
            <Scale className="size-4" /> 보존된 모순 — 승자 없음 (no winner)
          </div>
          {contradictions.length === 0 ? (
            <div className="text-sm text-muted">이 클러스터에 보존된 모순이 없습니다.</div>
          ) : (
            <div className="space-y-4">
              {contradictions.map((c, i) => (
                <div key={i} className="grid grid-cols-[1fr_auto_1fr] items-stretch gap-3">
                  <ContradictionSide side={c.a} />
                  <div className="flex items-center"><Scale className="size-5 text-contra-text" /></div>
                  <ContradictionSide side={c.b} />
                </div>
              ))}
            </div>
          )}
          <Callout tone="contra" className="mt-4">양측 모두 <code className="font-mono">knowledge/</code>에 보존되며 <code className="font-mono">.okc/</code> provenance에 기록됩니다. 유일한 우회는 클러스터 Regenerate입니다.</Callout>
        </TabsContent>

        <TabsContent value="omission" className="p-5 text-sm text-muted">
          synthesis에서 누락된 후보를 사유와 함께 제안합니다. 하단 "Omission 제안" 버튼으로 추가하세요.
        </TabsContent>

        <TabsContent value="taxonomy" className="p-5">
          <div className="flex flex-wrap gap-4 text-[13px] text-muted">
            <span>proposal: <code className="font-mono">{detail.proposal_hash ?? "—"}</code></span>
            <span>critic: <code className="font-mono">{detail.critic_hash ?? "—"}</code></span>
            <span>taxonomy: <code className="font-mono">{detail.taxonomy_hash ?? "—"}</code></span>
          </div>
        </TabsContent>
      </Tabs>
    </Card>
  );
}

function ContradictionSide({ side }: { side: ContraSide }) {
  return (
    <div className="rounded-lg border border-contra-line bg-contra-soft/40 p-3">
      {side.owner && <div className="mb-1 text-[12px] font-medium text-contra-text">{side.owner}</div>}
      <div className="text-sm">{side.claim}</div>
      {side.evidence && <div className="mt-1.5 border-l-2 border-contra-line pl-2 text-[13px] text-muted">{side.evidence}</div>}
    </div>
  );
}

function FindingCard({ finding, waived, onWaive }: { finding: CriticFindingView; waived: boolean; onWaive: () => void }) {
  return (
    <div className="rounded-lg border border-line p-2.5">
      <div className="flex items-center justify-between gap-2">
        <SeverityBadge severity={finding.severity} />
        {waived && <WaivedBadge />}
      </div>
      <div className="mt-1.5 text-[13px]">{finding.message}</div>
      {finding.kind && <div className="mt-0.5 text-[12px] text-faint">{finding.kind}</div>}
      <div className="mt-2">
        {finding.blocking ? (
          <Tooltip content={WAIVE_FORBIDDEN}>
            <span><Button size="sm" variant="outline" disabled data-testid={`waive-${finding.finding_id}`}><Undo2 className="size-3.5" /> Waive 불가</Button></span>
          </Tooltip>
        ) : (
          <Button size="sm" variant="outline" onClick={onWaive} disabled={waived} data-testid={`waive-${finding.finding_id}`}>
            <Undo2 className="size-3.5" /> {waived ? "Waived" : "Waive"}
          </Button>
        )}
      </div>
    </div>
  );
}

function RationaleDialog({
  open, onOpenChange, title, description, label, onSubmit,
}: {
  open: boolean; onOpenChange: (v: boolean) => void; title: string; description?: string; label: string;
  onSubmit: (reason: string) => void;
}) {
  const [reason, setReason] = useState("");
  return (
    <Dialog open={open} onOpenChange={(v) => { onOpenChange(v); if (!v) setReason(""); }}>
      <DialogContent>
        <DialogHeader title={title} description={description} />
        <div className="space-y-1.5">
          <Label>{label}</Label>
          <Textarea value={reason} onChange={(e) => setReason(e.target.value)} data-testid="rationale-input" autoFocus />
        </div>
        <DialogFooter>
          <Button variant="ghost" onClick={() => onOpenChange(false)}>취소</Button>
          <Button variant="primary" disabled={!reason.trim()} onClick={() => { onSubmit(reason.trim()); setReason(""); }} data-testid="rationale-submit">확인</Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}

function OmissionDialog({ open, onOpenChange, onSubmit }: { open: boolean; onOpenChange: (v: boolean) => void; onSubmit: (key: string, reason: string) => void }) {
  const [key, setKey] = useState("");
  const [reason, setReason] = useState("");
  return (
    <Dialog open={open} onOpenChange={(v) => { onOpenChange(v); if (!v) { setKey(""); setReason(""); } }}>
      <DialogContent>
        <DialogHeader title="Omission 제안" description="누락 대상(document_id:target_id)과 사유를 입력합니다." />
        <div className="space-y-3">
          <div className="space-y-1.5">
            <Label>대상 키</Label>
            <Input value={key} onChange={(e) => setKey(e.target.value)} placeholder="document_id:target_id" className="font-mono" />
          </div>
          <div className="space-y-1.5">
            <Label>사유 (필수)</Label>
            <Textarea value={reason} onChange={(e) => setReason(e.target.value)} />
          </div>
        </div>
        <DialogFooter>
          <Button variant="ghost" onClick={() => onOpenChange(false)}>취소</Button>
          <Button variant="primary" disabled={!key.trim() || !reason.trim()} onClick={() => onSubmit(key.trim(), reason.trim())}>추가</Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}

// --- opaque-payload extractors (defensive; never assume exact schema) ---

function extractSynthesisMarkdown(proposal: unknown): string {
  if (typeof proposal === "string") return proposal;
  if (proposal && typeof proposal === "object") {
    const o = proposal as Record<string, unknown>;
    for (const k of ["synthesis", "markdown", "content", "body", "text", "note"]) {
      const v = o[k];
      if (typeof v === "string" && v.trim()) return v;
      if (v && typeof v === "object") {
        const inner = (v as Record<string, unknown>).content ?? (v as Record<string, unknown>).markdown;
        if (typeof inner === "string" && inner.trim()) return inner;
      }
    }
  }
  return "_이 클러스터의 synthesis 본문을 표시할 수 없습니다 (opaque payload)._";
}

interface ContraSide { owner?: string; claim: string; evidence?: string }
interface ContraPair { a: ContraSide; b: ContraSide }

function extractContradictions(raw: unknown): ContraPair[] {
  if (!Array.isArray(raw)) return [];
  const side = (v: unknown): ContraSide => {
    if (v && typeof v === "object") {
      const o = v as Record<string, unknown>;
      return {
        owner: (o.owner_display_name ?? o.owner ?? o.source_id) as string | undefined,
        claim: String(o.claim ?? o.statement ?? o.text ?? ""),
        evidence: (o.evidence ?? o.excerpt ?? o.quote) as string | undefined,
      };
    }
    return { claim: String(v ?? "") };
  };
  return raw.map((el) => {
    const o = (el ?? {}) as Record<string, unknown>;
    const a = o.a ?? o.left ?? o.source_a ?? o.first;
    const b = o.b ?? o.right ?? o.source_b ?? o.second;
    if (a || b) return { a: side(a), b: side(b) };
    return { a: side(el), b: { claim: "" } };
  });
}
