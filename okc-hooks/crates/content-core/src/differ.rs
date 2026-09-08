//! ManifestDiffer — 마지막 커밋 vs 현재 `Manifest` -> 정확 `ChangeSet` (순수 merge-join).
//!
//! `manifest_digest` 동일 시 O(1) 빈 `ChangeSet` 조기 반환(R-DIFF-04), 아니면 두 canonical 정렬
//! 시퀀스를 투 포인터 merge-join 으로 단일 패스 비교해 `added`/`modified`/`deleted` 를 계산한다
//! (R-DIFF-01/02/03/05). rename 감지는 없다 — 이름 변경은 deleted+added 로 표현된다(MVP, D4).
//! `last_committed` 는 U4 소유 인자이며 U1 은 지속하지 않는다(R-DIFF-06).
//!
//! 순수 표면으로서 panic-free-total 을 컴파일타임 clippy lint-gate 로 강제한다.
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing, clippy::panic)]

use std::cmp::Ordering;

use foundation::{ChangeSet, Manifest};

/// 정확 diff 연산의 네임스페이스(무상태 유닛 타입).
pub struct ManifestDiffer;

impl ManifestDiffer {
    /// `last_committed` 대비 `current` 의 정확 `ChangeSet` 을 계산한다.
    ///
    /// 두 다이제스트가 같으면 빈 `ChangeSet` 을 O(1) 로 반환한다(R-DIFF-04). 그 외에는 canonical
    /// 정렬(U0 불변식)을 전제로 투 포인터 merge-join 을 수행한다:
    /// - `current` 에만 있는 경로 -> `added`(current 엔트리).
    /// - 양쪽에 있으나 `(raw_sha256, size)` 가 다른 경로 -> `modified`(current 엔트리).
    /// - `last_committed` 에만 있는 경로 -> `deleted`(경로만).
    ///
    /// 세 목록은 서로소이며 각각 `relative_path` 오름차순으로 결정적 정렬된다(merge-join 특성).
    pub fn diff(last_committed: &Manifest, current: &Manifest) -> ChangeSet {
        // R-DIFF-04: no-op 조기 종료(전체 엔트리 비교 없이 O(1)).
        if last_committed.manifest_digest == current.manifest_digest {
            return ChangeSet {
                added: Vec::new(),
                modified: Vec::new(),
                deleted: Vec::new(),
            };
        }

        let mut added = Vec::new();
        let mut modified = Vec::new();
        let mut deleted = Vec::new();

        let mut prior = last_committed.entries.iter().peekable();
        let mut curr = current.entries.iter().peekable();

        loop {
            match (prior.peek(), curr.peek()) {
                (Some(prior_entry), Some(curr_entry)) => {
                    match prior_entry
                        .relative_path
                        .cmp(&curr_entry.relative_path)
                    {
                        Ordering::Less => {
                            deleted.push(prior_entry.relative_path.clone());
                            prior.next();
                        }
                        Ordering::Greater => {
                            added.push((**curr_entry).clone());
                            curr.next();
                        }
                        Ordering::Equal => {
                            // R-DIFF-02: raw_sha256 또는 size 차이로 modified 판정(mtime 미사용).
                            if prior_entry.raw_sha256 != curr_entry.raw_sha256
                                || prior_entry.size != curr_entry.size
                            {
                                modified.push((**curr_entry).clone());
                            }
                            prior.next();
                            curr.next();
                        }
                    }
                }
                (Some(prior_entry), None) => {
                    deleted.push(prior_entry.relative_path.clone());
                    prior.next();
                }
                (None, Some(curr_entry)) => {
                    added.push((**curr_entry).clone());
                    curr.next();
                }
                (None, None) => break,
            }
        }

        ChangeSet {
            added,
            modified,
            deleted,
        }
    }
}
