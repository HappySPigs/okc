## 컴포넌트 메서드 — okc-hooks "Watcher"

**단계**: INCEPTION → Application Design
**작성일**: 2026-09-08
**범위**: Application Design 수준의 메서드 시그니처 + 입출력 타입 + 오류 taxonomy만 다룬다. 상세 비즈니스 규칙·알고리즘은 각 단위 Functional Design으로 이월한다.
**표기**: 언어중립/Rust스러운 시그니처. 입출력 타입은 Foundation `CoreTypes`의 공유 값 타입을 참조한다. 협력자(`AuthTransport`/`SyncStateStore`/`ContentAddressing`/`SafetyLimitsValidator`/`StatusService`/`StructuredLogger` 등)는 생성 시 주입(필드)되므로 메서드 인자에서 생략한다.

> **`components.md` §0 통합 반영 요약(메서드 수준에도 적용)**:
> - **(수정1)** 싱크 계약 트레이트(`Logger`/`StatusSink`/`HistorySink`/`CriticalEventSink`)와 Q9 2축 상태 어휘 타입은 `CoreTypes`(파운데이션) 소유. U6 컴포넌트는 그 **구현**만 제공하며 아래 U6 섹션 메서드의 `impl CoreTypes::*` 표기가 이를 반영한다.
> - **(수정2)** `CriticalErrorNotifier`의 `TrayIndicator` 협력자는 nullable no-op 싱크로 주입 — 아래 `TrayIndicator::start()`가 헤드리스/비활성 시 `Ok(None)`을 반환하고 모든 메서드가 비치명적임으로 계약된다.
> - **(노트1·Q8=A)** `UploadProtocolDriver::transfer_blob`이 전송 직전 blob 바이트를 **스스로 재-읽기**하여 순수 `ContentAddressing`으로 재-해시하고 매니페스트 해시와 비교(불일치 시 `UploadError::HashMismatch`로 그 커밋 중단) — 아래 시그니처에 명시.
> - **(노트3·Q9=B)** `StatusService::update_probe()`(AutoUpdater 전용 순수 liveness)와 `health_check()`(운영 헬스)는 **별개 두 메서드**로 아래 StatusService 섹션에 등장.

---

## 공유 파운데이션 (Foundation)

### CoreTypes
CoreTypes는 주로 선언적(값 타입) 모듈이며, 실행 로직은 코덱과 소수의 순수 헬퍼로 한정된다.

```rust
// ── 프리미티브 ────────────────────────────────────────────────
struct RelativePath(String);        // 정규화된 볼트 상대경로 (POSIX 구분자)
struct Sha256Digest([u8; 32]);      // raw_sha256 = 표준 sha256sum
struct ManifestDigest([u8; 32]);    // 로컬 멱등/no-op 판정용 (권위 vault_content_id 아님)
struct Timestamp(/* UTC 단조 */);

// ── 매니페스트 ────────────────────────────────────────────────
struct ManifestEntry { relative_path: RelativePath, raw_sha256: Sha256Digest, size: u64 }
struct Manifest      { entries: Vec<ManifestEntry>, manifest_digest: ManifestDigest }

// ── 변경 집합 (FQ-2: 재스냅샷 diff 산출물, 이벤트 큐 아님) ─────
struct ChangeSet { added: Vec<ManifestEntry>, modified: Vec<ManifestEntry>, deleted: Vec<RelativePath> }

// ── 전송 결과 + 오류 taxonomy (Q4=A: 파운데이션 소유) ──────────
enum  ErrorClass      { Retryable, AuthAborted, Backpressure, Fatal }
struct ClassifiedError { class: ErrorClass, code: Option<String>, detail: String }
enum  TransferResult  { Success { bytes: u64 },
                        Partial { bytes: u64, resume_offset: u64 },
                        Failed  { error: ClassifiedError } }

// ── 동기화 상태 머신 (FQ-2; 지속/복구는 U4 SyncStateStore) ─────
enum SyncState { Idle, Dirty, Uploading, Committed }

// ── 관측 싱크 계약 + Q9 2축 상태 어휘 (§0 수정1: 파운데이션 소유) ──
trait Logger          { fn log(&self, record: LogRecord); /* … */ }
trait StatusSink      { /* set_operational/raise_condition/… (StatusService가 구현) */ }
trait HistorySink     { /* append (UploadHistoryStore가 구현) */ }
trait CriticalEventSink { /* report_* (CriticalErrorNotifier가 구현) */ }
enum  OperationalState { Idle, Syncing, Offline, Paused }
enum  ActiveCondition  { AuthFailed, ConsentBlocked, OverLimit, VaultUnavailable, UpdateRolledBack }
enum  LivenessSignal   { IdleReached, CredentialReadable }
struct StatusSnapshot  { /* operational + conditions + 부가 필드 (StatusService 섹션 참조) */ }
```

