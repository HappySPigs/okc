## 컴포넌트 정의 — okc-hooks "Watcher"

**단계**: INCEPTION → Application Design
**작성일**: 2026-09-08
**아키텍처**: 단일 배포 바이너리(RESILIENCY-01) 내부 논리 모듈 U1..U7 + 공유 파운데이션 + 오케스트레이션. GUI 없는 크로스플랫폼 백그라운드 데몬(nginx 방식).
**확정 결정**: Q2=B(단일 직렬 사이클), Q3=X(단일 JSON config), Q4=A(AuthTransport 단독 전송·taxonomy는 파운데이션), Q5=A(로컬 IPC), Q6=C(내용 해시 중복제거 + 권위 경로→해시 맵), Q8=A(전송 시 재검증), Q9=B(2축 상태 + update_probe 분리), **FQ-1=A(okc-core 의존성 0 — 표준 SHA-256/sha256sum + 경로 매니페스트만, 권위 vault_content_id는 서버)**, **FQ-2=A(최신 상태 대체 — 이벤트 큐 없음)**.
**출처**: 병렬 설계 4건 + 통합 1건 + 완전성·일관성 크리틱 1건 (workflow wf_24b657a5-1b8, 6/6 에이전트, 오류 0). 크리틱 판정 **CONCERNS**(차단 이슈 0·누락 요구사항 0·순환 0). 비차단 수정 2건은 아래 §0에 반영.

---

## §0. 설계 통합 반영 (크리틱 CONCERNS 해소 — 이 문서에서 확정)

크리틱이 지적한 비차단 수정 2건을 **컴포넌트 계약 수준에서 확정**한다(상세 동작은 Functional Design 이월).

### 수정 1 — 빌드 순서 역전 해소: 관측 싱크 계약을 파운데이션 `CoreTypes`로
승인된 빌드 순서 U1→U4→U5→U2→U3→U6→U7에서 5개 엣지가 순서를 역행한다(순환 아님): `UploadProtocolDriver`(U3)→`StatusService`/`StructuredLogger`(U6), `AuthTransport`(U5)→`StructuredLogger`(U6), `ConsentGate`(U5)→`StatusService`/`StructuredLogger`(U6). **해소(확정)**: 로깅·상태-push의 **계약(트레이트)** 과 **Q9 2축 상태 값 타입**을 **파운데이션 `CoreTypes`** 에 둔다:
- 싱크 트레이트: `Logger`(로그 파사드), `StatusSink`(상태 push), `HistorySink`(히스토리 append), `CriticalEventSink`(중대 이벤트 보고).
- 상태 어휘 타입: 운영 라이프사이클 enum `OperationalState{Idle, Syncing, Offline, Paused}` + 활성 조건집합 `ActiveCondition{AuthFailed, ConsentBlocked, OverLimit, VaultUnavailable, UpdateRolledBack}` + `LivenessSignal`/`StatusSnapshot` + `update_probe()`/`health_check()` 시그니처.

U6의 `StructuredLogger`/`StatusService`는 이 계약의 **구현만** 제공하고, 조립 루트 `WatcherDaemon`이 하위 단위로 주입한다(의존성 역전). 따라서 위 5개 엣지는 **파운데이션 계약 의존**으로 해석되어 빌드 순서 역전이 사라진다. 이는 이미 요구된 push-only 불변식(두 싱크가 파운데이션에만 의존)과 정합이다. → `component-dependency.md`에서 해당 엣지에 `[Foundation-contract]` 주석.

### 수정 2 — 선택적 컴포넌트에 대한 경성 의존 제거: `TrayIndicator`는 nullable no-op 싱크
`CriticalErrorNotifier`(U6)의 `TrayIndicator`(U6) 의존은 **선택적/nullable no-op 싱크**로 정의한다. `TrayIndicator`는 OPTIONAL(FR-18/US-E5-05, 헤드리스 데몬에서 부재 가능)이므로, 조립 루트가 트레이 부재 시 no-op 싱크를 주입한다. 이로써 중대오류의 표면화(로그 + 헬스 + CLI status, US-E5-04)는 트레이 부재와 무관하게 항상 성립한다. 상세 동작은 Functional Design 이월.

### 반영된 비차단 노트 (5)
1. **Q8=A 재검증 소유권**: `UploadProtocolDriver`(U3)가 전송 시점에 각 blob 바이트를 **스스로 재-읽기**(U3 수준 I/O)하여 순수 `ContentAddressing`으로 재-해시, 매니페스트 해시와 비교. 불일치 시 그 커밋을 중단하고 다음 사이클이 재스냅샷(FR-22 TOCTOU). `VaultScanner` 의존 불필요(바이트 스트림 해시).
2. **U2 체커의 push 소유권**: `FilesystemWatcher`/`VaultAvailabilityGuard`/`ReconciliationScheduler`는 의도적으로 `StatusService`/`StructuredLogger`를 **의존하지 않는다**(U2 순수성). US-E1-01(트리거 로그)·US-E1-06(vault-unavailable 표면화)의 상태/로그 push는 두 U2 체커·U6 싱크를 모두 의존하는 `SyncCycleCoordinator`가 수행한다.
3. **Q9=B 분리**: `update_probe()`(자동 업데이트 롤백 게이트용 순수 liveness)와 `health_check()`(운영 헬스)는 `StatusService`의 **별개 두 메서드**. `AutoUpdater`는 `update_probe()`만 소비 → 일시적 AuthFailed/OverLimit로 인한 롤백 루프 방지.
4. **PBT 소재**: PBT-06은 FQ-2에 따라 동기화 상태 머신 모델(idle→dirty→uploading→committed + persist/recover)로 재해석되며 스토리 US-E7-08(→`SyncStateStore`)로 커버. PBT-01/08/09·RESILIENCY-14는 Application Design 컴포넌트 소유자가 없음(Functional Design/NFR Design/Code Generation/Build-and-Test 이월; PBT는 이 단계 N/A).
5. **의도적 변형 항목의 커버는 잔존 의미로 성립**: NFR-08/US-E7-01 → 결정성 + sha256sum 일치(okc 스킴 조항 제거), NFR-17/US-E7-10 → Rust + proptest(okc 재사용 조항 제거) 모두 `ContentAddressing`; FR-02 → 표준 SHA-256 + 경로 매니페스트; FR-03/US-E3-01/02 → `SyncStateStore`; FR-12/NFR-11 → 재스냅샷으로 구조적 충족(`SyncStateStore`+`SyncCycleCoordinator`); FR-07/NFR-09 → Q6=C 해시 중복제거 + 경로 맵(`UploadProtocolDriver`). okc-interop/골든벡터/okc 재사용 컴포넌트 없음(FQ-1=A).

---

## 공유 파운데이션 (Foundation)

