//! U1 결정적 example/통합 테스트(proptest feature 없이 컴파일·실행).
//!
//! VaultScanner 임시 디렉터리 픽스처(심링크 SKIP·dotfile 포함·정렬·RootUnavailable), 빈-입력 SHA-256
//! 오라클, injective 프레이밍, SafetyLimits 총량/파일수 경계, ManifestBuilder round-trip 을 고정한다.

use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

use content_core::{
    ContentAddressing, LimitVerdict, LimitViolation, ManifestBuilder, ManifestDiffer, ScanError,
    SafetyLimitsValidator, VaultScanner,
};
use foundation::{
    ByteCount, MAX_FILE_BYTES, MAX_FILE_COUNT, MAX_VAULT_TOTAL_BYTES, Manifest, ManifestDigest,
    ManifestEntry, RelativePath, Sha256Digest,
};

/// 프로세스 유일 임시 디렉터리를 만든다.
fn make_temp_dir() -> PathBuf {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let n = COUNTER.fetch_add(1, Ordering::SeqCst);
    let mut path = std::env::temp_dir();
    path.push(format!("okc_u1_{}_{}", std::process::id(), n));
    std::fs::create_dir_all(&path).expect("temp dir 생성");
    path
}

/// 고정 크기 zero-hash 엔트리를 만든다(limits 테스트용, digest 무관).
fn entry(path: &str, size: u64) -> ManifestEntry {
    ManifestEntry {
        relative_path: RelativePath::normalize(path).expect("정규화"),
        raw_sha256: Sha256Digest::from_bytes([0u8; 32]),
        size: ByteCount::new(size),
    }
}

#[test]
fn empty_stream_matches_standard_sha256() {
    let digest = ContentAddressing::hash_stream(&[][..]).expect("빈 스트림 해시");
    // 표준 빈-입력 SHA-256.
    assert_eq!(
        digest.to_string(),
        "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
    );
}

#[test]
fn manifest_digest_is_injective_over_path_boundaries() {
    // ["ab","c"] vs ["a","bc"] — 길이-프리픽스가 경계 모호성을 제거해 서로 다른 다이제스트.
    let set_a = vec![entry("ab", 0), entry("c", 0)];
    let set_b = vec![entry("a", 0), entry("bc", 0)];
    assert_ne!(
        ContentAddressing::manifest_digest(&set_a),
        ContentAddressing::manifest_digest(&set_b)
    );
}

#[test]
fn scan_sorts_includes_dotfiles_skips_symlinks() {
    let root = make_temp_dir();
    std::fs::write(root.join("a.txt"), b"hello").expect("write a");
    std::fs::write(root.join(".hidden"), b"dot").expect("write hidden");
    std::fs::create_dir_all(root.join("sub")).expect("mkdir sub");
    std::fs::write(root.join("sub").join("b.txt"), b"world").expect("write b");

    #[cfg(unix)]
    std::os::unix::fs::symlink(root.join("a.txt"), root.join("link")).expect("symlink");

    let scanner = VaultScanner::new(root.clone());
    let snapshot = scanner.scan().expect("scan 성공");
    let paths: Vec<&str> = snapshot
        .files
        .iter()
        .map(|f| f.relative_path.as_str())
        .collect();

    // dotfile 포함, 심링크 제외, canonical 정렬.
    assert_eq!(paths, vec![".hidden", "a.txt", "sub/b.txt"]);
    // 심링크 경로가 산출되지 않았음을 재확인.
    assert!(!paths.contains(&"link"));
}

#[test]
fn scan_missing_root_returns_root_unavailable() {
    let root = make_temp_dir().join("does_not_exist");
    let scanner = VaultScanner::new(root);
    match scanner.scan() {
        Err(ScanError::RootUnavailable) => {}
        other => panic!("RootUnavailable 기대, 실제: {other:?}"),
    }
}

#[test]
fn builder_produces_consistent_manifest_and_open_reader_matches() {
    let root = make_temp_dir();
    std::fs::write(root.join("a.txt"), b"hello").expect("write a");
    std::fs::write(root.join("b.txt"), b"world!!").expect("write b");

    let scanner = VaultScanner::new(root.clone());
    let builder = ManifestBuilder::new(&scanner);
    let manifest = builder.build().expect("build 성공");

    // 파생값 일관: manifest_digest == digest(canonical(entries)).
    assert_eq!(
        manifest.manifest_digest,
        ContentAddressing::manifest_digest(&manifest.entries)
    );
    // 엔트리는 정렬·경로/크기 정확.
    let paths: Vec<&str> = manifest
        .entries
        .iter()
        .map(|e| e.relative_path.as_str())
        .collect();
    assert_eq!(paths, vec!["a.txt", "b.txt"]);

    // open_reader + hash_stream 이 매니페스트 해시와 일치.
    let a_path = RelativePath::normalize("a.txt").unwrap();
    let reader = scanner.open_reader(&a_path).expect("open");
    let streamed = ContentAddressing::hash_stream(reader).expect("hash");
    let a_entry = manifest
        .entries
        .iter()
        .find(|e| e.relative_path == a_path)
        .expect("a.txt 엔트리");
    assert_eq!(a_entry.raw_sha256, streamed);
    assert_eq!(a_entry.size, ByteCount::new(5));

    // 동일 매니페스트 diff 는 no-op.
    assert!(ManifestDiffer::diff(&manifest, &manifest).is_empty());
}

