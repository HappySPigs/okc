//! ProptestGenerators — U7a 도메인 제약을 준수하는 `proptest` `Strategy` 제너레이터 모음.
//!
//! 이 모듈은 test-support 논리 단위이며 런타임 의존성 그래프 **밖**에 위치한다. 비-default
//! cargo feature `proptest-support` 뒤에 게이트되어 릴리스/프로덕션 빌드 그래프에서 제외된다
//! (PBT-07). U0 도메인 제너레이터(`Version` 등)는 재정의하지 않고, U7a 고유 입력(게이트 시간
//! 파라미터, 아티팩트 트리 스펙, liveness 시퀀스)만 노출한다.

/// U7a 도메인 제너레이터(게이트 config / 아티팩트 스펙 / liveness 시퀀스 / 버전).
pub mod generators;
