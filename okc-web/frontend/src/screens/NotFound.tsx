import { useNavigate } from "react-router-dom";
import { Button } from "@/components/ui/button";

export function NotFound() {
  const navigate = useNavigate();
  return (
    <div className="flex min-h-screen flex-col items-center justify-center gap-4 bg-app px-4 text-center">
      <div className="text-4xl font-semibold text-faint">404</div>
      <p className="text-sm text-muted">요청한 페이지를 찾을 수 없습니다.</p>
      <Button variant="primary" onClick={() => navigate("/projects")}>프로젝트 목록</Button>
    </div>
  );
}