- `fn encode<T: Serialize>(value: &T) -> Result<Vec<u8>, CodecError>` — 무손실 직렬화(NFR-13).
- `fn decode<T: DeserializeOwned>(bytes: &[u8]) -> Result<T, CodecError>` — 역직렬화. **불변식**: 모든 `v`에 대해 `decode(encode(v)) == v`(US-E7-06).
- `impl ChangeSet { fn is_empty(&self) -> bool }` — no-op 사이클 판정(NFR-01).
- `impl ErrorClass { fn is_retryable(&self) -> bool }` — U4 재시도 게이트가 소비.
- 발생 오류: `CodecError`(직렬화 실패, Fatal 계열).
- *Functional Design 이월*: 구체 wire 포맷(canonical JSON vs CBOR) 선택, `SyncState` 전이 규칙, 필드 스키마 상세, 싱크 트레이트 메서드 최종 시그니처.

### ConfigProvider
- `load(path)` / `current() -> ConfigSnapshot` / `reload() -> Result<(), ConfigError>` / `subscribe(observer: &dyn ConfigReloadObserver)`.
- 단일 JSON(nginx식) 로드·스키마 검증(Q3=X). 리로드 시 관찰자 팬아웃(토큰 변경→ConsentGate, 로그레벨→StructuredLogger). 전체 스키마·기본값은 Functional Design 이월.

## U1 — Deterministic Content Core

### ContentAddressing
- `fn hash_stream<R: Read>(reader: R) -> Result<Sha256Digest, io::Error>`
  - 입력: 임의 바이트 스트림 / 출력: `raw_sha256`. 고정 버퍼 스트리밍(NFR-02), 결정적이며 표준 sha256sum과 일치(NFR-08). 발생 오류: 리더 I/O 오류. **(노트1)** U3의 전송 시 재검증도 이 메서드에 재-읽은 바이트 스트림을 흘려 재사용.
- `fn manifest_digest(entries: &[ManifestEntry]) -> ManifestDigest`
  - 입력: 매니페스트 엔트리 슬라이스 / 출력: 정렬된 `(relative_path, raw_sha256, size)`에 대한 결정적 해시. 순수·무결(I/O 없음).
- *Functional Design 이월*: 엔트리 정규화·정렬 도메인, 다이제스트 입력 인코딩 세부. (제약 고정: okc 스킴 재현 없음)

### VaultScanner
```rust
struct ScannedFile   { relative_path: RelativePath, size: u64 }
struct VaultSnapshot { root: PathBuf, files: Vec<ScannedFile>, captured_at: Timestamp }
enum   ScanError     { RootUnavailable, Io { path: RelativePath, source: io::Error }, /* … */ }
```
- `fn scan(&self) -> Result<VaultSnapshot, ScanError>`
  - 입력: 없음(config의 볼트 루트/제외 패턴 사용) / 출력: 일관 시점 열거 스냅샷. 발생 오류: `ScanError::RootUnavailable`(루트 부재/언마운트 — 판정은 U2가 해석), I/O 오류.
- `fn open_reader(&self, path: &RelativePath) -> Result<Box<dyn Read>, ScanError>`
  - 입력: 스캔된 상대경로 / 출력: 스트리밍 리더. 발생 오류: `ScanError::Io`.
- *Functional Design 이월*: 일관 시점 확보(디렉터리 워크 순서), 심링크/숨김/대용량 파일 처리, 제외 패턴 매칭 규칙.

### ManifestBuilder
```rust
enum BuildError { Scan(ScanError), Hash { path: RelativePath, source: io::Error } }
```
- `fn build(&self) -> Result<Manifest, BuildError>`
  - 입력: 없음(주입된 `VaultScanner`·`ContentAddressing` 사용) / 출력: 완성 `Manifest`(엔트리 + `manifest_digest`). 스트리밍으로 메모리 바운드(NFR-02), 결정적(NFR-08). 발생 오류: `BuildError`(스캔/해시 실패).
- *Functional Design 이월*: 중간 파일 I/O 실패 시 전체 중단 vs 스킵 정책, 진행 로그 훅.

### ManifestDiffer
- `fn diff(last_committed: &Manifest, current: &Manifest) -> ChangeSet`
  - 입력: 마지막 커밋 매니페스트(U4 `SyncStateStore`가 호출 시 인자로 전달) + 현재 매니페스트 / 출력: 정확한 `ChangeSet`(누락 0·오탐 0, NFR-12). 순수·무결. `current == last_committed`이면 빈 집합(no-op).
- *Functional Design 이월*: 수정 판정 기준(size+hash), 이름변경(rename) 취급 여부.

### SafetyLimitsValidator
```rust
struct SafetyLimits   { max_total_bytes: u64 /*20 GiB*/, max_file_bytes: u64 /*2 GiB*/, max_file_count: usize /*100k*/ }
enum   LimitViolation { TotalBytes { limit: u64, actual: u64 },
                        FileBytes  { path: RelativePath, limit: u64, actual: u64 },
                        FileCount  { limit: usize, actual: usize } }
enum   LimitVerdict   { WithinLimits, Exceeded(Vec<LimitViolation>) }
```
- `fn validate(manifest: &Manifest) -> LimitVerdict`
  - 입력: 매니페스트 / 출력: 판정(초과 시 실행 가능한 위반 목록, FR-05). 순수·무결, 경계-정확·단조(NFR-14). ≤10-sources는 서버(DEP-04)라 미포함.
