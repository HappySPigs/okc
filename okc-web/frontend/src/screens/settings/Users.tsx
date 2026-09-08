import { useState } from "react";
import { Info, MoreHorizontal, ShieldCheck, UserPlus } from "lucide-react";
import { ApiError, accountsApi } from "@/lib/api";
import { useAsync } from "@/lib/hooks";
import { formatDate } from "@/lib/format";
import type { CreateAccountResponse, Role } from "@/lib/types";
import { PageHeader, ErrorState } from "@/components/ui/page";
import { Button } from "@/components/ui/button";
import { Table, TBody, TD, TH, THead, TR } from "@/components/ui/table";
import { Badge } from "@/components/ui/badge";
import { SkeletonRows } from "@/components/ui/skeleton";
import { Metric, MetricGrid } from "@/components/domain/metric";
import { RoleBadge } from "@/components/domain/badges";
import { Dialog, DialogContent, DialogFooter, DialogHeader } from "@/components/ui/dialog";
import { DropdownContent, DropdownItem, DropdownMenu, DropdownTrigger } from "@/components/ui/dropdown";
import { Input, Label } from "@/components/ui/input";
import { Callout } from "@/components/domain/callout";
import { CopyButton } from "@/components/ui/copy-button";
import { toast } from "@/components/ui/toaster";

// E1-3 users & roles (minimal-functional). FR-AUTH-2/3/4.
export function Users() {
  const { data, error, loading, reload } = useAsync(() => accountsApi.list(), []);
  const [createOpen, setCreateOpen] = useState(false);
  const [created, setCreated] = useState<CreateAccountResponse | null>(null);

  const admins = data?.filter((a) => a.role === "admin").length ?? 0;

  const setRole = async (id: string, role: Role) => {
    try { await accountsApi.setRole(id, role); toast.success("역할이 변경되었습니다"); reload(); }
    catch (e) { if (e instanceof ApiError) toast.error(e.message); }
  };
  const setStatus = async (id: string, status: "active" | "disabled") => {
    try { await accountsApi.setStatus(id, status); toast.success("상태가 변경되었습니다"); reload(); }
    catch (e) { if (e instanceof ApiError) toast.error(e.message); }
  };

  return (
    <div>
      <PageHeader title="사용자·역할 관리"
        actions={<Button variant="primary" onClick={() => setCreateOpen(true)} data-testid="add-user"><UserPlus className="size-4" /> 사용자 추가</Button>} />

      {data && (
        <MetricGrid className="mb-5 lg:grid-cols-3">
          <Metric label="총 사용자" value={data.length} />
          <Metric label="admin" value={admins} />
          <Metric label="contributor" value={data.length - admins} />
        </MetricGrid>
      )}

      {loading && <SkeletonRows rows={3} />}
      {error && <ErrorState error={error} onRetry={reload} />}
      {data && (
        <Table>
          <THead><TR><TH>표시명</TH><TH>계정 ID</TH><TH>역할</TH><TH>상태</TH><TH>마지막 로그인</TH><TH /></TR></THead>
          <TBody>
            {data.map((a) => (
              <TR key={a.id}>
                <TD className="font-medium">{a.display_name}</TD>
                <TD className="font-mono text-[13px] text-muted">{a.email}</TD>
                <TD><div className="flex items-center gap-1.5"><RoleBadge role={a.role} />{a.role === "admin" && <Info className="size-3.5 text-faint" aria-label="curator_id로 기록됨" />}</div></TD>
                <TD><Badge tone={a.status === "active" ? "ok" : "neutral"}>{a.status}</Badge></TD>
                <TD className="text-muted">{formatDate(a.last_login_at)}</TD>
                <TD className="text-right">
                  <DropdownMenu>
                    <DropdownTrigger asChild><button className="rounded-md p-1 hover:bg-subtle"><MoreHorizontal className="size-4" /></button></DropdownTrigger>
                    <DropdownContent>
                      <DropdownItem onSelect={() => setRole(a.id, a.role === "admin" ? "contributor" : "admin")}>
                        <ShieldCheck className="size-4" /> {a.role === "admin" ? "contributor로 변경" : "admin으로 변경"}
                      </DropdownItem>
                      <DropdownItem destructive onSelect={() => setStatus(a.id, a.status === "active" ? "disabled" : "active")}>
                        {a.status === "active" ? "비활성화" : "활성화"}
                      </DropdownItem>
                    </DropdownContent>
                  </DropdownMenu>
                </TD>
              </TR>
            ))}
          </TBody>
        </Table>
      )}

      <Callout className="mt-5" icon={<Info className="size-4" />}>
        admin 계정 ID가 프로젝트 <code className="font-mono">curator_id</code>로 기록됩니다(okc-core 비검증).
      </Callout>

      <CreateUserDialog open={createOpen} onOpenChange={setCreateOpen} onCreated={(r) => { setCreated(r); reload(); }} />
      {created && (
        <Dialog open onOpenChange={() => setCreated(null)}>
          <DialogContent>
            <DialogHeader title="사용자가 생성되었습니다" description="임시 비밀번호는 지금만 표시됩니다(NFR-SEC-1)." />
            <div className="flex items-center justify-between gap-2 rounded-lg border border-line bg-subtle px-3 py-2">
              <span className="font-mono text-[13px]">{created.temp_password}</span>
              <CopyButton value={created.temp_password} label="복사" mono={false} />
            </div>
            <DialogFooter><Button variant="primary" onClick={() => setCreated(null)}>완료</Button></DialogFooter>
          </DialogContent>
        </Dialog>
      )}
    </div>
  );
}

function CreateUserDialog({ open, onOpenChange, onCreated }: { open: boolean; onOpenChange: (v: boolean) => void; onCreated: (r: CreateAccountResponse) => void }) {
  const [email, setEmail] = useState("");
  const [name, setName] = useState("");
  const [role, setRole] = useState<Role>("contributor");
  const [busy, setBusy] = useState(false);
  const [err, setErr] = useState<ApiError | null>(null);

  const create = async () => {
    if (!email.trim() || !name.trim()) return;
    setBusy(true); setErr(null);
    try {
      const r = await accountsApi.create(email.trim(), name.trim(), role);
      onOpenChange(false);
      onCreated(r);
      setEmail(""); setName("");
    } catch (e) { setErr(e instanceof ApiError ? e : null); }
    finally { setBusy(false); }
  };

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent>
        <DialogHeader title="사용자 추가" />
        <div className="space-y-3">
          <div className="space-y-1.5"><Label>이메일</Label><Input value={email} onChange={(e) => setEmail(e.target.value)} data-testid="user-email" /></div>
          <div className="space-y-1.5"><Label>표시명</Label><Input value={name} onChange={(e) => setName(e.target.value)} /></div>
          <div className="space-y-1.5">
            <Label>역할</Label>
            <select className="w-full rounded-lg border border-line bg-surface px-3 py-2 text-sm" value={role} onChange={(e) => setRole(e.target.value as Role)}>
              <option value="contributor">contributor</option>
              <option value="admin">admin</option>
            </select>
          </div>
          {err && <ErrorState error={err} />}
        </div>
        <DialogFooter>
          <Button variant="ghost" onClick={() => onOpenChange(false)}>취소</Button>
          <Button variant="primary" loading={busy} disabled={!email.trim() || !name.trim()} onClick={create} data-testid="user-create">생성</Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
