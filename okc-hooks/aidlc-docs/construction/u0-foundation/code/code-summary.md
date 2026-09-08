# U0 Foundation — Code Summary (생성 코드 요약)

**단계**: CONSTRUCTION -> Per-Unit Loop -> U0 Foundation -> Code Generation (Part 2, STEP 21)
**작성일**: 2026-09-08
**크레이트**: `foundation` (lib) · **공개 애그리게이트**: `CoreTypes`(순수 값/함수/계약 트레이트), `ConfigProvider`(JSON 로드·검증 + 활성 스냅샷 + 관찰자 팬아웃)
**주의**: 이 문서는 마크다운 요약이며 코드가 아니다. 애플리케이션 코드는 워크스페이스 루트 `crates/foundation/` 에 위치한다.

---

## 1. 생성 파일 목록 (경로)

**워크스페이스/매니페스트**
- `Cargo.toml` — `[workspace]` resolver 2, members `crates/foundation`; `[workspace.package]` Edition 2024 + pinned MSRV `1.85`; `[workspace.dependencies]` serde/ciborium/serde_json/url/thiserror/arc_swap/proptest 시리즈 핀.
- `crates/foundation/Cargo.toml` — `foundation` lib 멤버 매니페스트. 런타임 deps 는 workspace 상속, `proptest` 는 optional dependency, `[features] proptest-support = ["dep:proptest"]`(NON-DEFAULT, default 비어 있음).

**크레이트 루트**
- `crates/foundation/src/lib.rs` — 크레이트 루트. `#![deny(missing_docs)]`, 모듈 선언(`core_types`/`config`/`#[cfg] proptest_support`), 공개 재-export(STEP 19), `FOUNDATION_CONFIG_KEYS` 재-export.

**CoreTypes 애그리게이트 (`crates/foundation/src/core_types/`)**
- `mod.rs` — 애그리게이트 모듈 루트 + 서브모듈 공개 재-export.
- `path.rs` — PathNormalizer: `RelativePath`, `PathError`.
- `primitives.rs` — 원시 값 타입: `Sha256Digest`, `ManifestDigest`, `Timestamp`, `ByteCount`.
- `error.rs` — ErrorTaxonomy: `ErrorClass`, `TransportErrorClass`, `TransportError`, `ClassifiedError`, `TransferResult`, `CodecError`.
- `token.rs` — TokenSecret: redacting newtype.
- `sync_state.rs` — SyncStateModel: `SyncState`(+ T1..T8 전이 모델 doc).
- `status.rs` — StatusVocab: `OperationalState`, `ActiveCondition`, `LivenessSignal`, `ConsentState`, `StatusSnapshot`, `Health`, `HealthReason`, `Liveness`.
- `sink.rs` — SinkContracts: `Logger`, `StatusSink`, `HistorySink`, `CriticalEventSink`, `ConfigReloadObserver`, `ReadJudgment` + placeholder 레코드 타입(`LogLevel`/`LogRecord`/`LogFields`/`CycleId`/`UploadHistoryRecord`/`CycleOutcome`/`LimitReport`/`Version`/`RollbackReason`).
- `codec.rs` — Codec: `encode`/`decode`(ciborium).

**ConfigProvider 애그리게이트 (`crates/foundation/src/config/`)**
- `mod.rs` — 애그리게이트 공개 표면 + 서브모듈 재-export.
- `model.rs` — `WatcherConfig`, `ConfigSnapshot`, `FOUNDATION_CONFIG_KEYS`, R-LIMIT-01 상수(`MAX_VAULT_TOTAL_BYTES`/`MAX_FILE_BYTES`/`MAX_FILE_COUNT`).
- `url_validator.rs` — UrlValidator: `validate_https_url`, `UrlValidationError`.
- `validator.rs` — ConfigValidator: 2-pass `validate`, `ConfigError`(thiserror, 이슈 `Vec`), `ConfigIssue`.
- `loader.rs` — ConfigLoader: `resolve_config_path`(R-DISCOVER-01), `read_and_validate`.
- `observer.rs` — ObserverRegistry: 불변 순서 `Vec<Arc<dyn ConfigReloadObserver>>` + `catch_unwind` fan-out.
- `store.rs` — ConfigStore/ConfigProvider: `ArcSwapOption<WatcherConfig>` + `reload_mutex`.