### CoreTypes
- **단위**: Foundation
- **목적**: 모든 단위가 공유하는 값 타입 · 오류 taxonomy · 동기화 상태 타입 + 무손실 직렬화 코덱을 한 곳에 정의한다. 승인된 빌드 순서(U1→U4→U5→U2→U3→U6→U7)에서 하위 단위가 상위 단위 타입을 역참조하지 않게 하는 것이 핵심 목적이다. 특히 Q4=A에 따라 전송결과/오류 taxonomy를 파운데이션에 둬, U4 `RetryBackoffController`가 U3/U5를 역방향 의존 없이 재시도 판단에 쓸 수 있게 한다. **(§0 수정1 확정)**: 로깅·상태-push 계약 트레이트(`Logger`/`StatusSink`/`HistorySink`/`CriticalEventSink`)와 Q9 2축 상태 어휘 타입(`OperationalState`/`ActiveCondition`/`LivenessSignal`/`StatusSnapshot` + `update_probe()`/`health_check()` 시그니처)도 여기 소유.
- **책임**:
  - 매니페스트 값 타입: `ManifestEntry{relative_path, raw_sha256, size}`, `Manifest{entries, manifest_digest}`. FQ-1=A에 따라 권위 있는 okc `vault_content_id`는 서버 소유이므로 Watcher `Manifest`는 이를 담지 않고, 로컬 멱등/no-op 판정용 `manifest_digest`만 보유한다.
  - 변경 집합 타입 `ChangeSet{added, modified, deleted}`. FQ-2=A "최신 상태 대체" 모델에서 매 트리거 재스냅샷 diff의 산출물이며, 이벤트별 큐/합체(coalesce)용 타입은 두지 않는다.
  - 전송 결과 `TransferResult` + 오류 taxonomy `ErrorClass{Retryable, AuthAborted, Backpressure, Fatal}` + `ClassifiedError`(Q4=A). U5 `AuthTransport`가 산출, U4/U3가 소비하되 소유는 파운데이션.
  - 동기화 상태 머신 타입 `SyncState{Idle, Dirty, Uploading, Committed}`(FQ-2). 상태의 지속/복구(마지막 커밋 매니페스트 + dirty + 재개 오프셋)는 U4 `SyncStateStore` 소유 — 여기서는 타입만 정의.
  - 무손실 직렬화 코덱(NFR-13): 위 모든 타입 + 큐/상태/히스토리 레코드에 대해 `decode(encode(v)) == v` 라운드트립 보장(자유형 오류 문자열·유니코드 포함).
  - 공유 프리미티브: `RelativePath`(정규화된 POSIX 구분자 상대경로), `Sha256Digest`(= sha256sum 결과), `ManifestDigest`, `Timestamp`.
- **공개 인터페이스 요약**: 값 타입/열거형 선언 + `encode`/`decode` 코덱 함수 + 소수의 순수 헬퍼(`ChangeSet::is_empty` no-op 판정, `ErrorClass::is_retryable`) + §0 싱크 트레이트/상태 어휘 선언. 상태 전이 규칙·구체 직렬화 포맷(예: canonical JSON/CBOR 선택)·필드 세부는 Functional Design 이월.
- **커버**: NFR-13, US-E7-06, FR-02, Q4, Q9

### ConfigProvider
- **단위**: Foundation
- **목적**: nginx 방식의 **단일 JSON 설정 파일**(Q3=X)을 로드·검증·리로드하고, 타입드 config를 조립 루트(`WatcherDaemon`)가 아래 단위로 주입한다. 설정을 인증 단위에 두면 그래프가 뒤집히므로(U1..U6가 U5에 의존) 파운데이션에 둔다.
- **책임**:
  - 단일 JSON config 파일을 읽어 스키마 검증 후 타입드 `WatcherConfig`로 노출(잘못된/누락 값은 시작 시 Fatal 오류로 거부).
  - 토큰의 **1차 저장 경로 = config JSON 평문 필드(+ env 폴백)**(§6-B, US-E4-01). OS secure-store는 U5의 선택적 강화 경로이며 ConfigProvider의 관심사가 아니다.
  - 리로드 팬아웃: `reload()` 시 변경분을 관찰자에게 통지 — 토큰 변경 → U5 `ConsentGate` 재확인, 로그레벨 변경 → U6 `StructuredLogger`. 순환을 피하기 위해 **관찰자(observer) 구독 패턴**으로 구현: 구독자가 ConfigProvider에 등록하며, ConfigProvider는 U5/U6 타입을 참조하지 않는다.
  - U1이 소비하는 섹션 제공: 볼트 루트 경로, 제외 패턴(VaultScanner용). 그 밖 섹션(디바운스 T_debounce, T_recon, 백오프 스케줄, 청크 임계값 S, 로그/서비스/트레이 등)은 해당 단위가 소비.
- **공개 인터페이스 요약**: `load(path)`, `current()`(타입드 스냅샷), `reload()`, `subscribe(observer)` + `ConfigReloadObserver` 트레이트. 전체 스키마 필드 목록·검증 규칙·기본값은 Functional Design 이월(각 단위가 자기 섹션 스키마를 확정).
- **커버**: Q3, FR-13, NFR-05, RISK-01

---

## U1 — Deterministic Content Core (순수)

> U1 공통 불변식: `VaultScanner`를 통한 파일 읽기를 제외하면 지속/네트워크 I/O 금지. 마지막 커밋 매니페스트의 **저장**은 U1이 아니라 U4 `SyncStateStore` 소유. 의존은 `ConfigProvider`/`CoreTypes` 및 U1 내부 컴포넌트로 한정. NFR-17 확정(US-E7-10)에 따라 Rust로 구현하며, 무상태 결정적 로직은 proptest 속성(NFR-08/12/13/14)의 대상이다.

### ContentAddressing
- **단위**: U1
- **목적**: 표준 SHA-256으로 파일별 지문(`raw_sha256` = sha256sum)을 스트리밍 계산하고, 정렬된 매니페스트 엔트리에 대한 결정적 `manifest_digest`를 산출한다. FQ-1=A에 따라 **okc-core 콘텐츠 주소 스킴은 재현하지 않는다**.
- **책임**:
  - 바이트 스트림에 대한 증분/스트리밍 SHA-256으로 `raw_sha256` 산출 — 고정 크기 버퍼로 100k 파일 / 20 GiB / 파일당 최대 2 GiB에서도 메모리 비제한 없음(NFR-02).
  - 결정성 보장: 동일 바이트열은 항상 동일 해시, 값은 표준 `sha256sum`과 일치(NFR-08; US-E7-01은 okc 적합성 조항 제거 후 sha256sum 일치로 재해석).
  - `manifest_digest`: 경로 오름차순으로 정렬된 `(relative_path, raw_sha256, size)` 시퀀스에 대한 순수 결정적 해시 — 로컬 멱등/no-op 판정용(권위 vault_content_id 아님).
  - **(§0 노트1)** 전송 시점 재검증에도 재사용된다: U3 `UploadProtocolDriver`가 재-읽은 바이트 스트림을 이 컴포넌트로 재-해시.
- **공개 인터페이스 요약**: `hash_stream(reader)` → `Sha256Digest`, `manifest_digest(entries)` → `ManifestDigest`(순수·무결). 정렬 규칙·정규화·해시 도메인 세부는 Functional Design 이월(단, okc 스킴 재현 없음이 고정 제약).
- **커버**: FR-02, NFR-02, NFR-08, US-E7-01, US-E2-01, US-E7-10, NFR-17

### VaultScanner
- **단위**: U1
- **목적**: 설정된 볼트 루트를 일관된 시점(point-in-time)으로 열거하여 파일 목록과 스트리밍 리더를 제공한다(FR-22 스냅샷 관심사의 "열거" 부분).
- **책임**:
  - config의 볼트 루트 하위 파일을 제외 패턴을 적용해 열거, `{relative_path, size}` 엔트리와 캡처 시각을 담은 `VaultSnapshot` 산출.
  - 열거된 각 파일에 대해 스트리밍 리더를 여는 진입점 제공(U1 내 유일한 파일 읽기 관심사).
  - 루트 부재/언마운트/접근 불가 등은 오류로 표면화만 한다. **빈-볼트/사용불가를 "전부 삭제"로 해석하지 않는 파괴적-커밋 가드(US-E1-06)와 vault-unavailable 상태 분류는 U2 `VaultAvailabilityGuard` 소유** — VaultScanner는 판정하지 않는다.
