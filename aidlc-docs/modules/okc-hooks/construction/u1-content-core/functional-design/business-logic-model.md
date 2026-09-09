# U1 Deterministic Content Core — Business Logic Model (핵심 로직 · 알고리즘 · 데이터 흐름)

**단계**: CONSTRUCTION -> Per-Unit Loop -> **U1 `content-core`** -> Functional Design -> 산출물 3/3 (`business-logic-model.md`)
**작성일**: 2026-09-08
**크레이트**: `content-core` (lib) · **소속 컴포넌트**: `ContentAddressing`, `VaultScanner`, `ManifestBuilder`, `ManifestDiffer`, `SafetyLimitsValidator`
**전제(드롭리스트)**: FQ-1=A · FQ-2=A · Q2=A · Q2(사이클)=B(트리거당 1회 직렬 사이클) · U0 `CoreTypes`/코덱/`RelativePath` 구현 완료.

> **문서 성격**: `domain-entities.md`(타입)와 `business-rules.md`(규칙 R-*, 속성 PROP-U1-*) 위에서 동작하는 **컴포넌트별 알고리즘·워크플로·데이터 흐름**과 컴포넌트 합성(scan -> build -> diff -> limits), 교차 단위 경계를 기술한다. 타입·규칙을 재정의하지 않고 참조한다.
>
> **표기 규약**: 기술중립. Rust 시그니처는 참고용. ASCII 화살표(`A -> B`)만, 박스/유니코드 없음. Rust 타입은 backtick.

---

## 1. 컴포넌트 개요 및 순수성 경계

| 컴포넌트 | 책임 | I/O | 결정적? | 주 NFR |
|---|---|---|---|---|
| `ContentAddressing` | 파일 스트리밍 SHA-256 + 매니페스트 다이제스트 | 없음(스트림 소비만) | 예 | NFR-08 |
| `VaultScanner` | 볼트 루트 열거 + 파일별 스트리밍 리더 | **파일 읽기(U1 유일)** | 출력 정렬로 결정적 | NFR-02 |
| `ManifestBuilder` | scan + hash 결합 -> `Manifest` 재계산 | 파일 읽기(Scanner 경유) | 예 | NFR-02/08 |
| `ManifestDiffer` | 마지막 커밋 vs 현재 -> `ChangeSet` | 없음 | 예 | NFR-12 |
| `SafetyLimitsValidator` | 전송 전 3한도 검사 | 없음 | 예 | NFR-14 |

- **순수성 경계**: `VaultScanner`를 제외한 4개 컴포넌트는 순수·무결(pure, no side-effect)하다. 지속(마지막 커밋 매니페스트)은 U4, 관측 push(로그·상태·헬스)는 U8이 소유하며 U1은 값/판정만 반환한다(§0 노트2).

---

## 2. ContentAddressing — 알고리즘

### 2.1 hash_stream (파일 콘텐츠 -> raw_sha256)

참고 시그니처: `fn hash_stream<R: Read>(reader: R) -> Result<Sha256Digest, io::Error>`

```
reader(바이트 스트림)
  -> 고정 버퍼로 반복 read (NFR-02, 전량 적재 없음)
  -> SHA-256 상태에 증분 업데이트
  -> EOF 도달 시 finalize
  -> Sha256Digest (32바이트, == sha256sum, R-CA-01)
```

- **결정성(R-CA-02)**: 청크 분할 경계와 무관하게 동일 바이트열 -> 동일 다이제스트. 빈 스트림은 표준 빈-입력 SHA-256.
- **재사용 경계(Q8=A/노트1)**: U3 `UploadProtocolDriver`가 전송 직전 blob 바이트를 스스로 재-읽기하여 이 메서드로 재-해시하고 매니페스트 해시와 비교한다 — `hash_stream`은 임의 바이트 스트림을 받으므로 `VaultScanner` 의존 없이 재사용된다.

### 2.2 manifest_digest (정렬 엔트리 -> ManifestDigest)

