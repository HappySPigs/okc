# U3 upload-client — Domain Entities (값 타입 / 포트)

**단계**: CONSTRUCTION -> Per-Unit Loop -> **U3 Upload Protocol Client** -> Functional Design -> 산출물 1/3 (`domain-entities.md`)
**작성일**: 2026-09-08
**크레이트**: `upload-client` (lib) · **소속 컴포넌트**: `UploadProtocolDriver`
**전제(확정 답변)**: Q2=B(단일 직렬 사이클) · Q6=C(`raw_sha256` keying, 커밋 = 경로->해시 맵 + `manifest_digest`) · Q8=A(전송 전 재검증) · FQ-1=A(서버가 `vault_content_id` 소유) · FQ-2=A(최신 상태 대체) · DEP-03 [blocked-on-server]

> **문서 성격**: 이 문서는 U3가 **도입**하는 값 타입·프로토콜 봉투·포트 트레이트를 정의한다. U0/U1/U4/U5 소유 타입은 **이름으로 참조만** 하며 재정의하지 않는다. 결정 규칙·불변식은 자매 산출물 `business-rules.md`가, 알고리즘·상태전이 흐름은 `business-logic-model.md`가 소유한다.
>
> **표기 규약**: 기술중립 설계. Rust스러운 시그니처는 **참고용**이며 규칙의 개념 형상을 표현한다(인프라·스레딩·정확 와이어 바이트 배제). 다이어그램은 ASCII 박스 없이 화살표 표기(`A -> B`)와 표/목록으로 기술한다. English 식별자명은 원문 유지, Rust 타입/제네릭은 백틱으로 감싼다.

---

## 0. 재사용(소비 전용) 타입 — 재정의 금지

U3는 아래 상류 타입을 **소비만** 한다. 필드·의미는 원 크레이트가 소유하며 이 문서는 참조만 한다.

| 타입 | 소유 | U3 소비 지점 |
|---|---|---|
| `Manifest`, `ManifestEntry` | U0 | 사이클 입력(경로/`raw_sha256`/`size`) |
| `RelativePath`, `Sha256Digest`, `ManifestDigest`, `ByteCount`, `Timestamp` | U0 | 엔트리 식별·blob 키·no-op 판별·바이트 수·시각 |
| `TransportError`, `TransportErrorClass` | U0 | `AuthTransport::send` 실패 -> `UploadError::Transport` 래핑 |
| `ClassifiedError`, `ErrorClass`, `TransferResult` | U0 | 분류 결과 형상(상위 U4로 위임 시 참고) |
| `StatusSink` (트레이트) | U0 | 진행률·OverLimit push(주입) |
| `encode`/`decode` (CBOR 코덱) | U0 | 프로토콜 본문 잠정 직렬화(D-14, [blocked-on-server]) |
| `ContentAddressing::hash_stream`, `ContentAddressing::manifest_digest` | U1 | 전송 전 재검증 재-해시 · no-op digest(참고) |
| `SafetyLimitsValidator::validate`, `LimitVerdict`, `LimitViolation` | U1 | 매 사이클 런타임 한도 재검사 |
| `SyncStateStore`(`last_committed_manifest`/`resume_offset`/`persist_resume_offset`/`clear_resume_offsets`/`commit_manifest`) | U4 | no-op 기준선 · 재개 오프셋 read/write · 커밋 지속(주입) |
| `AuthTransport`(`send`), `OkcRequest`, `OkcResponse`, `HttpMethod`, `Headers`, `Body` | U5 | 유일 아웃바운드 HTTP 경로(주입) |

---

## 1. U3 도입 값 타입

### 1.1 `WantSet` — 서버 결여 blob 집합 (have/want 협상 산출)

```rust
struct WantSet { blobs: BTreeSet<Sha256Digest> } // want = 매니페스트 blob 해시 \ 서버 보유분
```

- **의미**: 서버가 아직 보유하지 않아 전송해야 하는 blob의 `raw_sha256` 집합(whole-file 단위, Q6=C). 다중 경로가 동일 콘텐츠면 하나의 해시로 dedup된다.
- **결정성**: `BTreeSet`(정렬 집합)으로 표현해 순회·round-trip이 결정적이다(component-methods의 `HashSet`은 집합 의미만 승계, 결정성 위해 정렬 집합 채택 — MVP).
- **불변식**: `WantSet.blobs ⊆ { e.raw_sha256 | e ∈ manifest.entries }`. 상세 R-WANT-01/02(`business-rules.md`).

