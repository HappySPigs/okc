//! ProptestGenerators — 도메인 제약을 준수하는 `proptest` `Strategy` 제너레이터 모음.
//!
//! 이 모듈은 test-support 논리 단위이며 런타임 의존성 그래프 **밖**에 위치한다. 비-default
//! cargo feature `proptest-support` 뒤에 게이트되어 릴리스/프로덕션 빌드 그래프에서 제외된다
//! (MNT-02/PBT-07). 크레이트 내부 property 테스트와 하류 단위(U1/U4 등)의 stateful PBT 가
//! 여기서 노출한 제너레이터를 재사용해 도메인 불변식을 위반하지 않는 입력을 생성한다.
//!
//! U0 는 제너레이터/모델 계약만 제공하며 stateful 실행 property 를 포함하지 않는다:
//! digest shuffle-invariance/diff apply oracle(PROP-DE-02/BL-02/BL-03) 은 U1,
//! `SyncState` stateful(PROP-DE-04/BR-06/BL-05) 은 U4 가 소유·실행한다.

/// 도메인 제약 준수 제너레이터(`RelativePath`/`Manifest`/오류 분류/`WatcherConfig` 등).
pub mod generators;
