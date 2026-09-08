import { useRef, useState } from "react";
import { useNavigate, useParams } from "react-router-dom";
import {
  FileArchive, Gauge, Loader2, ShieldAlert, TriangleAlert, Upload, XCircle,
} from "lucide-react";
import { ApiError, uploadApi } from "@/lib/api";
import { cn } from "@/lib/cn";
import type { IngestAccepted } from "@/lib/types";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Input, Label } from "@/components/ui/input";
import { Alert } from "@/components/ui/alert";
import { ProgressBar } from "@/components/domain/progress";
import { UploadShell } from "./UploadShell";

// E2-4 upload + validation. E2 "mini-polish": honest hostile-input defenses.
export function UploadPage() {
  const { token = "" } = useParams();
  const navigate = useNavigate();
  const fileRef = useRef<HTMLInputElement>(null);
  const [file, setFile] = useState<File | null>(null);
  const [ownerName, setOwnerName] = useState("");
  const [ownerKind, setOwnerKind] = useState("individual");
  const [dragging, setDragging] = useState(false);
  const [phase, setPhase] = useState<"idle" | "uploading">("idle");
  const [err, setErr] = useState<ApiError | null>(null);

  const submit = async () => {
    if (!file) return;
    setPhase("uploading"); setErr(null);
    try {
      const res: IngestAccepted = await uploadApi.upload(token, file, ownerName || undefined, ownerKind);
      navigate(`/upload/${token}/done`, { state: { accepted: res } });
    } catch (e) {
      setErr(e instanceof ApiError ? e : null);
      setPhase("idle");
    }
  };

  return (
    <UploadShell>
      <Card>
        <CardHeader>
          <CardTitle>Vault 업로드 & 검증</CardTitle>
        </CardHeader>
        <CardContent className="space-y-4">
          <div
            onDragOver={(e) => { e.preventDefault(); setDragging(true); }}
            onDragLeave={() => setDragging(false)}
            onDrop={(e) => { e.preventDefault(); setDragging(false); if (e.dataTransfer.files[0]) setFile(e.dataTransfer.files[0]); }}
            onClick={() => fileRef.current?.click()}
            className={cn(
              "flex cursor-pointer flex-col items-center gap-2 rounded-xl border-2 border-dashed p-8 text-center transition-colors",
              dragging ? "border-accent bg-accent-soft/40" : "border-line hover:border-line-hover",
            )}
            data-testid="dropzone"
          >
            <FileArchive className="size-7 text-faint" />
            {file ? (
              <div className="text-sm">
                <div className="font-medium">{file.name}</div>
                <div className="text-muted tabular">{(file.size / 1024).toFixed(0)} KB</div>
              </div>
            ) : (
              <div className="text-sm text-muted">zip / tar.zst 파일을 드래그하거나 클릭해 선택</div>
            )}
            <input ref={fileRef} type="file" accept=".zip,.zst,.tar,.tar.zst" className="hidden"
              onChange={(e) => setFile(e.target.files?.[0] ?? null)} data-testid="file-input" />
          </div>

          <div className="grid gap-3 sm:grid-cols-2">
            <div className="space-y-1.5">
              <Label htmlFor="owner">owner_display_name (장식 라벨)</Label>
              <Input id="owner" value={ownerName} onChange={(e) => setOwnerName(e.target.value)}
                placeholder="예: 데이터팀 / 홍길동" data-testid="owner-name" />
            </div>
            <div className="space-y-1.5">
              <Label>구분</Label>
              <div className="flex gap-2">
                {(["individual", "department"] as const).map((k) => (
                  <button key={k} type="button" onClick={() => setOwnerKind(k)}
                    className={cn("flex-1 rounded-lg border px-3 py-2 text-sm",
                      ownerKind === k ? "border-accent-line bg-accent-soft text-accent-text" : "border-line")}>
                    {k === "individual" ? "개인" : "부서"}
                  </button>
                ))}
              </div>
            </div>
          </div>

          <Alert variant="info" icon={<ShieldAlert className="size-4" />}>
            검증: 포맷 · 크기 상한 · Markdown 비율 · 경로 안전성(../ 거부) · 심볼릭 링크 거부 · 중복(content hash). 서버에서 실행됩니다.
          </Alert>

          {phase === "uploading" && (
            <div className="space-y-2">
              <div className="flex items-center gap-2 text-sm text-muted"><Loader2 className="size-4 animate-spin" /> 업로드 및 검증 중…</div>
              <ProgressBar value={1} />
            </div>
          )}

          {err && <UploadError err={err} />}

          <Button variant="primary" className="w-full justify-center" disabled={!file}
            loading={phase === "uploading"} onClick={submit} data-testid="upload-submit">
            <Upload className="size-4" /> 업로드
          </Button>
        </CardContent>
      </Card>
    </UploadShell>
  );
}

function UploadError({ err }: { err: ApiError }) {
  const violation = err.code === "PATH_UNSAFE" || err.code === "SYMLINK_REJECTED";
  const icon =
    err.code === "UPLOAD_TOO_LARGE" ? <Gauge className="size-4" /> :
    violation ? <ShieldAlert className="size-4" /> :
    err.isBusy ? <Loader2 className="size-4 animate-spin" /> :
    err.code === "DUPLICATE_SOURCE" ? <TriangleAlert className="size-4" /> :
    <XCircle className="size-4" />;
  return (
    <Alert variant={err.isBusy ? "warning" : "destructive"} icon={icon} title={err.code}>
      {err.message}
    </Alert>
  );
}
