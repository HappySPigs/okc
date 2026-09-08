import { createContext, useCallback, useContext, useEffect, useMemo, useState, type ReactNode } from "react";
import { ApiError, authApi, setUnauthorizedHandler } from "@/lib/api";
import { toast } from "@/components/ui/toaster";
import type { SessionView } from "@/lib/types";

interface AuthState {
  session: SessionView | null;
  loading: boolean;
  login: (email: string, password: string) => Promise<SessionView>;
  logout: () => Promise<void>;
  refresh: () => void;
}

const AuthContext = createContext<AuthState | null>(null);
export const SESSION_EXPIRED_FLAG = "okc_session_expired";

export function AuthProvider({ children }: { children: ReactNode }) {
  const [session, setSession] = useState<SessionView | null>(null);
  const [loading, setLoading] = useState(true);
  const [nonce, setNonce] = useState(0);

  useEffect(() => {
    let live = true;
    authApi
      .session()
      .then((s) => { if (live) setSession(s); })
      .catch(() => { if (live) setSession(null); })
      .finally(() => { if (live) setLoading(false); });
    return () => { live = false; };
  }, [nonce]);

  // Global 401 interceptor: any auth error drops the session; guards redirect.
  useEffect(() => {
    setUnauthorizedHandler(() => {
      setSession((prev) => {
        if (prev) {
          sessionStorage.setItem(SESSION_EXPIRED_FLAG, "1");
          toast.warning("세션이 만료되었습니다. 다시 로그인하세요.");
        }
        return null;
      });
    });
    return () => setUnauthorizedHandler(null);
  }, []);

  const login = useCallback(async (email: string, password: string) => {
    const s = await authApi.login(email, password);
    sessionStorage.removeItem(SESSION_EXPIRED_FLAG);
    setSession(s);
    return s;
  }, []);

  const logout = useCallback(async () => {
    try {
      await authApi.logout();
    } catch (e) {
      if (!(e instanceof ApiError)) throw e;
    }
    setSession(null);
  }, []);

  const refresh = useCallback(() => setNonce((n) => n + 1), []);

  const value = useMemo(
    () => ({ session, loading, login, logout, refresh }),
    [session, loading, login, logout, refresh],
  );
  return <AuthContext.Provider value={value}>{children}</AuthContext.Provider>;
}

export function useAuth(): AuthState {
  const ctx = useContext(AuthContext);
  if (!ctx) throw new Error("useAuth must be used within AuthProvider");
  return ctx;
}
