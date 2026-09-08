//! SafetyLimitsValidator — 전송 전 3한도 경계-정확·단조 검증 (순수).
//!
//! 총 볼트 크기 <= 20 GiB / 파일당 <= 2 GiB / 파일 수 <= 100k 를 검사한다. 한도 값은 U0 컴파일타임
//! 상수(`MAX_VAULT_TOTAL_BYTES`/`MAX_FILE_BYTES`/`MAX_FILE_COUNT`)를 참조하며 재정의하지 않는다
//! (R-LIMIT-01/D13). 경계-정확(`actual <= limit` 수용, `> limit` 만 거부, R-LIMIT-02), 첫 위반
//! short-circuit(단일 `LimitViolation`, R-LIMIT-04/D15), 총량 누적은 `saturating_add` 로
//! 오버플로 패닉을 방지한다(R-LIMIT-05).
//!
//! 순수 표면으로서 panic-free-total 을 컴파일타임 clippy lint-gate 로 강제한다.
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing, clippy::panic)]

use foundation::{MAX_FILE_BYTES, MAX_FILE_COUNT, MAX_VAULT_TOTAL_BYTES, Manifest, RelativePath};

/// 전송 전 프리플라이트 3한도의 묶음 뷰. 값은 U0 컴파일타임 상수이며 config 로 변경 불가.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SafetyLimits {
    /// 총 볼트 크기 상한(바이트) — 기본 `MAX_VAULT_TOTAL_BYTES`(20 GiB).
    pub max_total_bytes: u64,
    /// 파일당 크기 상한(바이트) — 기본 `MAX_FILE_BYTES`(2 GiB).
    pub max_file_bytes: u64,
    /// 파일 수 상한 — 기본 `MAX_FILE_COUNT`(100,000).
    pub max_file_count: u64,
}

impl SafetyLimits {
    /// U0 상수로 고정된 기본 한도를 반환한다(D13, Q2=A).
    pub const fn defaults() -> Self {
        SafetyLimits {
            max_total_bytes: MAX_VAULT_TOTAL_BYTES,
            max_file_bytes: MAX_FILE_BYTES,
            max_file_count: MAX_FILE_COUNT,
        }
    }
}

impl Default for SafetyLimits {
    fn default() -> Self {
        SafetyLimits::defaults()
    }
}

/// 한도 위반의 종류·값(actionable, FR-05). `Exceeded` 는 첫 발견 위반 하나만 담는다(MVP, D15).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LimitViolation {
    /// 총 볼트 크기 초과.
    TotalBytes {
        /// 초과된 한도 값(바이트).
        limit: u64,
        /// 실제(초과) 총량(바이트, saturating 누적).
        actual: u64,
    },
    /// 특정 파일이 파일당 한도 초과.
    FileBytes {
        /// 초과한 파일의 볼트 상대경로.
        path: RelativePath,
        /// 초과된 한도 값(바이트).
        limit: u64,
        /// 실제(초과) 파일 크기(바이트).
        actual: u64,
    },
    /// 파일 수 초과.
    FileCount {
        /// 초과된 한도 값(파일 수).
        limit: u64,
        /// 실제(초과) 파일 수.
        actual: u64,
    },
}

/// 3한도 검사 판정. `WithinLimits` 는 세 한도 모두 경계 내, `Exceeded` 는 정확히 하나의 위반을 지목한다.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LimitVerdict {
    /// 세 한도 모두 경계 내(`actual <= limit`).
    WithinLimits,
    /// 첫 발견 위반 하나를 지목(short-circuit).
    Exceeded(LimitViolation),
}

/// 3한도 검증 연산의 네임스페이스(무상태 유닛 타입).
pub struct SafetyLimitsValidator;

impl SafetyLimitsValidator {
    /// 매니페스트를 3한도로 검사해 `LimitVerdict` 를 반환한다(순수, 첫 위반 short-circuit).
    ///
    /// 검사 순서(R-LIMIT-05): (1) `FileCount` O(1), (2) 엔트리 순회하며 각 파일 `FileBytes` +
    /// `saturating_add` 누적 후 `TotalBytes`. `actual == limit` 은 수용, `> limit` 만 거부한다
    /// (경계-정확, R-LIMIT-02). 어떤 파일/바이트를 추가해도 판정은 `WithinLimits` 로 뒤집히지 않는다
    /// (단조, R-LIMIT-03).
    pub fn validate(manifest: &Manifest) -> LimitVerdict {
        let limits = SafetyLimits::defaults();

        let count = manifest.entries.len() as u64;
        if count > limits.max_file_count {
            return LimitVerdict::Exceeded(LimitViolation::FileCount {
                limit: limits.max_file_count,
                actual: count,
            });
        }

        let mut total: u64 = 0;
        for entry in &manifest.entries {
            let size = entry.size.get();
            if size > limits.max_file_bytes {
                return LimitVerdict::Exceeded(LimitViolation::FileBytes {
                    path: entry.relative_path.clone(),
                    limit: limits.max_file_bytes,
                    actual: size,
                });
            }
            total = total.saturating_add(size);
            if total > limits.max_total_bytes {
                return LimitVerdict::Exceeded(LimitViolation::TotalBytes {
                    limit: limits.max_total_bytes,
                    actual: total,
                });
            }
        }

        LimitVerdict::WithinLimits
    }
}
