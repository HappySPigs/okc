//! ProptestGenerators — U2 도메인 제약을 준수하는 `proptest` 제너레이터 모음.
//!
//! test-support 논리 단위이며 런타임 의존성 그래프 **밖**에 위치한다(비-default cargo feature
//! `proptest-support` 게이트, 릴리스 빌드 제외 — T9/PBT-07). U0 `Manifest`/`RelativePath` 등의
//! 제너레이터는 `foundation` 의 `proptest-support` 를 켜 **재사용**한다(단일 출처, 재작성 금지).

/// 도메인 제약 준수 제너레이터(이벤트 타임라인·스케줄러 명령·가드 입력 조합).
pub mod generators;
