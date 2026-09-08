import { useState } from "react";
import { useNavigate, useParams } from "react-router-dom";
import { Building2, Files, Info, Lock, LockOpen, TriangleAlert, UserCog } from "lucide-react";
import { ApiError, projectsApi, tokensApi } from "@/lib/api";
import { useAsync } from "@/lib/hooks";
import { useProjectContext } from "@/app/project-context";
import { formatDate } from "@/lib/format";
import { PageHeader, ErrorState } from "@/components/ui/page";
import { Button } from "@/components/ui/button";
import { Table, TBody, TD, TH, THead, TR } from "@/components/ui/table";
import { Badge } from "@/components/ui/badge";
import { SkeletonRows } from "@/components/ui/skeleton";
import { EmptyState } from "@/components/ui/empty-state";
import { SlotBar } from "@/components/domain/category-bar";
import { Callout } from "@/components/domain/callout";
import { ConfirmDialog } from "@/components/ui/dialog";
import { toast } from "@/components/ui/toaster";

// E3-4 Sources & Freeze. No dedicated GET /sources exists — registered sources
// are derived from token rows carrying a `registered_source_id`.
export function Sources() {
  const { id = "" } = useParams();
  const navigate = useNavigate();
  const { project, status, reload } = useProjectContext();
  const tokens = useAsync(() => tokensApi.list(id), [id]);
  const [freezeOpen, setFreezeOpen] = useState(false);
  const [freezing, setFreezing] = useState(false);
  const [err, setErr] = useState<ApiError | null>(null);

  const sources = tokens.data?.tokens.filter((t) => t.registered_source_id) ?? [];
  const used = tokens.data?.slot_usage.used ?? project?.source_count ?? 0;
  const frozen = project?.freeze_state === "frozen";
  const atCap = used >= 10;

  const doFreeze = async () => {
    setFreezing(true); setErr(null);
    try {
      await projectsApi.freeze(id);
      toast.success("소스가 고정되었습니다");
      setFreezeOpen(false);
      reload();
    } catch (e) {
      setErr(e instanceof ApiError ? e : null);
    } finally {
      setFreezing(false);
    }
  };

  return (
    <div>
      <PageHeader
        title="소스 & Freeze"
        description="업로드로 등록된 소스를 확인하고 재현 가능한 스냅샷으로 고정합니다."
        badge={<SlotBar used={used} limit={10} />}
        actions={
          <div className="flex gap-2">
            <Button variant="secondary" disabled={atCap} onClick={() => navigate(`/projects/${id}/tokens`)} data-testid="add-source">
              소스 추가 (토큰 발급)
            </Button>
            <Button variant="primary" disabled={frozen || used < 1} onClick={() => setFreezeOpen(true)} data-testid="freeze">
              {frozen ? <><Lock className="size-4" /> 고정됨</> : <><Lock className="size-4" /> 소스 고정</>}
            </Button>
          </div>
        }
      />

      {atCap && (
        <Callout tone="warn" icon={<TriangleAlert className="size-4" />} className="mb-4">
          소스 상한(10)에 도달했습니다. 11번째 소스는 발급·등록 단계에서 차단됩니다 (federation 미구현).
        </Callout>
      )}
      {frozen ? (
        <Callout tone="neutral" icon={<Lock className="size-4" />} className="mb-4">
          소스가 고정되어 편집이 잠겼습니다. 소스를 변경하면 이전 승인이 무효화됩니다(freeze-then-run).
        </Callout>
      ) : (
        <Callout icon={<LockOpen className="size-4" />} className="mb-4">
          아직 고정되지 않았습니다. 통합 실행 전에 소스를 고정하세요.
        </Callout>
      )}

      {tokens.loading && <SkeletonRows rows={3} />}
      {tokens.error && <ErrorState error={tokens.error} onRetry={tokens.reload} />}
      {tokens.data && sources.length === 0 && (
        <EmptyState icon={Files} title="등록된 소스가 없습니다" description="업로드 토큰을 발급해 기여자가 개인 Vault를 업로드하도록 하세요."
          action={<Button variant="primary" onClick={() => navigate(`/projects/${id}/tokens`)}>토큰 발급</Button>} />
      )}
      {sources.length > 0 && (
        <Table>
          <THead>
            <TR><TH>owner</TH><TH>SourceId</TH><TH>슬롯</TH><TH>상태</TH><TH>마지막 사용</TH></TR>
          </THead>
          <TBody>
            {sources.map((s) => (
              <TR key={s.token_id}>
                <TD className="flex items-center gap-1.5">
                  {s.owner_kind === "department" ? <Building2 className="size-4 text-faint" /> : <UserCog className="size-4 text-faint" />}
                  {s.owner_display_name ?? "—"}
                </TD>
                <TD className="font-mono text-[13px] text-muted">{s.registered_source_id}</TD>
                <TD className="tabular">#{s.slot_index}</TD>
                <TD><Badge tone="ok">등록됨</Badge></TD>
                <TD className="text-muted">{formatDate(s.last_used_at)}</TD>
              </TR>
            ))}
          </TBody>
        </Table>
      )}

      {status?.stale && (
        <Callout tone="warn" icon={<TriangleAlert className="size-4" />} className="mt-4">
          소스 세트가 freeze 이후 변경되었습니다. 재실행이 필요합니다.
        </Callout>
      )}

      <ConfirmDialog
        open={freezeOpen}
        onOpenChange={setFreezeOpen}
        title="소스 고정(Freeze)"
        description="고정하면 재현 가능한 스냅샷이 생성되고 소스 편집이 잠깁니다."
        confirmLabel="고정"
        loading={freezing}
        onConfirm={doFreeze}
        testid="freeze-confirm"
      >
        <Callout icon={<Info className="size-4" />}>
          고정 후 소스를 변경하면 모든 하위 taxonomy/cluster 승인이 무효화됩니다(freeze-then-run).
        </Callout>
        {err && <div className="mt-3"><ErrorState error={err} /></div>}
      </ConfirmDialog>
    </div>
  );
}