- **공개 인터페이스 요약**: `scan()` → `VaultSnapshot`(오류 시 `ScanError`), `open_reader(relative_path)` → 스트리밍 리더. 일관 시점 확보 방식(디렉터리 워크 순서, 심링크/숨김파일 처리)은 Functional Design 이월.
- **커버**: FR-22, US-E2-01, NFR-02

### ManifestBuilder
- **단위**: U1
- **목적**: `VaultScanner` 열거 결과와 `ContentAddressing` 해시를 결합해 사이클마다 볼트 `Manifest`를 재계산한다(US-E1-02의 "재계산" 절반).
- **책임**:
  - 스캔된 각 파일에 대해 리더를 열어 스트리밍으로 `raw_sha256`을 계산하고 `ManifestEntry`를 조립.
  - 엔트리 정렬 후 `manifest_digest`를 부여해 완성된 `Manifest` 산출.
  - 스트리밍 파이프라인으로 okc-core 상한에서도 메모리 바운드 유지(NFR-02), 결정성 유지(NFR-08).
- **공개 인터페이스 요약**: `build()` → `Manifest`(오류 시 `BuildError`). `VaultScanner`·`ContentAddressing`를 주입받아 U1 내부에서만 오케스트레이션(교차 단위 서비스 아님). 부분 실패(중간 파일 I/O 오류) 처리 정책은 Functional Design 이월.
- **커버**: FR-02, US-E1-02, NFR-02, NFR-08

### ManifestDiffer
- **단위**: U1
- **목적**: 마지막 커밋 매니페스트와 현재 매니페스트를 비교해 정확한 변경 집합을 산출한다(US-E1-02의 "diff" 절반).
- **책임**:
  - `diff(last_committed, current)` → `ChangeSet(added/modified/deleted)`를 누락 0·오탐 0으로 반환(NFR-12, US-E7-05).
  - `current == last_committed`이면 빈 `ChangeSet` 반환 → 업로드 미생성(no-op; NFR-01 일부).
  - `last_committed`는 U4 `SyncStateStore`가 보유하며 **호출 시 인자로 전달** — ManifestDiffer는 U4를 의존하지 않는다(순수 유지). FQ-2 "최신 상태 대체"에서 이 diff 결과가 곧 그 사이클의 최신 상태이므로 별도 합체 로직이 필요 없다.
- **공개 인터페이스 요약**: `diff(last_committed, current)` → `ChangeSet`(순수·무결). 수정 판정 기준(size+hash 비교), 이름변경 취급 등 세부는 Functional Design 이월.
- **커버**: FR-02, US-E1-02, NFR-12, US-E7-05, NFR-01

### SafetyLimitsValidator
- **단위**: U1
- **목적**: 전송 전에 클라이언트가 검사 가능한 SafetyLimits로 매니페스트를 사전검증한다(FR-05 preflight).
- **책임**:
  - 세 한도 검사: 총량 ≤20 GiB, 파일당 ≤2 GiB, 파일 수 ≤100k. 경계-정확(off-by-one 없음)·단조(파일/바이트 추가가 reject→accept로 뒤집히지 않음)(NFR-14, US-E7-07).
  - 초과 시 어떤 한도를 얼마나 초과했는지 실행 가능한 리포트를 담은 판정을 반환(FR-05). 판정 표면화(로그/CLI/헬스)는 U6, halt/resume 사이클 제어는 U3/오케스트레이션 소관.
  - 프로젝트 단위 ≤10-sources 한도는 서버 권위(DEP-04)이므로 **제외**.
- **공개 인터페이스 요약**: `validate(manifest)` → `LimitVerdict{WithinLimits | Exceeded(violations)}`(순수·무결). 한도 값의 상수/설정 여부(현재 okc-core 고정 캡)와 리포트 문구는 Functional Design 이월.
- **커버**: FR-05, US-E2-02, NFR-14, US-E7-07

---

## U2 — 변경 감지 & 트리거 (Change Detection & Trigger)

### FilesystemWatcher
**단위**: U2
**목적**: 설정된 볼트 루트를 플랫폼 네이티브 파일시스템 이벤트로 감시하고, 디바운스 정적 구간(T_debounce)이 지나면 동기화 사이클 트리거 신호를 방출한다.
**책임**:
- FSEvents(macOS) / inotify(Linux) / ReadDirectoryChangesW(Windows) 어댑터로 볼트 루트 하위의 생성·수정·삭제·이름변경 이벤트를 구독한다. (NFR-05 이식성)
- 디바운스 타이머를 내장한다: 이벤트마다 T_debounce를 리셋하고, 정적 구간에 도달하면 편집 버스트당 정확히 1회만 트리거한다.
- 감시 상태(watching / triggered / idle / paused)를 조회 가능하게 노출한다. **(§0 노트2)** 상태/로그 push는 자신이 하지 않고 `SyncCycleCoordinator`가 트리거를 받아 수행(U2 순수성 유지).
- pause / resume를 지원한다(U7 `RunStateController`가 호출 — 감시만 멈추고 데몬 프로세스는 상주).
- **주의**: 이벤트 감시는 무손실의 근거가 아니다. 런타임 이벤트 손실(inotify IN_Q_OVERFLOW / FSEvents 병합 / RDCW 버퍼 오버플로 / 무이벤트 네트워크 마운트)의 백스톱은 `ReconciliationScheduler`가 담당한다.
**공개 인터페이스 요약**: `start() -> TriggerStream`, `pause()`, `resume()`, `stop()`, `watch_state()`. 트리거 스트림은 `SyncCycleCoordinator`가 소비한다. 상세 디바운스/버스트 합치기 알고리즘 및 OS별 이벤트 정규화는 → Functional Design 이월.
**커버**: US-E1-01; FR-01; NFR-01(디바운스로 감지 지연 상한), NFR-05(감시 이식성 AC)

### ReconciliationScheduler
**단위**: U2
**목적**: 시작 시 1회 전체 재조정 스캔과, 실행 중 T_recon 주기 재조정 스캔을 트리거하여 이벤트 손실에 대한 백스톱을 제공한다.
**책임**:
- 데몬 시작 시(부트 자동시작 또는 수동 `watcher start`) 정상 감시 진입 **전에** 전체 재해시 + diff 트리거를 방출한다(정지 중 발생한 변경 복구; US-E1-03).
- 마지막 재조정 이후 T_recon이 경과하면 이벤트 유무와 무관하게 백스톱 트리거를 방출한다. 미관측 변경의 최대 미감지 창을 ≤ T_recon으로 상한한다(US-E1-04, NFR-03).
- 진행 중 사이클과 **직렬화**된다 — 재조정은 사이클 실행 중에는 동시 실행되지 않는다(직렬화 잠금은 `SyncCycleCoordinator`가 소유; 스케줄러는 busy 시 트리거를 건너뛰거나 다음 주기로 미룬다).
- 다음 재조정까지 남은 시간과 마지막 재조정 결과를 조회 가능하게 노출(push는 코디네이터가 수행, §0 노트2).
**공개 인터페이스 요약**: `run_startup_scan() -> TriggerSignal`, `tick(now) -> Option<TriggerSignal>`, `next_recon_due()`, `record_result(outcome)`, `last_result()`. 트리거는 `FilesystemWatcher`와 동일 스트림 규약으로 `SyncCycleCoordinator`에 전달되되 `TriggerKind::Reconciliation` 플래그로 구분. 스캔 실행 자체(재해시·diff)는 코어(U1) 위임이며 스케줄러는 트리거만 소유.
**커버**: US-E1-03, US-E1-04; FR-04; NFR-03(미관측 변경 검출 지연 ≤ T_recon)

