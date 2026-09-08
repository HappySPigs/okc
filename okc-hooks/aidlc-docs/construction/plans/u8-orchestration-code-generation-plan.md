# U8 Orchestration Code Generation Plan

**단계**: CONSTRUCTION -> W5 -> U8 Orchestration -> Code Generation
**패키지**: `crates/watcher-bin` (단일 배포 바이너리 + 테스트 가능한 내부 라이브러리 표면)
**컴포넌트**: `WatcherDaemon`, `SyncCycleCoordinator`, `SingleInstanceLock`
**계획 권위**: 이 문서는 U8 Code Generation의 단일 실행 기준이다.

## 1. 단위 컨텍스트

- U8의 1차 소유 스토리는 0개다. 조립 책임은 E1-E7 전체 런타임 경로를 관통하며, 특히 U8에 배치된 `SingleInstanceLock`이 US-E1-05/FR-23을 실현한다.
- 의존 단위는 U0, U1, U2, U3, U4, U5, U6, U7a, U7b 전부다. 하위 크레이트는 U8을 역참조하지 않는다.
- NFR Requirements/Design은 사용자 지시로 생략됐다. 구현 결정은 U8 Functional Design 3종, 실제 하위 공개 API, 기존 워크스페이스 관례에 근거한다.
- 고정 제약: 단일 직렬 사이클, latest-trigger coalescing, `sync-now` 채널 이벤트, 명시적 logger reload, federated config typed injection, advisory file lock, graceful shutdown, AutoUpdater background loop MVP 미배선.

## 2. 실행 단계

### Step 1 - 컨텍스트와 계약 검증

- [x] U8 Functional Design 3종을 읽는다.
- [x] U8 unit/dependency/story map을 읽는다.
- [x] 아홉 하위 크레이트의 실제 생성자, 트레이트, 오류, 값 타입을 대조한다.
- [x] 활성 Resiliency/PBT 규칙과 비활성 Security 상태를 확인한다.

### Step 2 - 패키지와 워크스페이스 배선

- [ ] `crates/watcher-bin/Cargo.toml`을 생성한다.
- [ ] 아홉 내부 크레이트를 모두 런타임 의존으로 선언한다.
- [ ] `serde`/`serde_json`/`thiserror`와 신호 처리용 최소 의존을 선언한다.
- [ ] `proptest`를 비기본 `proptest-support` feature 뒤에 둔다.
- [ ] 루트 workspace members에 `crates/watcher-bin`을 추가한다.

### Step 3 - Federated config 해소

- [ ] U0/U5/U6/U8 키의 정확한 union을 제공한다.
- [ ] U2/U3/U4/U6/U7a/U8 비-core 값을 raw JSON에서 검증해 타입드 값으로 해소한다.
- [ ] 누락값에는 문서화된 MVP 기본값과 플랫폼 data-dir 기본을 적용한다.
- [ ] 0/음수/잘못된 타입/잘못된 경로/잘못된 backoff를 시작 오류로 거부한다.
- [ ] non-core live reload가 재시작 반영이라는 MVP 경계를 보존한다.

### Step 4 - SingleInstanceLock

- [ ] `LockConfig`, `LockRecord`, `InstanceInfo`, `LockGuard`, `LockError`를 구현한다.
- [ ] OS advisory `File::try_lock`으로 live holder를 거부하고 PID/owner를 보고한다.
- [ ] free/stale 파일 획득 시 현재 레코드를 원자적으로 덮어쓴다.
- [ ] `LockGuard` drop으로 잠금을 해제한다.

### Step 5 - 교차 크레이트 어댑터와 제어 핸들러

- [ ] `SharedSyncStore`가 U3 `SyncStore`와 U8 coordinator store seam을 같은 U4 store에 위임한다.
- [ ] `VaultBlobSource`가 U1 `VaultScanner::open_reader`를 U3 `BlobSource`로 어댑트한다.
- [ ] 테스트 가능한 manifest/guard/consent/driver/run-state seam을 정의하고 실 구현을 연결한다.
- [ ] `DaemonControlHandlers`가 대부분의 op를 U7b `WatcherHandlers`에 위임한다.
- [ ] `sync_now`는 U8 channel에 edge event를 보내며 U7b sticky flag를 사용하지 않는다.
- [ ] `reload`는 U0 reload 성공 뒤 U6 logger reload를 명시적으로 체이닝한다.

### Step 6 - SyncCycleCoordinator

