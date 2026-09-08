import { useLocation, useNavigate, useParams } from "react-router-dom";
import { Fingerprint, Hash, Layers, PackageCheck, TriangleAlert } from "lucide-react";
import type { IngestAccepted } from "@/lib/types";
import { shortHash } from "@/lib/format";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { CopyButton } from "@/components/ui/copy-button";
import { Callout } from "@/components/domain/callout";
import { SlotBar } from "@/components/domain/category-bar";
import { UploadShell } from "./UploadShell";

// E2-5 upload complete & source registration status.
export function UploadDone() {
  const { token = "" } = useParams();
  const navigate = useNavigate();
  const accepted = (useLocation().state as { accepted?: IngestAccepted } | null)?.accepted ?? null;

  return (
    <UploadShell>
      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2"><PackageCheck className="size-5 text-ok" /> 업로드 완료</CardTitle>
        </CardHeader>
        <CardContent className="space-y-4">
          {!accepted ? (
            <Callout>등록 정보를 찾을 수 없습니다. 업로드 화면에서 다시 시도하세요.</Callout>
          ) : (
            <>
              <dl className="grid grid-cols-[auto_1fr] gap-x-6 gap-y-2.5 text-sm">
                <dt className="text-muted">SourceId</dt>
                <dd className="font-mono">{accepted.source_id}</dd>
                <dt className="text-muted flex items-center gap-1"><Fingerprint className="size-3.5" /> content hash</dt>
                <dd><CopyButton value={accepted.content_hash} label={shortHash(accepted.content_hash)} /></dd>
                {accepted.owner_display_name && (<><dt className="text-muted">owner</dt><dd>{accepted.owner_display_name}</dd></>)}
                <dt className="text-muted flex items-center gap-1"><Layers className="size-3.5" /> 슬롯</dt>
                <dd><SlotBar used={accepted.slot_index} limit={10} /></dd>
              </dl>

              {accepted.warnings.length > 0 && (
                <div className="space-y-1.5">
                  {accepted.warnings.map((w, i) => (
                    <Callout key={i} tone="warn" icon={<TriangleAlert className="size-4" />} title={w.code ?? w.name}>
                      {w.detail ?? w.name}
                    </Callout>
                  ))}
                </div>
              )}

              <Callout icon={<Hash className="size-4" />}>
                통합 대기 — 관리자가 소스를 freeze한 뒤 통합을 실행합니다. 등록은 소스 추가일 뿐, 병합(컴파일)은 관리자가 수행합니다.
              </Callout>
            </>
          )}
          <Button variant="secondary" onClick={() => navigate(`/upload/${token}`)} className="w-full justify-center">
            포털로 돌아가기
          </Button>
        </CardContent>
      </Card>
    </UploadShell>
  );
}
