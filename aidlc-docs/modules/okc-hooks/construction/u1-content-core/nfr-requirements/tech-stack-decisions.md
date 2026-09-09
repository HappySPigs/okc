# U1 Deterministic Content Core — Tech Stack Decisions (기술 스택 결정)

**단계**: CONSTRUCTION -> Per-Unit Loop -> **U1 `content-core`** -> NFR Requirements -> 산출물 2/2 (`tech-stack-decisions.md`)
**작성일**: 2026-09-08
**크레이트**: `content-core` (lib) · **소속 컴포넌트**: `ContentAddressing`, `VaultScanner`, `ManifestBuilder`, `ManifestDiffer`, `SafetyLimitsValidator`
**입력 아티팩트**: `functional-design/{domain-entities,business-rules,business-logic-model}.md`(U1 FD) · `plans/u1-content-core-functional-design-plan.md` §3 결정 표(D1~D19)·§5 드롭리스트 · 자매 산출물 `nfr-requirements.md`(U1-NFR-REL/PERF/MNT 카탈로그) · U0 `u0-foundation/nfr-requirements/tech-stack-decisions.md`(워크스페이스 스택 상속 원본, FROZEN) · 활성 확장 `property-based-testing.md`(PBT-07/09)·`resiliency-baseline.md`
**규칙**: `construction/nfr-requirements.md` · `common/content-validation.md`(유니코드 박스 문자 미사용, 화살표 표기 `A -> B`, 표/코드블록 검증)

> **문서 성격**: 이 문서는 U1의 **구체 크레이트/툴체인 선택을 기록하는 유일한 산출물**이다. 자매 산출물 `nfr-requirements.md`가 카테고리별 NFR(기술중립)을 정의하고, 이 문서는 그 NFR을 실현할 구체 기술 결정과 각 결정의 근거·전파 범위를 확정한다. U1은 U0가 확정한 워크스페이스 스택(Edition 2024 / 고정 MSRV 1.85 / serde / ciborium / thiserror / proptest)을 **상속**하며, 이 문서는 **U1이 추가로 도입하는 것**과 **U0에서 물려받는 것**만 명시한다. English 식별자(크레이트명·타입·rule ID·NFR ID)는 원문 유지, Rust 제네릭/타입은 백틱으로 감싼다.

---

## 0. 상속 원칙 (U1 = Wave-2, U0 소비)

U1 `content-core`는 의존성 DAG에서 **U0 `foundation`(Wave-1 루트) 바로 위**에 있으며 `foundation`을 path 의존으로 소비한다. U0가 워크스페이스 전역 기본값으로 확정한 **언어·edition·MSRV·공용 크레이트·PBT 프레임워크**를 `crate.workspace = true` / `[workspace.dependencies]` 상속으로 물려받아 **드리프트 없이** 동일 스택을 재사용한다.

- **U0에서 상속(재결정 아님)**: Rust · Edition 2024 · 고정 MSRV(1.85, `[workspace.package].rust-version`) · `serde`(1) · `ciborium`(0.2, U0 코덱 경유) · `thiserror`(2) · `proptest`(1, dev) + `proptest-support` 노출 패턴. 근거 정본 = U0 tech-stack-decisions.md.
- **U1이 추가 도입**: **`sha2`**(SHA-256 스트리밍 해시) — 유일한 신규 런타임 크레이트(§2). 파일 읽기/디렉터리 walk/바이트 프레이밍은 `std`로 충분(신규 의존 없음).
- **전파 표기 규약**: 각 결정 절 말미에 "전파 범위"를 명시한다.
- **버전 정책**: 외부 크레이트는 `[workspace.dependencies]`에 major/minor 계열로 핀 후 상속. **정확한 patch 핀과 MSRV(1.85) 대비 CI 검증은 Build-and-Test 이월**(U0와 동일 스탠스).

---

## 1. 언어 / 툴체인 (U0 상속, NFR-05/NFR-17)

