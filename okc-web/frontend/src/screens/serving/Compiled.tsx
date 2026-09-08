import { useEffect, useMemo, useState } from "react";
import { useNavigate, useParams } from "react-router-dom";
import Markdown from "react-markdown";
import {
  ArrowRight, Ban, FileText, FolderTree, Hammer, Package, PackageCheck, ScanSearch, ShieldCheck, Waypoints,
} from "lucide-react";
import { ApiError, projectsApi, servingApi } from "@/lib/api";
import { cn } from "@/lib/cn";
import { useAsync } from "@/lib/hooks";
import { useProjectContext } from "@/app/project-context";
import { shortHash } from "@/lib/format";
import type { CompileResultView } from "@/lib/types";
import { PageHeader, ErrorState } from "@/components/ui/page";
import { Card } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Skeleton } from "@/components/ui/skeleton";
import { EmptyState } from "@/components/ui/empty-state";
import { Metric, MetricGrid } from "@/components/domain/metric";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { ConfirmDialog } from "@/components/ui/dialog";
import { Callout } from "@/components/domain/callout";
import { CopyButton } from "@/components/ui/copy-button";
import { toast } from "@/components/ui/toaster";

// E5-1 Compiled Vault — the SINGLE compile trigger + read-only 3-root browser.
export function Compiled() {
  const { id = "" } = useParams();
  const navigate = useNavigate();
  const { status, reload } = useProjectContext();
  const [confirmOpen, setConfirmOpen] = useState(false);
  const [compiling, setCompiling] = useState(false);
  const [result, setResult] = useState<CompileResultView | null>(null);
  const [err, setErr] = useState<ApiError | null>(null);

  const checkpoint = status?.checkpoint;
  const ready = checkpoint === "ready_to_compile";
  const compiled = checkpoint === "verified" || !!result;

  const doCompile = async () => {
    setCompiling(true); setErr(null);
    try {
      const r = await projectsApi.compile(id);
      setResult(r);
      toast.success(`컴파일 완료 — ${r.file_count} files`);
      setConfirmOpen(false);
      reload();
    } catch (e) {
      setErr(e instanceof ApiError ? e : null);
    } finally {
      setCompiling(false);
    }
  };

  if (!status) return <Skeleton className="h-64 w-full" />;

  if (!ready && !compiled) {
    return (
      <div>
        <PageHeader title="Compiled Vault" />
        <EmptyState icon={Ban} title="아직 컴파일할 수 없습니다"
          description="리뷰(E4)에서 모든 클러스터를 승인하고 차단 findings를 0으로 만든 뒤 컴파일하세요."
          action={<Button variant="primary" onClick={() => navigate(`/projects/${id}/review`)}><ScanSearch className="size-4" /> 리뷰로 이동</Button>} />
      </div>
    );
  }

  return (
    <div>
      <PageHeader title="Compiled Vault" description="결정론적 · offline · Markdown 전용 · no-clobber 병합. 읽기 전용." />

      {ready && !compiled && (
        <Card className="mb-5 p-5">
          <div className="flex items-center justify-between">
            <div className="flex items-center gap-2 text-sm"><ShieldCheck className="size-5 text-ok" /> 승인 완료 · 차단 0 — 컴파일 준비됨</div>
            <Button variant="primary" onClick={() => setConfirmOpen(true)} data-testid="compile-run"><Hammer className="size-4" /> 컴파일 실행</Button>
          </div>
          {err && <div className="mt-3"><ErrorState error={err} /></div>}
        </Card>
      )}

      {compiled && <CompiledBrowser projectId={id} result={result} navigate={navigate} />}

      <ConfirmDialog
        open={confirmOpen} onOpenChange={setConfirmOpen}
        title="컴파일 실행" confirmLabel="실행" loading={compiling} onConfirm={doCompile} testid="compile-confirm"
        description="병합은 결정론적이며 offline·Markdown 전용입니다. 기존 산출물은 덮어쓰지 않습니다(no-clobber)."
      >
        <Callout icon={<PackageCheck className="size-4" />}>knowledge/ · legacy/ · .okc/ 3영역으로 병합됩니다.</Callout>
      </ConfirmDialog>
    </div>
  );
}