**ProptestGenerators (`crates/foundation/src/proptest_support/`, feature 게이트)**
- `mod.rs` — 모듈 루트(test-support, 런타임 그래프 밖).
- `generators.rs` — 도메인 제약 준수 `proptest` `Strategy` 제너레이터.

**테스트 (`crates/foundation/tests/`)**
- `prop_foundation.rs` — PBT(PROP-* 매핑), `#![cfg(feature = "proptest-support")]`.
- `example_units.rs` — 결정적 example 유닛 테스트(proptest 없이 실행).

---

## 2. 모듈별 책임 요약

- **`lib.rs`**: 크레이트 루트 lint-gate(`#![deny(missing_docs)]`), 모듈 배선, 두 애그리게이트 공개 표면 crate-root 재-export.
- **`core_types` (CoreTypes)**: 무상태·무IO 순수 값 타입 + 순수 함수(`encode`/`decode`, `is_retryable`, `normalize`, `is_empty`) + 하류 주입 계약 트레이트. 각 서브모듈은 순수 표면으로 module-level clippy lint-gate(`unwrap_used`/`expect_used`/`indexing_slicing`/`panic` deny) 적용.
- **`config` (ConfigProvider)**: JSON 로드 -> 2-pass 검증 -> 활성 스냅샷 스왑 -> 관찰자 팬아웃. `store.rs` 만 상태 보유(`ArcSwap`)로 순수 lint-gate 미적용, 나머지 순수 서브모듈(`model`/`url_validator`/`validator`)은 lint-gate 적용, `loader`(파일 IO/env) 는 미적용.
- **`proptest_support`**: `proptest-support` feature 뒤 게이트된 도메인 제너레이터. 릴리스 그래프 제외(MNT-02/PBT-07).

---

## 3. 공개 표면 요약 (crate-root 재-export)

**CoreTypes 값 타입**: `RelativePath`, `Sha256Digest`, `ManifestDigest`, `Timestamp`, `ByteCount`, `ManifestEntry`, `Manifest`, `ChangeSet`, `SyncState`, `ErrorClass`, `TransportErrorClass`, `TransportError`, `ClassifiedError`, `TransferResult`, `OperationalState`, `ActiveCondition`, `LivenessSignal`, `ConsentState`, `StatusSnapshot`, `Health`, `HealthReason`, `Liveness`, `TokenSecret`, `LogLevel`, `LogRecord`, `LogFields`, `CycleId`, `UploadHistoryRecord`, `CycleOutcome`, `LimitReport`, `Version`, `RollbackReason`.
**CoreTypes 함수/오류**: `encode`, `decode`, `CodecError`, `PathError`.
**CoreTypes 계약 트레이트**: `Logger`, `StatusSink`, `HistorySink`, `CriticalEventSink`, `ConfigReloadObserver`, `ReadJudgment`.
**ConfigProvider 표면**: `ConfigProvider`(`new(known_keys)`/`subscribe`/`load(cli_path)`/`current() -> ConfigSnapshot`/`reload() -> Result<(), ConfigError>`), `WatcherConfig`, `ConfigSnapshot`, `ObserverRegistry`, `ConfigError`, `ConfigIssue`, `UrlValidationError`, 함수 `validate`/`read_and_validate`/`resolve_config_path`/`validate_https_url`, 상수 `FOUNDATION_CONFIG_KEYS`/`MAX_VAULT_TOTAL_BYTES`/`MAX_FILE_BYTES`/`MAX_FILE_COUNT`.