| 항목 | 결정 | 근거 |
|---|---|---|
| 언어 | **Rust** (상속) | NFR-17 + FQ-1=A(okc-core 의존성 0, 표준 SHA-256 직접 구현). PBT `proptest`의 전제. |
| Edition | **2024** (상속) | U0 §1, `edition.workspace = true`. |
| MSRV | **고정 1.85** (상속) | U0 §1, `rust-version.workspace = true`. NFR-05 크로스플랫폼 재현 빌드 보장. |

**메커니즘**: U1 `Cargo.toml`은 `edition.workspace = true` / `rust-version.workspace = true`로 U0가 워크스페이스 루트에 둔 값을 상속한다.

**전파 범위**: **U1 내부**(상속만; 신규 워크스페이스 결정 없음).

---

## 2. SHA-256 해싱 = `sha2` (RustCrypto) — 유일 신규 크레이트

`ContentAddressing::hash_stream<R: Read>`(파일 콘텐츠 -> `Sha256Digest`)와 `manifest_digest`(프레이밍 바이트열 -> `ManifestDigest`)는 **표준 SHA-256**을 요구한다(FQ-1=A, R-CA-01, U1-NFR-REL-01/02). U0는 `Sha256Digest`/`ManifestDigest` **값 타입**만 소유하고 결정성 **계약**만 정의했으며, **계산 로직은 U1 소유**(`domain-entities.md` §1 각주)다. 따라서 SHA-256 구현 크레이트를 U1이 도입한다.

**결정**: **`sha2`**(RustCrypto) 크레이트로 스트리밍 SHA-256을 계산한다.

**후보 비교:**

| 크레이트 | 순수 Rust | 스트리밍 API | 크로스플랫폼(NFR-05) | 판정 |
|---|---|---|---|---|
| **`sha2`** | 예(RustCrypto, C 의존 0) | 예 — `Digest::update`(증분) + `finalize`, 고정 버퍼 스트리밍에 직접 적합(R-CA-03) | 시스템 라이브러리 불필요, 3-OS 재현 빌드에 유리 | **채택** |
| `ring` | 아니오(C/asm 포함) | 예 | 빌드 복잡도·크로스컴파일 부담 증가 | 기각(MVP 대비 과함) |
| `openssl` | 아니오(시스템 OpenSSL 링크) | 예 | 시스템 의존 -> 재현 빌드·패키징 부담(NFR-05 역행) | 기각 |

**채택 근거**: (a) `Update`/`Finalize` 증분 API가 U1-NFR-PERF-01(고정 버퍼 스트리밍, 전량 적재 없음)·R-CA-03에 직접 맞고, (b) 순수 Rust라 macOS/Windows/Linux 재현 빌드(NFR-05)에 유리하며, (c) `Sha256`은 널리 검증된 표준 구현이라 `sha256sum` 일치(U1-NFR-REL-01 Oracle)를 신뢰할 수 있고, (d) MVP 대비 `ring`/`openssl`의 시스템/빌드 복잡도가 불필요하다.

**사용 형태(참고)**: `hash_stream`은 고정 버퍼로 `reader`를 반복 읽어 `Sha256::update`에 증분 공급하고 `finalize`로 32바이트 `Sha256Digest`를 얻는다. `manifest_digest`는 canonical 정렬 후 길이-프리픽스 프레이밍 바이트열(`u64::to_be_bytes` 등 `std`)을 `Sha256`에 흘려 계산한다(신규 프레이밍 의존 없음).

**신규 크레이트 명세:**

| crate | version(핀 계열) | kind | MSRV/Edition 호환 | 근거 |
|---|---|---|---|---|
| **`sha2`** | `0.10` | normal | MSRV 1.85 / Edition 2024 호환(0.10 계열 MSRV는 1.85 하한 훨씬 아래, C 의존 0으로 3-OS 재현 빌드 무리 없음) | FQ-1=A · R-CA-01/02 · U1-NFR-REL-01/02 · NFR-08 |