### VaultAvailabilityGuard
**단위**: U2
**목적**: 빈 볼트 / 루트 없음 / 언마운트 / 접근 불가 상황을 vault-unavailable로 분류하고, 파괴적인 전부-삭제 diff 및 커밋을 보류하여 서버 콘텐츠의 비가역적 삭제를 방지한다(US-E1-06).
**책임**:
- 스냅샷 전 사전 점검: 볼트 루트의 도달 가능성(존재·마운트·접근 권한)을 판정한다.
- diff 사후 점검: 새 매니페스트가 0-파일인데 마지막 커밋에는 파일이 있었고 명시적 confirm-empty가 아니면 전부-삭제 변경 집합 생성을 거부하고 보류한다.
- vault-unavailable / held 판정을 **반환**한다. **(§0 노트2)** 실제 표면화(구조화 로그·status·헬스체크)는 이 판정을 받은 `SyncCycleCoordinator`가 수행 — 가드는 조용히 무시하지 않도록 판정만 명시적으로 낸다.
- 볼트가 파일과 함께 다시 사용 가능해지면 다음 사이클에서 확인 없이 정상 diff가 재개되도록 Proceed 판정을 낸다.
- 사용자가 진짜 빈 볼트를 의도할 때만 config 플래그 / CLI `confirm-empty`로 기록된 확인을 소비해 빈 상태를 수용한다.
**공개 인터페이스 요약**: `check_reachable(root) -> Availability`, `guard_diff(new_manifest, last_committed, availability) -> GuardVerdict`, `is_confirm_empty()`. 순수 분류기(파일시스템 접근 외 컴포넌트 의존 없음) — 상태 표면화는 코디네이터가 판정을 받아 수행. 부분 가용/심볼릭 링크/권한 세부 규칙은 → Functional Design 이월.
**커버**: US-E1-06; 관련: FR-09(파괴적 빈 커밋 방지 — 커밋 자체는 U3 소유), FR-02(0-파일 매니페스트 취급), FR-04(스캔이 0개를 볼 때)

### SingleInstanceLock
**단위**: U2
**목적**: 설치 단위로 단 하나의 인스턴스만 지속 상태(`SyncStateStore`)를 조작하도록 OS 수준 잠금을 강제하고, stale 잠금을 회수한다(FR-23).
**책임**:
- 데몬 기동 시 락파일(mac/Linux) / named mutex(Windows)로 설치 단위 잠금을 획득한다.
- 이미 살아 있는 인스턴스가 보유 중이면 시작을 거부하고 활성 인스턴스의 PID/소유자를 보고하며 비정상 종료 코드로 종료할 수 있게 정보를 반환한다.
- 이전 인스턴스가 크래시로 남긴 stale 잠금을 감지·회수하고 정상 진행한다.
- 어느 시점에도 큐/마지막 커밋 상태를 변경할 수 있는 인스턴스가 정확히 하나임을 보장(불변식) — graceful 종료 시 해제.
**공개 인터페이스 요약**: `acquire(config) -> Result<LockGuard, LockError::AlreadyRunning{pid,owner}>`, `holder_info(config) -> Option<InstanceInfo>`. `LockGuard`는 Drop 시 잠금 해제. stale 판정(기록된 PID liveness 검사)·OS별 회수 절차는 → Functional Design 이월. `WatcherDaemon`이 조립 루트에서 획득.
**커버**: US-E1-05; FR-23

---

## U4 — 동기화 상태 & 회복탄력성 (Sync State & Resilience)

### SyncStateStore
**단위**: U4
**목적**: **최신 상태 대체(latest-state-replacement) 모델의 지속 상태**를 소유한다 — 마지막 커밋 매니페스트 1개 + dirty(변경 대기) 표시 + 진행 중 업로드 재개 오프셋. **이벤트별 지속 큐가 아니다**(FQ-2=A).
**책임**:
- 마지막 커밋 매니페스트(권위 있는 경로→해시 맵 + 매니페스트 다이제스트; Q6=C)를 지속 저장하고 사이클 시작 시 diff 기준으로 읽어 준다.
- dirty 표시(커밋되지 않은 변경이 있음)를 지속한다 — 폴더가 진실의 원천이므로 이벤트 로그 대신 "다음 사이클이 재스냅샷해야 함" 신호만 유지.
- 진행 중 업로드의 blob별 마지막 ack 재개 오프셋을 지속하여 재개 전송(U3, FR-08)을 지원한다.
- 모든 쓰기는 원자적(temp+rename 또는 WAL)이어서 쓰기 도중 크래시가 상태를 손상시키지 않는다(NFR-03 무손실).
- 크래시 복구: 로드 시 부분 기록을 마지막 정상 상태로 롤백한다.
- 직렬화는 공유 파운데이션(CoreTypes) 무손실 코덱을 사용한다(NFR-13 라운드트립).
**공개 인터페이스 요약**: `open_and_recover(config)`, `last_committed_manifest()`, `is_dirty()`, `mark_dirty()`, `commit_manifest(m)`(원자적, dirty 클리어), `resume_offset(blob)`, `persist_resume_offset(blob, off)`, `clear_resume_offsets()`. 원자적 쓰기 메커니즘 선택(temp+rename vs WAL)·fsync 순서·부분쓰기 롤백 알고리즘은 → Functional Design 이월. PBT-06은 "동기화 상태 머신 모델"(idle→dirty→uploading→committed + persist/recover)로 축소 재해석.
**커버**: US-E3-01, US-E7-08(상태머신 모델), US-E7-09(zero-loss); FR-03, NFR-03, NFR-13; 구조적 충족: US-E3-02 / US-E7-04 / FR-12 / NFR-11("latest state wins"가 재스냅샷으로 구조적으로 성립)

### RetryBackoffController
**단위**: U4
**목적**: 사이클 실패·타임아웃·서버 백프레셔 시 전체 사이클을 지수 백오프로 재시도하도록 결정하고, 오프라인을 판정하며, 401은 재시도에서 제외한다(FR-11, NFR-04).
**책임**:
- CoreTypes 오류 taxonomy를 사용해 전송 결과를 분류한다: Transient(재시도) / Offline(타임아웃·연결오류) / Backpressure(PROJECT_BUSY·queue-full → 오류가 아닌 정상 지연으로 백오프) / AuthFailed(HTTP 401 → 재시도 안 함, 인증 흐름 U5로 위임) / Permanent.
- 실패 시 지수 백오프 + 상한(무한 증가 방지)으로 다음 재시도 시각을 계산한다.
- 오프라인은 별도 서버 왕복 없이 타임아웃/연결오류만으로 판정한다(US-E3-04).
- 성공 시 백오프와 연속 실패 카운트를 리셋한다.
- 연속 실패 횟수를 추적해 FR-19 케이스 2(N회 연속 실패) 알림 에스컬레이션 신호를 낸다(실제 알림은 U6 `CriticalErrorNotifier` 소유).
- 다음 재시도 예정 시각·연속 실패 횟수를 조회 가능하게 노출(push는 코디네이터가 수행).
**공개 인터페이스 요약**: `classify(err) -> RetryClass`, `on_failure(err, now) -> RetryDecision{ retry_after, is_offline, escalate }`, `on_success()`, `next_retry_at()`, `consecutive_failures()`. 구체 백오프 스케줄(초기 지연/배수/상한/지터)은 config 값이며 기본값은 → Functional/NFR Design 이월.
**커버**: US-E3-03, US-E3-04(오프라인 판정), US-E7-11(오프라인/재개 회복탄력성 전략 — 상세는 NFR Design 이월); FR-11, NFR-04

