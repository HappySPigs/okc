//! ProptestGenerators — U6 고유 타입의 도메인 제약 준수 `proptest` `Strategy` 제너레이터.
//!
//! 이 모듈은 test-support 논리 단위이며 런타임 의존성 그래프 **밖**에 위치한다. 비-default cargo
//! feature `proptest-support` 뒤에 게이트되어 릴리스/프로덕션 빌드 그래프에서 제외된다(PBT-07).
//!
//! U0 소유 타입(`Timestamp`/`ByteCount`/`LogLevel`/`ActiveCondition`/... 자유형 `detail`)의
//! 제너레이터는 U0 `proptest-support` 를 dev 에서 켜 재사용한다(재작성 금지). 이 모듈은 U6 가
//! 확정한 타입(`UploadHistoryRecord` 3필드 스키마·방출 `LogRecord`·상태/report 시퀀스)만 노출한다.

/// U6 고유 타입 제너레이터(`UploadHistoryRecord`/`LogRecord`/`HistoryQuery` 등).
pub mod generators;