- [ ] trigger cause logging과 monotonic cycle id를 구현한다.
- [ ] paused, vault-unavailable, destructive-empty, no-op, consent-blocked 분기를 우선순위대로 구현한다.
- [ ] 실제 업로드 전 dirty를 지속하고 U3 driver를 정확히 한 번 호출한다.
- [ ] 성공/실패를 status/history/critical sinks에 표면화한다.
- [ ] `UploadError::Transport`만 U4 backoff에 전달하고 나머지 변형은 재예약하지 않는다.
- [ ] 단일 receiver와 drain-to-latest로 사이클 비중첩을 보장한다.
- [ ] retry deadline과 shutdown/stop을 bounded polling으로 관측한다.

### Step 7 - WatcherDaemon과 trigger threading

- [ ] config -> lock -> state recovery -> U6 -> 하위 단위 -> control plane 순서로 조립한다.
- [ ] startup reconciliation을 live watcher 전에 enqueue한다.
- [ ] filesystem forwarder와 periodic reconciliation timer를 U8 trigger channel에 연결한다.
- [ ] SIGINT/SIGTERM과 IPC stop을 `ShutdownFlag`로 합류한다.
- [ ] 종료 시 watcher와 producer thread를 정리하고 lock을 마지막에 drop한다.
- [ ] AutoUpdater background source/loop는 명시적 MVP 미배선으로 유지한다.

### Step 8 - 얇은 main 진입점

- [ ] `run` 서비스 경로와 운영자 CLI 경로를 같은 바이너리에서 분기한다.
- [ ] 전역 `--config`를 해소하고 CLI에는 U7b parser/dispatcher를 재사용한다.
- [ ] CLI 경로에 native IPC client와 U7a-backed `NativeServiceOps`를 주입한다.
- [ ] 결과 메시지와 프로세스 종료코드를 그대로 표면화한다.

### Step 9 - 예제 기반 테스트

- [ ] lock 획득/live 충돌/drop 재획득을 검증한다.
- [ ] config 기본값/검증/known-key union을 검증한다.
- [ ] control handler의 sync-now edge와 reload chain을 검증한다.
- [ ] coordinator의 paused/hold/no-op/blocked/success/failure 주요 분기를 fake seam으로 고정한다.
- [ ] latest-trigger coalescing과 shutdown stop을 검증한다.

### Step 10 - Property-Based Testing

- [ ] PROP-U8-01 lock free/stale/live/drop 불변식을 도메인 입력으로 검증한다.
- [ ] PROP-U8-02 shutdown 이후 신규 cycle 없음과 in-flight 완결 모델을 검증한다.
- [ ] PROP-U8-03 wiring DAG의 역엣지/순환 부재를 검증한다.
- [ ] PROP-U8-04 cycle 분기의 배타성과 driver 0회/1회를 검증한다.
- [ ] PROP-U8-05 burst coalescing이 최신 trigger 하나만 남기고 중첩을 만들지 않음을 검증한다.
- [ ] PROP-U8-06 hold 판정별 status/log push를 검증한다.
- [ ] PROP-U8-07 federated raw/typed 의미와 timeout/수치 검증을 검증한다.
- [ ] example 테스트와 property 테스트를 별도 파일로 유지한다(PBT-10).

### Step 11 - 문서와 실제 검증

- [ ] `aidlc-docs/construction/u8-orchestration/code/code-summary.md`를 작성한다.
- [ ] `cargo fmt --all -- --check`를 통과한다.
- [ ] `cargo build --workspace`를 통과한다.
- [ ] `cargo test --workspace --features proptest-support`를 통과한다.
- [ ] `cargo clippy --workspace --all-targets --features proptest-support -- -D warnings`를 통과한다.
- [ ] 콘텐츠 검증과 dependency direction 검사를 수행한다.
- [ ] 상태/감사 기록을 갱신하고 Build-and-Test로 전환한다.

## 3. 산출 경로

- 애플리케이션 코드: `crates/watcher-bin/`
- workspace 배선: `Cargo.toml`, `Cargo.lock`
- 코드 요약: `aidlc-docs/construction/u8-orchestration/code/code-summary.md`
- 다음 단계: `aidlc-docs/construction/build-and-test/`

## 4. 확장 준수 계획

- Resiliency: 단일 바이너리, 단일 인스턴스, timeout/backoff, 상태 복구, graceful shutdown을 실제 배선으로 검증한다. 클라우드 HA/DR/auto-scaling은 로컬 단일 사용자 데몬에 해당 없음으로 기록한다.
- Property-Based Testing: PROP-U8-01..07을 `proptest`로 구현하고 shrinking/seed 재현성을 기본 설정 그대로 유지한다.
- Security Baseline: 비활성. 기존 token redaction 및 owner-only IPC 계약은 하위 구현을 그대로 보존한다.

## 5. 승인/실행 기록

- 기존 autopilot 지시로 per-stage gate가 면제되어 있다.
- 2026-09-08 사용자 입력 "현재 AIDLC 단계 확인해서 해당 단계부터 시작해"를 현재 단계 계속 실행 승인으로 기록한다.
