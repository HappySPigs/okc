//! ProptestGenerators — U1 전용 도메인 제약 준수 `proptest` `Strategy` 제너레이터.
//!
//! test-support 논리 단위이며 런타임 의존성 그래프 **밖**에 위치한다. 비-default cargo feature
//! `proptest-support` 뒤에 게이트되어 릴리스 빌드 그래프에서 제외된다(PBT-07). U0
//! `proptest-support` 제너레이터(`arb_manifest_entry` 등)를 재사용하고, U1 컴포넌트가 산출하는
//! 값의 불변식(실제 계산된 `manifest_digest` 를 갖는 "digest-consistent" 매니페스트, 상관 매니페스트
//! 쌍, 임의 바이트열)을 만족하는 제너레이터를 추가로 제공한다.

/// U1 도메인 제너레이터(임의 바이트열·digest-consistent 매니페스트·상관 매니페스트 쌍).
pub mod generators;
