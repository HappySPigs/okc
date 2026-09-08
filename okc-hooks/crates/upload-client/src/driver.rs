//! UploadProtocolDriver — 다단계 업로드 프로토콜 상태머신(트리거당 단일 직렬 사이클, Q2=B).
//!
//! `execute_cycle` 오케스트레이션(R-CYCLE-01): (1) 런타임 SafetyLimits 재검사 -> (2) no-op digest
//! 조기 종료 -> (3) have/want 협상 -> (4) want blob 순차 전송(각 blob = 전송 전 재검증 -> Send) ->
//! (5) 경로->해시 맵 + manifest_digest 커밋 -> (6) Done. 실패는 `UploadError` 로 즉시 반환하며
//! 백오프/재시도(sleep)는 하지 않는다(drive-not-sleep, D-16 — U4 소관).
//!
//! `CyclePhase` 상태 전이를 명시 열거해 PBT(PROP-U3-08)가 합법 전이만 밟음을 검증한다. transport/
//! blob-source/store 는 seam 으로 주입되어 테스트가 네트워크/파일시스템/권한 없이 구동한다.
//!
//! transport(AuthTransport)/blob-source/store IO 를 seam 을 통해 위임하는 오케스트레이션 모듈이므로
//! 순수 lint-gate 를 두지 않는다(단, 패닉 경로 없음 — unwrap/expect/panic/인덱싱 회피).

use std::io::{self, Read};
use std::sync::Arc;

use auth_consent::{AuthTransport, Body, Headers, HttpMethod, OkcRequest};
use content_core::{ContentAddressing, LimitVerdict, LimitViolation, SafetyLimitsValidator};
use foundation::{
    ActiveCondition, ByteCount, LimitReport, Manifest, ManifestEntry, Sha256Digest, StatusSink,
    decode, encode,
};

use crate::blob_source::BlobSource;
use crate::error::UploadError;
use crate::protocol::{
    BLOB_PATH, COMMIT_PATH, ChunkFrame, CommitOutcome, CommitRequest, NEGOTIATE_PATH,
    NegotiateRequest, NegotiateResponse, WantSet, compute_want, manifest_hashes, negotiate_entries,
    path_hash_map,
};
use crate::store::SyncStore;

/// 프로토콜 상태머신의 관측 가능한 진행 단계(사이클-로컬, 지속되지 않음, D-01).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CyclePhase {
    /// digest 동일 -> 조기 종료(FR-10).
    NoOp,
    /// 매니페스트 제시 -> WantSet 수신.
    Negotiate,
    /// want blob 순차 전송 진입.
    Transfer,
    /// 전송 직전 재-읽기 + 재-해시 가드(Q8=A).
    Reverify,
    /// 경로->해시 맵 + manifest_digest 전송.
    Commit,
    /// 커밋 성공(committed 또는 서버 no-op).
    Done,
    /// HashMismatch/OverLimit/Transport -> 커밋 미발행.
    Abort,
}

/// 한 사이클의 실행 결과 + 관측된 상태 전이 추적(PBT/예제 테스트 구동용).
#[derive(Debug)]
pub struct CycleReport {
    /// 사이클 결과(taxonomy 보존).
    pub outcome: Result<CommitOutcome, UploadError>,
    /// 밟은 `CyclePhase` 전이 순서(합법성 검증 대상, PROP-U3-08).
    pub phases: Vec<CyclePhase>,
}

/// 업로드 프로토콜 드라이버 — 생성 시 협력자(seam)와 config 값을 주입받는다.
pub struct UploadProtocolDriver {
    transport: AuthTransport,
    blobs: Box<dyn BlobSource>,
    store: Box<dyn SyncStore>,
    status: Arc<dyn StatusSink>,
    chunk_threshold_bytes: u64,
    chunk_size_bytes: u64,
}

impl UploadProtocolDriver {
    /// 주입 협력자(유일 HTTP 경로 `AuthTransport` + `BlobSource` + `SyncStore` + `StatusSink`)와
    /// 임계값 `S`(`chunk_threshold_bytes`) · 고정 `chunk_size`(`chunk_size_bytes`)로 구성한다(D-10).
    pub fn new(
        transport: AuthTransport,
        blobs: Box<dyn BlobSource>,
        store: Box<dyn SyncStore>,
        status: Arc<dyn StatusSink>,
        chunk_threshold_bytes: u64,
        chunk_size_bytes: u64,
    ) -> Self {
        UploadProtocolDriver {
            transport,
            blobs,
            store,
            status,
            chunk_threshold_bytes,
            chunk_size_bytes,
        }
    }

