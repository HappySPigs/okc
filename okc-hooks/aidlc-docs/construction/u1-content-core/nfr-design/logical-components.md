# U1 Deterministic Content Core — Logical Components (논리 컴포넌트 분해 + 추적성 맵)

**단계**: CONSTRUCTION -> Per-Unit Loop -> **U1 `content-core`** -> NFR Design -> 산출물 2/2 (`logical-components.md`)
**작성일**: 2026-09-08
**크레이트**: `content-core` (lib) · **소속 컴포넌트(공개)**: `ContentAddressing`, `VaultScanner`, `ManifestBuilder`, `ManifestDiffer`, `SafetyLimitsValidator`
**입력 아티팩트**: 자매 산출물 `nfr-design/nfr-design-patterns.md`(§11 패턴 -> 논리 컴포넌트 안착 맵이 이 문서의 권위 근거) · `functional-design/`(domain-entities §1~§4 · business-rules R-* · business-logic-model §1~§8) · `nfr-requirements/tech-stack-decisions.md`(§2 `sha2` · §3 `std` · §4 `thiserror` · §6 `proptest-support`) · U0 산출물 `u0-foundation/nfr-design/logical-components.md`(스타일 템플릿) · 활성 확장 `property-based-testing.md`(ON, Full)·`resiliency-baseline.md`(ON)
**규칙**: `construction/nfr-design.md` Step 6 · `common/content-validation.md`(ASCII 화살표 `A -> B`, 박스 문자 미사용, Rust 제네릭 백틱) · `common/ascii-diagram-standards.md`

> **문서 성격**: 이 문서는 Functional Design이 확정한 **5개 공개 컴포넌트**(`ContentAddressing`·`VaultScanner`·`ManifestBuilder`·`ManifestDiffer`·`SafetyLimitsValidator`)를 상위 단위로 유지한 채, 그 내부의 **이미 확정된 기능을 명명된 논리 컴포넌트로 전개**하고 U1이 소비하는 U0 타입/계약과의 seam을 명시한다. 이는 **발명이 아니라 확정 기능(FD 타입·규칙·흐름)의 명명·정렬**이며, FD 흐름에서 near-free로 파생된다. 각 논리 컴포넌트는 (a) 책임, (b) 공개 인터페이스 표면, (c) 안착하는 확정 NFR 패턴(자매 산출물 `nfr-design-patterns.md` §11 안착 맵과 1:1)으로 기술한다.
>
> **이것은 추적성 문서 맵이며 물리 모듈/크레이트 증식 강제가 아니다.** 논리 컴포넌트 -> 물리 파일/모듈 레이아웃 매핑은 **Code Generation에서 최소로** 결정된다(다수 논리 컴포넌트가 한 모듈에 상주할 수 있음). 목적은 NFR·규칙·PBT 속성 -> 명명 단위 추적성 극대화다.
>
> **표기 규약**: 화살표는 ASCII `A -> B`만 사용(유니코드 화살표 금지). 박스/선-그리기 문자 미사용. Rust 제네릭/타입/식별자(예: `Box<dyn Read>`, `Result<Manifest, BuildError>`, `Sha256::update`, `FileType::is_symlink`)는 백틱으로 감싼다.

---

## 1. 논리 컴포넌트 전개 개요

| 공개 컴포넌트 | 성격 | 논리 컴포넌트(확정 명명) |
|---|---|---|
| `ContentAddressing` | 순수 — 스트리밍 해시 + 다이제스트 프레이밍(I/O 없음) | StreamHasher · DigestFramer |
| `VaultScanner` | **U1 유일 파일 I/O** — 볼트 열거 + 스트리밍 리더 | DirWalker · ReaderFactory |
| `ManifestBuilder` | scan + hash 조립 -> `Manifest`(I/O는 `VaultScanner` 경유) | ManifestAssembler |
| `ManifestDiffer` | 순수 — 정확 diff(merge-join) | ManifestDiffer(단일 논리 컴포넌트) |
| `SafetyLimitsValidator` | 순수 — 3한도 경계-정확·단조 판정 | LimitsValidator |
| (오류/판정 값 계층) | U1 도입 오류·판정 값 타입(U0 관례 승계) | ErrorTaxonomy(`ScanError`/`BuildError`) · VerdictTypes(`LimitVerdict`/`LimitViolation`) |
| (런타임 그래프 밖) | 테스트 전용 — 비-default `proptest-support` feature | ProptestGenerators (test-support 논리 단위, §5.2) |