- *Functional Design 이월*: 한도 값의 상수 vs config 노출 여부(현재 okc-core 고정 캡), 리포트 렌더 문구.

---

## U2 컴포넌트 메서드

### FilesystemWatcher
```rust
// 생성: config에서 vault_root·T_debounce, 상태 싱크·로거는 조립 루트가 주입
fn new(cfg: &WatchConfig, status: StatusSink, log: Logger) -> Result<Self, WatchError>

// 감시 시작 → 디바운스 후 TriggerSignal 을 흘려보내는 스트림 반환
fn start(&mut self) -> Result<TriggerStream, WatchError>

fn pause(&mut self)                 // 감시 일시정지(데몬은 상주) — U7 RunStateController가 호출
fn resume(&mut self)
fn stop(self)
fn watch_state(&self) -> WatchState // { Watching, Triggered, Idle, Paused }
```
- `TriggerSignal { kind: TriggerKind::Debounced, cause_summary: String, observed_at: Instant }`
- `WatchError`(taxonomy): `{ Unsupported, RootMissing, OsWatchInit(source), Backend(source) }`
- **(노트2)** U2 순수성 유지: 트리거 로그/상태 push(US-E1-01)의 실제 표면화는 `SyncCycleCoordinator`가 수행(services.md 참조). 위 `status`/`log` 주입은 선택적 배선이며 핵심 push 소유자는 코디네이터.
- 상세 디바운스 타이머/버스트 합치기 로직, OS별 이벤트 스트림 정규화 → **Functional Design 이월**.

### ReconciliationScheduler
```rust
fn new(cfg: &ReconConfig, status: StatusSink, log: Logger) -> Self

fn run_startup_scan(&self) -> TriggerSignal          // kind = Reconciliation(Startup)
fn tick(&mut self, now: Instant) -> Option<TriggerSignal> // 주기 도래 & 사이클 idle 이면 Some
fn next_recon_due(&self) -> Instant
fn record_result(&mut self, outcome: CycleOutcome)
fn last_result(&self) -> Option<ReconResult>
```
- `TriggerSignal { kind: TriggerKind::Reconciliation(Startup|Periodic), .. }`
- 오류 없음(순수 스케줄링). 실제 재해시·diff 실행은 코어(U1)에 위임 — 이 컴포넌트는 트리거만 발행. 사이클 직렬화는 `SyncCycleCoordinator`의 단일-사이클 잠금이 강제.

### VaultAvailabilityGuard
```rust
fn new(cfg: &VaultConfig) -> Self

fn check_reachable(&self, root: &Path) -> Availability
//   Availability = { Reachable, RootMissing, Unmounted, Inaccessible }

fn guard_diff(
    &self,
    new_manifest: &Manifest,
    last_committed: Option<&Manifest>,
    availability: Availability,
) -> GuardVerdict
//   GuardVerdict = { Proceed, HoldVaultUnavailable(reason), HoldDestructiveEmpty }

fn is_confirm_empty(&self) -> bool   // config 플래그 / 기록된 confirm-empty 확인
```
- 오류 대신 판정(enum) 반환 — **(노트2)** 상태 표면화(US-E1-06 vault-unavailable을 로그/status/헬스로 노출)는 코디네이터가 판정을 받아 수행.
- 부분 가용·심볼릭 링크·권한 경계 세부 분류 규칙 → **Functional Design 이월**.

### SingleInstanceLock
```rust
fn acquire(cfg: &LockConfig) -> Result<LockGuard, LockError>
//   LockError = { AlreadyRunning { pid: u32, owner: String }, Io(source) }
//   내부적으로 stale 잠금은 자동 감지·회수

fn holder_info(cfg: &LockConfig) -> Option<InstanceInfo> // { pid, owner }
// LockGuard: Drop 시 잠금 해제
```
- stale 판정(기록 PID liveness 검사)·OS별(lockfile vs named mutex) 회수 절차 → **Functional Design 이월**.

---

## U4 컴포넌트 메서드

