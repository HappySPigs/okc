# U3 upload-client — Business Logic Model (알고리즘 / 워크플로 / 상태머신)

**단계**: CONSTRUCTION -> Per-Unit Loop -> **U3 Upload Protocol Client** -> Functional Design -> 산출물 3/3 (`business-logic-model.md`)
**작성일**: 2026-09-08
**크레이트**: `upload-client` (lib) · **소속 컴포넌트**: `UploadProtocolDriver`
**전제(확정 답변)**: Q2=B · Q6=C · Q8=A · FQ-1=A · FQ-2=A · DEP-03 [blocked-on-server]

> **문서 성격**: U3의 **알고리즘·워크플로·상태머신·데이터 흐름·교차 단위 경계**를 정의한다. 타입은 `domain-entities.md`, 규칙/불변식은 `business-rules.md`가 소유하며 이 문서는 그 rule-id(R-*)와 property-id(PROP-*)를 인용한다.
>
> **표기 규약**: 기술중립. 화살표는 ASCII(`A -> B`)만. 다이어그램은 ASCII 박스 없이 화살표·순서 목록·표로 기술. Rust 타입/제네릭은 백틱. Korean 산문.

---

## 1. `UploadProtocolDriver` — 톱레벨 워크플로

### 1.1 `execute_cycle` 오케스트레이션 (R-CYCLE-01)

입력: `snapshot: &VaultSnapshot`(재-읽기 경로 참조), `manifest: &Manifest`(당 사이클 재계산본). 협력자는 주입 필드(`AuthTransport`/`BlobSource`/`SyncStateStore`/`StatusSink`/S/chunk_size).

```
execute_cycle(snapshot, manifest):
  1. limits = SafetyLimitsValidator::validate(manifest)          # U1, R-LIMIT-RT-01
     if limits == Exceeded(v):
         StatusSink::raise_condition(OverLimit)                  # R-LIMIT-RT-02
         return Err(UploadError::OverLimit(report(v)))           # halt, last_committed 불변
     else:
         StatusSink::clear_condition(OverLimit)                  # R-LIMIT-RT-03 (자동 재개)

  2. if last_committed = SyncStateStore::last_committed_manifest():   # R-NOOP-01
         if manifest.manifest_digest == last_committed.manifest_digest:
             return Ok(CommitOutcome{ server_vault_content_id: None, committed: false })  # no-op 조기 종료

  3. want = negotiate(manifest)                                  # R-WANT-*, Negotiate 단계
         (오류 -> Err(UploadError::Transport(e)))

  4. transfer_wanted(snapshot, manifest, want)                   # Transfer 단계, blob 순차
         for each entry in manifest.entries whose raw_sha256 ∈ want.blobs:
             transfer_blob(entry, resume = SyncStateStore::resume_offset(&entry.raw_sha256))
         (HashMismatch -> Err; Transport -> Err)

  5. outcome = commit(manifest)                                  # R-COMMIT-*, Commit 단계
         (오류 -> Err(UploadError::Transport(e)))

  6. return Ok(outcome)                                          # Done
```

- 성공 시 `SyncStateStore::commit_manifest(manifest.clone())`로 마지막 커밋 갱신 + dirty clear + 재개 오프셋 clear를 **한 번의 원자 쓰기**로 반영한다(R-RESUME-03). 이 호출 시점 소유(U8 코디네이터 vs 드라이버)는 배선 관심사로, MVP는 드라이버가 커밋 성공 직후 호출한다.
- 반환 형상: `Result<CommitOutcome, UploadError>` — taxonomy 보존, sleep/retry 없음(R-ERR-01, D-16).

### 1.2 `CyclePhase` 상태머신 (PROP-U3-08)

상태 전이(모델; 가드는 R-* 규칙):

```
[start]
  -> (Exceeded)        Abort(OverLimit)                 # R-LIMIT-RT-02, 종료
  -> (digest 동일)      NoOp -> Done(committed=false)     # R-NOOP-01, 종료
  -> (그 외)            Negotiate
Negotiate
  -> (Ok want)         Transfer
  -> (TransportError)  Abort(Transport)                 # R-WANT-04
Transfer  (want blob 없으면 즉시 통과 -> Commit; R-WANT-02)
  -> (각 blob) Reverify
Reverify
  -> (hash == expected) Send(blob) -> (다음 blob) Transfer | (마지막) Commit
  -> (hash != expected)  Abort(HashMismatch)             # R-REVERIFY-02, 종료(커밋 미발행)
  -> (send TransportError) Abort(Transport)
Commit
  -> (Ok)              Done(committed = outcome.committed)
  -> (TransportError)  Abort(Transport)
```