---

## 2. `ContentAddressing` 컴포넌트 — 논리 컴포넌트 (순수)

> `ContentAddressing`은 U1의 결정성 코어다 — I/O 없이 바이트 스트림/엔트리를 소비해 다이제스트만 산출한다. `nfr-design-patterns.md` §6 순수 모듈 clippy lint-gate 대상.

### 2.1 StreamHasher
- **책임**: 파일 콘텐츠(임의 `Read` 스트림)를 고정 버퍼로 반복 읽어 표준 SHA-256을 증분 계산하고 32바이트 `Sha256Digest`를 산출(R-CA-01/02/03). `sha256sum`·okc-core `raw_sha256`과 바이트 일치. 파일시스템 의존 없음.
- **공개 인터페이스 표면**: `hash_stream<R: Read>(reader: R) -> Result<Sha256Digest, io::Error>`. 내부: `sha2::Sha256::update`(증분) + `finalize`, `const HASH_BUFFER_BYTES = 64 * 1024` 고정 읽기 버퍼(`nfr-design-patterns.md` §1). 불변식: 청크 분할 경계 무관(`hash_stream(concat(chunks)) == hash_stream(whole)`), 빈 스트림 = 표준 빈-입력 SHA-256.
- **안착 NFR 패턴**: `nfr-design-patterns.md` §1 스트리밍 고정 버퍼 해싱(**U1-NFR-PERF-01 / U1-NFR-REL-01**, R-CA-01/02/03) + §4 `R: Read` seam(U3 재검증 재사용) + §6 panic-free(순수 표면 lint-gate; I/O 실패는 `Result`로 표면화).

### 2.2 DigestFramer
- **책임**: canonical 정렬된 `ManifestEntry` 시퀀스를 길이-프리픽스 프레이밍(injective) 바이트열로 직렬화해 표준 SHA-256으로 `ManifestDigest`를 산출(R-CA-04/05). CBOR 지속 코덱과 분리된 안정 콘텐츠 지문.
- **공개 인터페이스 표면**: `manifest_digest(entries: &[ManifestEntry]) -> ManifestDigest`. 내부: `relative_path` 사전식 정렬(tie-break `(raw_sha256, size)`) 후 엔트리별 `len(path)`(`u64::to_be_bytes` BE) + path 바이트 + `raw_sha256`(32B) + `size`(`u64::to_be_bytes` BE) concat -> `sha2`. 불변식: 순서 무관 결정성 + 다른 논리 집합 -> 다른 바이트열(injective).
- **안착 NFR 패턴**: `nfr-design-patterns.md` §2 injective 길이-프리픽스 프레이밍(**U1-NFR-REL-02**, R-CA-04/05, D1 CBOR 분리) + §6 panic-free(순수 표면 lint-gate).

---

## 3. `VaultScanner` 컴포넌트 — 논리 컴포넌트 (U1 유일 파일 I/O)

> `VaultScanner`는 U1에서 유일하게 파일시스템 부수효과를 갖는 컴포넌트다. `std::fs`만 사용(신규 walk 크레이트 없음, T2). 순수 표면이 아니므로 §6 lint-gate 대상은 아니나 실패는 `Result`(`ScanError`)로만 표면화한다.