---

## U5 — 인증 & 동의 (Auth & Consent)

> 확정 결정 반영: **Q4=A**(전송/TLS/토큰/오류분류는 AuthTransport 단독, taxonomy 타입은 Foundation CoreTypes), **FQ-1=A**(okc-core 코드 의존 0). DEP-01..07 및 토큰 매치(Q3-custom)는 **[blocked-on-server] 목(mock) 계약**으로 검증한다.

### AuthTransport
- **단위**: U5 (Auth & Consent)
- **목적**: **얇은 전송 소유자(Q4=A)**. Watcher에서 서버로 나가는 유일한 HTTP 경로로, TLS만 사용(NFR-06) + 매 요청 토큰 첨부(FR-13) + 응답을 Foundation CoreTypes 오류 taxonomy로 분류 + 타임아웃 적용(NFR-04). 프로토콜 의미(U3)·재시도(U4)는 소유하지 않는다.
- **책임**:
  - 모든 요청을 **TLS 위에서만** 수립·전송 (NFR-06).
  - 매 요청에 `CredentialProvider`가 제공한 API 토큰을 헤더로 첨부 (FR-13).
  - config로 설정된 per-request 타임아웃 적용, 무한 대기 방지 (NFR-04).
  - 응답/실패를 **단일 오류 taxonomy(CoreTypes `ErrorClass`)**로 분류: 401 → AuthFailed, 5xx → ServerError, `PROJECT_BUSY`/queue-full(429 등) → Backpressure, 타임아웃 → Timeout, 연결 실패 → Network. 2xx는 원시 응답으로 상위(U3)에 반환해 프로토콜 해석을 위임. (US-E4-01, US-E4-03의 401 감지)
  - 오프라인 판정용 별도 서버 왕복(probe) 없음 — Timeout/Network 오류 클래스로 표현하고 판정은 코디네이터/U4가 수행 (NFR-04 정합).
  - **[blocked-on-server]**: 인증(DEP-01)·업로드 엔드포인트(DEP-03)·멀티테넌시(DEP-05, 토큰이 테넌트 스코핑)는 서버 소유. 실서버 전까지 토큰 매치(Q3-custom) 목 계약으로만 검증.
- **공개 인터페이스 요약**: `send`(인증된 TLS 전송 + 분류). (분류 매핑 표 자체는 내부 헬퍼)
- **커버**: US-E4-01, US-E4-03(401 분류), FR-13, NFR-06, NFR-04, DEP-01 [목], DEP-03 [목], DEP-05 [서버 소유].

### CredentialProvider
- **단위**: U5 (Auth & Consent)
- **목적**: 토큰 소스 결정. 1차 경로는 config JSON `token` 필드(또는 env 폴백, keyring-less 평문), 선택적 강화로 OS secure-store(US-E4-02). 토큰 존재/형식 유효성만 판정하며 권위 있는 유효성은 서버 401로 결정된다.
- **책임**:
  - config JSON `token` 필드 또는 env 변수에서 토큰 해석(1차 경로). (US-E4-01, FR-13)
  - config `secure_store` 옵션이 켜지고 데스크톱 세션 보안 저장소가 가용하면 secure-store에서 조회, 불가(헤드리스/데몬)하면 config/env로 **안전 폴백**(실행 중단 없음). (US-E4-02, NFR-06)
  - 토큰 부재/공백 시 CLI `status`+헬스체크로 관측 가능한 actionable 오류 산출(업로드 시작 차단). (US-E4-01 AC)
  - config 리로드/재시작 시 토큰 재해석(토큰 재입력 여정 지원). (US-E4-03)
  - **불변식**: 1차 경로는 config/env이며, 어떤 스토리도 secure-store 존재에 의존하지 않는다. (US-E4-02)
  - **[blocked-on-server]**: 토큰 발급/회전/폐기(DEP-01)와 테넌트 스코핑(DEP-05)은 서버 소유.
- **공개 인터페이스 요약**: `resolve_token`, `token_status`, `on_config_reload`.
- **커버**: US-E4-01, US-E4-02, US-E4-03, FR-13, NFR-06, DEP-01 [목], DEP-05 [서버 소유].

