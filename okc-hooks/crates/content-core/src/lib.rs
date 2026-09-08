#![deny(missing_docs)]
//! `content-core` 크레이트 (U1) — okc-hooks 동기화 사이클의 **결정성 콘텐츠 코어**.
//!
//! 5개 공개 컴포넌트를 노출한다:
//! - `ContentAddressing`: 파일 스트리밍 SHA-256(`hash_stream`) + 정렬 엔트리 다이제스트(`manifest_digest`).
//! - `VaultScanner`: 볼트 루트 열거(일관 시점 단일 패스) + 파일별 스트리밍 리더 — U1 유일 파일 I/O.
//! - `ManifestBuilder`: scan + hash 조립 -> 재계산된 `Manifest`(fail-fast).
//! - `ManifestDiffer`: 마지막 커밋 vs 현재 -> 정확 `ChangeSet`(merge-join, no-op 시 O(1) 빈 결과).
//! - `SafetyLimitsValidator`: 전송 전 3한도(총 <=20 GiB / 파일당 <=2 GiB / 파일 수 <=100k) 경계-정확 검증.
//!
//! 이 크레이트의 유일한 워크스페이스 의존은 `foundation`(U0)이며, `sha2`/`thiserror`/`std` 를
//! 외부·표준 의존으로 쓴다(`proptest` 는 `proptest-support` feature 하 dev-only).
//! U0 값 타입(`Manifest`/`ManifestEntry`/`ChangeSet`/`Sha256Digest`/`ManifestDigest`/`RelativePath`/
//! `ByteCount`)·CBOR 코덱·`RelativePath::normalize`·SafetyLimits 상수를 **소비만** 하며 재정의하지 않는다.
//!
//! 규약: 모든 doc 주석은 한국어, 코드 내 화살표는 ASCII `A -> B` 만 사용, Rust 타입/식별자는
//! doc 주석에서 백틱으로 감싼다(`Box<dyn Read>`, `Result<Manifest, BuildError>`).

/// ContentAddressing: 스트리밍 SHA-256 + 매니페스트 다이제스트(순수, I/O 없음).
pub mod content_addressing;

/// VaultScanner: 볼트 열거 + 스트리밍 리더 (U1 유일 파일 I/O).
pub mod scanner;

/// ManifestBuilder: scan + hash 조립 -> `Manifest` 재계산 (fail-fast).
pub mod builder;

/// ManifestDiffer: 마지막 커밋 vs 현재 -> `ChangeSet` (순수 merge-join).
pub mod differ;

/// SafetyLimitsValidator: 전송 전 3한도 경계-정확·단조 검증 (순수).
pub mod limits;

/// ErrorTaxonomy: U1 도입 운영 오류 타입(`ScanError`/`BuildError`, `thiserror` 파생).
pub mod error;

/// U1 도입 도메인 값 타입(`ScannedFile`/`VaultSnapshot`).
pub mod types;

/// ProptestGenerators: U1 전용 도메인 제너레이터 (test-support, 런타임 그래프 밖).
///
/// 비-default cargo feature `proptest-support` 뒤에 게이트되어 릴리스 빌드에서 제외된다.
#[cfg(feature = "proptest-support")]
pub mod proptest_support;

// ---------------------------------------------------------------------------
// 공개 API 표면 (crate-root 재-export). 재-export 아이템은 원본 doc 을 승계한다.
// ---------------------------------------------------------------------------

pub use content_addressing::{ContentAddressing, HASH_BUFFER_BYTES};
pub use scanner::VaultScanner;
pub use builder::ManifestBuilder;
pub use differ::ManifestDiffer;
pub use error::{BuildError, ScanError};
pub use limits::{LimitViolation, LimitVerdict, SafetyLimits, SafetyLimitsValidator};
pub use types::{ScannedFile, VaultSnapshot};