- **합법 종료 상태**: `Done`(NoOp 포함) 또는 `Abort`. `Abort`는 커밋을 발행하지 않으므로 `last_committed`가 불변이다(다음 사이클 재스냅샷, FQ-2=A).
- **단일 직렬성**: 전이는 한 사이클 내 한 방향으로만 진행하며 blob 병렬화·재시도 루프가 없다(Q2=B, D-16). PROP-U3-08이 합법 전이만 밟음을 model-based로 검증한다.

---

## 2. 하위 워크플로

### 2.1 `negotiate` — have/want 협상 (R-WANT-*, PROP-U3-01/02)

```
negotiate(manifest):
  req_body  = encode(NegotiateRequest{ manifest_digest, entries: project(manifest) })   # U0 CBOR, D-14
  resp      = AuthTransport::send(OkcRequest{ method: Post, path: "<negotiate>", body: req_body })?  # 유일 HTTP 경로
  nresp     = decode::<NegotiateResponse>(resp.body.as_bytes())?                        # 2xx 본문 해석은 U3
  manifest_hashes = { e.raw_sha256 for e in manifest.entries }
  return WantSet{ blobs: manifest_hashes \ nresp.server_has }                           # 순수 차집합
```

- 순수 차집합 `compute_want(manifest_hashes, server_has)`는 전송과 분리 가능한 순수 함수로 추출해 PROP-U3-01/02를 transport 없이 검증한다.
- 엔드포인트 경로 `<negotiate>`와 정확 본문 스키마는 [blocked-on-server]; mock `HttpTransport`가 canned `server_has`를 반환하는 계약으로 구동한다.

### 2.2 `transfer_wanted` / `transfer_blob` (R-XFER/R-CHUNK/R-REVERIFY, PROP-U3-03/04/05/06)

```
transfer_wanted(snapshot, manifest, want):
  for entry in manifest.entries:                        # 경로 오름차순 canonical(결정적 순차)
    if entry.raw_sha256 ∉ want.blobs: continue          # R-WANT-03 (서버 보유분 미전송)
    resume = SyncStateStore::resume_offset(&entry.raw_sha256)   # None -> 0
    transfer_blob(entry, resume)?

transfer_blob(entry, resume):
  # --- Reverify 패스 (R-REVERIFY-01, 전송 직전) ---
  reader   = BlobSource::open(&entry.relative_path)?
  actual   = ContentAddressing::hash_stream(reader)?    # U1 스트리밍 SHA-256
  if actual != entry.raw_sha256:
      return Err(UploadError::HashMismatch{ path, expected: entry.raw_sha256, actual })  # R-REVERIFY-02

  # --- 전송 패스 (별도 재-읽기, R-REVERIFY-03) ---
  if entry.size <= S:                                   # R-XFER-01
      send_single(entry)  or  fall back to chunked (실패/타임아웃 시)
  else:
      send_chunked(entry, start_offset = resume ?? 0)
```

- `send_chunked`: `ChunkPlan{ start_offset, total: entry.size, chunk_size }`로 분할. 각 청크마다:
  - `frame = ChunkFrame{ blob, offset, len, chunk_sha256 = hash_stream(chunk_bytes), bytes }`
  - `AuthTransport::send(OkcRequest{ method: Put/Patch, path: "<blob/offset>", body: encode(frame) })?`
  - ack 시 `SyncStateStore::persist_resume_offset(blob, offset + len)`   # R-RESUME-02
  - 진행량 > S이면 `StatusSink::set_resume_progress(transferred, total)`  # R-STATUS-01
- **재조립 데이터 흐름**(서버측, 클라이언트가 만족시키는 계약): `offset` 오름차순 concat == 원본(R-CHUNK-01), 재조립 `raw_sha256` == 매니페스트 해시(R-CHUNK-03). 임의 오프셋 재개 동치(R-CHUNK-02).

### 2.3 `commit` (R-COMMIT-*, PROP-U3-07 종료 정합)

```
commit(manifest):
  path_hash_map = PathHashMap{ (e.relative_path -> e.raw_sha256) for e in manifest.entries }   # BTreeMap 정렬
  req_body      = encode(CommitRequest{ path_hash_map, manifest_digest: manifest.manifest_digest })
  resp          = AuthTransport::send(OkcRequest{ method: Post, path: "<commit>", body: req_body })?
  outcome       = decode::<CommitOutcome>(resp.body.as_bytes())?      # server_vault_content_id + committed
  return outcome                                                     # 반복 커밋 -> committed=false (서버 no-op)
```

---

## 3. 데이터 흐름 요약 (사이클 1회)

```
U8 트리거 -> (VaultScanner scan + ManifestBuilder [U1]) -> snapshot + manifest
  -> U3.execute_cycle:
       SafetyLimitsValidator::validate [U1]     -> LimitDecision
       SyncStateStore::last_committed [U4]       -> no-op digest 비교
       AuthTransport::send [U5] (negotiate)       -> WantSet
       for want blob:
           BlobSource::open [U3 seam -> VaultScanner] -> bytes
           ContentAddressing::hash_stream [U1]   -> 재검증
           AuthTransport::send [U5] (chunk/single) + SyncStateStore::persist_resume_offset [U4]
           StatusSink::set_resume_progress [U0/U6]
       AuthTransport::send [U5] (commit)          -> CommitOutcome
       SyncStateStore::commit_manifest [U4]       -> 원자 갱신(offset clear 흡수)
  -> Result<CommitOutcome, UploadError> -> U8 (실패 시 U4 분류/백오프)
```