### 3.1 DirWalker
- **책임**: 주입된 볼트 루트(`PathBuf`)를 `std::fs::read_dir` 재귀 walk로 열거해 canonical 정렬된 `VaultSnapshot`을 산출. 심링크 SKIP(R-SCAN-02), dotfile 포함(R-SCAN-03), exclude 패턴 없음(R-SCAN-04), 루트 도달 불가 시 `ScanError::RootUnavailable` 값 반환(R-SCAN-05, 0-파일 매니페스트 미생성). 각 파일 경로는 U0 `RelativePath::normalize`로 위임 정규화(R-SCAN-01).
- **공개 인터페이스 표면**: `scan(&self) -> Result<VaultSnapshot, ScanError>`. 생성자는 볼트 루트 `PathBuf`를 U8 주입으로 받음(`ConfigProvider` 읽기 없음, `nfr-design-patterns.md` §0). 내부: `DirEntry::file_type()` -> `FileType::is_symlink()`(따라가지 않고 판별, `std`), `relative_path` 오름차순 정렬(R-SCAN-06).
- **안착 NFR 패턴**: `nfr-design-patterns.md` §4 `std` 포터빌리티 어댑터(**NFR-05**, R-SCAN-02/03/04, 플랫폼 중립 `std::fs`) + §5 루트 오류 값 반환(**U1-NFR-REL-06**, R-SCAN-05, 파괴적 빈 커밋 방지) + §0 생성자 주입(볼트 루트).

### 3.2 ReaderFactory
- **책임**: 스캔된 `RelativePath`에 대해 파일 스트리밍 리더를 연다. `ManifestBuilder`(조립)와 U3(전송 재검증)가 소비. I/O 실패는 `ScanError::Io { path, source }`.
- **공개 인터페이스 표면**: `open_reader(&self, path: &RelativePath) -> Result<Box<dyn Read>, ScanError>`. 내부: `std::fs::File`(+ 고정 버퍼)을 `Box<dyn Read>` 어댑터로 반환 -> `StreamHasher`가 전량 적재 없이 소비.
- **안착 NFR 패턴**: `nfr-design-patterns.md` §4 `Box<dyn Read>` 스트리밍 seam(**U1-NFR-REL-01 재사용 경계 / U1-NFR-PERF-01**) — 해싱을 파일시스템에 결합하지 않아 U3 재검증 재사용을 가능케 함.

---

## 4. `ManifestBuilder` / `ManifestDiffer` / `SafetyLimitsValidator` — 논리 컴포넌트

### 4.1 ManifestAssembler (`ManifestBuilder`)
- **책임**: `VaultScanner.scan` -> 파일별 `StreamHasher.hash_stream` -> `ManifestEntry` -> canonical 정렬 -> `DigestFramer.manifest_digest` -> `Manifest`를 조립. **fail-fast**(어느 파일이라도 실패 시 전체 중단, 부분 매니페스트 없음, R-BUILD-01/D12). 파일 순차 스트리밍(동시 다수 미적재, R-BUILD-02).
- **공개 인터페이스 표면**: `build(&self) -> Result<Manifest, BuildError>`. `BuildError::Scan(ScanError)` / `BuildError::Hash { path, source }`. 산출 `Manifest`는 U0 불변식 `manifest_digest == digest(canonical(entries))` 만족(R-BUILD-04).
- **안착 NFR 패턴**: `nfr-design-patterns.md` §5 fail-fast build(**U1-NFR-REL-06 / U1-NFR-MNT-01**, R-BUILD-01, `thiserror` 파생) + §1 순차 스트리밍(**U1-NFR-PERF-01**, R-BUILD-02) + §6 순수 조립부 panic-free.

