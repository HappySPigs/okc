import { useEffect, useState } from "react";
import { useLocation, useNavigate, useParams } from "react-router-dom";
import {
  Ban, Building2, Eye, EyeOff, KeyRound, MoreHorizontal, Plus, RefreshCw, TriangleAlert, UserCog,
} from "lucide-react";
import { ApiError, tokensApi } from "@/lib/api";
import { useAsync } from "@/lib/hooks";
import { formatDate } from "@/lib/format";
import type { IssuedToken } from "@/lib/types";
import { PageHeader, ErrorState } from "@/components/ui/page";
import { Button } from "@/components/ui/button";
import { Table, TBody, TD, TH, THead, TR } from "@/components/ui/table";
import { SkeletonRows } from "@/components/ui/skeleton";
import { EmptyState } from "@/components/ui/empty-state";
import { Metric, MetricGrid } from "@/components/domain/metric";
import { TokenStatusBadge } from "@/components/domain/badges";
import { Dialog, DialogContent, DialogFooter, DialogHeader } from "@/components/ui/dialog";
import { DropdownContent, DropdownItem, DropdownMenu, DropdownTrigger } from "@/components/ui/dropdown";
import { Input, Label } from "@/components/ui/input";
import { Callout } from "@/components/domain/callout";
import { CopyButton } from "@/components/ui/copy-button";
import { cn } from "@/lib/cn";
import { toast } from "@/components/ui/toaster";

export function Tokens() {
  const { id = "" } = useParams();
  const navigate = useNavigate();
  const { pathname } = useLocation();
  const { data, error, loading, reload } = useAsync(() => tokensApi.list(id), [id]);
  const [issueOpen, setIssueOpen] = useState(pathname.endsWith("/new"));
  const [revealed, setRevealed] = useState<IssuedToken | null>(null);

  useEffect(() => { if (pathname.endsWith("/new")) setIssueOpen(true); }, [pathname]);

  const active = data?.tokens.filter((t) => t.status === "active").length ?? 0;
  const used = data?.slot_usage.used ?? 0;
  const atCap = used >= 10;

  const rotate = async (tokenId: string) => {
    try {
      const t = await tokensApi.rotate(id, tokenId);
      setRevealed(t);
      reload();
    } catch (e) { if (e instanceof ApiError) toast.error(e.message); }
  };
  const revoke = async (tokenId: string) => {
    try {
      await tokensApi.revoke(id, tokenId);
      toast.success("토큰이 폐기되었습니다");
      reload();
    } catch (e) { if (e instanceof ApiError) toast.error(e.message); }
  };

  return (
    <div>
      <PageHeader
        title="업로드 토큰"
        description="기여자에게 발급하는 1회성 업로드 자격. 발급은 인증이 아니라 슬롯 부여입니다."
        actions={<Button variant="primary" disabled={atCap} onClick={() => setIssueOpen(true)} data-testid="issue-token"><Plus className="size-4" /> 토큰 발급</Button>}
      />

      {data && (
        <MetricGrid className="mb-5 lg:grid-cols-3">
          <Metric label="활성 토큰" value={active} />
          <Metric label="사용 슬롯" value={`${used}/10`} tone={atCap ? "warn" : undefined} />
          <Metric label="전체 토큰" value={data.tokens.length} />
        </MetricGrid>
      )}

      {atCap && (
        <Callout tone="warn" icon={<TriangleAlert className="size-4" />} className="mb-4">
          소스 상한(10) 도달 — 새 토큰 발급이 차단됩니다.
        </Callout>
      )}

      {loading && <SkeletonRows rows={3} />}
      {error && <ErrorState error={error} onRetry={reload} />}
      {data && data.tokens.length === 0 && (
        <EmptyState icon={KeyRound} title="발급된 토큰이 없습니다" description="첫 업로드 토큰을 발급하세요."
          action={<Button variant="primary" onClick={() => setIssueOpen(true)}><Plus className="size-4" /> 토큰 발급</Button>} />
      )}
      {data && data.tokens.length > 0 && (
        <Table>
          <THead>
            <TR><TH>라벨</TH><TH>슬롯</TH><TH>상태</TH><TH>생성</TH><TH>등록 소스</TH><TH /></TR>
          </THead>
          <TBody>
            {data.tokens.map((t) => (
              <TR key={t.token_id} className={cn(t.status === "revoked" && "opacity-50")}>
                <TD className="flex items-center gap-1.5">
                  {t.owner_kind === "department" ? <Building2 className="size-4 text-faint" /> : <UserCog className="size-4 text-faint" />}
                  {t.owner_display_name ?? <span className="text-faint">—</span>}
                  <span className="ml-1 font-mono text-[12px] text-faint">{t.selector}</span>
                </TD>
                <TD className="tabular">#{t.slot_index}</TD>
                <TD><TokenStatusBadge status={t.status} /></TD>
                <TD className="text-muted">{formatDate(t.created_at)}</TD>
                <TD className="font-mono text-[12px] text-muted">{t.registered_source_id ?? "—"}</TD>
                <TD className="text-right">
                  <DropdownMenu>
                    <DropdownTrigger asChild>
                      <button className="rounded-md p-1 hover:bg-subtle" data-testid={`token-actions-${t.token_id}`}><MoreHorizontal className="size-4" /></button>
                    </DropdownTrigger>
                    <DropdownContent>
                      <DropdownItem onSelect={() => rotate(t.token_id)}><RefreshCw className="size-4" /> 회전</DropdownItem>
                      <DropdownItem destructive onSelect={() => revoke(t.token_id)}><Ban className="size-4" /> 폐기</DropdownItem>
                    </DropdownContent>
                  </DropdownMenu>
                </TD>
              </TR>
            ))}
          </TBody>
        </Table>
      )}

      <IssueDialog
        projectId={id}
        open={issueOpen}
        onOpenChange={(v) => { setIssueOpen(v); if (!v && pathname.endsWith("/new")) navigate(`/projects/${id}/tokens`); }}
        onIssued={(t) => { setRevealed(t); reload(); }}
      />
      <RevealDialog token={revealed} onClose={() => setRevealed(null)} />
    </div>
  );
}