    /// 한 사이클을 실행한다(R-CYCLE-01). 성공 시 `CommitOutcome`, 실패 시 `UploadError` 반환.
    pub fn execute_cycle(&mut self, manifest: &Manifest) -> Result<CommitOutcome, UploadError> {
        self.execute_cycle_traced(manifest).outcome
    }

    /// `execute_cycle` 과 동일하되 밟은 `CyclePhase` 전이 추적을 함께 반환한다(테스트/관측용).
    pub fn execute_cycle_traced(&mut self, manifest: &Manifest) -> CycleReport {
        let mut phases = Vec::new();
        let outcome = self.run(manifest, &mut phases);
        // 실패 경로는 커밋을 발행하지 않으므로 종료 상태를 `Abort` 로 표기한다(OverLimit 는 run 이
        // 이미 push 하므로 중복하지 않는다).
        if outcome.is_err() && phases.last() != Some(&CyclePhase::Abort) {
            phases.push(CyclePhase::Abort);
        }
        CycleReport { outcome, phases }
    }

    /// 고정 단계 순서 오케스트레이션(R-CYCLE-01). 어느 단계가 halt/abort 하면 이후 단계는 미실행.
    fn run(
        &mut self,
        manifest: &Manifest,
        phases: &mut Vec<CyclePhase>,
    ) -> Result<CommitOutcome, UploadError> {
        // (1) 런타임 SafetyLimits 재검사(R-LIMIT-RT-01).
        match SafetyLimitsValidator::validate(manifest) {
            LimitVerdict::Exceeded(violation) => {
                // raise/clear 는 멱등(set 의미)이며 best-effort·infallible(R-LIMIT-RT-02/D-12).
                self.status.raise_condition(ActiveCondition::OverLimit);
                phases.push(CyclePhase::Abort);
                return Err(UploadError::OverLimit(LimitReport(describe_violation(
                    &violation,
                ))));
            }
            LimitVerdict::WithinLimits => {
                // 이전 사이클 over-limit 였다면 자동 해제 -> 자동 재개(R-LIMIT-RT-03).
                self.status.clear_condition(ActiveCondition::OverLimit);
            }
        }

        // (2) no-op digest 조기 종료(R-NOOP-01). 마지막 커밋 존재 + digest 동일 시 서버 왕복 없음.
        if let Some(last) = self.store.last_committed_manifest()
            && manifest.manifest_digest == last.manifest_digest {
                phases.push(CyclePhase::NoOp);
                phases.push(CyclePhase::Done);
                return Ok(CommitOutcome {
                    server_vault_content_id: None,
                    committed: false,
                });
            }

        // (3) Negotiate — 매니페스트 제시 -> WantSet.
        phases.push(CyclePhase::Negotiate);
        let want = self.negotiate(manifest)?;

        // (4) Transfer — want blob 순차 전송(각 blob = Reverify -> Send).
        self.transfer_wanted(manifest, &want, phases)?;

        // (5) Commit — 경로->해시 맵 + manifest_digest.
        phases.push(CyclePhase::Commit);
        let outcome = self.commit(manifest)?;

        // 커밋 성공 -> 마지막 커밋 갱신 + dirty clear + 재개 오프셋 clear(원자, R-RESUME-03).
        self.store.commit_manifest(manifest.clone())?;

        // (6) Done.
        phases.push(CyclePhase::Done);
        Ok(outcome)
    }

