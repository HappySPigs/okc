import { useNavigate } from "react-router-dom";
import { Ban, LogIn } from "lucide-react";
import { Card, CardContent } from "@/components/ui/card";
import { Button } from "@/components/ui/button";

// E1-4 403 — honest RBAC reflection (C-1). Contributors have no app-shell session.
export function Forbidden() {
  const navigate = useNavigate();
  return (
    <div className="flex min-h-screen items-center justify-center bg-app px-4">
      <Card className="w-full max-w-md">
        <CardContent className="flex flex-col items-center gap-4 p-8 text-center">
          <Ban className="size-10 text-danger" />
          <div>
            <h1 className="text-lg font-semibold">접근 권한이 없습니다</h1>
            <p className="mt-1 text-sm text-muted">
              이 작업은 admin 역할만 수행할 수 있습니다. contributor는 업로드 토큰 링크로만 접근합니다.
            </p>
          </div>
          <Button variant="primary" onClick={() => navigate("/login")}>
            <LogIn className="size-4" /> 로그인으로 이동
          </Button>
        </CardContent>
      </Card>
    </div>
  );
}