### SyncStateStore
```rust
// 열기 + 크래시 복구(부분 temp/WAL 기록을 마지막 정상 상태로 롤백)
fn open_and_recover(cfg: &StateConfig) -> Result<SyncStateStore, StateError>

fn last_committed_manifest(&self) -> Option<&Manifest>
fn is_dirty(&self) -> bool
fn mark_dirty(&self) -> Result<(), StateError>            // 원자적 지속
fn commit_manifest(&self, m: &Manifest) -> Result<(), StateError> // 원자적; dirty 클리어

// 진행 중 업로드 재개 오프셋(U3 재개 전송 지원)
fn resume_offset(&self, blob: &BlobId) -> Option<u64>
fn persist_resume_offset(&self, blob: &BlobId, off: u64) -> Result<(), StateError>
fn clear_resume_offsets(&self) -> Result<(), StateError>  // 커밋 성공 후
```
- `StateError`(taxonomy): `{ CorruptRecovered { recovered_to }, Io(source), Serde(source) }`
- `Manifest`·직렬화 코덱은 공유 파운데이션(CoreTypes) 소비(NFR-13).
- 원자적 쓰기 메커니즘(temp+rename vs WAL)·fsync 순서·부분쓰기 롤백 알고리즘 → **Functional Design 이월**(NFR-03). PBT는 동기화 상태 머신(idle→dirty→uploading→committed + persist/recover) 모델로 확정.

### RetryBackoffController
```rust
fn new(cfg: &BackoffConfig) -> Self

fn classify(&self, err: &TransportError) -> RetryClass
//   RetryClass = { Transient, Offline, Backpressure, AuthFailed, Permanent }
//   TransportError 는 CoreTypes(공유 파운데이션) taxonomy

fn on_failure(&mut self, err: &TransportError, now: Instant) -> RetryDecision
//   RetryDecision { retry_after: Option<Duration>, is_offline: bool, escalate: bool }
//   AuthFailed → retry_after=None(재시도 안 함, U5 인증 흐름 위임)
//   Backpressure → 정상 지연으로 백오프(오류 아님)

fn on_success(&mut self)                 // 백오프 + 연속 실패 카운트 리셋
fn next_retry_at(&self) -> Option<Instant>
fn consecutive_failures(&self) -> u32
```
- 구체 백오프 스케줄(초기 지연/배수/상한/지터), N 임계 기본값 → **Functional/NFR Design 이월**.

---

## U3 · U5 컴포넌트 메서드

시그니처는 언어중립/Rust스러운 형태다. 입력/출력 타입은 Foundation `CoreTypes`의 공유 값 타입을 참조한다(아래 "참조 타입" 참고). 상세 알고리즘·비즈니스 규칙은 각 단위 **Functional Design 이월**로 표기한다. `AuthTransport`/`SyncStateStore`/`ContentAddressing`/`SafetyLimitsValidator`/`StatusService` 등 협력자는 생성 시 주입(필드)되므로 메서드 인자에서 생략한다.

**참조 타입 (Foundation CoreTypes, Q4=A / FQ-1=A)**
```rust
// 오류 taxonomy — U4 재시도·ConsentGate 훅이 공유하는 단일 계약 (Q4=A, 파운데이션 소유)
enum ErrorClass { AuthFailed, ServerError, Backpressure, Timeout, Network }
struct TransportError { class: ErrorClass, http_status: Option<u16>, detail: String }

// 표준 SHA-256(=sha256sum) 기반, okc vault_content_id 로컬 재현 안 함 (FQ-1=A)
type Sha256 = [u8; 32];
struct ManifestEntry { relative_path: RelPath, raw_sha256: Sha256, size: u64 }
struct Manifest { entries: Vec<ManifestEntry>, manifest_digest: ManifestDigest } // 로컬 no-op 판정용 다이제스트
type PathHashMap = BTreeMap<RelPath, Sha256>;   // Q6=C 권위 있는 경로→해시 맵(커밋 페이로드)
```

### UploadProtocolDriver

```rust
// 톱레벨: URP 3~6단계를 한 사이클로 수행 (Q2=B 단일 직렬 사이클)
fn execute_cycle(&self, snapshot: &VaultSnapshot, manifest: &Manifest)
    -> Result<CommitOutcome, UploadError>;
```
- 흐름: 런타임 한도 재검사 → (다이제스트 동일 시 no-op 조기 종료, FR-10) → negotiate → transfer_wanted → commit.
- 오류는 taxonomy 그대로 상위에 반환(재시도 스케줄링 미소유). **상세 오케스트레이션 순서·no-op 조기종료 규칙은 Functional Design 이월.**
- 커버: US-E2-05, FR-06, FR-09, FR-10.

```rust
// US-E2-08: 매 사이클 SafetyLimitsValidator(U1) 재사용 재검사
fn check_runtime_limits(&self, manifest: &Manifest) -> LimitDecision;
enum LimitDecision { Within, Exceeded(LimitReport) }
```
- Exceeded 시 호출부는 동기화 halt + 마지막 정상 커밋 유지 + StatusService에 OverLimit push. **halt/resume 상태 전이 상세는 Functional Design 이월.**
- 커버: US-E2-08, DEP-04 [서버 권위 재검증은 blocked-on-server].