> **버전 핀 주석**: `sha2 = "0.10"`은 `[workspace.dependencies]`에 등재하는 major/minor 계열이다. 정확한 patch 핀과 MSRV 1.85 대비 CI 검증(`cargo check` MSRV 툴체인)은 **Build-and-Test 이월**(U0와 동일). RustCrypto 트레이트 크레이트(`digest`)는 `sha2`가 전이 의존으로 끌어오므로 U1이 직접 등재하지 않는다.

**전파 범위**: `sha2` 의존은 **U1 소유**. 계산 로직(`hash_stream`)은 **U3 `UploadProtocolDriver`**가 전송 시 blob 재검증(FR-22/Q8=A)에서 재사용하나, U3는 U1의 `hash_stream`을 호출하므로 `sha2`를 직접 등재할 필요가 없다(U1 경유).

---

## 3. 파일 읽기 / 디렉터리 walk = `std` (신규 의존 없음)

`VaultScanner`(U1 유일 파일 읽기)는 볼트 루트를 재귀 walk하고 파일별 스트리밍 리더를 연다(`business-logic-model.md` §3).

**결정**: **`std`만 사용**한다(신규 walk 크레이트 미도입).

- **디렉터리 walk**: `std::fs::read_dir` 재귀 순회. 심링크 SKIP(R-SCAN-02/D8)은 `DirEntry::file_type()` -> `FileType::is_symlink()`(따라가지 않고 판별, `std`)로 구현. dotfile 포함(R-SCAN-03)·exclude 패턴 없음(R-SCAN-04/D9)이라 필터 로직이 최소다.
- **스트리밍 리더**: `open_reader`는 `std::fs::File`(+ 고정 버퍼)을 `Box<dyn Read>`로 반환. `hash_stream`이 `Read`를 소비하므로 파일을 전량 적재하지 않는다(U1-NFR-PERF-01).
- **바이트 프레이밍**: `manifest_digest`의 길이-프리픽스 프레이밍은 `u64::to_be_bytes`/`Vec<u8>` 등 `std`로 충분.

**`walkdir` 크레이트를 쓰지 않는 이유**: `walkdir`는 편의를 주지만 (a) 심링크 SKIP·정렬은 U1이 어차피 `std` `FileType`/canonical 정렬로 직접 제어해야 하고(R-SCAN-02/06), (b) exclude 패턴이 없어(D9) walk 커스터마이즈 필요가 낮으며, (c) MVP 최소 의존 원칙상 `std` 재귀로 충분하다. 신규 의존을 정당화할 이득이 없어 미채택.

**전파 범위**: **U1 내부 한정**(`std`만; 신규 워크스페이스 의존 없음).

---

## 4. 오류 처리 = `thiserror`(상속) + 순수 판정 값 타입

U1은 U0가 확정한 워크스페이스 오류 관례(U0 §5, U0-NFR-MNT-01)를 승계한다.

| 부류 | 타입 | 파생 전략 | 근거 |
|---|---|---|---|
| **운영 오류(반환용)** | `ScanError` · `BuildError` | **`thiserror`** 파생(`Display`/`std::error::Error`), `source`로 `io::Error` 원인 보존 | 라이브러리 관용·워크스페이스 일관(U0 상속). 오류 국소성(`BuildError::Hash{path, source}`). |
| **판정 값 타입(throw 아님)** | `LimitVerdict` · `LimitViolation` | **순수 enum**(파생 없이 반환값) | `validate` 반환 **값** — 상위(U3/U8)가 소비·표면화. 지속/직렬화 대상 아님(FD §6.6). 구조화 변이 보존 -> actionable(FR-05). |

**`anyhow`를 쓰지 않는 이유(U0 상속)**: 타입소거는 `LimitViolation`의 구조화 변이(`TotalBytes`/`FileBytes`/`FileCount`)를 소실시켜 actionable 리포트(FR-05, U1-NFR-REL-04 AC-3)를 불가능하게 한다.