### ConsentGate
- **단위**: U5 (Auth & Consent)
- **목적**: RISK-01 최초 실행 고지 확인(US-E4-04) + 상시 동의 부여/참조 저장(US-E4-05) + 조회/철회(US-E4-06). 유효 동의가 없으면 `SyncCycleCoordinator`의 업로드/커밋 단계를 차단한다(감시·큐·상태는 유지).
- **책임**:
  - **최초 실행 고지(US-E4-04)**: RISK-01 고지 텍스트(연속성·비가역성·클라이언트 필터 없음 FR-15/NFR-07·로컬 평문 산출물) 제시. config 플래그 또는 CLI `acknowledge`로 확인이 기록되기 전까지 업로드 거부. (RISK-01, FR-15, NFR-07)
  - **상시 동의 부여(US-E4-05)**: 확인 기록 후 CLI `consent grant`/config로 1회 부여. 동의 부여 참조(부여 식별자 + 시각)를 로컬에 CoreTypes 코덱으로 지속 저장. (FR-14)
  - **조회(US-E4-06)**: `consent view` — 부여 참조·시각·현재 상태 반환.
  - **철회(US-E4-06)**: `consent withdraw` — 감시(FR-01)·변경 감지·상태는 **유지**하되 재동의 전까지 모든 업로드/커밋을 차단. `StatusService`에 ConsentBlocked push(Q9=B 활성 조건). 철회는 전진 방향(forward-only) — 이미 업로드된 콘텐츠 회수 불가(RISK-01).
  - **재동의**: 재부여 시 대기 중 업로드 재개 허용.
  - **업로드 게이트**: `is_upload_permitted()`를 코디네이터가 업로드 단계 전에 조회.
  - **RISK-02 인지 노트**: 잦은 편집이 서버 재승인 부담 유발 — 완화는 대부분 서버 몫(DEP-07), Watcher 측은 부분 완화(디바운스/스냅샷 대체/no-op 커밋)만.
  - **[blocked-on-server]**: 동의 persist/scope/**enforce**(DEP-02), integrate+승인+compile(DEP-06)은 서버 소유. 실효적 서버측 철회/삭제는 서버 대기.
- **공개 인터페이스 요약**: `ensure_acknowledged`, `acknowledge`, `disclosure_text`, `grant`, `view`, `withdraw`, `is_upload_permitted`.
- **커버**: US-E4-04, US-E4-05, US-E4-06, FR-14, FR-15, NFR-07, RISK-01, RISK-02, DEP-02 [blocked-on-server], DEP-06 [서버 소유], DEP-07 [blocked-on-server].

---

## U3 — 업로드 프로토콜 클라이언트 (Upload Protocol Client)

> 확정 결정 반영: **Q2=B**(단일 직렬 사이클), **Q6=C**(전송은 raw_sha256 중복제거, 커밋은 권위 있는 경로→해시 맵 + 매니페스트 다이제스트), **Q8=A**(전송 시 재-읽기·재-해시 재검증, freeze 없음), **FQ-1=A**(권위 vault_content_id는 서버 계산), **FQ-2=A**(최신 상태 대체). DEP-03/04는 [blocked-on-server].

### UploadProtocolDriver
- **단위**: U3 (Upload Protocol Client)
- **목적**: 업로드 프로토콜 3~6단계(have/want 협상 → 재개 가능 청크 전송 → 전송 시 재검증 → 커밋 → 멱등성)를 **한 트리거당 끝까지 도는 단일 직렬 사이클**(Q2=B)로 수행한다. HTTP를 직접 소유하지 않고 **U5 `AuthTransport`를 통해서만** 전송하며, 재개 오프셋은 U4 `SyncStateStore`를 사용하고, 진행률/"재개 중"/over-limit은 U6 `StatusService`로 push한다.
- **책임**:
  - **have/want 협상 (Q6=C)**: 경로 매니페스트를 서버로 보내고 서버가 결여한 blob 집합("want", `raw_sha256`=sha256sum 기준)만 회신받아, 그 집합만 전송한다. `want = 매니페스트가 참조하는 서로 다른 blob 해시 ∖ 서버 보유분`. (US-E2-03, FR-07, NFR-09)
  - **재개 가능 청크 전송**: config 임계값 S 초과 blob, 또는 단일 요청이 실패/타임아웃한 blob은 청크 전송(청크별 무결성 + `SyncStateStore`의 마지막 ack 오프셋에서 재개). S 이하는 단일 요청 허용. (US-E2-04, FR-08, NFR-10)
  - **전송 시 재검증 (Q8=A) — (§0 노트1)**: 각 blob을 전송 직전 **U3가 스스로 재-읽기**(U3 수준 I/O)하여 그 바이트 스트림을 순수 `ContentAddressing`으로 재-해시, 매니페스트 `raw_sha256`과 일치할 때만 전송. 불일치 시 그 커밋을 **중단**하고 다음 사이클이 폴더를 재스냅샷해 새 상태를 반영(추가 디스크 freeze 없음).
  - **커밋 (Q6=C, FQ-1=A)**: 모든 want blob이 서버에 존재하면 **권위 있는 경로→해시 맵 + 매니페스트 다이제스트**를 참조하는 commit을 전송. 권위 있는 `vault_content_id` 계산 및 바이트 구체화·경로 바인딩은 서버 책임(응답으로 서버측 content-id 수신 가능). (US-E2-05, FR-06, FR-09)
  - **멱등성/no-op**: 로컬 매니페스트 다이제스트가 마지막 커밋과 같으면 사이클을 no-op으로 조기 종료. 이미 존재하는 상태에 대한 반복 커밋도 서버에서 no-op. (US-E2-06, FR-10)
  - **진행률/"재개 중" 신호**: 대용량/초기 동기화(전송량 > S)에 대해 진행률(바이트/퍼센트)과 재개 상태를 `StatusService`로 push (CLI status/로그 소비). (US-E2-07)
  - **런타임 SafetyLimit halt/resume**: 매 사이클 시작 시 `SafetyLimitsValidator`(U1) 재사용으로 클라이언트 검사 가능 한도(≤20 GiB/≤2 GiB per file/≤100k files)를 재검사. 초과 시 동기화 중단·마지막 정상 커밋 유지, `StatusService`에 OverLimit push(FR-19 케이스 3 표면화 유도). 한도 이내 복귀 시 다음 사이클 자동 재개. (US-E2-08)
  - 전송 결과/오류는 `AuthTransport`가 부여한 CoreTypes 오류 taxonomy로 그대로 상위(SyncCycleCoordinator/RetryBackoffController)에 반환. **재시도 스케줄링은 소유하지 않음**(Q2=B — 코디네이터/U4 소관).
  - **동의 게이트는 소유하지 않음**: 업로드 차단은 `SyncCycleCoordinator`가 `ConsentGate`를 통해 사전 판정.
- **공개 인터페이스 요약**: `execute_cycle`(3~6단계 톱레벨), `check_runtime_limits`, `negotiate`(have/want), `transfer_wanted`, `transfer_blob`(청크+재검증+재개), `commit`.
- **커버**: US-E2-03, US-E2-04, US-E2-05, US-E2-06, US-E2-07, US-E2-08, US-E7-02(NFR-09 속성), US-E7-03(NFR-10 속성), FR-06, FR-07, FR-08, FR-09, FR-10, NFR-09, NFR-10, DEP-03 [blocked-on-server 목], DEP-04 [blocked-on-server: 서버 권위 재검증].

---

## U6 — 관측성 (Observability)

> **인터페이스 배치 규약 (§0 수정1 확정)**: 승인된 빌드 순서 U1→U4→U5→U2→U3→**U6**→**U7**에서 U2–U5는 U6보다 **먼저** 빌드되지만 로그를 남기고·상태를 밀어넣고·히스토리를 적재하고·중대 이벤트를 보고해야 한다. 따라서 이 표면들의 **싱크 인터페이스와 DTO는 공유 파운데이션 `CoreTypes`에 정의**한다 — `Logger`, `StatusSink`, `HistorySink`, `CriticalEventSink`, 그리고 상태 어휘 타입(`OperationalState`, `ActiveCondition`, `LivenessSignal`, `StatusSnapshot`). U6 컴포넌트는 이 인터페이스의 **구체 구현**이며 조립 루트(`WatcherDaemon`)가 하위 단위로 주입한다. 이로써 (a) push-only가 성립(U6는 U1–U5를 역참조하지 않음), (b) 빌드 순서 위반 없음(선행 단위는 `CoreTypes` 트레이트에만 의존).

### StructuredLogger
- **단위**: U6 Observability
- **목적**: 데몬의 활동·오류를 기계 판독 가능한 **JSON-line 구조화 로그**로 방출하는 주(primary) 관측 표면. GUI/트레이 없이 헤드리스에서 동일 동작.
- **책임**:
  - `CoreTypes::Logger` 파사드의 구체 구현. 모든 단위가 이 파사드를 통해 로그하고, 실제 직렬화·싱크 쓰기는 여기서 수행.
  - 각 레코드에 최소 `{timestamp, level, event, cycle_id, message, fields}` 필드 보장(상관관계=cycle_id).
  - 로그 파일 경로 · 최소 레벨 · 로테이션(크기/보존)을 `ConfigProvider` 설정으로 적용하고, config 리로드 시 재적용.
  - **로컬 평문 산출물**임을 전제(RISK-01) — 실패 시 오류 상세에 민감정보가 섞일 수 있음. 정리는 U7 `Uninstaller` 소관.
- **공개 인터페이스 요약**: `Logger` 구현(`log`/`event`) + `reload(LogConfig)`. 상세 로테이션·백프레셔 정책은 Functional Design 이월.
- **커버**: FR-17, NFR-15, RISK-01, US-E5-01

### StatusService
- **단위**: U6 Observability
- **목적**: **2축 상태 모델(Q9=B)** 의 단일 집계 지점. 운영 라이프사이클 `{idle|syncing|offline|paused}` + 동시 성립 가능한 활성 조건집합 `{AuthFailed, ConsentBlocked, OverLimit, VaultUnavailable, UpdateRolledBack}`. 각 단위가 **push-only**로 밀어넣고, 여기서 읽기(스냅샷/헬스/liveness)를 제공.
- **책임**:
  - `CoreTypes::StatusSink` 구현 — 하위 단위가 운영 상태 전이·조건 set/clear·마지막 성공 동기화 시각·**dirty 표시/재개 진행률**(FQ-2에 따라 "지속 큐 깊이" 대체)·startup liveness 신호를 밀어넣음.
  - `health_check()` **운영 헬스**(종료코드 계약의 근거, US-E5-02) — 운영 상태 + 활성 조건집합에서 healthy/unhealthy 도출. 종료코드로의 매핑은 `OperatorCli`가 수행.
  - `update_probe()` **순수 liveness**(프로세스 기동 + idle 도달 + 자격증명 소스 읽힘) — **(§0 노트3)** 운영 조건집합과 **분리**해 좋은 새 버전이 일시적 AuthFailed/OverLimit 때문에 오판 롤백되는 **롤백 루프를 방지**. liveness 신호는 startup 시 각 단위가 push한 값을 읽어 판정(외부 U5 호출 없음 → push-only 유지). `AutoUpdater`는 이 메서드만 소비.
  - `snapshot()` — CLI `status` 표면용 현재 2축 상태 + 부가 필드(마지막 성공 시각, dirty/재개 진행, 동의 상태, offline/backoff) 제공.
- **공개 인터페이스 요약**: `StatusSink`(push 뮤테이터) + `snapshot()` + `health_check()` + `update_probe()`. 조건→상태 매핑 규칙 상세는 Functional Design 이월.
- **커버**: US-E5-02, NFR-15, FR-19, FR-20, NFR-16

### UploadHistoryStore
- **단위**: U6 Observability
- **목적**: **append-only** 로컬 업로드 히스토리. 사후 검토·감사(P1 운영 / P2 감사 관점)용. CLI 질의(`--since`/`--status`/`--content-id`).
- **책임**:
  - `CoreTypes::HistorySink` 구현 — 업로드 사이클 종료(성공/실패/부분)마다 1건 append. 기존 레코드 **수정/삭제 불가**(불변식).
  - 레코드 = `{content_id/매니페스트 다이제스트, snapshot_hash, timestamp, status(Success|Failure|Partial), bytes_transferred, error_detail}`. 직렬화 라운드트립 무손실(NFR-13; 자유형 오류 문자열·유니코드 포함).
  - 시간/상태/content-id 기준 질의 제공.
  - 히스토리 파일 경로는 `ConfigProvider`로 설정. **로컬 평문**(RISK-01) — 정리는 `Uninstaller`.
- **공개 인터페이스 요약**: `append(record)` + `query(filter)`. 저장 포맷·인덱싱 상세는 Functional Design 이월.
- **커버**: FR-16, NFR-13, NFR-15, RISK-01, US-E5-03

### CriticalErrorNotifier
- **단위**: U6 Observability
- **목적**: **닫힌 4조건**만 능동 표면화하고 그 외 일시 오류는 로그 전용으로 남겨 알림 피로를 없앰(FR-19).
- **책임**:
  - `CoreTypes::CriticalEventSink` 구현 — 하위 단위/오케스트레이션이 4조건 이벤트를 보고: (1) 인증 실패/HTTP 401, (2) **N회 연속** 사이클 실패(N config, 기본 3 — 연속 카운터를 여기서 유지), (3) 로컬 프리플라이트 limit-exceeded, (4) 자동 업데이트 롤백.
  - 표면화 3중 액션: **상향 심각도 구조화 로그**(`StructuredLogger`) + **`StatusService`에 해당 활성 조건 raise**(→ `health_check()` unhealthy 전환 + CLI status 반영) + **선택적 데스크톱 팝업**(`TrayIndicator`; 부재해도 표면화 막지 않음).
  - **(§0 수정2)** `TrayIndicator` 의존은 **선택적/nullable no-op 싱크**로 주입받는다 — 트레이 부재(헤드리스)여도 로그+헬스+CLI status 표면화(US-E5-04)는 항상 성립.
  - 4조건 밖 이벤트는 표면화 없이 로그 전용 유지, 헬스 healthy 유지.
- **공개 인터페이스 요약**: `report_auth_failure` / `report_cycle_result` / `report_preflight_exceeded` / `report_update_rollback`(best-effort, infallible). 임계 판정·디바운스·중복 억제 상세는 Functional Design 이월.
- **커버**: FR-19, NFR-15, US-E5-04

### TrayIndicator
- **단위**: U6 Observability
- **목적**: 데스크톱 세션에서만 쓰는 **선택적** 시스템 트레이/메뉴바 상태 아이콘(FR-18, confirm-or-drop). 헤드리스/비활성 시 no-op.
- **책임**:
  - 활성(config on) + 데스크톱 세션에서 `StatusService` 스냅샷을 반영해 idle/syncing/offline/error 아이콘 렌더.
  - `CriticalErrorNotifier`가 보낸 데스크톱 팝업을 best-effort로 표시.
  - **어떤 스토리도 트레이 존재에 의존하지 않음** — 부재/실패는 관측(로그+CLI+헬스)에 영향 없음(§0 수정2 nullable no-op 싱크로 주입). 채택/제거는 Functional Design에서 최종 확정.
- **공개 인터페이스 요약**: `start(TrayConfig) -> Option<Handle>`(헤드리스/비활성 시 None) + `render(snapshot)` + `notify(msg)` + `stop()`. 플랫폼별 아이콘/메뉴 상세는 Functional Design 이월.
- **커버**: FR-18, NFR-15, US-E5-05

---

## U7 — 수명주기 · 서비스 패키징 & CLI (Lifecycle, Service Packaging & CLI)

### ServiceManager
- **단위**: U7 Lifecycle, Service Packaging & CLI
- **목적**: Watcher를 **OS 네이티브 백그라운드 서비스**로 설치·자동시작(FR-21). 3-OS 통일 CLI 계약의 백엔드.
- **책임**:
  - 현재 OS에 맞는 서비스 유닛 등록/해제 — macOS launchd / Linux systemd / Windows Service. 부팅/로그인 자동시작 구성.
  - `install/uninstall/start/stop/status/restart` 를 세 OS에서 **동일 계약**으로 제공. `restart`는 `AutoUpdater`가 업데이트 후 재기동에 사용.
  - 서비스 실행 계정/작업 디렉터리/자동시작 여부를 `ConfigProvider`에서 취득(`ServiceSpec`).
- **공개 인터페이스 요약**: `install(ServiceSpec)` / `uninstall()` / `start()` / `stop()` / `restart()` / `status() -> ServiceRegistration`. 플랫폼별 유닛 파일 템플릿·권한 상세는 Functional Design / Infrastructure Design 이월.
- **커버**: FR-21, NFR-05, US-E6-01

### AutoUpdater
- **단위**: U7 Lifecycle, Service Packaging & CLI
- **목적**: 자동 업데이트 + **`update_probe()` 기반 헬스 게이트 자동 롤백**(FR-20/NFR-16). 방치형 데몬이 나쁜 업데이트로 조용히 죽지 않게 함.
- **책임**:
  - 채널 config에 따라 업데이트 확인/적용(새 버전 스테이징 → `ServiceManager.restart()` 로 재기동).
  - 업데이트 후 헬스 게이트: bounded time 내 `StatusService.update_probe()` 통과 여부 판정 — (1) 기동, (2) idle 도달, (3) 자격증명 소스 읽힘. 실패/타임아웃 시 **직전 정상 버전으로 자동 롤백**. **(§0 노트3)** `health_check()`가 아니라 `update_probe()`만 소비.
  - 롤백 시 `CriticalErrorNotifier.report_update_rollback`(FR-19 케이스 4) + 구조화 로그.
  - **롤백 백오프/보류 상태 지속**으로 재시도 루프 방지(Q9=B가 `update_probe`를 운영 헬스와 분리한 이유와 정합).
- **공개 인터페이스 요약**: `check_for_update() -> Option<UpdateInfo>` / `apply_update(UpdateInfo) -> UpdateOutcome{Committed|RolledBack}` / `rollback(Version)`. 다운로드 검증·아티팩트 레이아웃·게이트 폴링 상세는 Functional Design / Infrastructure Design 이월.
- **커버**: FR-20, NFR-16, NFR-05, US-E6-02

### Uninstaller
- **단위**: U7 Lifecycle, Service Packaging & CLI
- **목적**: 깨끗한 제거/폐기 — 로컬 평문 산출물 + 저장 토큰 삭제 + 서비스 등록 해제(US-E6-04). 볼트 원본 불변, **idempotent**, **데몬 비실행 상태에서도 동작**.
- **책임**:
  - `ServiceManager.uninstall()` 로 서비스 등록 해제(및 중지 확인).
  - 로컬 평문 산출물 삭제 — 업로드 히스토리(U6 `UploadHistoryStore` 산출물), 매니페스트/SyncState(U4 `SyncStateStore` 산출물, 마지막 커밋 매니페스트+dirty+재개 오프셋), 구조화 로그(U6). **데몬이 꺼져 있어도 되도록** 라이브 컴포넌트 호출이 아니라 `ConfigProvider`로 해소한 **경로 기반 삭제**를 사용.
  - 토큰 제거 — config/env 필드 + (선택 경로 사용 시) OS 보안 저장소는 U5 `CredentialProvider`에 위임.
  - 볼트 원본 파일 절대 미접촉. 부분 실패(권한 등) 시 무엇이 남았는지 보고. 이미 삭제된 항목은 no-op.
- **공개 인터페이스 요약**: `uninstall(UninstallOptions) -> UninstallReport{removed, skipped(reason)}`. 산출물 경로 목록 확정·플랫폼별 토큰 삭제 상세는 Functional Design 이월.
- **커버**: US-E6-04, RISK-01, NFR-05, FR-13, FR-16, FR-17

### RunStateController
- **단위**: U7 Lifecycle, Service Packaging & CLI
- **목적**: 실행 상태 제어 — pause/resume + sync-now + stop(FR-01/FR-06/FR-21, US-E6-03). paused 상태 **재시작 후에도 지속**.
- **책임**:
  - 데몬의 권위 있는 **run-state 보유자**(running|paused, sync_requested, stop_requested). Q2=B 단일 직렬 사이클 모델에서 오케스트레이션(`SyncCycleCoordinator`/`WatcherDaemon`)이 이 상태를 **읽어** 동작(역방향 의존 회피 — 오케스트레이션 → RunStateController).
  - `pause`/`resume`: 감시/트리거 억제. paused를 `ConfigProvider`(상태 파일)로 지속하고 `StatusService`에 push.
  - `request_sync_now`: 디바운스 대기 없이 즉시 사이클 트리거 신호(멱등 FR-10이므로 변경 없으면 no-op 커밋).
  - `request_stop`: 진행 중 작업을 안전히 마무리/지속 상태 보존 후 종료하도록 셧다운 신호.
- **공개 인터페이스 요약**: `pause()` / `resume()` / `request_sync_now()` / `request_stop(StopMode)` / `current() -> RunState`. 셧다운 유예·신호 전달 메커니즘 상세는 Functional Design 이월.
- **커버**: FR-21, FR-01, FR-06, US-E6-03

### OperatorCli
- **단위**: U7 Lifecycle, Service Packaging & CLI
- **목적**: 운영자 CLI 명령 집합 — GUI 없는 데몬의 사실상 운영자 API. `--json` 기계 판독 출력 + 헬스 종료코드 계약.
- **책임**:
  - 서브커맨드 파싱/디스패치: `status`/`health`/`pause`/`resume`/`sync-now`/`stop`/`history`/`consent`/`reload`/`install`/`uninstall`.
  - **실행 중 데몬 대상 명령**(status/health/pause/resume/sync-now/stop/history/consent/reload)은 `ControlPlane` **클라이언트**로 IPC 전송.
  - **서비스 수명주기 명령**(install/uninstall; 데몬 비실행 시에도 동작)은 `ServiceManager`/`Uninstaller` 직접 호출.
  - 결과를 사람용/`--json` 출력으로 렌더링하고, **`health` 결과를 프로세스 종료코드로 매핑**(0 healthy / 비정상 non-zero, US-E5-02 계약).
- **공개 인터페이스 요약**: `run(CliArgs) -> ExitCode` / `dispatch(Command) -> CliResult`. 명령별 인자 스키마·출력 포맷 상세는 Functional Design 이월.
- **커버**: NFR-15, NFR-05, US-E5-02, US-E5-03, US-E6-01, US-E6-03

### ControlPlane
- **단위**: U7 Lifecycle, Service Packaging & CLI
- **목적**: **로컬 IPC 서버/클라이언트(Q5=A)** — `OperatorCli` ↔ 실행 중 `WatcherDaemon` 통로. 포트 불필요, 소유자 권한 제한.
- **책임**:
  - 서버(데몬 측): Unix 도메인 소켓(mac/Linux) + 명명 파이프(Windows) 바인딩, **소유 사용자 권한으로 접근 제한**(파일/파이프 권한 + peer 자격증명), accept 루프.
  - **작은 버전드 요청/응답 프로토콜**(`ControlRequest`/`ControlResponse`에 프로토콜 버전 포함, 버전 불일치 처리).
  - 디스패치: status/health → `StatusService`; history → `UploadHistoryStore`; pause/resume/sync-now/stop → `RunStateController`; consent → U5 `ConsentGate`; reload → `ConfigProvider`.
  - 클라이언트(CLI 측): `connect`/`request` 로 요청 전송 — `OperatorCli`가 사용.
- **공개 인터페이스 요약**: 서버 `serve(IpcEndpoint)` / `handle(ControlRequest) -> ControlResponse` / `shutdown()`; 클라이언트 `connect(endpoint) -> ControlClient` / `request(req) -> ControlResponse`. 프레이밍·인증·버전 협상 상세는 Functional Design 이월.
- **커버**: NFR-15, NFR-05, US-E5-02, US-E6-03

---

> 오케스트레이션 서비스(`WatcherDaemon`, `SyncCycleCoordinator`) 정의는 `services.md`, 의존성 그래프·통신 패턴·데이터 흐름·네이밍 레지스트리·검증은 `component-dependency.md` 참조.
