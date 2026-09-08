import { useState, type FormEvent } from "react";
import { useLocation, useNavigate } from "react-router-dom";
import { Eye, EyeOff, KeyRound, LogIn, ShieldCheck, TriangleAlert } from "lucide-react";
import { ApiError } from "@/lib/api";
import { SESSION_EXPIRED_FLAG, useAuth } from "@/app/auth";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { Button } from "@/components/ui/button";
import { Input, Label } from "@/components/ui/input";
import { Alert } from "@/components/ui/alert";

export function Login() {
  const navigate = useNavigate();
  const location = useLocation();
  const { login } = useAuth();
  const [tab, setTab] = useState("password");
  const sessionExpired = sessionStorage.getItem(SESSION_EXPIRED_FLAG) === "1";

  return (
    <div className="flex min-h-screen items-center justify-center bg-app px-4">
      <div className="w-full max-w-sm">
        <div className="mb-6 flex items-center justify-center gap-2">
          <KeyRound className="size-5 text-accent" />
          <span className="text-lg font-semibold tracking-tight">okc-web</span>
        </div>
        {sessionExpired && (
          <Alert variant="warning" className="mb-3" icon={<TriangleAlert className="size-4" />}>
            세션이 만료되었습니다. 다시 로그인하세요.
          </Alert>
        )}
        <Card>
          <CardHeader>
            <CardTitle>로그인</CardTitle>
            <CardDescription>관리자 로그인 또는 업로드 토큰으로 계속합니다.</CardDescription>
          </CardHeader>
          <CardContent>
            <Tabs value={tab} onValueChange={setTab}>
              <TabsList className="mb-4">
                <TabsTrigger value="password" data-testid="login-tab-password">비밀번호</TabsTrigger>
                <TabsTrigger value="token" data-testid="login-tab-token">업로드 토큰</TabsTrigger>
              </TabsList>
              <TabsContent value="password">
                <PasswordForm
                  onSuccess={() => navigate((location.state as { from?: string })?.from ?? "/projects", { replace: true })}
                  login={login}
                />
              </TabsContent>
              <TabsContent value="token">
                <TokenForm onGo={(t) => navigate(`/upload/${encodeURIComponent(t)}`)} />
              </TabsContent>
            </Tabs>
          </CardContent>
        </Card>
        <p className="mt-4 px-2 text-center text-[12px] leading-5 text-faint">
          인증·권한은 okc-web이 강제합니다. okc-core는 호출자를 인증하지 않습니다.
        </p>
      </div>
    </div>
  );
}

function PasswordForm({ onSuccess, login }: { onSuccess: () => void; login: (e: string, p: string) => Promise<unknown> }) {
  const [email, setEmail] = useState("");
  const [password, setPassword] = useState("");
  const [show, setShow] = useState(false);
  const [busy, setBusy] = useState(false);
  const [err, setErr] = useState<ApiError | null>(null);

  const submit = async (e: FormEvent) => {
    e.preventDefault();
    if (!email.trim() || !password) return;
    setBusy(true); setErr(null);
    try {
      await login(email.trim(), password);
      onSuccess();
    } catch (e) {
      setErr(e instanceof ApiError ? e : null);
    } finally {
      setBusy(false);
    }
  };

  const message =
    err?.code === "ACCOUNT_DISABLED" ? "비활성화된 계정입니다." :
    err?.isAuth || err?.code === "VALIDATION_FAILED" ? "계정 ID 또는 비밀번호가 올바르지 않습니다." :
    err?.message;

  return (
    <form onSubmit={submit} className="space-y-3">
      <div className="space-y-1.5">
        <Label htmlFor="email">계정 ID (이메일)</Label>
        <Input id="email" type="email" autoComplete="username" value={email}
          onChange={(e) => setEmail(e.target.value)} data-testid="login-email" />
      </div>
      <div className="space-y-1.5">
        <Label htmlFor="password">비밀번호</Label>
        <div className="relative">
          <Input id="password" type={show ? "text" : "password"} autoComplete="current-password"
            value={password} onChange={(e) => setPassword(e.target.value)} data-testid="login-password" />
          <button type="button" onClick={() => setShow((s) => !s)}
            className="absolute right-2 top-1/2 -translate-y-1/2 text-faint" aria-label="비밀번호 표시">
            {show ? <EyeOff className="size-4" /> : <Eye className="size-4" />}
          </button>
        </div>
      </div>
      {message && <Alert variant="destructive" icon={<TriangleAlert className="size-4" />}>{message}</Alert>}
      <Button type="submit" variant="primary" className="w-full justify-center" loading={busy} data-testid="login-submit">
        <LogIn className="size-4" /> 로그인
      </Button>
    </form>
  );
}

function TokenForm({ onGo }: { onGo: (token: string) => void }) {
  const [token, setToken] = useState("");
  return (
    <form onSubmit={(e) => { e.preventDefault(); if (token.trim()) onGo(token.trim()); }} className="space-y-3">
      <div className="space-y-1.5">
        <Label htmlFor="token">업로드 토큰</Label>
        <Input id="token" value={token} onChange={(e) => setToken(e.target.value)}
          placeholder="selector.verifier" className="font-mono" data-testid="login-token" />
      </div>
      <Alert variant="info" icon={<ShieldCheck className="size-4" />}>
        토큰 업로드는 로그인이 필요 없습니다. 앱 셸 세션을 만들지 않고 업로드 포털로 이동합니다.
      </Alert>
      <Button type="submit" variant="primary" className="w-full justify-center" data-testid="login-token-go">
        업로드 포털로 이동
      </Button>
    </form>
  );
}