**serde 필요 여부**: U1이 **도입하는** 타입(`ScannedFile`/`VaultSnapshot`/`SafetyLimits`/`LimitVerdict`/`LimitViolation`/`ScanError`/`BuildError`)은 **지속되지 않으므로** serde 파생이 불필요하다(FD §6.6). NFR-13 round-trip(U1-NFR-REL-05) 대상은 **U0 소유 타입**(`Manifest`/`ChangeSet`, 이미 serde 파생)이며 U1은 U0 `encode`/`decode`를 호출할 뿐이다. 따라서 U1은 `serde`를 **직접 의존으로 등재하지 않는다**(U0 타입 구성 시 파생은 U0에 있음).

**panic-free total 연계(U1-NFR-REL-07)**: `validate`의 총량 누적은 `u64::saturating_add`(std)로 오버플로 패닉을 방지하고(R-LIMIT-05), 순수 표면(`hash_stream`·`manifest_digest`·`diff`)은 `unwrap`/슬라이스 패닉 없이 값/`Result`로 표면화한다. 이는 크레이트 선택과 무관한 **구현 규율**이며 PBT no-panic 속성(§6)으로 검증한다.

**전파 범위**: `thiserror` 의존은 워크스페이스 상속(U0 등재). 판정 값 타입은 **U3/U8**가 소비.

---

## 5. 설정 값 주입 = 생성자 주입 (federated-config 정합, U0 FROZEN)

U1은 **U0 `ConfigProvider`를 직접 읽지 않는다**. 필요한 설정 값은 U8 composition root(`watcher-bin`)가 조립 시점에 **생성자 주입(downward injection)** 한다(wave-level DEC-FEDERATED-KEYS Q6=A).

| U1이 필요로 하는 값 | 종류 | 원천 / 주입 방식 |
|---|---|---|
| 볼트 루트 경로(`vault_path`) | U0 **6 CORE 필드** 중 하나(`PathBuf`) | U8이 U0 `ConfigSnapshot`에서 해소한 루트 경로를 `VaultScanner` 생성자에 주입 |
| SafetyLimits(20 GiB / 2 GiB / 100k) | **U0 컴파일타임 상수**(config 아님) | `MAX_VAULT_TOTAL_BYTES`/`MAX_FILE_BYTES`/`MAX_FILE_COUNT` 직접 참조(D13, Q2=A) |
| exclude/ignore 패턴 | **없음**(D9/MVP-trim) | 미지원 — U1은 federated config 키를 등록하지 않음(U0가 미지 키로 거부) |

- **federated 키 없음**: FD 계획 D9로 exclude_patterns가 MVP에서 제거되어, **U1은 U0 known-key set에 등록하는 federated 설정 키가 없다**. SafetyLimits는 config가 아니라 U0 상수다. 따라서 U1에는 request_timeout_s/backoff류 non-core 파라미터 주입 대상도 없다.
- **U0 FROZEN 유지**: U1은 U0 core-field 리로드 관찰자를 구현하지 않는다(볼트 루트는 조립 시점 주입값; 라이브 리로드는 MVP 범위 밖, 재시작으로 변경). U1은 순수 계산 값만 반환하고 config 스냅샷을 보관하지 않는다.

**전파 범위**: 주입 계약은 **U8 composition root** 소유. U1은 생성자 파라미터로만 값을 받는다(`ConfigProvider` 읽기 없음).

---

## 6. 속성 기반 테스트 = `proptest`(상속) + `proptest-support` feature (PBT-07/09)

### 6.1 프레임워크 = `proptest` (U0 상속, PBT-09)

PBT-09(프레임워크 선택)는 U0에서 **`proptest`** 로 확정되어 워크스페이스 전역 dev-dependency로 상속된다(재선택 없음). U1은 FD 계획 D19로 이를 확정하며 U1의 PROP-U1-01~05를 `proptest`로 구현한다.

