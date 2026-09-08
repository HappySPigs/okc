//! ProptestGenerators — U3 도메인 제약을 준수하는 `proptest` `Strategy` 제너레이터 모음.
//!
//! 이 모듈은 test-support 논리 단위이며 런타임 의존성 그래프 **밖**에 위치한다. 비-default cargo
//! feature `proptest-support` 뒤에 게이트되어 릴리스/프로덕션 빌드 그래프에서 제외된다(PBT-07).
//! U0 도메인 제너레이터(`Manifest`/`Sha256Digest`/`ByteCount`)는 `foundation::proptest_support::
//! generators` 를 재사용하며 재정의하지 않는다(드리프트 방지).

/// U3 도메인 제너레이터(blob 바이트·청크 크기·`server_has` 집합).
pub mod generators;
