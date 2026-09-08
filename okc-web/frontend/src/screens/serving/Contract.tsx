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
  "okc-mcp는 미구현입니다. 청킹·임베딩·vector index·쿼리는 out-of-scope이며, 코어 임베딩은 일시적이라 컴파일 산출물을 재임베딩해야 합니다.";
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
      <PageHeader title="okc-mcp 연동 계약" badge={<Badge tone="warn"><PlugZap className="size-3" /> 이연(Deferred)</Badge>}
        description="okc-mcp가 소비할 위치·형식 계약. 완성물이 아니라 정직한 계약입니다." />

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

      <Callout tone="warn" className="mt-5" icon={<Info className="size-4" />} title="okc-mcp UNIMPLEMENTED">
        {contract.data?.out_of_scope ?? OUT_OF_SCOPE}
      </Callout>
    </div>
  );
}
