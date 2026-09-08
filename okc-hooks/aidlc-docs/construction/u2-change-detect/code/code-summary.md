# U2 change-detect — Code Summary (코드 요약)

**크레이트**: `change-detect` (lib) · **단계**: CONSTRUCTION -> Per-Unit Loop -> U2 -> Code Generation
**성격**: 변경 감지·트리거 발행·가드 판정. U2 는 순수 판정/신호 반환자다 — `StatusSink`/`Logger` 를 주입받지 않고 상태를 디스크에 지속하지 않는다(R-PURE-01/02). 관측 push·오케스트레이션은 U8 소관.

## 1. 구현된 컴포넌트 + 공개 API 표면

crate-root 재-export(`lib.rs`)로 3개 애그리게이트 + 공유 규약 노출:

- `FilesystemWatcher` — `start(root: &Path, backend: &dyn WatchBackend, t_debounce: Duration) -> Result<(FilesystemWatcher, TriggerStream), WatchError>`, `watch_state()`, `last_event_at() -> Option<Instant>`, `pause()`, `resume()`, `stop()`. `WatchBackend` 트레이트(`subscribe`) + 구현 `NotifyBackend`/`DegradedBackend`, 값 `RawFsEvent`(`Created`/`Modified`/`Deleted`/`Overflow`), `WatchState`, `Subscription`.
- `ReconciliationScheduler` — `new(t_recon)`, `run_startup_scan(now) -> TriggerSignal`, `tick(now, busy) -> Option<TriggerSignal>`, `record_result(outcome, at)`, `next_recon_due()`, `last_result()`; 값 `CycleOutcome`(`NoOp`/`Committed`/`Held`/`Failed`), `ReconResult`.
- `VaultAvailabilityGuard` — `new(confirm_empty: bool)`, `check_reachable(&self, root: &Path) -> Availability`, `guard_diff(new, last_committed, availability) -> GuardVerdict`; 값 `Availability`(`Reachable`/`RootMissing`/`Unmounted`/`Inaccessible`), `GuardVerdict`(`Proceed`/`HoldVaultUnavailable`/`HoldDestructiveEmpty`), `HoldReason`.
- 공유 규약 — `TriggerSignal`/`TriggerKind`(`Debounced`/`Reconciliation(ReconPhase)`)/`ReconPhase`/`TriggerStream`(= `mpsc::Receiver`), `WatchError`/`BackendCause`, config 뷰 `WatchConfig`/`ReconConfig`/`VaultConfig`/`ConfigViewError`, 순수 상태기계 `Debouncer` + `simulate_debounce`.

## 2. 모듈 레이아웃

`src/lib.rs`(재-export + `#![deny(missing_docs)]`), `trigger.rs`, `error.rs`, `config_views.rs`, `debounce.rs`(순수 상태기계), `watcher.rs`(I/O·스레드 경계), `scheduler.rs`, `guard.rs`, `proptest_support/{mod.rs, generators.rs}`. 순수 리프 모듈(`trigger`/`debounce`/`scheduler`/`guard`/`config_views`)은 `#![deny(clippy::unwrap_used, expect_used, indexing_slicing, panic)]` lint-gate. `watcher` 는 스레드/`notify` I/O 를 다뤄 lint-gate 비적용.

## 3. 외부 의존성

- `foundation` (path, U0): `RelativePath`(+`normalize`), `Manifest`, `Timestamp`.
- `notify` — OS 네이티브 감시(FSEvents/inotify/ReadDirectoryChangesW), `WatchBackend` 어댑터 뒤 캡슐화(공개 API 에 미노출).
- `thiserror` — 오류 파생. `std` — 스레드/`mpsc`/fs stat.
- `proptest` — optional, `proptest-support` feature 하 dev-only(`default = []`).

## 4. 적용된 MVP 축소

- 볼트-전체 단일 디바운스 타이머(D1) — 파일별 디바운스·rename 추적 없음. rename 은 delete+create 로 정규화(D2).
- 고정 duration/플래그(D4) — 적응형 튜닝 없음. `debounce_ms`/`reconciliation_interval_s` 는 `> 0` 검증만.
- 재조정은 async 미도입, 순수 `tick(now)` 판정자(D7) — 상위 데몬이 단조 시점 공급. `next_due` 는 발행-시각 앵커(완료 시각 재계산 안 함, R-RECON-04). busy 중 tick 무발행·무스택(R-RECON-06).
- `DegradedBackend` 무이벤트 폴백(D3/T7) — 네이티브 어댑터 부재 시 정확성은 재조정 백스톱(`<= T_recon`)이 보장.
- 백엔드 오버플로/rescan -> `Overflow` 로 정규화 후 합성 change(D6). `Paused` 는 이벤트 폐기(버퍼링 없음, D13).
- `Availability` 4 명명 조건(D9) — 세 unavailable 변이는 동일 HOLD, `Unmounted` 구분은 best-effort. 가드는 무상태·멱등(hold 미보유 -> 복구 시 자동 재개).
- 트리거는 `ChangeSet` 미탑재(FQ-2) — U8 이 재스냅샷. 트리거 채널은 `std::sync::mpsc`(순서 보존 단방향, D14).

## 5. 테스트 커버리지 (26건)

- 단위(`src/**` 인라인, 19건): `watcher`(버스트 합치기 스레드 e2e, `RootMissing`, degraded 무트리거) 3; `debounce`(단일/밀집 버스트/경계 gap 분할/overflow/빈 타임라인) 5; `scheduler`(startup 앵커/startup 전 tick/만료 발행/busy 무발행/record_result 불변) 5; `guard`(G1~G4 판정표 + 무상태 자동재개) 6.
- Property(`tests/prop_change_detect.rs`, `proptest-support` 게이트, 7건): PROP-U2-01/02(버스트당 정확히 1 트리거 + 발행 시각 상한), PROP-U2-03(overflow 삼킴 없음), PROP-U2-05(백엔드 무관 등가), PROP-U2-04/06(백스톱 상한 `next_due <= last_emit + T_recon` + busy 직렬화), PROP-U2-07(파괴적-빈-커밋 불가), PROP-U2-08(무상태 자동 재개 멱등) + 예제 emit-anchor 불변식 1.
- 제너레이터(`proptest_support/generators.rs`): `arb_event_timeline`(경과 ms 기반), `arb_sched_commands`(`SchedCmd`), `arb_guard_case`, `arb_availability`/`arb_cycle_outcome`; U0 `arb_manifest`/`arb_relative_path` 재사용.

## 6. 검증 사실 (확정)

전체 워크스페이스가 빌드되며 모든 크레이트 테스트가 통과한다 (foundation 32 / content-core 17 / change-detect 26 / sync-state 15 / auth-consent 35 / observability 24 = 총 149건, 실패 0). 그리고 `cargo clippy --all-targets --features proptest-support -- -D warnings` 가 모든 크레이트에서 CLEAN 이다.