#[test]
fn diff_reports_added_modified_deleted() {
    let mut prev_entries = vec![entry("keep.txt", 1), entry("gone.txt", 2), entry("change.txt", 3)];
    // Manifest 엔트리는 정규 정렬(경로 오름차순) 불변식을 가진다(ManifestBuilder 보장).
    // ManifestDiffer 는 이 정렬을 전제한 sorted-merge 이므로 prev 도 정렬해 불변식을 지킨다.
    prev_entries.sort();
    let mut curr_entries = vec![
        entry("keep.txt", 1),
        entry("new.txt", 4),
    ];
    // change.txt 를 수정(크기 변경).
    curr_entries.push(entry("change.txt", 99));
    curr_entries.sort();

    let prev = Manifest {
        manifest_digest: ContentAddressing::manifest_digest(&prev_entries),
        entries: prev_entries,
    };
    let current = Manifest {
        manifest_digest: ContentAddressing::manifest_digest(&curr_entries),
        entries: curr_entries,
    };

    let change = ManifestDiffer::diff(&prev, &current);
    let added: Vec<&str> = change.added.iter().map(|e| e.relative_path.as_str()).collect();
    let modified: Vec<&str> = change.modified.iter().map(|e| e.relative_path.as_str()).collect();
    let deleted: Vec<&str> = change.deleted.iter().map(|p| p.as_str()).collect();

    assert_eq!(added, vec!["new.txt"]);
    assert_eq!(modified, vec!["change.txt"]);
    assert_eq!(deleted, vec!["gone.txt"]);
}

#[test]
fn total_bytes_boundary_exact() {
    // 10 파일 * 2 GiB = 정확히 20 GiB -> WithinLimits.
    let at_limit: Vec<ManifestEntry> = (0..10)
        .map(|i| entry(&format!("f{i}.bin"), MAX_FILE_BYTES))
        .collect();
    let manifest = Manifest {
        manifest_digest: ManifestDigest::from_bytes([0u8; 32]),
        entries: at_limit,
    };
    assert_eq!(
        SafetyLimitsValidator::validate(&manifest),
        LimitVerdict::WithinLimits
    );
    assert_eq!(MAX_VAULT_TOTAL_BYTES, 10 * MAX_FILE_BYTES);

    // 11 파일 * 2 GiB = 22 GiB > 20 GiB -> TotalBytes 초과.
    let over: Vec<ManifestEntry> = (0..11)
        .map(|i| entry(&format!("f{i}.bin"), MAX_FILE_BYTES))
        .collect();
    let manifest = Manifest {
        manifest_digest: ManifestDigest::from_bytes([0u8; 32]),
        entries: over,
    };
    match SafetyLimitsValidator::validate(&manifest) {
        LimitVerdict::Exceeded(LimitViolation::TotalBytes { limit, actual }) => {
            assert_eq!(limit, MAX_VAULT_TOTAL_BYTES);
            assert!(actual > MAX_VAULT_TOTAL_BYTES);
        }
        other => panic!("TotalBytes 초과 기대, 실제: {other:?}"),
    }
}

#[test]
fn file_count_boundary_exact() {
    // 정확히 100k 파일 -> WithinLimits(크기 0).
    let at_limit: Vec<ManifestEntry> = (0..MAX_FILE_COUNT)
        .map(|i| entry(&format!("f{i}"), 0))
        .collect();
    let manifest = Manifest {
        manifest_digest: ManifestDigest::from_bytes([0u8; 32]),
        entries: at_limit,
    };
    assert_eq!(
        SafetyLimitsValidator::validate(&manifest),
        LimitVerdict::WithinLimits
    );

    // 100k + 1 -> FileCount 초과.
    let over: Vec<ManifestEntry> = (0..(MAX_FILE_COUNT + 1))
        .map(|i| entry(&format!("f{i}"), 0))
        .collect();
    let manifest = Manifest {
        manifest_digest: ManifestDigest::from_bytes([0u8; 32]),
        entries: over,
    };
    match SafetyLimitsValidator::validate(&manifest) {
        LimitVerdict::Exceeded(LimitViolation::FileCount { limit, actual }) => {
            assert_eq!(limit, MAX_FILE_COUNT);
            assert_eq!(actual, MAX_FILE_COUNT + 1);
        }
        other => panic!("FileCount 초과 기대, 실제: {other:?}"),
    }
}