### 6.2 제너레이터 노출 = U1 `proptest-support` 비기본 feature (Q10=A 미러, PBT-07)

**결정**: U0의 `proptest-support`(비기본 feature) 제너레이터를 재사용하고, U1 전용 제너레이터를 **U1 자신의 비기본 `proptest-support` feature**로 노출한다(U0 패턴 미러).

- **U0에서 재사용(dev-dependency, U0 feature 활성)**: `arb_manifest_entry` · `arb_relative_path` · `arb_sha256_digest` · `arb_byte_count` · `arb_manifest`.
- **U1 전용 제너레이터(U1 `proptest-support` feature로 노출)**:
  - 임의 `Vec<u8>`(빈·1바이트·경계·대용량 축소본) + 임의 청크 분할 시퀀스 — PROP-U1-01(해싱 결정성/Oracle).
  - 경로 유일 엔트리 집합 + 순열 + 경로 프리픽스 중첩 케이스 — PROP-U1-02(순서 무관/injective).
  - 상관 매니페스트 쌍(기저 + add/modify/delete 파생) — PROP-U1-03(diff 오라클/재구성).
  - 경계 조준 SafetyLimits 입력(limit-1 / limit / limit+1) + reject 증분 시퀀스 — PROP-U1-04(경계-정확·단조).
  - digest-consistent 매니페스트(엔트리 집합에서 실제 `manifest_digest` 계산) + 빈/단일/다수 `ChangeSet` — PROP-U1-05(round-trip).

- **feature 게이트**: 비기본 feature이므로 `proptest`가 U1의 비테스트/프로덕션 빌드에 유출되지 않는다(`proptest`는 `proptest-support` 하의 optional dev-dependency). 제너레이터 단일-출처(U0) 원칙 유지 -> 단위 간 드리프트 방지.

**전파 범위**: `proptest`는 워크스페이스 공용 dev-dependency(U0 상속). U1 `proptest-support` feature는 U1 자체 테스트가 활성화하며, 향후 U1 산출값을 재검증하는 하위 단위(예: U3 재검증 테스트)가 dev에서 재사용 가능.

### 6.3 시드 재현성 및 이월(PBT-08)

`proptest`는 실패 시 시드를 기록하고 회귀 파일(`proptest-regressions`)로 최소 실패 케이스를 고정한다. **케이스 수·shrink 튜닝·고정 시드 정책·CI 파이프라인 통합(PBT-08)은 Code Generation / Build-and-Test 이월**(U0와 동일). 이 문서는 프레임워크 상속(PBT-09)과 U1 제너레이터 노출 방식(PBT-07)만 확정한다.

---

## 7. 문서 / 품질 린트 (U0 정합)

| 항목 | 결정 | 근거 |
|---|---|---|
| 커버리지 게이트 | **전역 커버리지 % 게이트 없음** | U0-NFR-MNT-03 정합. 실질 검증 = PBT(PROP-U1-01~05) + 예제 앵커(`VaultScanner` I/O·심링크 SKIP·fail-fast, PBT-10). |
| 공개 API 문서 | **`#![deny(missing_docs)]` on `content-core` public items** | `hash_stream`(U3 재사용)·`validate`(U3 재검사)·`build`/`diff`(U8 사이클) 등 상위 단위 소비 공개 표면 문서화를 컴파일타임에 강제. |

**이월**: CI 커버리지 통합의 구체 방식은 **Build-and-Test 이월**. PBT-10(예제 병행)은 Code Generation에서 property test와 example 테스트를 병행하는 형태로 적용.

**전파 범위**: `#![deny(missing_docs)]`는 **U1 `content-core` 공개 항목 한정**.

---

## 8. Autopilot Decisions (질문 대체 — RECOMMENDED · MVP 편향)

