import { useNavigate, useParams } from "react-router-dom";
import { Ban, Building2, Clock, Info, ShieldCheck, ShieldX, Upload, UserCog } from "lucide-react";
import { uploadApi } from "@/lib/api";
import { useAsync } from "@/lib/hooks";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Alert } from "@/components/ui/alert";
import { Badge } from "@/components/ui/badge";
import { Skeleton } from "@/components/ui/skeleton";
import { Callout } from "@/components/domain/callout";
import { UploadShell } from "./UploadShell";

// E2-3 upload portal (token landing). Contributor surface — no app shell.
export function UploadPortal() {
  const { token = "" } = useParams();
  const navigate = useNavigate();
  const { data, error, loading } = useAsync(() => uploadApi.target(token), [token]);

  return (
    <UploadShell>
      <Card>
        <CardHeader>
          <CardTitle>개인 Vault 업로드</CardTitle>
        </CardHeader>
        <CardContent className="space-y-4">
          {loading && <Skeleton className="h-24 w-full" />}
          {error && <PortalError code={error.code} message={error.message} />}
          {data && (
            <>
              <div className="flex items-center justify-between">
                <div className="text-sm">
                  <div className="font-medium">{data.project_name}</div>
                  <div className="text-muted">슬롯 #{data.slot_index} / 10</div>
                </div>
                <Badge tone="ok"><ShieldCheck className="size-3" /> 유효</Badge>
              </div>
              {data.owner_display_name && (
                <div className="flex items-center gap-1.5 text-sm text-muted">
                  {data.owner_kind === "department" ? <Building2 className="size-4" /> : <UserCog className="size-4" />}
                  {data.owner_display_name}
                </div>
              )}
              <Callout icon={<Info className="size-4" />}>
                허용 포맷: zip / tar.zst. Markdown 전용 병합 — 비-Markdown 첨부는 병합 출력에 포함되지 않습니다.
              </Callout>
              <Button variant="primary" className="w-full justify-center"
                onClick={() => navigate(`/upload/${token}/file`)} data-testid="portal-start">
                <Upload className="size-4" /> 업로드 시작
              </Button>
            </>
          )}
        </CardContent>
      </Card>
    </UploadShell>
  );
}

function PortalError({ code, message }: { code: string; message: string }) {
  const map: Record<string, { icon: React.ReactNode; text: string }> = {
    TOKEN_INVALID: { icon: <ShieldX className="size-4" />, text: "유효하지 않은 토큰입니다." },
    TOKEN_EXPIRED: { icon: <Clock className="size-4" />, text: "만료된 토큰입니다. 관리자에게 재발급을 요청하세요." },
    TOKEN_REVOKED: { icon: <Ban className="size-4" />, text: "폐기된 토큰입니다." },
    SOURCE_CAP_EXCEEDED: { icon: <Ban className="size-4" />, text: "소스 상한(10)에 도달했습니다." },
    PROJECT_BUSY: { icon: <Clock className="size-4" />, text: "다른 작업이 진행 중입니다. 업로드는 대기 큐로 처리됩니다." },
  };
  const e = map[code];
  return (
    <Alert variant={code === "PROJECT_BUSY" ? "warning" : "destructive"} icon={e?.icon} title={code}>
      {e?.text ?? message}
    </Alert>
  );
}