참고 시그니처: `fn manifest_digest(entries: &[ManifestEntry]) -> ManifestDigest`

```
entries
  -> canonical 정렬: relative_path 오름차순(바이트 사전식), tie-break (raw_sha256, size)
  -> 각 엔트리 길이-프리픽스 프레이밍 (R-CA-04, domain-entities §3):
       len(path:BE) || path_bytes || raw_sha256(32B) || size(u64 BE)
  -> concat 전체를 표준 SHA-256
  -> ManifestDigest (로컬 no-op 판별자, vault_content_id 아님 — FQ-1=A)
```

- **순서 무관 결정성(R-CA-05)**: 정렬을 계산 내부에서 강제하므로 발견 순서와 무관하게 동일 다이제스트. injective 프레이밍으로 서로 다른 엔트리 집합의 충돌 없음(PROP-U1-02).

---

## 3. VaultScanner — 알고리즘 (U1 유일 파일 읽기)

참고 시그니처: `fn scan(&self) -> Result<VaultSnapshot, ScanError>` · `fn open_reader(&self, path: &RelativePath) -> Result<Box<dyn Read>, ScanError>`

### 3.1 scan 흐름

```
config vault_path(루트)
  -> 루트 도달 가능 확인 -> 불가 시 ScanError::RootUnavailable (R-SCAN-05, 판정은 U2)
  -> 재귀 디렉터리 walk:
       디렉터리 엔트리 순회
         심링크? -> SKIP (R-SCAN-02, 따라가지 않음)
         디렉터리? -> 재귀
         일반 파일? -> 상대경로 계산 -> RelativePath::normalize (R-SCAN-01)
                        + size 취득 -> ScannedFile 수집
       (dotfile 포함 — R-SCAN-03; exclude 패턴 없음 — R-SCAN-04)
  -> 수집된 ScannedFile 을 relative_path 오름차순 canonical 정렬 (R-SCAN-06)
  -> VaultSnapshot { root, files(정렬됨), captured_at }
```

- **일관 시점(D11)**: OS FS 스냅샷을 쓰지 않는 단일 패스 walk. `captured_at`이 논리 시점을 표시하고, 열거~해시~전송 사이 TOCTOU는 U3 재검증(FR-22)이 폐쇄한다.
- **결정적 출력**: walk 순서(플랫폼별 디렉터리 반환 순서)와 무관하게 정렬 출력(R-SCAN-06).

### 3.2 open_reader

- 스캔된 `RelativePath`에 대해 스트리밍 리더(`Box<dyn Read>`)를 연다. `ManifestBuilder`와 U3(재검증)가 소비. I/O 실패는 `ScanError::Io { path, source }`.

---

## 4. ManifestBuilder — 알고리즘 (scan + hash -> Manifest)

참고 시그니처: `fn build(&self) -> Result<Manifest, BuildError>`

```
build:
  1. VaultScanner.scan()  -> VaultSnapshot | Err(ScanError -> BuildError::Scan)   (R-BUILD-01)
  2. for each ScannedFile in snapshot.files (정렬된 순서):
       reader = VaultScanner.open_reader(path)          | Err -> BuildError::Hash
       digest = ContentAddressing.hash_stream(reader)    | Err -> BuildError::Hash (fail-fast, 스킵 없음)
       -> ManifestEntry { relative_path, raw_sha256: digest, size }
  3. entries 는 이미 경로 오름차순(snapshot 정렬 승계); canonical 형상 재확인
  4. manifest_digest = ContentAddressing.manifest_digest(&entries)
  5. Manifest { entries, manifest_digest }              (R-BUILD-04 파생값 일관)
```

- **fail-fast(R-BUILD-01/D12)**: 2단계 어느 파일이라도 실패하면 전체 중단 -> `BuildError`. 부분 매니페스트 없음(파괴적 diff 방지).
- **메모리 바운드(R-BUILD-02/NFR-02)**: 파일을 순차 스트리밍, 동시 다수 적재 없음.

---