> 재-export 아이템은 원본 정의의 doc 주석을 승계하므로 `#![deny(missing_docs)]` 하에서도 별도 doc 불필요.

---

## 4. STEP 22.2 매핑 표 (논리 컴포넌트 -> 파일/모듈 -> 규칙 -> NFR id -> PBT 속성)

| 논리 컴포넌트 | 파일/모듈 | 규칙(R-*) | NFR id | PBT 속성 |
|---|---|---|---|---|
| PathNormalizer | `core_types/path.rs` | (정규화 규칙) | REL-02 | PROP-DE-03 |
| (원시 값 타입) | `core_types/primitives.rs` | R-CODEC-01 | REL-01 | PROP-DE-01 |
| ErrorTaxonomy | `core_types/error.rs` | R-CLASS-01/02/03 | REL-01, REL-02, MNT-01 | PROP-BR-04 |
| TokenSecret | `core_types/token.rs` | R-TOKEN-01/02 | SEC-02 | PROP-BR-02 |
| SyncStateModel | `core_types/sync_state.rs` | T1..T8 | REL-01 | PROP-DE-01 (DE-04/BL-05 -> U4) |
| StatusVocab | `core_types/status.rs` | (Q9 2축) | REL-01 | PROP-DE-01 (독립 없음) |
| (Manifest 값 모델) | `core_types/manifest.rs` | R-NOOP-01/02 | REL-01 | PROP-DE-01/BL-01 (DE-02/BL-02/BL-03 -> U1) |
| Codec | `core_types/codec.rs` | R-CODEC-01 | REL-01, REL-02, PERF-01 | PROP-DE-01/BR-01/BL-01 |
| SinkContracts | `core_types/sink.rs` | R-OBSERVER-02 | SEC-02 | 없음 |
| ConfigLoader | `config/loader.rs` | R-DISCOVER-01, R-RELOAD-03/04 | USE-01 | (PROP-BR-05 흐름) |
| ConfigValidator | `config/validator.rs` | R-CFG-STRICT-01, R-LIMIT-01 | USE-01, MNT-01 | PROP-BR-05 |
| UrlValidator | `config/url_validator.rs` | (§4.1 https) | SEC-01 | 없음(BR-05 흡수) |
| ConfigStore | `config/store.rs` | R-RELOAD-01/02/05, R-TRIGGER-01, R-OBSERVER-03 | REL-03, REL-05 | PROP-BR-02/BR-03/BL-04 |
| ObserverRegistry | `config/observer.rs` | R-OBSERVER-01/02/03, R-RELOAD-05 | REL-04 | 없음 |
| ProptestGenerators | `proptest_support/` | (feature `proptest-support`) | MNT-02 | (전 property 지원) |

---

## 5. 빌드/검증 상태 (정적 리뷰 verdict + 실행 이연)

- **cargo/rustc 미설치**: 이 환경에는 cargo/rustc 가 설치되어 있지 않으므로 `cargo build`/`cargo test`/`cargo clippy` 의 **실제 실행은 Build-and-Test 단계로 이연**된다. 코드는 Rust 1.85(Edition 2024)에서 clean 컴파일되도록 작성되었다.
- **정적 리뷰 verdict**: 크로스-모듈 타입/시그니처 정합성 및 계획서 STEP 대비 구현 일치성에 대해 정적 리뷰를 수행했으며, U0 슬라이스는 계획서(single source of truth)의 STEP 1..20 을 준수하는 것으로 확인되었다. 확정 크레이트 외 의존성 미도입, `#![deny(missing_docs)]` 및 순수 모듈 clippy lint-gate 배치, redacting `TokenSecret`, 2-pass 검증(Q5=C), keep-last-good + `catch_unwind` fan-out, federated known-key 주입이 모두 코드에 반영됨.
- **이연 항목**: 정확한 patch 핀, MSRV CI 검증, PBT-08(case count/shrink/seed/CI 통합) 은 Build-and-Test 로 이연.