function CompiledBrowser({ projectId, result, navigate }: { projectId: string; result: CompileResultView | null; navigate: (p: string) => void }) {
  const files = useAsync(() => servingApi.files(projectId), [projectId]);
  const verify = useAsync(() => servingApi.verify(projectId), [projectId]);
  const [selected, setSelected] = useState<string | null>(null);

  const notServed = files.error && (files.error.code === "NOT_FOUND" || files.error.httpStatus === 404);

  return (
    <>
      <MetricGrid className="mb-5 lg:grid-cols-4">
        <Metric label="노트 수" value={files.data?.files.length ?? result?.file_count ?? "—"} />
        <Metric label="verify" value={verify.data ? (verify.data.valid ? "PASS" : "FAIL") : "—"} tone={verify.data?.valid ? "ok" : verify.data ? "danger" : undefined} />
        <Metric label="plan" value={<span className="font-mono text-sm">{shortHash(files.data?.bound_integration_plan_id ?? result?.integration_plan_id, 6, 4)}</span>} />
        <Metric label="출력 경로" value={result?.path ? <CopyButton value={result.path} label="복사" mono={false} /> : "—"} />
      </MetricGrid>

      {notServed ? (
        <Callout icon={<Package className="size-4" />}>
          컴파일 산출물은 생성되었지만 아직 서빙되지 않았습니다. 병합 트리를 브라우징하고 provenance를 검증하려면 서빙에서 <b>publish</b> 하세요.
          <div className="mt-2"><Button size="sm" variant="secondary" onClick={() => navigate(`/projects/${projectId}/serving`)}>서빙으로 이동 <ArrowRight className="size-3.5" /></Button></div>
        </Callout>
      ) : files.loading ? (
        <Skeleton className="h-80 w-full" />
      ) : files.error ? (
        <ErrorState error={files.error} onRetry={files.reload} />
      ) : (
        <div className="grid gap-4 lg:grid-cols-[260px_1fr_220px]">
          <Card className="p-2"><FileTree files={files.data?.files ?? []} selected={selected} onSelect={setSelected} /></Card>
          <NoteViewer projectId={projectId} path={selected} />
          <Card className="p-3">
            <div className="mb-2 text-[12px] font-medium text-faint">노트 메타</div>
            {selected ? (
              <Button size="sm" variant="secondary" className="w-full justify-center"
                onClick={() => navigate(`/projects/${projectId}/serving/verify?note=${encodeURIComponent(selected)}`)} data-testid="to-provenance">
                <Waypoints className="size-4" /> Provenance & Verify
              </Button>
            ) : <div className="text-[13px] text-muted">노트를 선택하세요.</div>}
          </Card>
        </div>
      )}
    </>
  );
}

function FileTree({ files, selected, onSelect }: { files: string[]; selected: string | null; onSelect: (p: string) => void }) {
  const groups = useMemo(() => {
    const g: Record<string, string[]> = {};
    for (const f of files) {
      const root = f.split("/")[0] + "/";
      (g[root] ??= []).push(f);
    }
    return g;
  }, [files]);
  return (
    <div className="space-y-2" data-testid="file-tree">
      {Object.entries(groups).map(([root, list]) => (
        <div key={root}>
          <div className="flex items-center gap-1.5 px-1 py-1 text-[12px] font-medium text-muted"><FolderTree className="size-3.5" /> {root}</div>
          {list.map((f) => (
            <button key={f} onClick={() => onSelect(f)}
              className={cn("flex w-full items-center gap-1.5 truncate rounded-md px-2 py-1 text-left text-[13px] hover:bg-subtle", f === selected && "bg-subtle font-medium")}>
              <FileText className="size-3.5 shrink-0 text-faint" /> <span className="truncate">{f.split("/").slice(1).join("/")}</span>
            </button>
          ))}
        </div>
      ))}
    </div>
  );
}

function NoteViewer({ projectId, path }: { projectId: string; path: string | null }) {
  const [text, setText] = useState<string>("");
  const [loading, setLoading] = useState(false);
  useEffect(() => {
    if (!path) { setText(""); return; }
    let live = true;
    setLoading(true);
    servingApi.fileText(projectId, path).then((t) => { if (live) setText(t); }).catch(() => { if (live) setText("_불러올 수 없습니다._"); }).finally(() => { if (live) setLoading(false); });
    return () => { live = false; };
  }, [projectId, path]);

  if (!path) return <Card className="flex items-center justify-center p-10 text-sm text-muted">노트를 선택하세요.</Card>;
  return (
    <Card className="p-0">
      <div className="border-b border-line px-4 py-2 font-mono text-[12px] text-muted">{path}</div>
      <Tabs defaultValue="rendered">
        <TabsList className="px-3 pt-1">
          <TabsTrigger value="rendered">렌더링</TabsTrigger>
          <TabsTrigger value="raw">Raw</TabsTrigger>
        </TabsList>
        <TabsContent value="rendered" className="p-5">
          {loading ? <Skeleton className="h-40 w-full" /> : (
            <article className="max-w-none space-y-2 text-sm leading-6 [&_h1]:text-lg [&_h1]:font-semibold [&_h2]:mt-4 [&_h2]:font-semibold [&_code]:font-mono [&_ul]:list-disc [&_ul]:pl-5">
              <Markdown>{text}</Markdown>
            </article>
          )}
        </TabsContent>
        <TabsContent value="raw" className="p-5">
          <pre className="max-h-96 overflow-auto rounded-lg border border-line bg-[var(--slate-1)] p-3 font-mono text-[12px]">{text}</pre>
        </TabsContent>
      </Tabs>
    </Card>
  );
}