### 1.2 `PathHashMap` — 커밋 경로->해시 권위 맵

```rust
struct PathHashMap { entries: BTreeMap<RelativePath, Sha256Digest> }
```

- **의미**: 커밋이 참조하는 권위 있는 경로->콘텐츠 해시 바인딩(Q6=C). `Manifest.entries`에서 `(relative_path -> raw_sha256)`로 투영한다.
- **결정성**: `BTreeMap`(경로 오름차순) — canonical 정렬로 커밋 요청이 결정적.

### 1.3 프로토콜 봉투 — [blocked-on-server], 잠정 mock 계약

서버 프로토콜은 미확정이므로 아래는 **mock/seam 계약 기준의 잠정 봉투**다(D-14, DEP-03). 본문 바이트는 U0 CBOR 코덱으로 직렬화되어 `OkcRequest.body`/`OkcResponse.body`(`Body(Vec<u8>)`)에 실린다.

```rust
// 협상 요청: 매니페스트를 서버에 제시
struct NegotiateRequest { manifest_digest: ManifestDigest, entries: Vec<(RelativePath, Sha256Digest, ByteCount)> }
// 협상 응답: 서버가 보유한 blob 해시(또는 결여분) -> WantSet 계산 입력
struct NegotiateResponse { server_has: BTreeSet<Sha256Digest> }

// 커밋 요청: 권위 경로->해시 맵 + no-op 판별자
struct CommitRequest { path_hash_map: PathHashMap, manifest_digest: ManifestDigest }
// 커밋 결과: 서버 권위 vault_content_id(있으면) + 실제 커밋 여부(false=no-op)
struct CommitOutcome { server_vault_content_id: Option<VaultContentId>, committed: bool }

// FQ-1=A: 권위 있는 서버 소유 식별자(로컬 ManifestDigest와 별개, 불투명 문자열)
struct VaultContentId(String)
```

- `NegotiateResponse`가 `server_has`를 주면 U3가 `WantSet = manifest_hashes \ server_has`를 로컬 계산한다(NFR-09 순수 차집합, D-14 목 계약). 서버가 want를 직접 주는 변형도 계약 동형이나 MVP는 `server_has` 기준 로컬 차집합으로 고정(속성 검증 용이).
- `VaultContentId`는 서버 권위 식별자로 불투명(opaque)하며 로컬 판정에 쓰지 않는다(FQ-1=A).

### 1.4 재개 청크 프레이밍

```rust
struct ChunkPlan  { blob: Sha256Digest, total: ByteCount, chunk_size: ByteCount, start_offset: ByteCount }
struct ChunkFrame { blob: Sha256Digest, offset: ByteCount, len: ByteCount, chunk_sha256: Sha256Digest, bytes: Body }
```

- `ChunkPlan`: `start_offset`(= `SyncStateStore::resume_offset` 또는 0)부터 `total`까지 `chunk_size`(주입 config, 고정)로 분할하는 계획. 마지막 청크는 `<= chunk_size`.
- `ChunkFrame`: 오프셋·길이·**청크별 무결성 해시**(`chunk_sha256`, FR-08 per-chunk integrity) 동반 전송 프레임. 오프셋 순서로 concat하면 원본 blob 바이트열을 재구성한다(NFR-10, R-CHUNK-01).
- `chunk_sha256`은 U1 `ContentAddressing::hash_stream`으로 계산(청크 바이트 스트림).

### 1.5 `CyclePhase` — 프로토콜 상태머신 상태 (D-01)

```rust
enum CyclePhase {
    NoOp,        // digest 동일 -> 조기 종료(FR-10)
    Negotiate,   // 매니페스트 POST -> WantSet 수신
    Transfer,    // want blob 순차 전송(진입마다 Reverify 후 Send)
    Reverify,    // 전송 직전 재-읽기 + 재-해시 가드(Q8=A)
    Commit,      // 경로->해시 맵 + manifest_digest 전송
    Done,        // 커밋 성공(committed 또는 서버 no-op)
    Abort,       // HashMismatch/OverLimit/Transport -> 커밋 미발행
}
```

- 단일 값의 관측 가능한 진행 단계이며 지속되지 않는다(사이클-로컬). 합법 전이·PBT model은 `business-logic-model.md` §2.

### 1.6 `LimitDecision` — 런타임 한도 재검사 결과 (component-methods)

```rust
enum LimitDecision { Within, Exceeded(LimitReport) } // LimitReport: U0 자리표시 타입
```