```rust
// have/want 협상 (Q6=C: raw_sha256 키)
fn negotiate(&self, manifest: &Manifest) -> Result<WantSet, TransportError>;
struct WantSet { blobs: HashSet<Sha256> } // want = 참조 blob 해시 ∖ 서버 보유
```
- AuthTransport.send로 매니페스트 POST → want 집합 수신. **집합 차집합/멱등성 증명(NFR-09, US-E7-02)의 최종 속성·제너레이터는 Functional Design 이월(PBT-01).**
- 오류: `TransportError`(AuthFailed/ServerError/Backpressure/Timeout/Network).
- 커버: US-E2-03, FR-07, NFR-09, US-E7-02.

```rust
// want 집합 전체 전송 + 진행률 push
fn transfer_wanted(&self, snapshot: &VaultSnapshot, manifest: &Manifest, want: &WantSet)
    -> Result<(), UploadError>;
```
- 각 want blob에 대해 transfer_blob 호출; 전송량 > S면 ProgressReport를 StatusService로 push(US-E2-07).
- 커버: US-E2-04, US-E2-07, FR-08.

```rust
// 단일 blob 전송: 임계값 S 판정 + 청크/단일 + 재검증(Q8=A) + 재개(재개 오프셋)
fn transfer_blob(&self, blob: &BlobRef, expected: &Sha256, resume: Option<ResumeOffset>)
    -> Result<(), UploadError>;
struct ResumeOffset { bytes_acked: u64 } // SyncStateStore(U4)에서 읽기/쓰기
```
- S 이하 → 단일 요청 허용; S 초과 또는 단일 요청 실패/타임아웃 → 청크 전송(청크별 무결성).
- **재검증(Q8=A / 노트1)**: 전송 직전 U3가 blob 바이트를 **스스로 재-읽기**하여 순수 `ContentAddressing::hash_stream`으로 재-해시 → `expected` 불일치 시 `UploadError::HashMismatch` 반환(해당 커밋 중단, 다음 사이클 재스냅샷). `VaultScanner` 의존 불필요(바이트 스트림 해시).
- 재개: `resume`(마지막 ack 오프셋)부터 이어서 전송, 진행 중 오프셋은 SyncStateStore에 지속.
- 오류: `UploadError::{Transport, HashMismatch, Aborted}`.
- **청크 크기·청크별 무결성 스킴·바이트 단위 재조립 라운드트립(NFR-10, US-E7-03) 최종 속성은 Functional Design 이월(PBT-01).**
- 커버: US-E2-04, FR-08, NFR-10, US-E7-03.

```rust
// 커밋 (Q6=C, FQ-1=A): 경로→해시 맵 + 매니페스트 다이제스트 참조
fn commit(&self, manifest: &Manifest) -> Result<CommitOutcome, UploadError>;
struct CommitOutcome { server_vault_content_id: Option<VaultContentId>, committed: bool /* false=no-op */ }
```
- CommitRequest = { path_hash_map: PathHashMap, manifest_digest: ManifestDigest }. 서버가 바이트 구체화·경로 바인딩·권위 있는 vault_content_id 계산(DEP-03 [목]). 반복 커밋은 no-op(FR-10).
- 커버: US-E2-05, US-E2-06, FR-06, FR-09, FR-10, DEP-03 [blocked-on-server].

```rust
enum UploadError {
    Transport(TransportError),
    HashMismatch { path: RelPath, expected: Sha256, actual: Sha256 }, // Q8=A 중단
    OverLimit(LimitReport),                                           // US-E2-08
    Aborted,
}
```

### AuthTransport

```rust
// 인증된 TLS 전송 + 타임아웃 + 오류 분류 (Q4=A). 유일한 HTTP 경로.
fn send(&self, req: OkcRequest) -> Result<OkcResponse, TransportError>;

struct OkcRequest  { method: HttpMethod, path: String, headers: Headers, body: Body }
struct OkcResponse { status: u16, headers: Headers, body: Body } // 2xx만 Ok로 반환
```
- 동작: TLS 강제(NFR-06) → CredentialProvider.resolve_token() 결과를 헤더 첨부(FR-13) → config 타임아웃 적용(NFR-04) → 응답/실패를 `ErrorClass`로 분류.
- 분류 매핑(내부): 401→AuthFailed, 5xx→ServerError, PROJECT_BUSY/queue-full(429 등)→Backpressure, 타임아웃→Timeout, 연결 실패→Network. **정확한 상태코드→클래스 매핑 표는 Functional Design 이월.**
- 프로토콜 의미(want 해석 등)와 재시도는 미소유. 2xx 응답 본문 해석은 U3가 수행.
- 커버: US-E4-01, US-E4-03(401 분류), FR-13, NFR-06, NFR-04, DEP-01/DEP-03/DEP-05 [목/서버].

### CredentialProvider

```rust
fn resolve_token(&self) -> Result<Token, CredentialError>;
fn token_status(&self) -> TokenStatus;
fn on_config_reload(&self);  // config 리로드/재시작 후 재해석 (US-E4-03)

enum TokenSource { Config, Env, SecureStore }
struct TokenStatus { present: bool, source: TokenSource }
enum CredentialError { Missing, Empty, SecureStoreUnavailable /* → config/env 폴백 */ }
```
- `resolve_token`: secure_store 옵션 on + 세션 가용 시 secure-store 조회, 아니면 config `token`/env 폴백; 부재/공백 시 `Missing`/`Empty`.
- `token_status`: CLI status/헬스체크 표면에 존재/소스 노출.
- **폴백 우선순위·secure-store 백엔드별 조회 상세는 Functional Design 이월.**
- 커버: US-E4-01, US-E4-02, US-E4-03, FR-13, NFR-06, DEP-01/DEP-05 [목/서버].