AUTOPILOT(사용자 승인, 게이트 waived) 하에 이 NFR 단계에서 저자가 확정한 열린 항목이다. FD 계획 §3(D1~D19)은 재오픈하지 않으며, 아래는 **기술 스택/NFR 실현 선택**에 국한한다.

| # | 주제(topic) | 선택(chosen) | MVP-trim? | 근거 |
|---|---|---|---|---|
| T1 | SHA-256 구현 크레이트 | **`sha2`**(RustCrypto, 순수 Rust, 스트리밍 `Update`/`Finalize`) | 아니오 | 스트리밍 결정성(R-CA-03)·크로스플랫폼 재현 빌드(NFR-05)에 직접 맞음. `ring`/`openssl`은 MVP 대비 빌드/시스템 의존 과함(§2). |
| T2 | 디렉터리 walk / 파일 읽기 | **`std`만**(`fs::read_dir` 재귀 + `FileType::is_symlink` SKIP + `File`+고정버퍼 스트리밍) | **예** | `walkdir` 등 신규 의존 불필요 — exclude 패턴 없음(D9)·심링크 SKIP/정렬은 `std`로 직접 제어(§3). |
| T3 | 다이제스트 프레이밍 구현 | **`std` 바이트 프레이밍**(`u64::to_be_bytes` 길이-프리픽스 + concat) + `sha2` | 아니오 | injective 인코딩(R-CA-04)·CBOR 코덱과 분리(D1). 신규 직렬화 의존 없음(§2). |
| T4 | U1 도입 타입의 serde 파생 | **없음**(U1 도입 타입 미지속) | **예** | `ScannedFile`/`VaultSnapshot`/`Limit*`/`*Error`는 지속 대상 아님(FD §6.6). NFR-13 round-trip은 U0 타입 대상 -> `serde` 직접 의존 불필요(§4). |
| T5 | 설정 값 취득 방식 | **생성자 주입**(볼트 루트 = U0 core-field, U8 주입); federated 키 없음(exclude_patterns 제거) | **예** | wave-level federated-config(Q6=A) 정합, U0 FROZEN 유지. SafetyLimits = U0 상수(§5). |
| T6 | 오류 파생 | `thiserror`(운영 오류) 상속 + 순수 판정 값 타입 | 아니오 | U0-NFR-MNT-01 워크스페이스 관례 승계(§4). |
| T7 | PBT 제너레이터 노출 | U0 `proptest-support` 재사용 + U1 전용 `proptest-support` 비기본 feature | 아니오 | 단일-출처·드리프트 방지·proptest 프로덕션 유출 차단(U0 미러, §6). |
| T8 | 성능 수치 게이트 | **없음**(정성 계약만) | **예** | 처리 비용은 입력 크기에 선형·유계, 단일 사용자 로컬(U0 정성-성능 스탠스 미러, `nfr-requirements.md` §3.1). |
| T9 | 문서 린트 | `#![deny(missing_docs)]` + no 커버리지 %-게이트 + 예제 앵커 | 아니오 | 다수 상위 단위 소비 공개 표면 -> near-free 문서 강제(U0 정합, §7). |

---

## 9. 의존성 요약표

**범례**: version-policy = 외부 크레이트는 `[workspace.dependencies]` 핀 후 상속(inherit), 내부는 `[workspace.package]` 상속 + `publish=false` + path 의존. kind = normal(런타임) / dev(테스트). propagation = 소비 방향.