### 4.2 ManifestDiffer (`ManifestDiffer`)
- **책임**: 마지막 커밋 vs 현재 `Manifest`를 비교해 `ChangeSet` 산출. `manifest_digest` 동일 시 O(1) 빈 `ChangeSet` 조기 반환(R-DIFF-04), 아니면 canonical 투 포인터 merge-join으로 `added`/`modified`/`deleted` 단일 패스 계산(R-DIFF-01/02/03/05). 순수·무결·O(n+m). `last_committed`는 U4 소유 인자(U1 미지속, R-DIFF-06).
- **공개 인터페이스 표면**: `diff(last_committed: &Manifest, current: &Manifest) -> ChangeSet`. 불변식: 세 목록 서로소, 지목 안 된 경로는 양쪽 `(raw_sha256, size)` 동일, `apply(prev, diff) == current`(PROP-U1-03).
- **안착 NFR 패턴**: `nfr-design-patterns.md` §3 merge-join 정확 diff + O(1) no-op(**U1-NFR-REL-03**, R-DIFF-01..06) + §6 순수 표면 panic-free.

### 4.3 LimitsValidator (`SafetyLimitsValidator`)
- **책임**: 전송 전 3한도(총 <=20 GiB / 파일당 <=2 GiB / 파일 수 <=100k, **U0 상수**)를 경계-정확(`actual <= limit` accept, R-LIMIT-02)·단조(R-LIMIT-03)로 검사. 단일 패스: FileCount(O(1)) -> per-file FileBytes + `saturating_add` 누적 TotalBytes(R-LIMIT-05). 첫 위반 short-circuit(단일 `LimitViolation`, R-LIMIT-04/D15). 순수(R-LIMIT-06).
- **공개 인터페이스 표면**: `validate(manifest: &Manifest) -> LimitVerdict`. `LimitVerdict = WithinLimits | Exceeded(LimitViolation)`. 한도 값은 U0 `MAX_VAULT_TOTAL_BYTES`/`MAX_FILE_BYTES`/`MAX_FILE_COUNT` 참조(D13, `nfr-design-patterns.md` §0).
- **안착 NFR 패턴**: `nfr-design-patterns.md` §6 panic-free total(**U1-NFR-REL-07**, `u64::saturating_add`, R-LIMIT-05/06) + **U1-NFR-REL-04**(경계-정확·단조·actionable, R-LIMIT-01..06).

---

## 5. 오류/판정 값 계층 + 테스트 지원 논리 컴포넌트

### 5.1 ErrorTaxonomy / VerdictTypes
- **ErrorTaxonomy(`ScanError` / `BuildError`)**: 운영 오류. **`thiserror`** 파생(`Display`/`std::error::Error`)으로 원인(`source: io::Error`) 보존. `BuildError::Scan(ScanError)` / `Hash{path, source}`, `ScanError::RootUnavailable` / `Io{path, source}`. U0-NFR-MNT-01 워크스페이스 관례 승계. 지속 대상 아님(NFR-13 round-trip 비대상, FD §6.6).
- **VerdictTypes(`LimitVerdict` / `LimitViolation`)**: `validate`가 반환하는 **순수 판정 값 enum**(throw 아님). 구조화 변이(`TotalBytes`/`FileBytes{path,..}`/`FileCount`)를 보존해 actionable(FR-05). 상위(U3/U8)가 소비·표면화하며 U1은 push 안 함.
- **안착 NFR 패턴**: `nfr-design-patterns.md` §5 fail-fast + U0 taxonomy 경유(**U1-NFR-MNT-01**). `anyhow`식 타입소거 미채택(구조화 변이 소실 방지, U0 상속). `serde` 직접 의존 없음 — U1 도입 타입 미지속(T4).

