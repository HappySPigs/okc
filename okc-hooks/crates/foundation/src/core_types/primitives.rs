//! 원시 값 타입 — 콘텐츠 해시(`Sha256Digest`), 매니페스트 판별자(`ManifestDigest`),
//! 시각(`Timestamp`), 바이트 수(`ByteCount`).
//!
//! 모두 무손실 CBOR round-trip(NFR-13, US-E7-06) 대상이며 결정적 직렬화를 갖는다.
//! 순수 리프 모듈로서 panic-free-total(U0-NFR-REL-02)을 컴파일타임으로 강제한다.
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing, clippy::panic)]

use core::fmt;

use serde::{Deserialize, Serialize};

/// 파일 바이트에 대한 표준 SHA-256 다이제스트(= `sha256sum`, FQ-1=A).
///
/// 정확히 32바이트이며 동일 바이트열은 동일 다이제스트를 산출한다(결정성; 실제 계산은 U1 소유).
/// `Display` 는 소문자 16진(hex) 표현이다(표현 관심사; 저장/해시 입력은 raw 32바이트).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Sha256Digest([u8; 32]);

impl Sha256Digest {
    /// 32바이트 옥텟 배열로부터 다이제스트를 구성한다.
    pub fn from_bytes(bytes: [u8; 32]) -> Self {
        Sha256Digest(bytes)
    }

    /// 내부 32바이트 옥텟 배열 참조를 반환한다.
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

impl fmt::Display for Sha256Digest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for byte in &self.0 {
            write!(f, "{byte:02x}")?;
        }
        Ok(())
    }
}

/// 하나의 `Manifest` 전체에 대한 로컬 no-op 판별자(discriminator).
///
/// 정규-정렬된 `(relative_path, raw_sha256, size)` 3-튜플 시퀀스 위에서 계산되며(계산은 U1),
/// 권위 있는 `vault_content_id` 가 **아니다**(FQ-1=A, 서버 소유). `Sha256Digest` 와는
/// 의미가 달라 컴파일타임에 구별되는 별개 newtype 로 유지한다(혼용 차단).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct ManifestDigest([u8; 32]);

impl ManifestDigest {
    /// 32바이트 옥텟 배열로부터 매니페스트 다이제스트를 구성한다.
    pub fn from_bytes(bytes: [u8; 32]) -> Self {
        ManifestDigest(bytes)
    }

    /// 내부 32바이트 옥텟 배열 참조를 반환한다.
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

impl fmt::Display for ManifestDigest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for byte in &self.0 {
            write!(f, "{byte:02x}")?;
        }
        Ok(())
    }
}

/// UTC 전용 시점(instant). Unix epoch 기준 나노초로 저장되어 결정적·무손실 직렬화를 갖는다.
///
/// 자연 시간순 `Ord`(과거 -> 미래)이며 히스토리 질의(`--since`)·정렬에 사용된다.
/// 로컬 타임존은 표현 관심사이므로 저장하지 않는다. 외부 시간 크레이트에 의존하지 않는다.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Timestamp(i64);

impl Timestamp {
    /// Unix epoch(UTC) 기준 나노초로부터 시각을 구성한다.
    pub fn from_unix_nanos(nanos: i64) -> Self {
        Timestamp(nanos)
    }

    /// Unix epoch(UTC) 기준 나노초 값을 반환한다.
    pub fn as_unix_nanos(&self) -> i64 {
        self.0
    }
}

/// 바이트 수를 나타내는 파운데이션 프리미티브(비음, 64비트).
///
/// 바이트 수를 표현하는 모든 필드(`ManifestEntry.size`, `TransferResult` 의 bytes/resume_offset,
/// `StatusSnapshot.resume` 쌍 등)를 이 타입으로 일원화한다. 무손실 round-trip 대상(NFR-13).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct ByteCount(u64);

impl ByteCount {
    /// 원시 바이트 수로부터 구성한다.
    pub fn new(value: u64) -> Self {
        ByteCount(value)
    }

    /// 원시 바이트 수(`u64`)를 반환한다.
    pub fn get(&self) -> u64 {
        self.0
    }
}
