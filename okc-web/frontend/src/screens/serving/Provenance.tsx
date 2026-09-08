import { useMemo } from "react";
import { useNavigate, useParams, useSearchParams } from "react-router-dom";
import {
  ArrowRight, BadgeCheck, Fingerprint, GitMerge, Package, Scale, ShieldAlert, Waypoints,
} from "lucide-react";
import { servingApi } from "@/lib/api";
import { useAsync } from "@/lib/hooks";
import { shortHash } from "@/lib/format";
import { PageHeader, ErrorState } from "@/components/ui/page";
import { Card } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { Skeleton } from "@/components/ui/skeleton";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { Table, TBody, TD, TH, THead, TR } from "@/components/ui/table";
import { Callout } from "@/components/domain/callout";
import { CopyButton } from "@/components/ui/copy-button";

// ★FOCAL #3 — Provenance & Verify (E5-2). Lineage graph + verify PASS + no-winner
// contradiction split + always-on authenticity disclaimer.
export function Provenance() {
  const { id = "" } = useParams();
  const navigate = useNavigate();
  const [params] = useSearchParams();
  const note = params.get("note") ?? "";

  const verify = useAsync(() => servingApi.verify(id), [id]);
  const explain = useAsync(() => (note ? servingApi.explain(id, note) : Promise.reject(new Error("no note"))), [id, note]);

  const notServed = verify.error && (verify.error.code === "NOT_FOUND" || verify.error.httpStatus === 404);

  const owners = explain.data?.owner_labels ?? {};
  const sources = useMemo(
    () => Object.entries(owners).map(([sid, meta]) => ({ sid, name: meta?.display_name ?? sid, kind: meta?.kind })),
    [owners],
  );
  const contradictions = extractContradictions(explain.data?.record);
  const hash = findHash(explain.data?.record) ?? verify.data?.artifact_path;

  return (
    <div>
      <PageHeader
        title="Provenance & Verify"
        badge={verify.data ? (verify.data.valid
          ? <Badge tone="ok" fill="solid" className="okc-scale-in"><BadgeCheck className="size-3" /> Verified</Badge>
          : <Badge tone="danger"><ShieldAlert className="size-3" /> FAIL</Badge>) : undefined}
        description={note ? <span className="font-mono text-[13px]">{note}</span> : "노트를 선택해 계보를 추적하세요 (Compiled Vault에서 진입)."}
      />

      {/* Always-on authenticity disclaimer */}
      <Callout className="mb-5" icon={<ShieldAlert className="size-4" />}>
        verify는 산출물의 내부 일관성 증명이며 발행자 진위 보증이 아닙니다(NFR-DET-1). provenance는 파일 단위 보증입니다(스팬 단위 아님).
      </Callout>

      {notServed && (
        <Callout icon={<Package className="size-4" />}>
          이 프로젝트는 아직 서빙되지 않았습니다. verify/provenance는 publish 후 사용할 수 있습니다.
          <div className="mt-2"><Button size="sm" variant="secondary" onClick={() => navigate(`/projects/${id}/serving`)}>서빙으로 이동 <ArrowRight className="size-3.5" /></Button></div>
        </Callout>
      )}

      {!notServed && (
        <>
          {hash && (
            <div className="mb-4 flex items-center gap-2 text-sm">
              <Fingerprint className="size-4 text-faint" /> <span className="text-muted">결정성 해시</span>
              <CopyButton value={String(hash)} label={shortHash(String(hash))} />
            </div>
          )}

          {/* Hero lineage graph */}
          <Card className="mb-5 p-5">
            <div className="mb-3 flex items-center gap-2 text-sm font-medium"><Waypoints className="size-4 text-accent" /> Provenance Lineage</div>
            {explain.loading ? <Skeleton className="h-48 w-full" /> : <LineageGraph sources={sources} note={note} />}
          </Card>

          <Tabs defaultValue="provenance">
            <TabsList>
              <TabsTrigger value="provenance">Provenance</TabsTrigger>
              <TabsTrigger value="verify">Verify</TabsTrigger>
              <TabsTrigger value="contradictions">모순 {contradictions.length > 0 && `(${contradictions.length})`}</TabsTrigger>
            </TabsList>

            <TabsContent value="provenance" className="pt-4">
              {sources.length === 0 ? <div className="text-sm text-muted">기여 소스 정보가 없습니다.</div> : (
                <Table>
                  <THead><TR><TH>owner_display_name</TH><TH>SourceId</TH><TH>구분</TH></TR></THead>
                  <TBody>
                    {sources.map((s) => (
                      <TR key={s.sid}><TD>{s.name}</TD><TD className="font-mono text-[13px] text-muted">{s.sid}</TD><TD>{s.kind ?? "—"}</TD></TR>
                    ))}
                  </TBody>
                </Table>
              )}
              <Callout className="mt-3">owner_display_name은 비검증 장식 라벨입니다.</Callout>
            </TabsContent>

            <TabsContent value="verify" className="pt-4">
              {verify.loading && <Skeleton className="h-16 w-full" />}
              {verify.error && <ErrorState error={verify.error} onRetry={verify.reload} />}
              {verify.data && (
                <div className="space-y-3">
                  <div className="flex items-center gap-3">
                    <span className="text-sm text-muted">내부 일관성 검증</span>
                    {verify.data.valid
                      ? <Badge tone="ok" fill="solid"><BadgeCheck className="size-3" /> PASS</Badge>
                      : <Badge tone="danger"><ShieldAlert className="size-3" /> FAIL</Badge>}
                  </div>
                  {!verify.data.valid && <Callout tone="danger">검증 실패는 재컴파일로만 해소됩니다.</Callout>}
                </div>
              )}
            </TabsContent>

            <TabsContent value="contradictions" className="pt-4">
              <div className="mb-3 flex items-center justify-center gap-2 text-sm font-medium text-contra-text"><Scale className="size-4" /> 모순은 보존됩니다 (승자 선택 불가)</div>
              {contradictions.length === 0 ? <div className="text-sm text-muted">이 노트에 보존된 모순이 없습니다.</div> : (
                <div className="space-y-3">
                  {contradictions.map((c, i) => (
                    <div key={i} className="grid grid-cols-[1fr_auto_1fr] items-stretch gap-3">
                      <div className="rounded-lg border border-contra-line bg-contra-soft/40 p-3 text-sm">{c.a}</div>
                      <div className="flex items-center"><Scale className="size-5 text-contra-text" /></div>
                      <div className="rounded-lg border border-contra-line bg-contra-soft/40 p-3 text-sm">{c.b}</div>
                    </div>
                  ))}
                </div>
              )}
            </TabsContent>
          </Tabs>

          <div className="mt-6">
            <Button variant="secondary" onClick={() => navigate(`/projects/${id}/serving/contract`)}><GitMerge className="size-4" /> 서빙 계약 보기</Button>
          </div>
        </>
      )}
    </div>
  );
}