### 5.2 ProptestGenerators — 런타임 그래프 밖 test-support 논리 단위 (PBT-07)
- **배치**: 도메인 제너레이터(`proptest`)는 **런타임 의존 그래프(§6) 밖**의 별도 test-support 논리 단위다. U1 자신의 **비-default `proptest-support` feature**로 게이트되어 릴리스 런타임 바이너리에 포함되지 않는다(U0 미러, `nfr-design-patterns.md` §8).
- **책임**: U0 `proptest-support` 제너레이터(`arb_manifest_entry`/`arb_relative_path`/`arb_sha256_digest`/`arb_byte_count`/`arb_manifest`) 재사용 + U1 전용 제너레이터 제공 — 임의 `Vec<u8>` + 청크 분할 시퀀스(PROP-U1-01), 경로 유일 엔트리 집합 + 순열 + 프리픽스 중첩(PROP-U1-02), 상관 매니페스트 쌍(PROP-U1-03), 경계 조준 SafetyLimits 입력 limit-1/limit/limit+1 + reject 증분 시퀀스(PROP-U1-04), digest-consistent 매니페스트 + 빈/단일/다수 `ChangeSet`(PROP-U1-05).
- **의존 방향**: `ProptestGenerators -> {ContentAddressing, ManifestDiffer, SafetyLimitsValidator, U0 proptest-support}`. 런타임 컴포넌트는 이 단위에 의존하지 않으므로 §6 런타임 DAG를 오염시키지 않는다.
- **이월**: 구체 구현·shrinking·고정 시드·CI 통합(PBT-08)은 Code Generation / Build-and-Test 이월.

---

## 6. 크레이트 의존 엣지 (비순환 + foundation-only 확인)

> 규약: `A -> B` = "A가 B에 의존한다(B의 타입/함수/계약을 사용)". U1 순수 컴포넌트는 U1 내부에서만 서로 의존하며, `VaultScanner`가 유일한 I/O 리프다. **U1 크레이트는 외부로 `foundation`(U0)에만 의존**한다.

### 6.1 U1 내부 의존 엣지 표

| 논리 컴포넌트 | 소속 공개 컴포넌트 | 의존 대상 |
|---|---|---|
| StreamHasher | `ContentAddressing` | `sha2`(외부) · U0 `Sha256Digest` |
| DigestFramer | `ContentAddressing` | StreamHasher(SHA-256 재사용) · U0 `ManifestEntry`/`ManifestDigest` · `std`(프레이밍) |
| DirWalker | `VaultScanner` | U0 `RelativePath::normalize` · ErrorTaxonomy(`ScanError`) · `std::fs` |
| ReaderFactory | `VaultScanner` | ErrorTaxonomy(`ScanError`) · `std::fs::File` |
| ManifestAssembler | `ManifestBuilder` | DirWalker · ReaderFactory · StreamHasher · DigestFramer · ErrorTaxonomy(`BuildError`) · U0 `Manifest`/`ManifestEntry` |
| ManifestDiffer | `ManifestDiffer` | U0 `Manifest`/`ManifestEntry`/`ChangeSet`/`RelativePath` (순수, `std`만) |
| LimitsValidator | `SafetyLimitsValidator` | U0 SafetyLimits 상수 · VerdictTypes · U0 `Manifest`/`ByteCount` · `std`(`saturating_add`) |
| ErrorTaxonomy | (값 계층) | `thiserror`(외부, U0 등재) · U0 `RelativePath` · `std::io::Error` |
| VerdictTypes | (값 계층) | U0 `RelativePath` (순수 enum) |

### 6.2 ASCII 의존 스케치 (박스 미사용)

```
U1 조립/판정 계층:

  ManifestAssembler  ->  DirWalker + ReaderFactory  (VaultScanner, 유일 I/O)
        |                      |
        +-> StreamHasher       +-> ScanError (ErrorTaxonomy)
        +-> DigestFramer
        +-> BuildError (ErrorTaxonomy)

  ManifestDiffer     ->  (U0 Manifest/ChangeSet, 순수)
  LimitsValidator    ->  VerdictTypes + U0 SafetyLimits 상수 (순수)

  방향 요약:  U1 컴포넌트  ->  U0 foundation (한 방향; U0는 U1을 import 안 함)
             U1 내부       ->  StreamHasher/DigestFramer/ErrorTaxonomy 리프
             U1            ->  {sha2, thiserror, std} (외부; proptest는 dev-feature)
                                                => 사이클 없음
```

