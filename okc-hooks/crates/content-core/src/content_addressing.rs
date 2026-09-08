//! ContentAddressing — 파일 콘텐츠 스트리밍 SHA-256 + 정렬 엔트리 매니페스트 다이제스트.
//!
//! 두 순수 연산을 제공한다(파일시스템 의존 없음):
//! - `hash_stream`: 임의 `Read` 스트림을 고정 버퍼로 반복 읽어 표준 SHA-256(`Sha256Digest`)을 산출.
//!   결과는 `sha256sum` 과 바이트 일치한다(R-CA-01/02/03, NFR-08). U3 전송 재검증이 재사용한다.
//! - `manifest_digest`: canonical 정렬된 `ManifestEntry` 시퀀스를 injective 길이-프리픽스 프레이밍으로
//!   직렬화해 표준 SHA-256(`ManifestDigest`)을 산출(R-CA-04/05). U0 CBOR 코덱과 분리된 안정 콘텐츠 지문.
//!
//! 순수 표면으로서 panic-free-total(U1-NFR-REL-07)을 컴파일타임 clippy lint-gate 로 강제한다.
//! I/O 실패(`hash_stream`)는 패닉이 아니라 `io::Error` `Result` 로 표면화한다.
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing, clippy::panic)]

use std::io::{self, Read};

use foundation::{ManifestDigest, ManifestEntry, Sha256Digest};
use sha2::{Digest, Sha256};

/// `hash_stream` 이 사용하는 고정 읽기 버퍼 크기(64 KiB) — 파일 크기와 무관한 상수 메모리(NFR-02, D3).
pub const HASH_BUFFER_BYTES: usize = 64 * 1024;

/// 결정성 콘텐츠 주소화 연산의 네임스페이스(무상태 유닛 타입).
pub struct ContentAddressing;

impl ContentAddressing {
    /// 임의 `Read` 스트림의 콘텐츠에 대한 표준 SHA-256(`Sha256Digest`)을 계산한다.
    ///
    /// 고정 크기 버퍼(`HASH_BUFFER_BYTES`)로 EOF 까지 반복 읽어 증분 해시하므로 파일 전체를
    /// 메모리에 적재하지 않는다(R-CA-03). 청크 분할 경계와 무관하게 동일 바이트열은 동일 다이제스트를
    /// 산출하며(R-CA-02), 빈 스트림은 SHA-256 표준 빈-입력 다이제스트를 낸다. 읽기 I/O 실패는
    /// `io::Error` 로 반환한다(패닉 없음).
    pub fn hash_stream<R: Read>(mut reader: R) -> Result<Sha256Digest, io::Error> {
        let mut hasher = Sha256::new();
        let mut buffer = [0u8; HASH_BUFFER_BYTES];
        loop {
            let read = reader.read(&mut buffer)?;
            if read == 0 {
                break;
            }
            // `buffer.get(..read)` 는 인덱싱 연산자를 쓰지 않아 lint-gate 를 통과하며,
            // `read <= HASH_BUFFER_BYTES` 이므로 항상 `Some` 이다.
            if let Some(chunk) = buffer.get(..read) {
                hasher.update(chunk);
            }
        }
        Ok(finalize_sha256(hasher))
    }

    /// canonical 정렬된 엔트리 시퀀스에 대한 `ManifestDigest`(로컬 no-op 판별자)를 계산한다.
    ///
    /// 엔트리를 `relative_path` 바이트 사전식 오름차순(tie-break `(raw_sha256, size)`)으로 정렬한 뒤
    /// 각 엔트리를 길이-프리픽스 프레이밍(`len(path)` BE `u64` + path 바이트 + `raw_sha256` 32B +
    /// `size` BE `u64`)으로 직렬화·concat 해 표준 SHA-256 에 흘린다(R-CA-04). 정렬을 계산 내부에서
    /// 강제하므로 발견/삽입 순서와 무관하게 동일 다이제스트를 낸다(R-CA-05). 길이 프리픽스가
    /// `["ab","c"]` vs `["a","bc"]` 경계 모호성을 제거해 injective 인코딩을 보장한다.
    pub fn manifest_digest(entries: &[ManifestEntry]) -> ManifestDigest {
        let mut sorted: Vec<&ManifestEntry> = entries.iter().collect();
        sorted.sort();
        let mut hasher = Sha256::new();
        for entry in sorted {
            let path_bytes = entry.relative_path.as_str().as_bytes();
            let path_len = path_bytes.len() as u64;
            hasher.update(path_len.to_be_bytes());
            hasher.update(path_bytes);
            hasher.update(entry.raw_sha256.as_bytes());
            hasher.update(entry.size.get().to_be_bytes());
        }
        let digest = finalize_sha256(hasher);
        ManifestDigest::from_bytes(*digest.as_bytes())
    }
}

/// 완료된 `Sha256` 상태에서 32바이트 `Sha256Digest` 를 추출한다(인덱싱·패닉 매크로 없음).
fn finalize_sha256(hasher: Sha256) -> Sha256Digest {
    let output = hasher.finalize();
    let mut bytes = [0u8; 32];
    // `output` 은 정확히 32바이트이므로 `copy_from_slice` 는 길이 불일치 없이 성공한다.
    bytes.copy_from_slice(output.as_slice());
    Sha256Digest::from_bytes(bytes)
}