    /// have/want 협상(R-WANT-*). 순수 차집합 `compute_want` 로 want 를 로컬 계산한다(D-14).
    fn negotiate(&self, manifest: &Manifest) -> Result<WantSet, UploadError> {
        let request = NegotiateRequest {
            manifest_digest: manifest.manifest_digest,
            entries: negotiate_entries(manifest),
        };
        let body = encode(&request).map_err(|_| UploadError::Aborted)?;
        let response = self
            .transport
            .send(OkcRequest {
                method: HttpMethod::Post,
                path: NEGOTIATE_PATH.to_string(),
                headers: Headers::new(),
                body: Body::from_bytes(body),
            })
            .map_err(UploadError::Transport)?;
        let negotiate: NegotiateResponse =
            decode(response.body.as_bytes()).map_err(|_| UploadError::Aborted)?;
        Ok(compute_want(&manifest_hashes(manifest), &negotiate.server_has))
    }

    /// want blob 을 경로 오름차순 canonical 순서로 순차 전송한다(R-WANT-03: 서버 보유분 미전송).
    ///
    /// 다중 경로 동일 콘텐츠는 하나의 해시로 dedup 해 한 번만 전송한다(R-WANT-01).
    fn transfer_wanted(
        &mut self,
        manifest: &Manifest,
        want: &WantSet,
        phases: &mut Vec<CyclePhase>,
    ) -> Result<(), UploadError> {
        phases.push(CyclePhase::Transfer);
        let mut sent: std::collections::BTreeSet<Sha256Digest> = std::collections::BTreeSet::new();
        for entry in &manifest.entries {
            if !want.blobs.contains(&entry.raw_sha256) || sent.contains(&entry.raw_sha256) {
                continue;
            }
            phases.push(CyclePhase::Reverify);
            self.transfer_blob(entry)?;
            sent.insert(entry.raw_sha256);
        }
        Ok(())
    }

    /// 단일 blob 전송 — 전송 직전 재-읽기+재-해시 가드(R-REVERIFY-01) 후 단일/청크 전송한다.
    fn transfer_blob(&mut self, entry: &ManifestEntry) -> Result<(), UploadError> {
        // --- Reverify 패스: 재-읽기 + 스트리밍 재-해시(전량 적재 없음) ---
        let reader = self
            .blobs
            .open(&entry.relative_path)
            .map_err(|_| UploadError::Aborted)?;
        let actual = ContentAddressing::hash_stream(reader).map_err(|_| UploadError::Aborted)?;
        if actual != entry.raw_sha256 {
            // 열거~해시~전송 간 파일 변경(TOCTOU) -> 커밋 미발행, 다음 사이클 재스냅샷(R-REVERIFY-02).
            return Err(UploadError::HashMismatch {
                path: entry.relative_path.clone(),
                expected: entry.raw_sha256,
                actual,
            });
        }

        // --- 전송 패스(별도 재-읽기, R-REVERIFY-03) ---
        let resume = self
            .store
            .resume_offset(&entry.raw_sha256)
            .map(|offset| offset.get())
            .unwrap_or(0);
        // R-XFER-01: size <= S 이며 재개 오프셋이 없으면 단일 요청, 그 외(> S 또는 재개)는 청크.
        // MVP: 단일 요청 실패 시 청크 폴백은 도입하지 않는다(drive-not-sleep, U4 가 사이클 재시도).
        if resume == 0 && entry.size.get() <= self.chunk_threshold_bytes {
            self.send_single(entry)
        } else {
            self.send_chunked(entry, resume)
        }
    }

    /// 단일 요청 전송(`size <= S`) — blob 전량을 하나의 프레임으로 보낸다.
    fn send_single(&self, entry: &ManifestEntry) -> Result<(), UploadError> {
        let mut reader = self
            .blobs
            .open(&entry.relative_path)
            .map_err(|_| UploadError::Aborted)?;
        let mut bytes = Vec::new();
        reader
            .read_to_end(&mut bytes)
            .map_err(|_| UploadError::Aborted)?;
        let chunk_sha256 =
            ContentAddressing::hash_stream(bytes.as_slice()).map_err(|_| UploadError::Aborted)?;
        let len = bytes.len() as u64;
        let frame = ChunkFrame {
            blob: entry.raw_sha256,
            offset: ByteCount::new(0),
            len: ByteCount::new(len),
            chunk_sha256,
            bytes,
        };
        self.send_frame(&entry.raw_sha256, 0, &frame)
    }

