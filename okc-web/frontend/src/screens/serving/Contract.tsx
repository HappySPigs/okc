import { useParams } from "react-router-dom";
import { Code2, FileJson, Info, PlugZap } from "lucide-react";
import { servingApi } from "@/lib/api";
import { useAsync } from "@/lib/hooks";
import { PageHeader } from "@/components/ui/page";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Badge } from "@/components/ui/badge";
import { Skeleton } from "@/components/ui/skeleton";
import { Table, TBody, TD, TH, THead, TR } from "@/components/ui/table";
import { Callout } from "@/components/domain/callout";
import { CopyButton } from "@/components/ui/copy-button";

// E5-4 okc-mcp consumption contract (base). Location + format only — no chat/query UI.
const OUT_OF_SCOPE =
  "RAG 검색(청킹·임베딩·vector index·쿼리)과 MCP 도구 표면은 okc-mcp의 책임으로 okc-web 범위 밖입니다. okc-web은 read-only Markdown만 서빙하며, 코어 임베딩은 일시적이라 소비자는 lexical 검색을 쓰거나 산출물을 재임베딩해 semantic 검색을 구성합니다.";
const LAYOUT = ["knowledge/", "legacy/", ".okc/"];
const ENDPOINTS = [
  { method: "GET", path: "/files" }, { method: "GET", path: "/file?path=" },
  { method: "GET", path: "/verify" }, { method: "GET", path: "/explain?path=" },
];

export function Contract() {
  const { id = "" } = useParams();
  const contract = useAsync(() => servingApi.contract(id), [id]);
  const base = `${window.location.origin}/api/serving/${id}`;

  return (
    <div>
      <PageHeader title="okc-mcp 연동 계약" badge={<Badge tone="neutral"><PlugZap className="size-3" /> read-only 계약</Badge>}
        description="okc-mcp가 소비할 위치·형식·read-only 엔드포인트 계약입니다. RAG 검색은 okc-mcp 담당으로 okc-web 범위 밖입니다." />

      {contract.loading && <Skeleton className="h-32 w-full" />}

      <div className="grid gap-4 lg:grid-cols-2">
        <Card>
          <CardHeader><CardTitle className="flex items-center gap-2 text-sm"><Code2 className="size-4" /> 소비 위치</CardTitle></CardHeader>
          <CardContent className="space-y-2 text-sm">
            <div className="flex items-center justify-between gap-2">
              <span className="text-muted">read API base</span>
              <CopyButton value={contract.data?.location.read_api_base ?? base} />
            </div>
            {contract.data?.location.local_dir && (
              <div className="flex items-center justify-between gap-2">
                <span className="text-muted">local dir</span>
                <CopyButton value={contract.data.location.local_dir} label="복사" mono={false} />
              </div>
            )}
          </CardContent>
        </Card>

        <Card>
          <CardHeader><CardTitle className="flex items-center gap-2 text-sm"><FileJson className="size-4" /> 형식</CardTitle></CardHeader>
          <CardContent className="space-y-2 text-sm">
            <div className="text-muted">Markdown 전용 (병합 Vault)</div>
            <div className="flex gap-1.5">
              {(contract.data?.format.layout ?? LAYOUT).map((l) => <Badge key={l} tone="neutral" className="font-mono">{l}</Badge>)}
            </div>
          </CardContent>
        </Card>
      </div>

      <Card className="mt-4">
        <CardHeader><CardTitle className="text-sm">read-only 엔드포인트</CardTitle></CardHeader>
        <CardContent>
          <Table>
            <THead><TR><TH>method</TH><TH>path</TH></TR></THead>
            <TBody>
              {(contract.data?.endpoints.map((e) => ({ method: e.method, path: e.path })) ?? ENDPOINTS).map((e, i) => (
                <TR key={i}><TD className="font-mono text-[13px]">{e.method}</TD><TD className="font-mono text-[13px]">{e.path}</TD></TR>
              ))}
            </TBody>
          </Table>
        </CardContent>
      </Card>

      <Callout tone="warn" className="mt-5" icon={<Info className="size-4" />} title="RAG 검색은 범위 밖 — okc-mcp 담당">
        {contract.data?.out_of_scope ?? OUT_OF_SCOPE}
      </Callout>
    </div>
  );
}