## 5. ManifestDiffer — 알고리즘 (정확 diff)

참고 시그니처: `fn diff(last_committed: &Manifest, current: &Manifest) -> ChangeSet`

```
diff:
  0. if current.manifest_digest == last_committed.manifest_digest:
        return empty ChangeSet  (R-DIFF-04, O(1) no-op 조기 종료, FR-10)
  1. 두 매니페스트 모두 canonical 정렬(U0 불변식) -> 투 포인터 merge-join:
        i -> last_committed.entries, j -> current.entries
        compare relative_path:
          last < current  -> deleted.push(last.path);         i++
          last > current  -> added.push(current.entry);        j++
          동일 경로:
             (raw_sha256 또는 size 다름) -> modified.push(current.entry)  (R-DIFF-02)
             동일 -> (변경 없음, 목록 미포함)
             i++, j++
        잔여 last -> deleted, 잔여 current -> added
  2. added/modified/deleted 각각 relative_path 오름차순(merge-join 특성상 자동 정렬)
  -> ChangeSet { added, modified, deleted }
```

- **정확성(R-DIFF-03/NFR-12)**: 누락 0·오탐 0, 세 목록 서로소. `apply(prev, diff) == current`(PROP-U1-03).
- **범위(R-DIFF-01/D4)**: added/modified/deleted만 — rename은 deleted+added로 표현(MVP).
- **순수·결정적(R-DIFF-05)**: I/O·상태 없음. O(n+m).
- **경계(R-DIFF-06)**: `last_committed`는 U4가 전달하는 인자(U1 미지속). 최초 사이클 취급은 호출자(U8) 결정.

---

## 6. SafetyLimitsValidator — 알고리즘 (경계-정확·단조)

참고 시그니처: `fn validate(manifest: &Manifest) -> LimitVerdict`

```
validate:
  1. count = manifest.entries.len()
     if count > MAX_FILE_COUNT (100k):  return Exceeded(FileCount{limit, actual: count})  (R-LIMIT-05 순서 1)
  2. total = 0
     for entry in manifest.entries:
        if entry.size > MAX_FILE_BYTES (2 GiB):
             return Exceeded(FileBytes{path: entry.path, limit, actual: entry.size})  (첫 위반 short-circuit, R-LIMIT-04)
        total = total.saturating_add(entry.size)   (오버플로 방지, R-LIMIT-05)
        if total > MAX_VAULT_TOTAL_BYTES (20 GiB):
             return Exceeded(TotalBytes{limit, actual: total})
  3. return WithinLimits
```

- **경계-정확(R-LIMIT-02/D14)**: `actual <= limit` accept, `> limit`만 reject. 정확히 20 GiB/2 GiB/100k는 통과.
- **단조(R-LIMIT-03)**: 추가는 reject를 accept로 뒤집지 않음(PROP-U1-04).
- **첫 위반 short-circuit(R-LIMIT-04/D15/MVP-trim)**: 단일 `LimitViolation`만 반환.
- **순수(R-LIMIT-06)**: OverLimit 표면화·halt는 U3/U8 소관, U1은 판정 값만 반환.

---

## 7. 컴포넌트 합성 및 사이클 데이터 흐름

한 트리거당 1회 직렬 사이클(Q2=B)에서 U8 `SyncCycleCoordinator`가 U1 컴포넌트를 다음 순서로 구동한다(협상/전송/커밋은 U3, 지속은 U4):

```
[U8 트리거]
  -> ManifestBuilder.build()                         (U1: scan -> hash -> Manifest)   = current
  -> SafetyLimitsValidator.validate(current)         (U1: 프리플라이트 3한도)
        Exceeded -> [U8] OverLimit 표면화 + 사이클 halt (마지막 커밋 유지), 종료
        WithinLimits -> 계속
  -> last = SyncStateStore.last_committed_manifest()  (U4 소유, 인자로 전달)
  -> ManifestDiffer.diff(last, current)               (U1: 정확 ChangeSet)
        is_empty() -> no-op 사이클 종료 (FR-10/NFR-01, 업로드 미생성)
        비어있지 않음 -> [U3] negotiate/transfer/commit (전송 시 ContentAddressing.hash_stream 재검증)
  -> 성공 시 [U4] commit_manifest(current) 지속
```