### ConsentGate

```rust
fn ensure_acknowledged(&self) -> Result<(), ConsentError>; // US-E4-04: 미확인 시 차단
fn acknowledge(&self) -> Result<(), ConsentError>;          // RISK-01 고지 확인 기록
fn disclosure_text(&self) -> &'static str;                  // RISK-01 고지 텍스트
fn grant(&self) -> Result<ConsentGrant, ConsentError>;      // US-E4-05: 상시 동의 부여
fn view(&self) -> ConsentStatus;                            // US-E4-06: 조회
fn withdraw(&self) -> Result<(), ConsentError>;             // US-E4-06: 철회(업로드만 차단)
fn is_upload_permitted(&self) -> ConsentDecision;           // 코디네이터 업로드 게이트

struct ConsentGrant  { grant_id: Uuid, granted_at: Timestamp }
struct ConsentStatus { acknowledged: bool, grant: Option<ConsentGrant>, state: ConsentState }
enum ConsentState    { NotAcknowledged, AcknowledgedNotGranted, Granted, Withdrawn }
enum ConsentDecision { Permitted, Blocked(BlockReason) }
enum BlockReason     { NeedsAcknowledgment, NotGranted, Withdrawn }
enum ConsentError    { NotAcknowledged, AlreadyGranted, NoGrant, PersistFailed }
```
- `withdraw`: state→Withdrawn, StatusService에 ConsentBlocked push, 감시/감지는 유지·업로드/커밋만 차단(전진 방향, RISK-01). 재부여 시 대기 업로드 재개.
- 동의 부여 참조는 CoreTypes 코덱으로 로컬 지속 저장(무손실 round-trip). **고지 텍스트 최종 문안·상태 전이 규칙·저장 파일 포맷은 Functional Design 이월.**
- **[blocked-on-server]**: 서버측 persist/scope/enforce(DEP-02), integrate/승인/compile(DEP-06), 고승인 감내(DEP-07)는 서버 대기.
- 커버: US-E4-04, US-E4-05, US-E4-06, FR-14, FR-15, NFR-07, RISK-01, RISK-02, DEP-02/DEP-06/DEP-07.

---

## U6 · U7 컴포넌트 메서드

> 표기 규약: `CoreTypes` 정의 타입: `Timestamp`(UTC/ISO-8601), `CycleId`, `ContentId`, `ManifestDigest`, `RawSha256`, `ByteCount`, `Version`, `TransportError`(Q4=A), `LimitReport`, `ConfigSnapshot`. 싱크 트레이트(`Logger`/`StatusSink`/`HistorySink`/`CriticalEventSink`)와 상태 어휘(`OperationalState`/`ActiveCondition`/`LivenessSignal`/`StatusSnapshot`)도 `CoreTypes` 소유(§0 수정1). **상세 비즈니스 규칙/알고리즘은 모두 Functional Design 이월.**

### StructuredLogger  (impl `CoreTypes::Logger`)

```rust
// Logger 파사드 구현 — 모든 단위가 CoreTypes::Logger 로 호출, 여기서 직렬화/쓰기.
fn log(&self, record: LogRecord);                                  // JSON-line 1건 방출 (best-effort, infallible surface)
fn event(&self, level: Level, event: &str,
         cycle_id: Option<CycleId>, fields: Fields);               // 편의 래퍼 → LogRecord 조립 후 log()
fn reload(&self, cfg: &LogConfig) -> Result<(), LogError>;         // config 리로드 시 경로/레벨/로테이션 재적용

// 타입: LogRecord{ timestamp, level, event, cycle_id, message, fields }, Level{Trace..Error}
// 오류: LogError{ Io, InvalidPath, RotationFailed }
```
- 상세(로테이션 트리거·백프레셔·동시성): Functional Design 이월.

### StatusService  (impl `CoreTypes::StatusSink`)