### 6.3 비순환 및 foundation-only 확인
- **비순환(acyclic)**: 모든 엣지는 U1 상위 조립부 -> 순수 리프(StreamHasher/DigestFramer/ErrorTaxonomy/VerdictTypes) 또는 -> U0 방향으로만 흐르며 역방향 엣지가 없다.
- **foundation-only 외부 의존**: U1 크레이트의 유일한 워크스페이스 크레이트 의존은 `foundation`(U0, path 의존)이다 — 다른 단위(U2~U8)를 import하지 않는다. `sha2`(신규, 런타임)·`thiserror`(U0 등재 상속)·`std`가 외부/표준 의존이며 `proptest`는 `proptest-support` 하 dev-only feature다(§5.2). U0는 U1을 import하지 않으므로 DAG 비순환(U0 = Wave-1 루트, U1 = Wave-2)이 유지된다.
- **U0 소비 계약(재정의 없음)**: U1은 U0 `CoreTypes` 값 타입(`Manifest`/`ManifestEntry`/`ChangeSet`/`RelativePath`/`Sha256Digest`/`ManifestDigest`/`ByteCount`)·CBOR 코덱(`encode`/`decode`, REL-05 round-trip 재확인용)·`RelativePath::normalize`·SafetyLimits 상수를 **소비만** 한다(FROZEN). U1은 U0 push-only 싱크 트레이트(`Logger`/`StatusSink` 등)를 **구현하지 않으며**, 관측·표면화는 값 반환 후 U8이 수행한다(`nfr-design-patterns.md` §4).

---

## 7. PBT 타깃 정렬 (PBT-01 / 확장 Full)

> 신규 PBT 결정 없음(PBT-09 프레임워크 = `proptest`, U0 상속). 확정 속성(FD의 PROP-U1-01~05)을 명명 논리 컴포넌트에 정렬해 추적성을 강화한다. NFR ID -> 속성 매핑은 `nfr-design-patterns.md` §8/§11과 일관.

### 7.1 컴포넌트 -> Testable Property 정렬표

| 논리 컴포넌트 | 정렬 속성(FD 소유) | 카테고리 | 실현 NFR / 규칙 |
|---|---|---|---|
| StreamHasher | PROP-U1-01 (`hash_stream` 결정성 + `sha256sum` 오라클, 청크 무관) | Idempotence + Oracle | U1-NFR-REL-01(R-CA-01/02/03) |
| DigestFramer | PROP-U1-02 (순서 무관 결정성 + injective 프레이밍) | Invariant + Oracle | U1-NFR-REL-02(R-CA-04/05) |
| ManifestDiffer | PROP-U1-03 (`apply(prev, diff)==current`, 서로소, no-op 동치) | Oracle + Invariant | U1-NFR-REL-03(R-DIFF-01..06) |
| LimitsValidator | PROP-U1-04 (경계-정확 accept/reject + 단조 + actionable 위반) | Invariant + Monotonicity | U1-NFR-REL-04(R-LIMIT-01..06) |
| ManifestAssembler / ManifestDiffer 산출값 | PROP-U1-05 (U0 코덱 round-trip 재확인) | Round-trip | U1-NFR-REL-05(코덱=U0 소유) |
| LimitsValidator · ManifestDiffer · DigestFramer | REL-07 no-panic(대값 `size` 누적 `saturating_add`, 임의 엔트리 집합) | Invariant | U1-NFR-REL-07(§6 lint-gate) |

> **속성 없음(No PBT properties identified)**: DirWalker/ReaderFactory(`VaultScanner` I/O)는 값 속성이 없어 **임시 디렉터리 픽스처 기반 예제/통합 테스트**로 판정(심링크 SKIP·hidden 포함·fail-fast 시나리오, FD §6.6, blocking 아님). ErrorTaxonomy(`ScanError`/`BuildError`)는 지속 대상이 아니어서 round-trip 속성이 없다.

