import type { ReactNode } from "react";
import { Navigate, useLocation } from "react-router-dom";
import { Loader2 } from "lucide-react";
import { useAuth } from "./auth";

/** Admin-only gate. Contributors have NO app-shell session, so a present session
 *  is always an active admin (backend enforces this). Unauthenticated → /login. */
export function RequireAdmin({ children }: { children: ReactNode }) {
  const { session, loading } = useAuth();
  const location = useLocation();
  if (loading) {
    return (
      <div className="flex min-h-screen items-center justify-center text-faint">
        <Loader2 className="size-5 animate-spin" />
      </div>
    );
  }
  if (!session) {
    return <Navigate to="/login" replace state={{ from: location.pathname }} />;
  }
  return <>{children}</>;
}