    /// 재개 가능 청크 전송(`size > S` 또는 재개) — 고정 `chunk_size` 로 스트리밍 분할·전송한다.
    fn send_chunked(&mut self, entry: &ManifestEntry, resume: u64) -> Result<(), UploadError> {
        let blob = entry.raw_sha256;
        let total = entry.size.get();
        let step = self.chunk_size_bytes.max(1);
        let mut reader = self
            .blobs
            .open(&entry.relative_path)
            .map_err(|_| UploadError::Aborted)?;

        // 재개 구간 `[0, resume)` 을 건너뛴다(Read 는 seek 불가 -> 읽어 폐기; MVP). 재조립 동치는
        // 서버가 `[0, resume)` 을 이미 보유하는 것으로 성립한다(R-CHUNK-02).
        let mut offset = resume.min(total);
        if offset > 0 {
            let mut skip = (&mut *reader).take(offset);
            io::copy(&mut skip, &mut io::sink()).map_err(|_| UploadError::Aborted)?;
        }

        while offset < total {
            let len = step.min(total - offset);
            let mut buffer = vec![0u8; len as usize];
            reader
                .read_exact(&mut buffer)
                .map_err(|_| UploadError::Aborted)?;
            let chunk_sha256 = ContentAddressing::hash_stream(buffer.as_slice())
                .map_err(|_| UploadError::Aborted)?;
            let frame = ChunkFrame {
                blob,
                offset: ByteCount::new(offset),
                len: ByteCount::new(len),
                chunk_sha256,
                bytes: buffer,
            };
            self.send_frame(&blob, offset, &frame)?;
            offset += len;
            // ack 마다 진행 오프셋 지속(R-RESUME-02) — 크래시/중단 후 마지막 ack 부터 재개.
            self.store
                .persist_resume_offset(blob, ByteCount::new(offset))?;
            // 대용량/청크 전송에서만 진행률 오버레이 push(R-STATUS-01) — best-effort·infallible.
            if total > self.chunk_threshold_bytes {
                self.status
                    .set_resume_progress(ByteCount::new(offset), ByteCount::new(total));
            }
        }
        Ok(())
    }

    /// 한 청크 프레임을 유일 HTTP 경로(`AuthTransport::send`)로 전송한다(R-HTTP-01).
    fn send_frame(
        &self,
        blob: &Sha256Digest,
        offset: u64,
        frame: &ChunkFrame,
    ) -> Result<(), UploadError> {
        let body = encode(frame).map_err(|_| UploadError::Aborted)?;
        let path = format!("{BLOB_PATH}/{blob}/{offset}");
        self.transport
            .send(OkcRequest {
                method: HttpMethod::Put,
                path,
                headers: Headers::new(),
                body: Body::from_bytes(body),
            })
            .map_err(UploadError::Transport)?;
        Ok(())
    }

    /// 커밋(R-COMMIT-01) — 권위 경로->해시 맵 + manifest_digest 를 전송하고 결과를 해석한다.
    fn commit(&self, manifest: &Manifest) -> Result<CommitOutcome, UploadError> {
        let request = CommitRequest {
            path_hash_map: path_hash_map(manifest),
            manifest_digest: manifest.manifest_digest,
        };
        let body = encode(&request).map_err(|_| UploadError::Aborted)?;
        let response = self
            .transport
            .send(OkcRequest {
                method: HttpMethod::Post,
                path: COMMIT_PATH.to_string(),
                headers: Headers::new(),
                body: Body::from_bytes(body),
            })
            .map_err(UploadError::Transport)?;
        decode(response.body.as_bytes()).map_err(|_| UploadError::Aborted)
    }
}

/// `LimitViolation` 을 사람이 읽는 요약 문자열로 사상한다(U0 `LimitReport` 자리표시 채움).
fn describe_violation(violation: &LimitViolation) -> String {
    match violation {
        LimitViolation::TotalBytes { limit, actual } => {
            format!("총 볼트 크기 초과: actual={actual} > limit={limit} bytes")
        }
        LimitViolation::FileBytes {
            path,
            limit,
            actual,
        } => format!(
            "파일당 크기 초과: path={} actual={actual} > limit={limit} bytes",
            path.as_str()
        ),
        LimitViolation::FileCount { limit, actual } => {
            format!("파일 수 초과: actual={actual} > limit={limit}")
        }
    }
}
