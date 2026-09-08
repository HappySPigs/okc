import { useCallback, useEffect, useMemo, useState } from "react";
import { Link, NavLink, Outlet, useLocation, useNavigate } from "react-router-dom";
import {
  ChevronsUpDown, CircleUser, FileText, Files, FolderGit2, Info, KeyRound,
  LayoutDashboard, LogOut, Package, Radio, ScanSearch, ShieldCheck, Users, Workflow,
  type LucideIcon,
} from "lucide-react";
import { ApiError, projectsApi, reviewApi } from "@/lib/api";
import { cn } from "@/lib/cn";
import type { ProjectStatusView, ProjectView, ReviewGateView } from "@/lib/types";
import { useAuth } from "./auth";
import { ProjectContext, projectIdFromPath } from "./project-context";
import {
  DropdownContent, DropdownItem, DropdownLabel, DropdownMenu, DropdownSeparator, DropdownTrigger,
} from "@/components/ui/dropdown";
import { Tooltip } from "@/components/ui/tooltip";
import { Badge } from "@/components/ui/badge";
import { Alert } from "@/components/ui/alert";
import { CheckpointBadge } from "@/components/domain/badges";
import { History } from "lucide-react";

interface NavDef {
  to: string;
  label: string;
  icon: LucideIcon;
  end?: boolean;
  badge?: React.ReactNode;
  disabled?: boolean;
  disabledReason?: string;
  testid: string;
}

export function AppShell() {
  const { pathname } = useLocation();
  const projectId = projectIdFromPath(pathname);
  const { session, logout } = useAuth();
  const navigate = useNavigate();

  const [project, setProject] = useState<ProjectView | null>(null);
  const [status, setStatus] = useState<ProjectStatusView | null>(null);
  const [gate, setGate] = useState<ReviewGateView | null>(null);
  const [error, setError] = useState<ApiError | null>(null);
  const [loading, setLoading] = useState(false);
  const [nonce, setNonce] = useState(0);
  const reload = useCallback(() => setNonce((n) => n + 1), []);

  useEffect(() => {
    if (!projectId) {
      setProject(null); setStatus(null); setGate(null); setError(null);
      return;
    }
    let live = true;
    setLoading(true);
    Promise.all([projectsApi.get(projectId), projectsApi.status(projectId)])
      .then(([p, s]) => { if (live) { setProject(p); setStatus(s); setError(null); } })
      .catch((e) => { if (live) setError(e instanceof ApiError ? e : null); })
      .finally(() => { if (live) setLoading(false); });
    // Gate is best-effort (unavailable before integration).
    reviewApi.gate(projectId).then((g) => { if (live) setGate(g); }).catch(() => { if (live) setGate(null); });
    return () => { live = false; };
  }, [projectId, nonce]);

  const ctx = useMemo(
    () => ({ projectId, project, status, error, loading, reload }),
    [projectId, project, status, error, loading, reload],
  );

  const compiledReady = status?.checkpoint === "ready_to_compile" || status?.checkpoint === "verified";
  const blockingCount = gate?.blocking_items.length ?? 0;

  const navItems: NavDef[] = projectId
    ? [
        { to: `/projects/${projectId}`, label: "Overview", icon: LayoutDashboard, end: true, testid: "nav-overview" },
        {
          to: `/projects/${projectId}/sources`, label: "Sources", icon: Files, testid: "nav-sources",
          badge: status ? <span className="tabular text-[12px] text-faint">{status.source_count}/10</span> : undefined,
        },
        { to: `/projects/${projectId}/tokens`, label: "Upload Tokens", icon: KeyRound, testid: "nav-tokens" },
        { to: `/projects/${projectId}/integration`, label: "Integration", icon: Workflow, testid: "nav-integration" },
        {
          to: `/projects/${projectId}/review`, label: "Review", icon: ScanSearch, testid: "nav-review",
          badge: blockingCount > 0
            ? <Badge tone="danger" className="px-1">⛔{blockingCount}</Badge>
            : gate ? <Badge tone="ok" className="px-1">✓</Badge> : undefined,
        },
        {
          to: `/projects/${projectId}/compiled`, label: "Compiled Vault", icon: Package, testid: "nav-compiled",
          disabled: !compiledReady, disabledReason: "컴파일 승인 완료 후 활성화됩니다",
        },
        { to: `/projects/${projectId}/serving`, label: "Serving", icon: Radio, testid: "nav-serving" },
      ]
    : [{ to: "/projects", label: "Projects", icon: FolderGit2, end: true, testid: "nav-projects" }];

  return (
    <ProjectContext.Provider value={ctx}>
      <div className="flex min-h-screen bg-app">
        {/* Sidebar */}
        <aside className="fixed inset-y-0 left-0 flex w-60 flex-col border-r border-line bg-surface">
          <div className="flex h-14 items-center gap-2 border-b border-line px-4">
            <FolderGit2 className="size-4 text-accent" />
            <Link to="/projects" className="font-semibold tracking-tight">okc-web</Link>
          </div>

          {projectId && project && (
            <div className="border-b border-line px-3 py-2.5">
              <Link
                to="/projects"
                className="flex items-center justify-between rounded-lg border border-line px-2.5 py-1.5 text-sm hover:bg-subtle"
                data-testid="project-switcher"
              >
                <span className="truncate font-medium">{project.name}</span>
                <ChevronsUpDown className="size-3.5 shrink-0 text-faint" />
              </Link>
            </div>
          )}

          <nav className="flex-1 space-y-0.5 overflow-y-auto p-2">
            {navItems.map((item) => <NavItem key={item.to} {...item} />)}
          </nav>

          {/* Account menu */}
          <div className="border-t border-line p-2">
            <DropdownMenu>
              <DropdownTrigger asChild>
                <button
                  className="flex w-full items-center gap-2 rounded-lg px-2 py-2 text-left hover:bg-subtle"
                  data-testid="account-menu-trigger"
                >
                  <CircleUser className="size-6 text-faint" />
                  <div className="min-w-0 flex-1">
                    <div className="truncate text-[13px] font-medium">{session?.display_name}</div>
                    <div className="truncate text-[11px] text-faint">{session?.email}</div>
                  </div>
                  <ShieldCheck className="size-3.5 text-accent" />
                </button>
              </DropdownTrigger>
              <DropdownContent align="start" className="w-56">
                <DropdownLabel>{session?.curator_label}</DropdownLabel>
                <div className="flex items-start gap-1.5 px-2 pb-1.5 text-[11px] text-faint">
                  <Info className="mt-0.5 size-3 shrink-0" />
                  <span>이 계정 ID(<code className="font-mono">curator_id</code>)가 통합 감사에 기록됩니다. okc-core는 이 라벨을 검증하지 않습니다.</span>
                </div>
                <DropdownSeparator />
                <DropdownItem onSelect={() => navigate("/settings/users")}>
                  <Users className="size-4" /> 사용자·역할 관리
                </DropdownItem>
                <DropdownItem onSelect={() => navigate("/settings/account")}>
                  <CircleUser className="size-4" /> 내 계정
                </DropdownItem>
                <DropdownSeparator />
                <DropdownItem destructive onSelect={() => void logout()} data-testid="logout">
                  <LogOut className="size-4" /> 로그아웃
                </DropdownItem>
              </DropdownContent>
            </DropdownMenu>
          </div>
        </aside>

        {/* Main column */}
        <div className="flex min-h-screen flex-1 flex-col pl-60">
          <header className="sticky top-0 z-30 flex h-14 items-center justify-between border-b border-line bg-surface/90 px-6 backdrop-blur">
            <Breadcrumb pathname={pathname} projectName={project?.name} />
            {status && (
              <button onClick={() => navigate(`/projects/${projectId}/integration`)} data-testid="run-state-pill">
                <CheckpointBadge checkpoint={status.checkpoint} stale={status.stale} />
              </button>
            )}
          </header>

          {status?.stale && (
            <div className="border-b border-warn-line bg-warn-soft/60 px-6 py-2">
              <Alert variant="warning" icon={<History className="size-4" />} className="border-0 bg-transparent p-0">
                소스가 변경되어 이전 승인이 무효화되었습니다. 다시 통합을 실행하세요.
              </Alert>
            </div>
          )}

          <main className="flex-1 p-6">
            <Outlet />
          </main>
        </div>
      </div>
    </ProjectContext.Provider>
  );
}