---

## 8. RESILIENCY-01 매핑 — U1 = 결정성 코어(사이클 정확성 기반)

- **분류**: U1 `content-core`는 **동기화 사이클 정확성의 결정성 코어**다 — U3(재검증 재사용)·U8(build/diff/validate 구동)·U4(산출 `Manifest` 지속)가 U1의 순수 계약에 의존한다. U1의 결정성/무손실 계약이 무너지면 no-op 판정·정확 diff·안전 한도가 무너진다.
- **회복력 기여(순수 lib 범위, RESILIENCY-01)**:
  - **fail-fast build + 루트 오류 반환(U1-NFR-REL-06, ManifestAssembler/DirWalker)** — 부분 매니페스트/0-파일 매니페스트가 diff에서 파괴적 커밋(잘못된 삭제/빈 커밋)을 유발하는 것을 방지.
  - **panic-free total(U1-NFR-REL-07, LimitsValidator/ManifestDiffer/DigestFramer)** — 100k/20 GiB 근접 대값에서도 `saturating_add`·순수 표면 lint-gate로 산술/인덱싱 크래시 회피.
  - **스트리밍 메모리 바운드(U1-NFR-PERF-01, StreamHasher/ManifestAssembler)** — 고정 버퍼 순차 처리로 OOM 회피.
  - **무손실 산출값(U1-NFR-REL-05, ManifestAssembler/ManifestDiffer)** — U1 산출 `Manifest`가 U4 zero-loss 지속(RPO=0, NFR-03)의 입력.
- **N/A 범위**: RTO/RPO 수치·DR/HA·서킷브레이커·auto-scaling·배포/롤백·카오스/DR 테스팅(RESILIENCY-02/05~14)은 순수 lib에 부적용. U1 신규 인프라 통제 없음(`nfr-design-patterns.md` §10/§12와 일관).

---

## 9. 확장 컴플라이언스 요약 (완료 게이트용)

| 확장 | 활성 | 이 산출물 판정 | 근거 |
|---|---|---|---|
| **Property-Based Testing** | ON (Full) | **준수 — blocking 없음** | §7이 각 논리 컴포넌트를 확정 속성(PROP-U1-01~05 + REL-07 no-panic)에 정렬(PBT-01), ProptestGenerators를 U1 `proptest-support` test-support 논리 단위로 문서화(PBT-07). 신규 PBT 결정 없음(PBT-09는 NFR-Req 충족). `VaultScanner` I/O는 예제/통합 판정(FD §6.6). PBT-08은 Code Generation/Build-and-Test 이월 |
| **Resiliency Baseline** | ON | **준수(부분 적용 + 대체로 N/A) — blocking 없음** | §8이 U1 = 결정성 코어 분류와 하류 의존(U3/U4/U8) 매핑을 컴포넌트 입도로 확정. REL-05/06/07·PERF-01 계약의 컴포넌트 안착 명시. RTO/RPO/DR/HA/서킷브레이커/카오스는 순수 lib에 N/A. 신규 resiliency 결정 없음 |
| **Security Baseline** | OFF | **N/A — 미로딩·미강제** | U1은 토큰/시크릿 미취급. `ScanError`/`Manifest`는 값 반환일 뿐 U1이 지속·로그하지 않음(push는 U8). RISK-01은 문서화된 수용 위험. 신규 강제 통제 없음 |

**블로킹 판정**: 이 산출물에 blocking finding 없음. §2~§5 컴포넌트 안착은 `nfr-design-patterns.md` §11 맵과 1:1이며, §6 의존 그래프는 비순환(U1 = Wave-2, 외부 의존은 `foundation`(U0)뿐, U0 미-import)이다.