```rust
// --- push-only 뮤테이터 (하위 단위가 StatusSink 로 주입받아 호출) ---
fn set_operational(&self, state: OperationalState);                // idle|syncing|offline|paused
fn raise_condition(&self, cond: ActiveCondition);                  // AuthFailed|ConsentBlocked|OverLimit|VaultUnavailable|UpdateRolledBack
fn clear_condition(&self, cond: ActiveCondition);
fn record_sync_success(&self, at: Timestamp, id: ContentId);
fn set_dirty(&self, dirty: bool);                                  // FQ-2: 큐 깊이 대체 — 변경 대기 표시
fn set_resume_progress(&self, transferred: ByteCount, total: ByteCount);
fn set_liveness(&self, signal: LivenessSignal);                    // startup: IdleReached | CredentialReadable

// --- 읽기 ---
fn snapshot(&self) -> StatusSnapshot;                              // CLI status 표면
fn health_check(&self) -> Health;                                  // 운영 헬스 (종료코드 매핑은 OperatorCli)
fn update_probe(&self) -> Liveness;                                // 순수 liveness (운영 조건과 분리 → 롤백 루프 방지)

// 타입: Health{ Healthy | Unhealthy{ reasons: Vec<HealthReason> } }
//       Liveness{ Alive | NotReady{ missing: Vec<LivenessSignal> } }
//       StatusSnapshot{ operational, conditions:Set<ActiveCondition>, last_success:Option<Timestamp>,
//                       dirty:bool, resume:Option<(ByteCount,ByteCount)>, consent:ConsentState, offline:bool }
// 뮤테이터는 in-memory, infallible.
```
- **(노트3)** `update_probe()`는 AutoUpdater 전용 순수 liveness — `health_check()`(운영 헬스)와 **분리**되어, 일시적 AuthFailed/OverLimit 때문에 좋은 새 버전이 오판 롤백되는 롤백 루프를 방지.
- 조건→운영상태 결합 규칙, health/liveness 판정 임계: Functional Design 이월.

### UploadHistoryStore  (impl `CoreTypes::HistorySink`)

```rust
fn append(&self, record: UploadHistoryRecord) -> Result<(), HistoryError>;  // append-only, 수정/삭제 불가
fn query(&self, filter: HistoryQuery) -> Result<Vec<UploadHistoryRecord>, HistoryError>;

// 타입: UploadHistoryRecord{ content_id: ContentId|ManifestDigest, snapshot_hash: RawSha256,
//                            timestamp: Timestamp, status: UploadStatus{Success|Failure|Partial},
//                            bytes_transferred: ByteCount, error_detail: Option<String> }
//       HistoryQuery{ since: Option<Timestamp>, status: Option<UploadStatus>, content_id: Option<ContentId> }
// 오류: HistoryError{ Io, Serde, Corrupt }
```
- 저장 포맷(append 로그 vs 임베디드 DB)·인덱싱·NFR-13 라운드트립 제너레이터: Functional Design 이월.

### CriticalErrorNotifier  (impl `CoreTypes::CriticalEventSink`)

```rust
fn report_auth_failure(&self, detail: TransportError);                       // 케이스1: 401/토큰 거부
fn report_cycle_result(&self, outcome: CycleOutcome);                        // 케이스2: 연속 실패 카운트 (성공 시 리셋)
fn report_preflight_exceeded(&self, report: LimitReport);                    // 케이스3
fn report_update_rollback(&self, from: Version, to: Version, reason: RollbackReason); // 케이스4

// 각 report 는 내부적으로: StructuredLogger(상향 심각도) + StatusService.raise_condition + TrayIndicator.notify(옵션)
// 타입: CycleOutcome{ Success | Failure{ error: TransportError } }
// best-effort surface, infallible.
```
- **(§0 수정2)** `TrayIndicator` 협력자는 nullable no-op 싱크로 주입 — 트레이 부재 시에도 로그+헬스+CLI status 표면화(US-E5-04)는 항상 성립.
- N 임계값(기본 3, config)·중복 억제·표면화 디바운스: Functional Design 이월.

### TrayIndicator

```rust
fn start(&self, cfg: &TrayConfig) -> Result<Option<TrayHandle>, TrayError>;  // 헤드리스/비활성 → Ok(None)
fn render(&self, snapshot: &StatusSnapshot);                                 // idle/syncing/offline/error 아이콘
fn notify(&self, msg: NotificationMessage);                                  // 선택적 데스크톱 팝업
fn stop(&self);

// 오류: TrayError{ Unsupported, InitFailed }  — 모두 비치명적(부재가 다른 기능 차단 안 함)
```
- 플랫폼별 트레이 백엔드·메뉴 항목: Functional Design 이월(채택/제거 confirm-or-drop).

### ServiceManager

```rust
fn install(&self, spec: ServiceSpec) -> Result<(), ServiceError>;   // launchd/systemd/Windows Service 등록 + 자동시작
fn uninstall(&self) -> Result<(), ServiceError>;                    // idempotent 등록 해제
fn start(&self) -> Result<(), ServiceError>;
fn stop(&self) -> Result<(), ServiceError>;
fn restart(&self) -> Result<(), ServiceError>;                      // AutoUpdater 재기동용
fn status(&self) -> Result<ServiceRegistration, ServiceError>;      // registered/running

// 타입: ServiceSpec{ exec_path, working_dir, account, autostart:bool }  (ConfigProvider 취득)
//       ServiceRegistration{ registered:bool, running:bool, pid:Option<u32> }
// 오류: ServiceError{ UnsupportedPlatform, PermissionDenied, NotInstalled, Io }
```
- 플랫폼별 유닛 파일 템플릿/권한: Functional Design / Infrastructure Design 이월.

