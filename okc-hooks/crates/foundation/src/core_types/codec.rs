//! Codec — 내부 지속 상태의 CBOR 인코딩/디코딩(ciborium).
//!
//! `encode`/`decode` 는 파일 IO 없이 순수 바이트 변환만 한다(저장 IO 는 U4/U5/U6 소관).
//! `decode` 는 잘림/손상/적대적 CBOR 바이트열에도 **패닉하지 않으며** 실패는 오직
//! `CodecError`(Fatal)로 표면화된다(U0-NFR-REL-02). R-CODEC-01 무손실 round-trip 불변식의
//! 실행 지점이다(`decode(encode(v)) == v`, NFR-13, US-E7-06).
//!
//! 순수 리프 모듈로서 panic-free-total 을 컴파일타임으로 강제한다.
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing, clippy::panic)]

use serde::Serialize;
use serde::de::DeserializeOwned;

use super::error::CodecError;

/// 값을 CBOR 바이트열로 인코딩한다(버퍼드 `Vec<u8>`, 파일 IO 없음).
///
/// 인코딩 실패는 `CodecError::Encode` 로 표면화한다.
pub fn encode<T: Serialize>(value: &T) -> Result<Vec<u8>, CodecError> {
    let mut buffer: Vec<u8> = Vec::new();
    ciborium::into_writer(value, &mut buffer).map_err(|e| CodecError::Encode(format!("{e:?}")))?;
    Ok(buffer)
}

/// CBOR 바이트열을 대상 타입으로 디코딩한다.
///
/// 잘림/손상/적대적 입력에도 패닉하지 않고, 실패는 `CodecError::Decode` 로만 표면화한다.
pub fn decode<T: DeserializeOwned>(bytes: &[u8]) -> Result<T, CodecError> {
    ciborium::from_reader(bytes).map_err(|e| CodecError::Decode(format!("{e:?}")))
}
