import { useState } from "react";
import { useNavigate, useParams } from "react-router-dom";
import { Code2, ExternalLink, RefreshCw, Server, Terminal } from "lucide-react";
import { ApiError, servingApi } from "@/lib/api";
import { useAsync } from "@/lib/hooks";
import { PageHeader, ErrorState } from "@/components/ui/page";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Switch } from "@/components/ui/switch";
import { Skeleton } from "@/components/ui/skeleton";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { Table, TBody, TD, TH, THead, TR } from "@/components/ui/table";
import { ConfirmDialog } from "@/components/ui/dialog";
import { Callout } from "@/components/domain/callout";
import { CopyButton } from "@/components/ui/copy-button";
import { ServingStatusBadge } from "@/components/domain/badges";
import { toast } from "@/components/ui/toaster";

const ENDPOINTS = [
  { method: "GET", path: "/files", desc: "병합 트리 파일 목록" },
  { method: "GET", path: "/file?path=", desc: "노트 본문(Markdown)" },
  { method: "GET", path: "/verify", desc: "내부 일관성 검증" },
  { method: "GET", path: "/explain?path=", desc: "파일 단위 provenance" },
  { method: "GET", path: "/contract", desc: "okc-mcp 소비 계약" },
];

// E5-3 serving overview & publish (base).
export function Serving() {
  const { id = "" } = useParams();
  const navigate = useNavigate();
  const { data, error, loading, reload } = useAsync(() => servingApi.status(id), [id]);
  const [confirmOpen, setConfirmOpen] = useState(false);
  const [busy, setBusy] = useState(false);
  const [actionErr, setActionErr] = useState<ApiError | null>(null);

  const live = data?.status === "live" || data?.status === "stale";
  const base = `${window.location.origin}/api/serving/${id}`;

  const toggle = async (next: boolean) => {
    if (next) { setConfirmOpen(true); return; }
    setBusy(true); setActionErr(null);
    try { await servingApi.unpublish(id); toast.success("서빙이 중단되었습니다"); reload(); }
    catch (e) { setActionErr(e instanceof ApiError ? e : null); }
    finally { setBusy(false); }
  };

  const publish = async () => {
    setBusy(true); setActionErr(null);
    try { await servingApi.publish(id); toast.success("서빙이 공개되었습니다 (LIVE)"); setConfirmOpen(false); reload(); }
    catch (e) { setActionErr(e instanceof ApiError ? e : null); }
    finally { setBusy(false); }
  };

  return (
    <div>
      <PageHeader title="서빙" description="병합 Vault를 read-only HTTP API로 공개합니다 (admin 전용)."
        actions={
          <div className="flex gap-1 rounded-lg border border-line p-0.5 text-[13px]">
            <button onClick={() => navigate(`/projects/${id}/serving`)} className="rounded-md bg-subtle px-2.5 py-1 font-medium">개요</button>
            <button onClick={() => navigate(`/projects/${id}/serving/verify`)} className="rounded-md px-2.5 py-1 text-muted hover:bg-subtle">Provenance</button>
            <button onClick={() => navigate(`/projects/${id}/serving/contract`)} className="rounded-md px-2.5 py-1 text-muted hover:bg-subtle">계약</button>
          </div>
        }
      />

      {loading && <Skeleton className="h-40 w-full" />}
      {error && <ErrorState error={error} onRetry={reload} />}

      {data && (
        <>
          <Card className="mb-5 p-5">
            <div className="flex items-center justify-between">
              <div className="flex items-center gap-3">
                <ServingStatusBadge status={data.status} />
                {data.published_by && <span className="text-[13px] text-muted">by {data.published_by}</span>}
              </div>
              <label className="flex items-center gap-2 text-sm">
                공개 <Switch checked={live} onCheckedChange={toggle} data-testid="publish-switch" />
              </label>
            </div>
            {data.status === "stale" && (
              <Callout tone="warn" className="mt-3">
                소스가 freeze 이후 변경되었습니다. 재컴파일이 필요합니다.
                <div className="mt-2"><Button size="sm" variant="secondary" onClick={() => navigate(`/projects/${id}/compiled`)}><RefreshCw className="size-3.5" /> 재컴파일</Button></div>
              </Callout>
            )}
            {actionErr && <div className="mt-3"><ErrorState error={actionErr} /></div>}
          </Card>

          <Card>
            <CardHeader><CardTitle className="flex items-center gap-2 text-sm"><Server className="size-4" /> 엔드포인트</CardTitle></CardHeader>
            <CardContent>
              <Tabs defaultValue="endpoints">
                <TabsList>
                  <TabsTrigger value="endpoints">경로</TabsTrigger>
                  <TabsTrigger value="curl">cURL</TabsTrigger>
                </TabsList>
                <TabsContent value="endpoints" className="pt-3">
                  <div className="mb-2 flex items-center gap-2 text-[13px]">
                    <span className="text-muted">base</span> <CopyButton value={base} />
                  </div>
                  <Table>
                    <THead><TR><TH>method</TH><TH>path</TH><TH>설명</TH></TR></THead>
                    <TBody>
                      {ENDPOINTS.map((e) => (
                        <TR key={e.path}><TD><span className="font-mono text-[13px]">{e.method}</span></TD>
                          <TD className="font-mono text-[13px]">{e.path}</TD><TD className="text-muted">{e.desc}</TD></TR>
                      ))}
                    </TBody>
                  </Table>
                </TabsContent>
                <TabsContent value="curl" className="pt-3">
                  <div className="flex items-center gap-2 rounded-lg border border-line bg-[var(--slate-1)] p-3 font-mono text-[12px]">
                    <Terminal className="size-3.5 shrink-0 text-faint" />
                    <span className="truncate">curl {base}/files</span>
                    <CopyButton value={`curl ${base}/files`} label="복사" mono={false} className="ml-auto" />
                  </div>
                </TabsContent>
              </Tabs>
            </CardContent>
          </Card>

          <Callout className="mt-5" icon={<Code2 className="size-4" />}>
            read-only 서빙 API는 okc-mcp(비인간) 소비 대상입니다. 채팅/쿼리 UI는 제공하지 않습니다.
            <a className="ml-1 inline-flex items-center gap-1 text-accent-text" href={`${base}/files`} target="_blank" rel="noreferrer">라이브 열기 <ExternalLink className="size-3" /></a>
          </Callout>
        </>
      )}

      <ConfirmDialog open={confirmOpen} onOpenChange={setConfirmOpen} title="서빙 공개" confirmLabel="공개"
        loading={busy} onConfirm={publish} testid="publish-confirm"
        description="verified 상태의 병합 Vault를 read-only로 공개합니다. verify 미통과 시 거부됩니다.">
        {actionErr && <ErrorState error={actionErr} />}
      </ConfirmDialog>
    </div>
  );
}