### AutoUpdater

```rust
fn check_for_update(&self) -> Result<Option<UpdateInfo>, UpdateError>;      // 채널 config
fn apply_update(&self, update: UpdateInfo) -> Result<UpdateOutcome, UpdateError>;
        // 스테이징 → ServiceManager.restart() → 헬스 게이트(StatusService.update_probe, bounded time)
        // → 통과: Committed / 실패·타임아웃: 자동 rollback → CriticalErrorNotifier.report_update_rollback
fn rollback(&self, to: Version) -> Result<(), UpdateError>;                 // 직전 정상 버전 복원

// 타입: UpdateInfo{ version:Version, artifact_ref, checksum }, UpdateOutcome{ Committed | RolledBack }
// 오류: UpdateError{ Download, Verify, RestartFailed, GateTimeout, Io }
```
- **(노트3)** 헬스 게이트는 `StatusService.update_probe()`만 소비(운영 `health_check()` 아님) → 롤백 루프 방지.
- 롤백 백오프/보류 상태 지속·아티팩트 레이아웃·게이트 폴링 주기: Functional Design / Infrastructure Design 이월.

### Uninstaller

```rust
fn uninstall(&self, opts: UninstallOptions) -> Result<UninstallReport, UninstallError>;
        // 1) ServiceManager.uninstall()  2) 경로 기반 삭제: 히스토리(U6)/매니페스트·SyncState(U4)/로그(U6)
        // 3) 토큰: config/env 필드 + (옵션) CredentialProvider 보안저장소  4) 볼트 원본 미접촉, idempotent

// 타입: UninstallOptions{ purge_token:bool, purge_logs:bool, ... }
//       UninstallReport{ removed: Vec<Artifact>, skipped: Vec<(Artifact, Reason)> }
// 오류: UninstallError{ PartialFailure(UninstallReport), Io }  — 부분 실패 시 잔존 항목 보고
```
- 산출물 경로 최종 목록·플랫폼별 토큰 삭제: Functional Design 이월.

### RunStateController

```rust
fn pause(&self)  -> Result<(), RunStateError>;         // 지속(config/상태파일) + StatusService.set_operational(Paused)
fn resume(&self) -> Result<(), RunStateError>;
fn request_sync_now(&self) -> Result<(), RunStateError>;   // 즉시 사이클 신호 (오케스트레이션이 관측)
fn request_stop(&self, mode: StopMode) -> Result<(), RunStateError>;  // graceful 셧다운 신호
fn current(&self) -> RunState;                          // 오케스트레이션이 read (역참조 회피)

// 타입: RunState{ mode: RunMode{Running|Paused}, sync_requested:bool, stop_requested:Option<StopMode> }
//       StopMode{ Graceful | Immediate }
// 오류: RunStateError{ Persist, Io }
```
- 셧다운 유예 정책·신호 전달(채널/notify) 메커니즘: Functional Design 이월.

### OperatorCli

```rust
fn run(&self, args: CliArgs) -> ExitCode;              // 파싱 → dispatch → 렌더링/종료코드
fn dispatch(&self, cmd: Command) -> CliResult;         // 데몬 대상 → ControlPlane; install/uninstall → ServiceManager/Uninstaller

// 타입: Command{ Status|Health|Pause|Resume|SyncNow|Stop|History(HistoryQuery)|Consent(..)|Reload|Install|Uninstall }
//       CliArgs{ command, json:bool, .. },  CliResult{ output, exit_code }
// health → ExitCode 매핑 (0 healthy / non-zero + 사유)  [US-E5-02 종료코드 계약]
```
- 명령별 인자 스키마·`--json` 출력 스키마: Functional Design 이월.

### ControlPlane

```rust
// --- 서버 (데몬 측) ---
fn serve(&self, endpoint: IpcEndpoint) -> Result<(), IpcError>;    // UDS(mac/Linux)/명명 파이프(Windows), 소유자 권한
fn handle(&self, req: ControlRequest) -> ControlResponse;          // → StatusService/UploadHistoryStore/RunStateController/ConsentGate/ConfigProvider
fn shutdown(&self);

// --- 클라이언트 (CLI 측, OperatorCli 사용) ---
fn connect(&self, endpoint: IpcEndpoint) -> Result<ControlClient, IpcError>;
fn request(&self, req: ControlRequest) -> Result<ControlResponse, IpcError>;

// 타입: ControlRequest{ proto_version:u16, op: Command, payload },  ControlResponse{ proto_version, result }
//       IpcEndpoint{ SocketPath | NamedPipe }
// 오류: IpcError{ Bind, Permission, Connect, Protocol, VersionMismatch, Io }
```
- 프레이밍·peer 자격증명 확인·버전 협상 상세: Functional Design 이월.

---

> 서비스(오케스트레이션) 메서드는 `services.md`, 의존성 그래프는 `component-dependency.md` 참조.
