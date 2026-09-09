# U4 sync-state — Code Summary (코드 요약)

**크레이트**: `sync-state` (lib) · **단계**: CONSTRUCTION -> Per-Unit Loop -> U4 -> Code Generation
**성격**: 회복력/재시도 단위. DAG 상 U0 `foundation` 에만 의존하는 소비자 말단(비순환). U0 sink/observer 계약을 구현하지 않고 `RetryDecision` 을 반환만 하며, status/notify push 는 U8 소관.

## 1. 구현된 컴포넌트 + 공개 API 표면

crate-root 재-export(`lib.rs`)로 상태 공유 없는 2개 애그리게이트 노출:

- `SyncStateStore` (유일 I/O·상태 보유) — `open_and_recover(cfg: &StateConfig) -> Result<Self, StateError>`, `last_committed_manifest() -> Option<&Manifest>`, `is_dirty()`, `mark_dirty()`, `commit_manifest(m: Manifest)`, `resume_offset(&Sha256Digest) -> Option<ByteCount>`, `persist_resume_offset(blob, off)`, `clear_resume_offsets()`. 값 `PersistedState`(`last_committed`/`dirty`/`resume_offsets`, `Serialize`+`Deserialize`), `ResumeOffsetMap`(= `BTreeMap<Sha256Digest, ByteCount>`), `StateConfig`, `RecoveredState`(`Empty`/`LastGood`), `StateError`(`CorruptRecovered`/`Io`/`Serde`).
- `RetryBackoffController` (순수·결정적 인메모리) — `new(cfg: &BackoffConfig, n: u32, seed: u64)`, `classify(err: &TransportError) -> RetryClass`(연관 함수), `on_failure(err, now: Instant) -> RetryDecision`, `on_success()`, `next_retry_at() -> Option<Instant>`, `consecutive_failures() -> u32`. 값 `RetryClass`(`Transient`/`Offline`/`Backpressure`/`AuthFailed`/`Permanent`), `JitterMode`(`Full`/`None`), `BackoffConfig`(+`Default`), `RetryDecision`(`retry_after`/`is_offline`/`escalate`).

## 2. 모듈 레이아웃

`src/lib.rs`(재-export + `#![deny(missing_docs)]`), `store.rs`(fs I/O 애그리게이트), `retry.rs`(순수 상태머신 + 내부 `SplitMix64` PRNG), `proptest_support/{mod.rs, generators.rs}`. `retry` 는 순수 리프라 `#![deny(clippy::unwrap_used, expect_used, indexing_slicing, panic)]` lint-gate. `store` 는 `std::fs` I/O 를 다뤄 lint-gate 비적용(단 프로덕션 경로 무-`unwrap`/`panic`).

## 3. 외부 의존성

- `foundation` (path, U0): `Manifest`, `Sha256Digest`, `ByteCount`, `encode`/`decode`, `CodecError`, `TransportError`/`TransportErrorClass`.
- `serde` — `PersistedState` 파생(CBOR round-trip).
- `thiserror` — `StateError` 파생. `std` — fs·시간.
- `proptest` — optional, `proptest-support` feature 하 dev-only(`default = []`); dev-deps 에서 `foundation/proptest-support` 재사용. U4 는 신규 외부 크레이트를 0건 도입한다.

## 4. 적용된 MVP 축소

- 최신-상태-대체 모델(FQ-2) — 커밋 이력 없이 `last_committed` 1개만 보관. `dirty` 는 누적 이벤트 큐가 아닌 단일 boolean(R-STATE-05).
- 프로세스 내 단일 writer 전제(R-STATE-03) — 잠금/멀티-writer 없음.
- 세 필드는 개별 저장하지 않고 한 번의 원자 쓰기로 함께 교체(R-STATE-02/06). 원자 파이프라인 = `encode -> 같은 디렉터리 temp -> fsync -> atomic rename -> 부모 dir fsync(best-effort)`. 부모 dir fsync 는 non-Unix 에서 no-op.
- 손상/절단/잔존-temp 는 패닉 없이 복구 — 최종 파일 손상 시 빈 상태 + `dirty=true` 재기록 후 `CorruptRecovered`(비치명 신호, 복잡 복구 없음). 잔존 temp 는 정리 후 last-good 채택.
- 재시도는 순수·결정적 — 주입 시계(`now: Instant`) + 주입 seed(D-13). 전역 시계/async 없음. `Permanent` 는 전송 경로에서 산출되지 않는 예약 변형. `classify` 는 5원 total 매핑. full jitter 는 비암호 `SplitMix64`.
- `AuthFailed` 는 재시도 안 함(U5 위임, 카운터 불변). `Offline`/`Backpressure` 는 에스컬레이션 제외; `escalate` 는 `Transient && consecutive >= N` 에서만. `base_delay` 는 NaN/무한/음수/초과를 모두 `cap` 으로 절단(비감소·유계).

## 5. 테스트 커버리지 (15건)

- 예제/앵커(`tests/example_sync_state.rs`, feature 무관, 10건): 빈 상태 open, commit+resume 재-open 생존, latest-state-wins, 잔존 temp 정리·last-good 채택, 손상 -> empty+dirty 복구; classify total 매핑, 백오프 단조·유계(1/2/4/8/8/8) + `on_success` 리셋, N 임계 escalate(Transient 전용), Offline/Backpressure 무-escalate, `AuthFailed` 카운터 불변·무-retry.
- Property(`tests/prop_sync_state.rs`, `proptest-support` 게이트, 5건): PROP-U4-03(`PersistedState` CBOR round-trip), PROP-U4-06(classify 전역성 + U0 매핑 오라클, non-`Permanent`), PROP-U4-05/07(백오프 상태머신 불변식 `retry_after <= cap`, `is_offline => !escalate`, escalate 게이트) + 지터 없는 비감소, PROP-U4-01/02/04(참조 모델 대조 명령 시퀀스 + `Recover`/`RecoverWithresidualTemp` 크래시 삽입 + latest-wins).
- 제너레이터(`proptest_support/generators.rs`): `arb_persisted_state`, `arb_resume_offsets`, `arb_backoff_config`(경계 cap/multiplier), `arb_jitter_mode`; U0 `arb_manifest`/`arb_sha256_digest`/`arb_byte_count`/`arb_transport_error` 재사용.

## 6. 검증 사실 (확정)

전체 워크스페이스가 빌드되며 모든 크레이트 테스트가 통과한다 (foundation 32 / content-core 17 / change-detect 26 / sync-state 15 / auth-consent 35 / observability 24 = 총 149건, 실패 0). 그리고 `cargo clippy --all-targets --features proptest-support -- -D warnings` 가 모든 크레이트에서 CLEAN 이다.
