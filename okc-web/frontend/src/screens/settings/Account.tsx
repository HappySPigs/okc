import { useState } from "react";
import { ApiError, accountsApi } from "@/lib/api";
import { useAuth } from "@/app/auth";
import { PageHeader } from "@/components/ui/page";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Input, Label } from "@/components/ui/input";
import { RoleBadge } from "@/components/domain/badges";
import { Callout } from "@/components/domain/callout";
import { Info } from "lucide-react";
import { toast } from "@/components/ui/toaster";

// E1-5 my account (minimal). Backend exposes password change per account id.
export function Account() {
  const { session } = useAuth();
  const [pw, setPw] = useState("");
  const [busy, setBusy] = useState(false);

  const changePassword = async () => {
    if (!session || !pw) return;
    setBusy(true);
    try { await accountsApi.changePassword(session.account_id, pw); toast.success("비밀번호가 변경되었습니다"); setPw(""); }
    catch (e) { if (e instanceof ApiError) toast.error(e.message); }
    finally { setBusy(false); }
  };

  return (
    <div className="max-w-xl">
      <PageHeader title="내 계정" />
      <Card className="mb-4">
        <CardHeader><CardTitle className="text-sm">프로필</CardTitle></CardHeader>
        <CardContent className="space-y-2 text-sm">
          <div className="flex justify-between"><span className="text-muted">표시명</span><span>{session?.display_name}</span></div>
          <div className="flex justify-between"><span className="text-muted">계정 ID</span><span className="font-mono">{session?.email}</span></div>
          <div className="flex justify-between"><span className="text-muted">역할</span>{session && <RoleBadge role={session.role} />}</div>
        </CardContent>
      </Card>

      <Card>
        <CardHeader><CardTitle className="text-sm">비밀번호 변경</CardTitle></CardHeader>
        <CardContent className="space-y-3">
          <div className="space-y-1.5">
            <Label htmlFor="np">새 비밀번호</Label>
            <Input id="np" type="password" value={pw} onChange={(e) => setPw(e.target.value)} data-testid="new-password" />
          </div>
          <Button variant="primary" loading={busy} disabled={!pw} onClick={changePassword} data-testid="change-password">변경</Button>
        </CardContent>
      </Card>

      <Callout className="mt-4" icon={<Info className="size-4" />}>
        이 계정 ID(<code className="font-mono">curator_id</code>)가 통합 감사에 기록됩니다. okc-core는 이 라벨을 검증하지 않습니다.
      </Callout>
    </div>
  );
}