- U1 `SafetyLimitsValidator::validate(&Manifest) -> LimitVerdict`를 감싸 U3 사이클 결정으로 투영한다(`WithinLimits -> Within`, `Exceeded(v) -> Exceeded(LimitReport)`). `LimitReport`는 U0 소유 자리표시로 위반 요약 문자열을 담는다.

### 1.7 `UploadError` — 사이클 실패 taxonomy (component-methods)

```rust
enum UploadError {
    Transport(TransportError),                                       // AuthTransport::send 실패(분류 포함)
    HashMismatch { path: RelativePath, expected: Sha256Digest, actual: Sha256Digest }, // Q8=A 커밋 중단
    OverLimit(LimitReport),                                          // US-E2-08 halt
    Aborted,                                                         // 그 외 사이클 중단
}
```

- 드라이버는 이를 **taxonomy 그대로 상위(U8)에 반환**하며 재시도/sleep하지 않는다(D-16). U8이 `Transport(err)`를 U4 `RetryBackoffController`로 분류·백오프한다.

---

## 2. U3 도입 포트 트레이트 (주입 seam)

### 2.1 `BlobSource` — 재-읽기 바이트 소스 seam (D-03)

```rust
trait BlobSource: Send + Sync {
    // 상대경로 blob의 스트리밍 리더를 연다(재검증·전송 재-읽기용). 전량 적재 없음.
    fn open(&self, path: &RelativePath) -> Result<Box<dyn Read>, BlobSourceError>;
}
enum BlobSourceError { NotFound, Io(String) } // 진단용 상세(토큰/콘텐츠 원문 미포함)
```

- **근거**: §0 노트1 "U3가 스스로 재-읽기, `VaultScanner` 의존 불필요(바이트 스트림 해시)". U3는 `VaultScanner`(U1)에 직접 의존하지 않고 이 포트만 의존한다. U8 조립루트가 `VaultScanner::open_reader`를 이 포트로 배선한다.
- **테스트 격리**: mock `BlobSource`(임의 바이트/변조 바이트 반환)로 재검증 가드(PROP-U3-06)와 청크 재조립(PROP-U3-03/04)을 상류 파일시스템 없이 PBT 구동한다.
- `Send + Sync`: 데몬 멀티스레드 공유(U0 싱크 계약과 동형).

### 2.2 전송 의존 (신규 트레이트 아님)

`UploadProtocolDriver`는 유일 HTTP 경로로 주입된 `AuthTransport`(U5 구체 struct)의 `send(OkcRequest) -> Result<OkcResponse, TransportError>`를 호출한다. 별도 래퍼 트레이트를 도입하지 않는다(D-04). 프로토콜 PBT는 `AuthTransport`를 하위 `HttpTransport` mock seam(`crates/auth-consent/src/transport/types.rs`의 `HttpTransport::execute`)으로 구성해 구동한다.

---

## 3. `UploadProtocolDriver` 주입 의존(구성 필드) — 참조 요약

메서드 인자가 아니라 **생성 시 주입**되는 협력자(component-methods 규약):

- `AuthTransport`(U5, 유일 HTTP 경로)
- `BlobSource`(§2.1, 재-읽기 seam; U8이 `VaultScanner` 배선)
- `SyncStateStore`(U4, 재개 오프셋 read/write · 커밋 지속 · no-op 기준선) — 단일 writer 전제(FR-23, U8/Q2=B)
- `StatusSink`(U0 계약, U6 구현 주입; 진행률·OverLimit push)
- 주입 config 값: 임계값 `S`(`chunk_threshold_bytes`) + `chunk_size`(`chunk_size_bytes`) (D-10)

> `ContentAddressing`/`SafetyLimitsValidator`(U1)는 무상태 유닛 타입이라 필드 주입 없이 순수 호출한다.

---

## 4. Testable Properties 대상 타입 노트 (PBT-01 — 상세는 `business-rules.md`)

- `WantSet`/`PathHashMap`/`NegotiateResponse.server_has`: 집합 차집합·멱등 제너레이터 대상(PROP-U3-01/02).
- `ChunkPlan`/`ChunkFrame` + blob 바이트: 청크 분할·재조립·재개 제너레이터 대상(PROP-U3-03/04/05).
- `CyclePhase`: 상태머신 model-based 전이 제너레이터 대상(PROP-U3-08).
- `UploadError`/`CommitOutcome`: no-op/HashMismatch/OverLimit 판정 형상(PROP-U3-06/07).
- `VaultContentId`: **No PBT properties identified**(불투명 서버 식별자, 로컬 판정 미참여).
