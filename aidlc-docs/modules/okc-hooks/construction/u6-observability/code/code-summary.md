# U6 observability — Code Summary (코드 요약)

**크레이트**: `observability` (lib) · **단계**: CONSTRUCTION -> Per-Unit Loop -> U6 -> Code Generation
**성격**: push-only 관측 계층. U0 `foundation` 이 계약으로 소유한 sink 트레이트를 구현하고, 하위 단위(U1~U5/U8)가 `Arc<dyn _>` 로 주입받아 값을 push 한다. U6 는 값을 받아 방출/집계/판정/지속만 한다(R-PUSH-01). 유일한 내부 의존은 `foundation`.

## 1. 구현된 컴포넌트 + 공개 API 표면

4개 실체 싱크 + no-op 트레이 어댑터(`lib.rs` 재-export):

- `StructuredLogger` (`Logger` + `ConfigReloadObserver`) — `new(cfg: LoggerConfig, clock, provider)`, `log(record)`/`event(...)`(infallible), `reload() -> Result<(), LogError>`. `serde_json` JSON-line 방출 + 방출 시각 stamping + 레벨 필터 + 크기 rotation. 값 `EmittedLogLine`(`Serialize`+`Deserialize`).
- `StatusService` (`StatusSink` + `ReadJudgment`) — `new()`, sink 뮤테이터(`set_operational`/`raise_condition`/`clear_condition`/`record_sync_success`/`set_dirty`/`set_resume_progress`/`set_liveness`), `set_failure_escalation(bool)`(U6 내부), `snapshot() -> StatusSnapshot`, `health_check() -> Health`, `update_probe() -> Liveness`. 단일 `Mutex<StatusInner>` 집계.
- `UploadHistoryStore` (`HistorySink`) — `new(path, logger: Option<Arc<dyn Logger>>)`, `append(record)`(infallible), `query(filter: HistoryQuery) -> Result<Vec<UploadHistoryRecord>, HistoryError>`. 값 `HistoryQuery`(`since`/`only_failures`, AND, `matches`).
- `CriticalErrorNotifier` (`CriticalEventSink`) — `new(logger, status, tray, threshold)`, `report_auth_failure`/`report_cycle_result`/`report_preflight_exceeded`/`report_update_rollback`. 닫힌 4 진입점, 각 3중 fan-out.
- `TrayIndicator` — MVP no-op(`new`/`start -> Ok(None)`/`render`/`notify`/`stop`). 값 `TrayConfig`/`TrayHandle`/`NotificationMessage`/`TrayError`.
- seam `Clock`(+`SystemClock`), config 뷰 `ObservabilityConfig`/`LoggerConfig` + `OBSERVABILITY_CONFIG_KEYS`, 오류 `LogError`/`HistoryError`(`thiserror`).

## 2. 모듈 레이아웃

`src/lib.rs`(재-export + `#![deny(missing_docs)]`), `logger.rs`, `status.rs`, `history.rs`, `critical.rs`, `tray.rs`, `clock.rs`, `config.rs`, `error.rs`, `proptest_support/{mod.rs, generators.rs}`. 순수/무동작 리프(`clock`? seam / `config`/`tray`/`error`)는 `#![deny(clippy::unwrap_used, expect_used, indexing_slicing, panic)]` lint-gate. `Mutex`/파일 I/O 보유 모듈(`logger`/`status`/`history`/`critical`)은 비적용(단 poison 시 `into_inner` 회복, 패닉 없음).

## 3. 외부 의존성

- `foundation` (path, U0): sink 트레이트(`Logger`/`StatusSink`/`HistorySink`/`CriticalEventSink`/`ReadJudgment`/`ConfigReloadObserver`), 값 타입(`LogRecord`/`StatusSnapshot`/`UploadHistoryRecord`/`Health`/`Liveness`/`ActiveCondition`/`OperationalState`/`Timestamp` 등), `ConfigProvider`, `encode`/`decode`, `CodecError`.
- `serde` + `serde_json` — `EmittedLogLine` JSON-line 직렬화.
- `thiserror` — 오류 파생. `std` — 파일 I/O·`Mutex`.
- `proptest` — optional, `proptest-support` 하 dev-only(`default = []`).

