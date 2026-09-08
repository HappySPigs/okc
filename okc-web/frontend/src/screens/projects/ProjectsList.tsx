import { useEffect, useState } from "react";
import { useLocation, useNavigate } from "react-router-dom";
import { FolderGit2, Info, Lock, LockOpen, Plus } from "lucide-react";
import { ApiError, projectsApi } from "@/lib/api";
import { useAsync } from "@/lib/hooks";
import { useAuth } from "@/app/auth";
import { formatDate } from "@/lib/format";
import { PageHeader, ErrorState } from "@/components/ui/page";
import { Button } from "@/components/ui/button";
import { Table, TBody, TD, TH, THead, TR } from "@/components/ui/table";
import { Badge } from "@/components/ui/badge";
import { SkeletonRows } from "@/components/ui/skeleton";
import { EmptyState } from "@/components/ui/empty-state";
import { Metric, MetricGrid } from "@/components/domain/metric";
import { SlotBar } from "@/components/domain/category-bar";
import { Dialog, DialogContent, DialogFooter, DialogHeader } from "@/components/ui/dialog";
import { Input, Label } from "@/components/ui/input";
import { Callout } from "@/components/domain/callout";

// E3-1 project list + E3-2 create modal.
export function ProjectsList() {
  const navigate = useNavigate();
  const { pathname } = useLocation();
  const { data, error, loading, reload } = useAsync(() => projectsApi.list(), []);
  const [createOpen, setCreateOpen] = useState(pathname === "/projects/new");

  useEffect(() => { if (pathname === "/projects/new") setCreateOpen(true); }, [pathname]);

  const frozen = data?.filter((p) => p.freeze_state === "frozen").length ?? 0;

  return (
    <div>
      <PageHeader
        title="프로젝트"
        description="통합 파이프라인 단위. 프로젝트당 소스 ≤ 10."
        actions={<Button variant="primary" onClick={() => setCreateOpen(true)} data-testid="new-project"><Plus className="size-4" /> 새 프로젝트</Button>}
      />

      {data && data.length > 0 && (
        <MetricGrid className="mb-5 lg:grid-cols-2">
          <Metric label="총 프로젝트" value={data.length} />
          <Metric label="고정된 프로젝트" value={frozen} />
        </MetricGrid>
      )}

      {loading && <SkeletonRows rows={4} />}
      {error && <ErrorState error={error} onRetry={reload} />}
      {data && data.length === 0 && (
        <EmptyState icon={FolderGit2} title="아직 프로젝트가 없습니다" description="새 프로젝트를 만들어 통합 파이프라인을 시작하세요."
          action={<Button variant="primary" onClick={() => setCreateOpen(true)}><Plus className="size-4" /> 새 프로젝트</Button>} />
      )}
      {data && data.length > 0 && (
        <Table>
          <THead>
            <TR>
              <TH>프로젝트</TH><TH>curator_id</TH><TH>소스</TH><TH>고정</TH><TH>생성</TH>
            </TR>
          </THead>
          <TBody>
            {data.map((p) => (
              <TR key={p.id} className="cursor-pointer" onClick={() => navigate(`/projects/${p.id}`)} data-testid={`project-row-${p.id}`}>
                <TD className="font-medium">{p.name}</TD>
                <TD className="font-mono text-[13px] text-muted">{p.curator_id}</TD>
                <TD><SlotBar used={p.source_count} limit={10} /></TD>
                <TD>{p.freeze_state === "frozen"
                  ? <Badge tone="neutral"><Lock className="size-3" /> frozen</Badge>
                  : <Badge fill="outline"><LockOpen className="size-3" /> unfrozen</Badge>}</TD>
                <TD className="text-muted">{formatDate(p.created_at)}</TD>
              </TR>
            ))}
          </TBody>
        </Table>
      )}

      <CreateProjectDialog open={createOpen} onOpenChange={(v) => { setCreateOpen(v); if (!v && pathname === "/projects/new") navigate("/projects"); }} />
    </div>
  );
}

function CreateProjectDialog({ open, onOpenChange }: { open: boolean; onOpenChange: (v: boolean) => void }) {
  const navigate = useNavigate();
  const { session } = useAuth();
  const [name, setName] = useState("");
  const [busy, setBusy] = useState(false);
  const [err, setErr] = useState<ApiError | null>(null);

  const create = async () => {
    if (!name.trim()) return;
    setBusy(true); setErr(null);
    try {
      const p = await projectsApi.create(name.trim());
      onOpenChange(false);
      navigate(`/projects/${p.id}`);
    } catch (e) {
      setErr(e instanceof ApiError ? e : null);
    } finally {
      setBusy(false);
    }
  };

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent>
        <DialogHeader title="새 프로젝트" description="통합 파이프라인 단위를 생성합니다." />
        <div className="space-y-3">
          <div className="space-y-1.5">
            <Label htmlFor="pname">프로젝트명</Label>
            <Input id="pname" value={name} onChange={(e) => setName(e.target.value)} data-testid="project-name" autoFocus />
          </div>
          <div className="space-y-1.5">
            <Label>curator_id</Label>
            <div className="rounded-lg border border-line bg-subtle px-3 py-2 font-mono text-[13px] text-muted">{session?.curator_label}</div>
          </div>
          <Callout icon={<Info className="size-4" />}>
            curator_id는 okc-core가 검증하지 않는 표시용 라벨입니다(권한 강제는 okc-web RBAC 담당).
          </Callout>
          {err && <ErrorState error={err} />}
        </div>
        <DialogFooter>
          <Button variant="ghost" onClick={() => onOpenChange(false)}>취소</Button>
          <Button variant="primary" loading={busy} disabled={!name.trim()} onClick={create} data-testid="project-create">생성</Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