function NavItem({ to, label, icon: Icon, end, badge, disabled, disabledReason, testid }: NavDef) {
  if (disabled) {
    return (
      <Tooltip content={disabledReason}>
        <div
          className="flex cursor-not-allowed items-center gap-2.5 rounded-lg px-2.5 py-2 text-sm text-faint opacity-60"
          data-testid={testid}
        >
          <Icon className="size-4" />
          <span className="flex-1">{label}</span>
        </div>
      </Tooltip>
    );
  }
  return (
    <NavLink
      to={to}
      end={end}
      data-testid={testid}
      className={({ isActive }) =>
        cn(
          "relative flex items-center gap-2.5 rounded-lg px-2.5 py-2 text-sm text-muted transition-colors hover:bg-subtle hover:text-fg",
          isActive && "bg-subtle font-medium text-fg before:absolute before:-left-2 before:top-1.5 before:bottom-1.5 before:w-0.5 before:rounded-full before:bg-accent",
        )
      }
    >
      <Icon className="size-4" />
      <span className="flex-1">{label}</span>
      {badge}
    </NavLink>
  );
}

const SECTION_LABELS: Record<string, string> = {
  sources: "Sources", tokens: "Upload Tokens", integration: "Integration",
  review: "Review", taxonomy: "Taxonomy", clusters: "Cluster", regenerate: "Regenerate",
  compiled: "Compiled Vault", serving: "Serving", verify: "Provenance & Verify",
  contract: "okc-mcp 계약", settings: "설정", users: "사용자·역할", account: "내 계정", projects: "Projects",
};

function Breadcrumb({ pathname, projectName }: { pathname: string; projectName?: string }) {
  const parts = pathname.split("/").filter(Boolean);
  const crumbs: { label: string; icon?: LucideIcon }[] = [];
  if (parts[0] === "projects" && parts[1] && parts[1] !== "new") {
    crumbs.push({ label: projectName ?? "프로젝트", icon: FileText });
    if (parts[2]) crumbs.push({ label: SECTION_LABELS[parts[2]] ?? parts[2] });
    if (parts[3] === "taxonomy" || parts[3] === "clusters") crumbs.push({ label: SECTION_LABELS[parts[3]] ?? parts[3] });
    if (parts[2] === "serving" && parts[3]) crumbs.push({ label: SECTION_LABELS[parts[3]] ?? parts[3] });
  } else if (parts[0] === "projects") {
    crumbs.push({ label: "Projects" });
  } else if (parts[0] === "settings") {
    crumbs.push({ label: SECTION_LABELS[parts[1]] ?? "설정" });
  }
  return (
    <div className="flex items-center gap-1.5 text-sm text-muted">
      {crumbs.map((c, i) => (
        <span key={i} className="flex items-center gap-1.5">
          {i > 0 && <span className="text-faint">▸</span>}
          <span className={cn(i === crumbs.length - 1 && "font-medium text-fg")}>{c.label}</span>
        </span>
      ))}
    </div>
  );
}
