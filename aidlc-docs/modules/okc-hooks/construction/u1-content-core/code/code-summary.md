# U1 content-core — Code Summary (코드 요약)

**크레이트**: `content-core` (lib) · **단계**: CONSTRUCTION -> Per-Unit Loop -> U1 -> Code Generation
**성격**: 동기화 사이클의 결정성 콘텐츠 코어. 유일한 워크스페이스 의존은 `foundation`(U0)이며, `foundation` 값 타입/코덱/상수를 소비만 하고 재정의하지 않는다.

## 1. 구현된 컴포넌트 + 공개 API 표면

crate-root 재-export(`lib.rs`)로 노출되는 5개 컴포넌트 + 값 계층:

- `ContentAddressing` (순수, I/O 없음) — `hash_stream<R: Read>(reader: R) -> Result<Sha256Digest, io::Error>`, `manifest_digest(entries: &[ManifestEntry]) -> ManifestDigest`, `const HASH_BUFFER_BYTES: usize = 64 * 1024`.
- `VaultScanner` (U1 유일 파일 I/O) — `new(root: PathBuf)`, `scan(&self) -> Result<VaultSnapshot, ScanError>`, `open_reader(&self, path: &RelativePath) -> Result<Box<dyn Read>, ScanError>`.
- `ManifestBuilder<'a>` — `new(scanner: &'a VaultScanner)`, `build(&self) -> Result<Manifest, BuildError>` (fail-fast 조립).
- `ManifestDiffer` (순수) — `diff(last_committed: &Manifest, current: &Manifest) -> ChangeSet` (투 포인터 merge-join).
- `SafetyLimitsValidator` (순수) — `validate(manifest: &Manifest) -> LimitVerdict`; 값 타입 `SafetyLimits`(+`defaults()`), `LimitVerdict`(`WithinLimits`/`Exceeded`), `LimitViolation`(`TotalBytes`/`FileBytes`/`FileCount`).
- 값/오류 계층 — `ScannedFile`, `VaultSnapshot`(`types`), `ScanError`(`RootUnavailable`/`Io`), `BuildError`(`Scan`/`Hash`, `thiserror` 파생).

## 2. 모듈 레이아웃

`src/lib.rs`(재-export + `#![deny(missing_docs)]`), `content_addressing.rs`, `scanner.rs`, `builder.rs`, `differ.rs`, `limits.rs`, `error.rs`, `types.rs`, `proptest_support/{mod.rs, generators.rs}`(feature 게이트). 순수 모듈(`content_addressing`/`differ`/`limits`/`types`)은 상단에 `#![deny(clippy::unwrap_used, expect_used, indexing_slicing, panic)]` lint-gate 적용. `scanner`/`builder`는 I/O 조율이라 lint-gate 비적용이나 패닉 경로 없음.

## 3. 외부 의존성

- `foundation` (path, U0): `Manifest`/`ManifestEntry`/`ChangeSet`/`Sha256Digest`/`ManifestDigest`/`RelativePath`/`ByteCount`/`Timestamp`, `encode`/`decode`, `RelativePath::normalize`, `MAX_VAULT_TOTAL_BYTES`/`MAX_FILE_BYTES`/`MAX_FILE_COUNT`.
- `sha2` — U1 도입 유일 신규 런타임 크레이트(표준 SHA-256).
- `thiserror` — 오류 파생. `std` — 파일 I/O·프레이밍.
- `proptest` — optional, `proptest-support` feature 하 dev-only(`default = []`이라 프로덕션 그래프 미유입).

## 4. 적용된 MVP 축소

- rename/content-move 감지 없음 — 이름 변경은 `deleted` + `added`로 표현(D4).
- exclude/ignore 패턴 없음 — U1은 federated config 키를 등록하지 않는다(D9). 볼트 루트는 U8 생성자 주입(`ConfigProvider` 직접 읽기 없음).
- 심링크 SKIP(R-SCAN-02), dotfile 포함(R-SCAN-03). 신규 walk 크레이트(`walkdir`) 미도입 — `std::fs`만 사용.
- 스트리밍 버퍼는 튜닝 불가 단일 고정 `const`(64 KiB, D3).
- `Exceeded`는 첫 위반 하나만 담는 short-circuit(D15). 총량 누적은 `saturating_add`(오버플로 시 `TotalBytes` 위반 판정).
- 수치 성능 게이트 없음 — 정성 계약(전량 적재 없음·선형 비용)만.

## 5. 테스트 커버리지 (17건)

- 예제/통합 (`tests/example_content_core.rs`, 8건): 빈-입력 SHA-256 오라클, injective 프레이밍(`["ab","c"]` vs `["a","bc"]`), `VaultScanner` 정렬·dotfile 포함·심링크 SKIP, `RootUnavailable`, `ManifestBuilder` 일관성 + `open_reader`/`hash_stream` 일치, diff added/modified/deleted, TotalBytes 경계(정확 20 GiB), FileCount 경계(정확 100k).
- Property (`tests/prop_content_core.rs`, `proptest-support` 게이트, 9건): PROP-U1-01(해싱 결정성 + `sha2` 오라클 / 청크 경계 무관), PROP-U1-02(`manifest_digest` 순열 무관), PROP-U1-03(diff apply 오라클 + 서로소 + no-op 동치), PROP-U1-04(파일당 한도 limit-1/limit/limit+1 경계-정확 / 단조 reject), PROP-U1-05(`Manifest`·`ChangeSet` U0 코덱 round-trip), REL-07(대값 누적 no-panic).
- 제너레이터(`proptest_support/generators.rs`): `arb_bytes`(버퍼 경계 가로지름), `arb_consistent_manifest`, `arb_manifest_pair`(add/modify/delete 상관 쌍); U0 제너레이터 재사용.

## 6. 검증 사실 (확정)

전체 워크스페이스가 빌드되며 모든 크레이트 테스트가 통과한다 (foundation 32 / content-core 17 / change-detect 26 / sync-state 15 / auth-consent 35 / observability 24 = 총 149건, 실패 0). 그리고 `cargo clippy --all-targets --features proptest-support -- -D warnings` 가 모든 크레이트에서 CLEAN 이다.