- **U1이 소유하지 않는 것**: 트리거 발행(U2/U8), 마지막 커밋 지속(U4), 관측 push(U8), 전송/커밋(U3), 동의 게이트(U5). U1은 `Manifest`/`ChangeSet`/`LimitVerdict`/`VaultSnapshot`/`Sha256Digest` 산출과 판정만 담당.
- **런타임 한도 재검사**: U3가 매 사이클 시작 시 `SafetyLimitsValidator`(U1)를 재사용해 런타임 한도를 재검사한다(US-E2-08) — U1의 순수 `validate`가 그대로 재사용된다.

**텍스트 설명(다이어그램 대안)**: U1은 "폴더 -> 정렬 지문 목록 + 다이제스트(build)" 단방향 파이프라인, "두 지문 목록 비교 -> 변경 집합(diff)", "매니페스트 -> 3한도 판정(validate)"의 세 순수 연산을 제공하고, 파일 바이트 스트리밍 해시(hash_stream)는 build 내부와 U3 재검증에서 공유된다. 사이클 오케스트레이션과 상태·표면화는 상위 단위가 소유한다.

---

## 8. 컴포넌트별 Testable-Properties 노트

| 컴포넌트 | PROP-U1-* | 카테고리 | 실행 위치 |
|---|---|---|---|
| `ContentAddressing.hash_stream` | PROP-U1-01 | Idempotence + Oracle | U1(proptest, 표준 sha256 오라클) |
| `ContentAddressing.manifest_digest` | PROP-U1-02 | Invariant(순서 무관) + Oracle | U1 |
| `ManifestDiffer.diff` | PROP-U1-03 | Oracle(재구성) + Invariant(서로소) | U1 |
| `SafetyLimitsValidator.validate` | PROP-U1-04 | Invariant(경계) + Monotonicity | U1 |
| `ManifestBuilder`(산출 Manifest) / `diff`(산출 ChangeSet) | PROP-U1-05 | Round-trip(U0 코덱, U1 산출값 재확인) | U1(코덱은 U0 소유) |
| `VaultScanner` | (없음) | 예제/통합(임시 디렉터리 픽스처, 심링크 SKIP·hidden 포함 시나리오) | U1 |

- 제너레이터는 U0 `proptest_support`(`arb_manifest_entry`/`arb_relative_path`/`arb_sha256_digest`/`arb_byte_count`) 재사용 + U1 전용(임의 `Vec<u8>`·상관 매니페스트 쌍·경계 조준 SafetyLimits 입력·digest-consistent 매니페스트) 추가. shrinking·고정 시드·CI(PBT-08)는 Code Generation/Build-and-Test 이월.

---

## 9. 확장 컴플라이언스 요약 (이 산출물 범위)

| 확장 | 활성 | 이 산출물 적용 | 판정 |
|---|---|---|---|
| **Property-Based Testing** | ON(Full) | **강제·준수** | §8에 컴포넌트별 PROP-U1-01~05 매핑, 카테고리·실행 위치·제너레이터 기재. VaultScanner는 예제/통합 판정 |
| **Resiliency Baseline** | ON | 부분 적용 + N/A | fail-fast build(§4, 파괴적 부분 diff 방지)·루트 오류 반환(§3, 파괴적 빈 커밋 방지)·스트리밍(§2, OOM 회피)이 회복력 기여. U1 산출 `Manifest`가 U4 zero-loss 지속(RPO=0)의 입력. RTO/RPO 수치·배포/HA/DR은 U1 순수 로직에 N/A |
| **Security Baseline** | OFF | N/A | 미로딩·미강제. U1은 토큰/시크릿 미취급. RISK-01은 문서화된 수용 위험 |