function LineageGraph({ sources, note }: { sources: { sid: string; name: string }[]; note: string }) {
  const shown = sources.slice(0, 5);
  const H = Math.max(160, shown.length * 44 + 40);
  const midY = H / 2;
  const noteLabel = note ? note.split("/").slice(1).join("/") || note : "compiled note";
  return (
    <svg viewBox={`0 0 720 ${H}`} className="w-full" role="img" aria-label="provenance lineage">
      {/* edges: source -> synthesis -> note */}
      {shown.map((_, i) => {
        const y = 30 + i * 44;
        return (
          <path key={`e-${i}`} d={`M 190 ${y} C 280 ${y}, 300 ${midY}, 360 ${midY}`}
            className="okc-draw" style={{ ["--okc-dash" as string]: "300" }}
            fill="none" stroke="var(--accent-line)" strokeWidth={1.5} />
        );
      })}
      <path d={`M 470 ${midY} C 540 ${midY}, 560 ${midY}, 600 ${midY}`} className="okc-draw"
        style={{ ["--okc-dash" as string]: "160" }} fill="none" stroke="var(--accent)" strokeWidth={2} />

      {/* source nodes */}
      {shown.map((s, i) => {
        const y = 30 + i * 44;
        return (
          <g key={s.sid}>
            <rect x={16} y={y - 14} width={174} height={28} rx={8} fill="var(--surface)" stroke="var(--line)" />
            <text x={28} y={y + 4} fontSize={11} fill="var(--fg)">{truncate(s.name, 22)}</text>
          </g>
        );
      })}
      {sources.length > shown.length && (
        <text x={28} y={30 + shown.length * 44} fontSize={11} fill="var(--faint)">+{sources.length - shown.length} more</text>
      )}
      {shown.length === 0 && (
        <g><rect x={16} y={midY - 14} width={174} height={28} rx={8} fill="var(--surface)" stroke="var(--line)" /><text x={28} y={midY + 4} fontSize={11} fill="var(--faint)">source</text></g>
      )}

      {/* synthesis node */}
      <rect x={360} y={midY - 18} width={110} height={36} rx={10} fill="var(--accent-soft)" stroke="var(--accent-line)" />
      <text x={378} y={midY + 4} fontSize={12} fill="var(--accent-text)">synthesis</text>

      {/* compiled note node */}
      <rect x={600} y={midY - 18} width={110} height={36} rx={10} fill="var(--surface)" stroke="var(--accent)" />
      <text x={612} y={midY + 4} fontSize={11} fill="var(--fg)">{truncate(noteLabel, 13)}</text>
    </svg>
  );
}

function truncate(s: string, n: number) { return s.length > n ? s.slice(0, n - 1) + "…" : s; }

function findHash(record: unknown): string | undefined {
  if (record && typeof record === "object") {
    const o = record as Record<string, unknown>;
    for (const k of ["content_hash", "hash", "manifest_hash", "deterministic_hash", "digest"]) {
      if (typeof o[k] === "string") return o[k] as string;
    }
  }
  return undefined;
}

function extractContradictions(record: unknown): { a: string; b: string }[] {
  const raw = record && typeof record === "object" ? (record as Record<string, unknown>).contradictions : undefined;
  if (!Array.isArray(raw)) return [];
  return raw.map((el) => {
    const o = (el ?? {}) as Record<string, unknown>;
    const a = o.a ?? o.left ?? o.source_a ?? o.first ?? el;
    const b = o.b ?? o.right ?? o.source_b ?? o.second ?? "";
    const text = (v: unknown) => typeof v === "object" && v ? String((v as Record<string, unknown>).claim ?? (v as Record<string, unknown>).text ?? JSON.stringify(v)) : String(v ?? "");
    return { a: text(a), b: text(b) };
  });
}