function IssueDialog({
  projectId, open, onOpenChange, onIssued,
}: {
  projectId: string;
  open: boolean;
  onOpenChange: (v: boolean) => void;
  onIssued: (t: IssuedToken) => void;
}) {
  const [kind, setKind] = useState("individual");
  const [owner, setOwner] = useState("");
  const [ttl, setTtl] = useState("");
  const [busy, setBusy] = useState(false);
  const [err, setErr] = useState<ApiError | null>(null);

  const issue = async () => {
    setBusy(true); setErr(null);
    try {
      const t = await tokensApi.issue(projectId, {
        owner_display_name: owner || null,
        owner_kind: kind,
        ttl_seconds: ttl ? Number(ttl) * 3600 : null,
      });
      onOpenChange(false);
      onIssued(t);
      setOwner(""); setTtl("");
    } catch (e) {
      setErr(e instanceof ApiError ? e : null);
    } finally {
      setBusy(false);
    }
  };

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent>
        <DialogHeader title="업로드 토큰 발급" description="발급된 토큰 시크릿은 이 순간 한 번만 표시됩니다." />
        <div className="space-y-3">
          <div className="space-y-1.5">
            <Label>구분</Label>
            <div className="flex gap-2">
              {(["individual", "department"] as const).map((k) => (
                <button key={k} type="button" onClick={() => setKind(k)}
                  className={cn("flex-1 rounded-lg border px-3 py-2 text-sm", kind === k ? "border-accent-line bg-accent-soft text-accent-text" : "border-line")}>
                  {k === "individual" ? "개인" : "부서"}
                </button>
              ))}
            </div>
          </div>
          <div className="space-y-1.5">
            <Label htmlFor="owner">owner_display_name (선택, 비검증 라벨)</Label>
            <Input id="owner" value={owner} onChange={(e) => setOwner(e.target.value)} data-testid="token-owner" />
          </div>
          <div className="space-y-1.5">
            <Label htmlFor="ttl">만료 (시간, 선택)</Label>
            <Input id="ttl" type="number" min={1} value={ttl} onChange={(e) => setTtl(e.target.value)} placeholder="무제한" />
          </div>
          {err && <ErrorState error={err} />}
        </div>
        <DialogFooter>
          <Button variant="ghost" onClick={() => onOpenChange(false)}>취소</Button>
          <Button variant="primary" loading={busy} onClick={issue} data-testid="token-issue-submit"><KeyRound className="size-4" /> 발급</Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}

function RevealDialog({ token, onClose }: { token: IssuedToken | null; onClose: () => void }) {
  const [show, setShow] = useState(false);
  useEffect(() => { setShow(false); }, [token]);
  if (!token) return null;
  // Human portal (SPA) URL. The backend's upload_url (/u/{token}) is the JSON
  // API; contributors open the /upload/{token} portal which calls that API.
  const uploadUrl = `${window.location.origin}/upload/${token.token}`;
  return (
    <Dialog open={!!token} onOpenChange={(v) => { if (!v) onClose(); }}>
      <DialogContent>
        <DialogHeader title="토큰이 발급되었습니다" description={`슬롯 #${token.slot_index} / 10`} />
        <div className="space-y-3">
          <Callout tone="warn" icon={<TriangleAlert className="size-4" />}>
            이 토큰은 지금만 표시됩니다. 서버에는 해시만 저장됩니다(NFR-SEC-1).
          </Callout>
          <div className="space-y-1.5">
            <Label>토큰 시크릿</Label>
            <div className="flex items-center gap-2">
              <div className="flex-1 truncate rounded-lg border border-line bg-subtle px-3 py-2 font-mono text-[13px]">
                {show ? token.token : "•".repeat(Math.min(28, token.token.length))}
              </div>
              <Button variant="ghost" size="icon" onClick={() => setShow((s) => !s)} aria-label="토큰 표시">
                {show ? <EyeOff className="size-4" /> : <Eye className="size-4" />}
              </Button>
              <CopyButton value={token.token} label="복사" mono={false} testid="copy-token" />
            </div>
          </div>
          <div className="space-y-1.5">
            <Label>업로드 URL</Label>
            <CopyButton value={uploadUrl} className="w-full justify-between" />
          </div>
        </div>
        <DialogFooter>
          <Button variant="primary" onClick={onClose}>완료</Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