| crate | version-policy | kind | feature | 신규? | grounding |
|---|---|---|---|---|---|
| `sha2` | workspace-inherited (핀: `0.10`) | normal | — | **신규(U1 도입)** | FQ-1=A · R-CA-01/02 · U1-NFR-REL-01/02 · NFR-08 (§2) |
| `thiserror` | workspace-inherited (핀: `2`) | normal | — | 아니오(U0 등재) | U0 §5 상속 · `ScanError`/`BuildError` 파생 (§4) |
| `proptest` | workspace-inherited (핀: `1`) | **dev** | `proptest-support`(U1 비기본, 제너레이터 노출) | 아니오(U0 등재) | PBT-09(U0 상속)/PBT-07 · D19 (§6) |
| `foundation`(U0) | `[workspace.package]` 상속, `publish=false`, path 의존 | normal | (U0 `proptest-support` = dev 활성) | 아니오 | 값 타입·코덱·`RelativePath::normalize`·SafetyLimits 상수 소비(FROZEN) |
| `std` (walk/read/프레이밍) | — | — | — | 아니오 | `VaultScanner` I/O·`manifest_digest` 프레이밍·`saturating_add` (§3, §4) |

> **미채택(의도적)**: `walkdir`(§3, `std`로 충분) · `serde`(U1 직접 등재 불필요, §4) · `ring`/`openssl`(§2, MVP 대비 과함). patch 핀·MSRV 1.85 대비 CI 검증은 **Build-and-Test 이월**.

---

## 10. 범위 절제 기록 (Scope Trims / Deferrals)

| 절제 항목 | 결정 | 근거 / 이월처 |
|---|---|---|
| **`walkdir` 등 walk 크레이트** | **미도입** | exclude 패턴 없음(D9)·심링크 SKIP/정렬 `std` 직접 제어(§3). MVP 최소 의존. |
| **U1 도입 타입 serde 파생** | **없음** | 미지속(FD §6.6). NFR-13 대상은 U0 타입(§4). `serde` 직접 의존 불필요. |
| **exclude/ignore federated config 키** | **없음(제거)** | D9/MVP-trim. U1은 U0 known-key set에 키 미등록(§5). |
| **성능 수치 게이트** | **없음(정성 계약만)** | U0 정성-성능 스탠스 미러(`nfr-requirements.md` §3.1). |
| **스트리밍 버퍼 구체 크기** | **NFR Design 이월** | D3 — 고정 버퍼 스트리밍 방침만 확정, 수치는 NFR Design. |
| **PBT-08 상세**(케이스 수/shrink/시드/CI) | **이월** | Code Generation / Build-and-Test(§6.3, U0와 동일). |
| **patch 핀 + MSRV-CI 검증** | **이월** | Build-and-Test(§9, U0와 동일). 이 문서는 major/minor 계열만 확정. |

---

## 11. 확장 컴플라이언스 요약 (완료 게이트용)

| 확장 | 활성 | 이 문서 적용 판정 | 근거 |
|---|---|---|---|
| **Property-Based Testing** | ON (Full) | **강제·준수** | §6이 프레임워크 `proptest`(U0 상속, PBT-09) + U1 `proptest-support` feature(PBT-07, 제너레이터 노출) 확정. PBT-08 상세는 Build-and-Test 이월(명시). blocking 없음. |
| **Resiliency Baseline** | ON | **부분 적용 + 대체로 N/A** | `sha2` 스트리밍(§2)이 OOM 회피(U1-NFR-PERF-01), `thiserror` fail-fast 오류(§4)가 파괴적 부분 커밋 방지(U1-NFR-REL-06), `saturating_add`(§4)가 panic-free total(REL-07) 실현. RTO/RPO 수치·DR·HA·배포/롤백은 U1(순수 lib)에 **N/A**(RESILIENCY-02). |
| **Security Baseline** | OFF | **N/A** | 미로딩·미강제. U1은 토큰/시크릿 미취급 — 신규 보안 크레이트/통제 없음. RISK-01은 문서화된 수용 위험. |

**N/A NFR 카테고리(기술 선택 없음)**: 확장성 · 가용성 · 성능 수치 목표(스트리밍 정성 계약만) · Resiliency DR/RTO/RPO · 사용성(사람-대면) · 보안 강제 통제. 근거 상세는 자매 산출물 `nfr-requirements.md` §7 N/A 판정표.