## 4. 적용된 MVP 축소

- `TrayIndicator` 는 MVP no-op(D1) — 3-OS 트레이 백엔드/트레이 크레이트 미도입(GUI-less 데몬 모델). `tray_enabled` 는 파싱만. 트레이 부재는 로그+status fan-out(비의존 2개)을 막지 않는다.
- 로그 rotation 은 크기 임계 + 보존 개수 근사(D2). `chrono`/`time` 등 신규 시간 크레이트 미도입 — `Clock` seam + U0 `Timestamp` 재사용(D4).
- push 표면은 best-effort·infallible·비차단(R-PUSH-02); 오류를 표면화하는 것은 읽기/재적용 경로(`query`/`reload`)뿐(R-PUSH-03).
- U6 는 비코어 config 를 `ConfigProvider` 로 읽지 않음 — U8 이 RESOLVED TYPED `ObservabilityConfig`/`LoggerConfig` 를 생성자 주입. 코어 필드(`log_level`/`notify_consecutive_failures`)만 `ConfigProvider` 경유. live-reload 대상은 `log_level` 뿐(rotation 파라미터는 재시작 반영).
- 히스토리 포맷 = 고정폭 `u64` LE length prefix + U0 CBOR payload. truncated-tail 관용(트레일링 부분 프레임 -> 정상 prefix 반환), 파일 중간 손상만 `HistoryError::Corrupt`(R-HIST-04).
- health flip 은 닫힌 3집합(`AuthFailed`/`OverLimit`/`UpdateRolledBack`) + escalation 플래그에서만 — `VaultUnavailable`/`ConsentBlocked` 는 flip 안 함(알림 피로 회피). `update_probe` 는 두 liveness 신호에만 의존(축1/축2 격리). 연속실패 escalation 은 임계 도달 순간 1회 발화, Success 시 리셋(R-CRIT-02).

## 5. 테스트 커버리지 (24건)

- 단위(`src/**` 인라인, 15건): status(닫힌3집합 health flip/probe 격리/snapshot offline·consent 파생/조건 멱등 집합) 4; logger(JSON-line 왕복/레벨 필터/`TokenSecret` redaction/bad-path 비치명/크기 rotation·보존 개수) 5; history(append 순서 무손실/only_failures·since 필터/truncated-tail prefix/파일 부재 빈결과) 4; critical(auth_failure fan-out/escalation 임계 1회 발화·Success 리셋) 2.
- Property(`tests/prop_observability.rs`, `proptest-support` 게이트, 9건): PROP-U6-DE-01(`UploadHistoryRecord` round-trip), PROP-U6-DE-02(`EmittedLogLine` JSON-line round-trip·개행 없음), PROP-U6-BR-01(레벨 필터 `emitted iff level >= active`), PROP-U6-BR-02(2축 독립성 오라클), PROP-U6-BR-04(health 오라클 = 닫힌3집합 ∩ 조건 또는 escalation), PROP-U6-BR-05(`update_probe` 격리), PROP-U6-BR-07(연속실패 상태머신 참조모델 대조), PROP-U6-BR-08(append-only + query AND 오라클), PROP-U6-BR-09(truncated-tail 관용 prefix).
- 제너레이터(`proptest_support/generators.rs`): `arb_upload_history_record`/`arb_log_record`/`arb_log_fields`/`arb_history_query`/`arb_status_commands`(`StatusCommand`); U0 제너레이터 재사용.

## 6. 검증 사실 (확정)

전체 워크스페이스가 빌드되며 모든 크레이트 테스트가 통과한다 (foundation 32 / content-core 17 / change-detect 26 / sync-state 15 / auth-consent 35 / observability 24 = 총 149건, 실패 0). 그리고 `cargo clippy --all-targets --features proptest-support -- -D warnings` 가 모든 크레이트에서 CLEAN 이다.