---

## 4. 교차 단위 경계 (소유 vs 소비 API)

| 관심사 | 소유 | U3 소비 방식(실제 API) |
|---|---|---|
| 값 타입·오류 taxonomy·코덱·`StatusSink` 계약 | U0 | 이름 참조 + `encode`/`decode` 호출 |
| blob 재-해시 · manifest digest | U1 `ContentAddressing` | `hash_stream(reader)` 순수 호출(재검증), `manifest_digest`는 참고 |
| 런타임 한도 재검사 | U1 `SafetyLimitsValidator` | `validate(&Manifest) -> LimitVerdict` 순수 호출 |
| 파일 열거·리더 | U1 `VaultScanner` | **직접 미의존** — `BlobSource` seam으로 주입(U8이 `open_reader` 배선, §0 노트1) |
| 재개 오프셋·마지막 커밋·커밋 지속 | U4 `SyncStateStore` | `last_committed_manifest`/`resume_offset`/`persist_resume_offset`/`commit_manifest` 호출(주입) |
| HTTP·토큰·TLS·전송 오류 분류 | U5 `AuthTransport` | `send(OkcRequest) -> Result<OkcResponse, TransportError>` 호출(유일 경로); 2xx 본문 해석만 U3 |
| 재시도/백오프 스케줄 | U4 `RetryBackoffController` | **미소유** — `UploadError`(특히 `Transport`)를 상위 반환, U4가 분류·지연 |
| 동의 게이트 | U5 `ConsentGate` | **미소유** — U8이 사이클 전 검사 |
| 트리거 소싱·사이클 직렬화·운영 상태 push·`record_sync_success` | U8 `SyncCycleCoordinator` | **미소유** — U3는 진행률·OverLimit push만(R-STATUS-02) |

- **경계 불변식**: U3는 상류 W2 크레이트의 소스를 수정하지 않고 공개 API만 소비한다(W3 소비자). 위 API는 모두 실재 확인됨(`crates/*/src/**`).

---

## 5. 컴포넌트별 Testable Properties 노트 (PBT-01)

`UploadProtocolDriver`가 U3의 유일 컴포넌트이며 모든 속성을 담는다:

- **순수 함수 계층**(transport 무관): `compute_want`(PROP-U3-01/02), 청크 분할/재조립(`ChunkPlan`/`ChunkFrame`, PROP-U3-03/04/05). 도메인 제너레이터(PBT-07): 랜덤 매니페스트, 임의 blob 바이트열, 임의 `chunk_size`/오프셋, 임의 `server_has` 집합.
- **상태머신 계층**(mock 구동): 재검증 가드(PROP-U3-06, 변조 바이트 mock `BlobSource`), no-op 조기 종료(PROP-U3-07, 호출 카운팅 mock 전송), 합법 전이(PROP-U3-08, `HttpTransport` mock seam으로 성공/실패/over-limit/변조 시나리오 구동).
- shrinking·고정 시드·CI 통합(PBT-08) 요구. 프레임워크(PBT-09) 및 정확 시드 정책은 NFR Requirements 이월.
- **No PBT properties identified**: 진행률 push 값(best-effort 관측), `VaultContentId`(불투명 서버 식별자).

---

## 6. 확장 컴플라이언스 요약 (완료 게이트용)

| 확장 | 활성 | 이 산출물 반영 |
|---|---|---|
| **Property-Based Testing** | ON (Full) | §5 + `business-rules.md` §11의 PROP-U3-01..08(각 카테고리 라벨·제너레이터·근거). 순수 함수 + 상태머신 이원 커버. 미준수 시 blocking. |
| **Resiliency Baseline** | ON | 재개 청크(R-CHUNK/R-RESUME, FR-08)·전송 전 재검증(R-REVERIFY, Q8=A)·no-op 멱등(R-NOOP, FR-10)이 중단/부분 실패에도 상태 손상 없이 진행/재개 보장(High criticality). drive-not-sleep(R-CYCLE-01/D-16)로 백오프는 U4 위임. 크래시-재개 하니스(RESILIENCY-14)는 PROP-U3-04에 씨앗, 상세 NFR Design 이월. RTO/HA/DR·배포는 인프라 N/A. |
| **Security Baseline** | OFF | N/A — 미로딩. 토큰/TLS(NFR-06)는 U5 소관(U3는 `send`만 호출, 토큰 미취급). 콘텐츠 필터링 없음(NFR-07, RISK-01 수용). |
