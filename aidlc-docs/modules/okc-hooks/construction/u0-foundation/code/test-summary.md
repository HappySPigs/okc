# 테스트 커버리지 요약 (PBT + example)

**파일**: `crates/foundation/tests/prop_foundation.rs`(PBT), `crates/foundation/tests/example_units.rs`(example), 제너레이터 `crates/foundation/src/proptest_support/generators.rs`.
**feature 게이팅**: PBT 크레이트 전체는 `#![cfg(feature = "proptest-support")]` 뒤에 게이트된다. 기본 빌드에서는 빈 크레이트로 컴파일되고, `cargo test --features proptest-support` 로만 property 가 실행된다(프로덕션 그래프에 proptest 미유입, MNT-02/PBT-07). example 테스트는 feature 없이 실행된다.

---

## 1. Property-Based Tests (PROP-* 매핑)

- **PROP-DE-01 / PROP-BR-01 / PROP-BL-01 (codec round-trip, US-E7-06 매핑)** — `roundtrip!` 매크로가 14개 CBOR 타입에 대해 `decode(encode(v)) == v` 를 인스턴스화: `RelativePath`, `Sha256Digest`, `ManifestDigest`, `Timestamp`, `ByteCount`, `ManifestEntry`, `Manifest`, `ChangeSet`, `SyncState`, `TransportError`, `ClassifiedError`, `TransferResult`, `StatusSnapshot`, `TokenSecret`(값-보존). 이 군이 **US-E7-06** 수용 증거.
- **PROP-DE-03 (`normalize` 멱등)** — `prop_normalize_idempotent`: `normalize(normalize(p)) == normalize(p)` + 결과 상대/`.`/`..` 세그먼트 없음.
- **no-panic 불변식 (REL-02)** — `prop_decode_arbitrary_no_panic`(임의 바이트열 `decode` 무패닉, `Manifest`/`SyncState`/`TransferResult`), `prop_normalize_arbitrary_no_panic`(임의 문자열 `normalize` 무패닉).
- **PROP-BR-04 (`is_retryable` total)** — `prop_is_retryable_total`: 전 변형에서 패닉 없이 bool 반환.
- **PROP-BR-05 (strict unknown-key reject)** — `prop_unknown_key_reject`: 주입된 모든 미지/오타 키가 이슈 목록에 `UnknownKey` 로 전수 포함되며 REJECT.
- **PROP-BR-02 (config parse round-trip)** — `prop_config_roundtrip`: `validate(serialize(cfg)) == cfg`(토큰 값-보존 포함); `prop_invalid_config_rejected`: invalid JSON 은 `Err`.
- **PROP-BR-03 / PROP-BL-04 (reload 멱등 + keep-last-good)** — `prop_reload_idempotent_keep_last_good`: 동일 유효 config 2회 reload 시 `current()` 동일(멱등), 이후 invalid reload 는 `Err` + 이전 스냅샷 보존(부분 적용 없음). temp 파일 기반.

### 제너레이터(`generators.rs`) 요약
- 도메인 불변식 준수: `arb_relative_path`(정규화 통과만), `arb_manifest`(canonical 정렬 + 경로 유일), `arb_transfer_result`(`Partial.resume_offset <= bytes` 보장), `arb_detail`(빈/유니코드/개행 경계), `arb_timestamp`/`arb_byte_count`(0/1/MIN/MAX 경계), 전-variant 열거(`arb_error_class`/`arb_transport_error_class`/`arb_sync_state`/`arb_log_level`/상태 어휘).
- config: `arb_valid_watcher_config`/`arb_valid_config_json`(절대 vault + https + notify>=1), `arb_config_with_unknown_keys`(미지/오타 키 주입), `arb_invalid_config_json`(bad scheme/상대 vault/필수 누락).

---

## 2. Example Unit Tests (`example_units.rs`, MNT-03/Q13=A)

비즈니스-크리티컬 경로는 property + example 을 둘 다 보유(코어 경로 PBT-only 금지):
- **2-pass Q5=C**: `two_pass_lists_all_unknown_keys`(미지 키 전수 수집, field 위반 미수집, `issues().len() == 2`), `two_pass_first_field_violation_after_clean`(clean 후 첫 field 위반: 상대 `vault_path`, `notify=0`).
- **keep-last-good**: `reload_keeps_last_good_on_invalid`(유효 load 후 invalid reload -> `Err` + `vault_path` 이전 스냅샷 유지).
- **observer catch_unwind 격리**: `observer_panic_is_isolated`(`PanicObserver` + `CountingObserver`; 패닉해도 카운터 1 증가 + `current()` 무손상).
- **https-only 거부**: `https_only_enforced`(`https` OK, `http` -> `NotHttps`, scheme-less -> `Malformed`, config 경로에서 http -> `server_endpoint` field 위반).
- **token redaction**: `token_is_redacted_but_serialize_preserves_value`(`Debug`=`TokenSecret(***)`, `Display`=`***`, encode/decode 값-보존).
- **is_retryable 매핑**: `is_retryable_mapping_is_fixed`(R-CLASS-01/02 매핑 표 고정).
- **RelativePath 거부 케이스**: `relative_path_rejects_invalid_inputs`(절대/드라이브/`..`/`.`/빈값 거부 + 정상 정규화).
- **Manifest 정규 정렬 / ChangeSet::is_empty**: `manifest_entries_sort_canonically_by_path`, `change_set_is_empty_semantics`.

---

## 3. 위임 및 이연

- **하류 위임(생성/실행)**: PROP-DE-02 / PROP-BL-02 / PROP-BL-03(digest shuffle-invariance + diff apply oracle) -> **U1**; PROP-DE-04 / PROP-BR-06 / PROP-BL-05(SyncState stateful) -> **U4**. U0 는 제너레이터/모델 계약만 제공하며 stateful 실행 property 를 포함하지 않는다.
- **PBT-08 이연**: case count/shrink tuning/fixed-seed vs seed-logging/CI 통합은 **Build-and-Test** 단계로 이연.
- **실행 이연**: cargo/rustc 미설치로 `cargo test --features proptest-support` 실제 실행은 Build-and-Test 로 이연(테스트 코드는 Rust 1.85 에서 clean 컴파일되도록 작성됨).
