# U1 Deterministic Content Core — Domain Entities (도메인 엔티티 / 값 타입)

**단계**: CONSTRUCTION -> Per-Unit Loop -> **U1 `content-core`** -> Functional Design -> 산출물 1/3 (`domain-entities.md`)
**작성일**: 2026-09-08
**크레이트**: `content-core` (lib) · **소속 컴포넌트**: `ContentAddressing`, `VaultScanner`, `ManifestBuilder`, `ManifestDiffer`, `SafetyLimitsValidator`
**전제(드롭리스트)**: FQ-1=A(표준 SHA-256, 권위 `vault_content_id`는 서버) · FQ-2=A(최신-상태-대체, 재스냅샷 diff) · Q2=A(SafetyLimits = 컴파일타임 상수, U0 소유) · U0 `CoreTypes` 값 타입·CBOR 코덱·`RelativePath` 정규화 이미 구현.

> **문서 성격**: 이 문서는 U1이 **도입/특화**하는 값 타입만 정의한다. U0가 소유하는 타입(`Manifest`/`ManifestEntry`/`ChangeSet`/`RelativePath`/`Sha256Digest`/`ManifestDigest`/`ByteCount`)은 **이름으로 참조**하고 재정의하지 않는다 — 여기서는 U1이 그 타입들을 **어떻게 사용/확장**하는지만 기술한다. 결정 규칙은 `business-rules.md`, 알고리즘·흐름은 `business-logic-model.md`가 소유한다.
>
> **표기 규약**: 기술중립 설계. Rust 시그니처는 참고용(reference-only). ASCII 화살표(`A -> B`)만, 박스/유니코드 다이어그램 없음. Rust 타입/제네릭은 backtick(`Vec<u8>`, `Box<dyn Read>`).

---

## 1. U0에서 소비하는 타입 (재정의 없음 — 참조 방식만 명시)

| U0 타입 | U1에서의 역할 | U1이 산출/소비 |
|---|---|---|
| `RelativePath` | 스캔·엔트리·diff 삭제 목록의 경로 식별자. U1은 파일시스템 경로를 `RelativePath::normalize`로 변환해 사용 | 산출(scan) / 소비(diff 키) |
| `Sha256Digest` | 파일별 `raw_sha256`. `ContentAddressing::hash_stream`의 출력 | 산출 |
| `ManifestDigest` | 정렬된 매니페스트 전체의 로컬 no-op 판별자. `ContentAddressing::manifest_digest`의 출력 | 산출 |
| `ByteCount` | 파일 크기·총량. `ManifestEntry.size`·SafetyLimits 누적에 사용 | 산출/소비 |
| `ManifestEntry` | `{relative_path, raw_sha256, size}` — `ManifestBuilder`가 파일당 1개 조립 | 산출 |
| `Manifest` | `{entries(canonical 정렬), manifest_digest}` — 사이클마다 재계산 | 산출(build) / 소비(diff·validate 입력) |
| `ChangeSet` | `{added, modified, deleted}` — `ManifestDiffer::diff`의 출력, `is_empty()`로 no-op 판정 | 산출 |

- **정규화 위임**: U1은 경로 정규화 로직을 재구현하지 않는다. 파일시스템에서 얻은 상대경로 문자열을 `RelativePath::normalize(&str) -> Result<RelativePath, PathError>`(U0)에 넘겨 정규화·검증한다. `..`/절대경로/드라이브 접두는 U0가 거부한다.
- **다이제스트 계산 소유**: `ManifestDigest`/`Sha256Digest` **값 타입은 U0**, **계산 로직은 U1**(`ContentAddressing`). U0는 결정성 계약만 정의했다(`domain-entities.md` U0 §1.2).

---

## 2. U1이 도입하는 값 타입

### 2.1 ScannedFile

- **목적**: `VaultScanner`가 볼트 열거에서 발견한 **파일 1개의 경로+크기**(해시 이전 단계). `ContentAddressing`이 아직 해시하지 않은 상태의 중간 레코드다.
- **필드 스키마**

  | 필드 | 개념 타입 | 의미 |
  |---|---|---|
  | `relative_path` | `RelativePath` | 볼트 루트 기준 정규화 경로(U0 정규화 통과) |
  | `size` | `ByteCount` | 열거 시점의 파일 바이트 크기 |

  참고 형상: `struct ScannedFile { relative_path: RelativePath, size: ByteCount }`

- **불변식**: `relative_path`는 정규화됨(U0 불변식 상속). 심링크는 `ScannedFile`로 산출되지 않는다(R-SCAN-02 SKIP). `size`는 열거 시점 값이며 해시 시점과 다를 수 있다(TOCTOU) — 최종 정합은 U3 전송 재검증이 담당(FR-22).
- **동등성/정렬**: `relative_path` 오름차순(canonical). 매니페스트 정렬과 동일 키.

### 2.2 VaultSnapshot

- **목적**: **한 사이클의 일관 시점 열거 결과** — 발견된 모든 `ScannedFile` + 루트 + 캡처 시각. `ManifestBuilder`의 입력이자, U3가 전송 시 파일 바이트를 재-읽기하기 위한 경로 원천이다.
- **필드 스키마**

  | 필드 | 개념 타입 | 의미 |
  |---|---|---|
  | `root` | 파일시스템 경로(`PathBuf`) | 열거 대상 볼트 루트(config `vault_path`) |
  | `files` | `ScannedFile`의 canonical 정렬 시퀀스 | 발견된 파일 목록(경로 오름차순) |
  | `captured_at` | `Timestamp` | 열거를 시작/완료한 시각(진단·상태 표면화용) |

  참고 형상: `struct VaultSnapshot { root: PathBuf, files: Vec<ScannedFile>, captured_at: Timestamp }`

- **일관 시점의 의미(D11)**: `VaultSnapshot`은 **OS 레벨 파일시스템 스냅샷이 아니다**. 단일 walk 패스로 얻은 최선 노력(best-effort) 열거이며, 열거 중 볼트가 바뀌어도 TOCTOU는 U3 전송 시 blob 재해시(FR-22/Q8=A)로 폐쇄된다. `captured_at`은 그 논리 시점을 표시한다.
- **불변식**: `files`는 `relative_path` 오름차순 canonical 정렬, 경로 유일. 0-파일 스냅샷(빈 볼트)은 유효값(파괴적 판정은 U2).

### 2.3 SafetyLimits (U0 상수의 뷰)

- **목적**: 전송 전 프리플라이트 3한도의 **묶음 뷰**. 값은 **U0 컴파일타임 상수**를 참조하며 U1은 재정의하지 않는다(D13, Q2=A).
- **필드 스키마 / 값 원천**

  | 필드 | 개념 타입 | 값 | U0 원천 상수 |
  |---|---|---|---|
  | `max_total_bytes` | `u64` | 20 GiB | `MAX_VAULT_TOTAL_BYTES` |
  | `max_file_bytes` | `u64` | 2 GiB | `MAX_FILE_BYTES` |
  | `max_file_count` | `u64` | 100,000 | `MAX_FILE_COUNT` |

  참고 형상: `struct SafetyLimits { max_total_bytes: u64, max_file_bytes: u64, max_file_count: u64 }` — 기본값은 U0 상수로 고정 구성.

- **불변식**: 상수. config로 변경 불가(대응 config 키는 U0 R-CFG-STRICT-01로 미지 키 거부). ≤10-sources 프로젝트 한도는 서버 권위(DEP-04)라 U1 미포함.

### 2.4 LimitViolation / LimitVerdict

- **목적**: `SafetyLimitsValidator::validate`의 판정 결과. 초과 시 **어떤 한도를 얼마나** 넘었는지 실행 가능한(actionable) 정보(FR-05).
- **변이/필드**

  `LimitViolation` (variants):

  | 변이 | 필드 | 의미 |
  |---|---|---|
  | `TotalBytes` | `limit: u64`, `actual: u64` | 총 볼트 크기 초과 |
  | `FileBytes` | `path: RelativePath`, `limit: u64`, `actual: u64` | 특정 파일이 파일당 한도 초과 |
  | `FileCount` | `limit: u64`, `actual: u64` | 파일 수 초과 |

  `LimitVerdict` (variants): `WithinLimits` | `Exceeded(LimitViolation)`

  참고 형상:
  ```
  enum LimitViolation { TotalBytes { limit: u64, actual: u64 },
                        FileBytes  { path: RelativePath, limit: u64, actual: u64 },
                        FileCount  { limit: u64, actual: u64 } }
  enum LimitVerdict   { WithinLimits, Exceeded(LimitViolation) }
  ```

- **MVP 차이(D15)**: `Exceeded`는 **단일** `LimitViolation`을 담는다(모든 위반 누적 리스트가 아님) — 첫 위반에서 short-circuit. component-methods 원안의 `Exceeded(Vec<LimitViolation>)`에서 MVP로 단일화한다. accept/reject boolean은 이와 무관하게 단조(monotone)로 유지된다(R-LIMIT-03).
- **불변식**: `WithinLimits`는 세 한도 **모두** 경계 내(`actual <= limit`). `Exceeded`는 정확히 하나의 위반을 지목하되, 그 존재는 "적어도 하나 초과"와 동치.

### 2.5 ScanError

- **목적**: `VaultScanner`의 열거/리더 오류. **판정(vault-unavailable) 해석은 U1이 하지 않고 값으로 반환**하여 U2 `VaultAvailabilityGuard`가 해석한다(§0 노트2).
- **변이**

  | 변이 | 필드 | 의미 |
  |---|---|---|
  | `RootUnavailable` | (없음) | 루트 부재/언마운트/접근 불가 — 볼트 경계 자체 도달 불가 |
  | `Io` | `path: RelativePath`, `source: io::Error` | 특정 파일/디렉터리 열거·읽기 I/O 오류 |

  참고 형상: `enum ScanError { RootUnavailable, Io { path: RelativePath, source: io::Error } }`

- **불변식**: `RootUnavailable`은 루트 수준 실패(0-파일 매니페스트를 만들지 않고 오류로 표면화 — 파괴적 빈 커밋 방지의 U1측 근거, US-E1-06). U1은 이를 로그/상태로 push하지 않는다(반환만).

### 2.6 BuildError

- **목적**: `ManifestBuilder::build`가 스캔 또는 해시 단계에서 실패했음을 표현. fail-fast 정책(D12)에 따라 부분 매니페스트를 반환하지 않는다.
- **변이**

  | 변이 | 필드 | 의미 |
  |---|---|---|
  | `Scan` | `ScanError` | 열거 단계 실패(래핑) |
  | `Hash` | `path: RelativePath`, `source: io::Error` | 특정 파일 스트리밍 해시 중 I/O 오류 |

  참고 형상: `enum BuildError { Scan(ScanError), Hash { path: RelativePath, source: io::Error } }`

- **불변식**: `build`는 `Ok(Manifest)` 또는 `Err(BuildError)`만 반환한다 — **부분 성공(일부 파일 스킵) 없음**(D12). 어떤 파일이라도 해시 실패 시 전체 사이클을 중단해 diff가 그 파일을 "삭제"로 오인하지 않게 한다.

---

## 3. 다이제스트 도메인 인코딩 규약 (D1 — U0에서 이월된 세부)

`ManifestDigest`는 **정렬된 `(relative_path, raw_sha256, size)` 3-튜플 시퀀스**에 대한 표준 SHA-256이다. U0는 결정성 계약만 정의했고, **정확한 입력 인코딩은 U1이 확정**한다(U0 `domain-entities.md` §1.2 이월).

- **canonical 정렬**: 엔트리를 `relative_path` 바이트 사전식 오름차순으로 정렬(tie-break `(raw_sha256, size)`). U0 `ManifestEntry`의 derive `Ord`와 일치.
- **길이-프리픽스 프레이밍(D1)**: 각 엔트리를 다음 순서로 결정적 바이트열로 직렬화한 뒤 전체를 concat하여 SHA-256에 흘린다:
  1. `relative_path` UTF-8 바이트의 길이(고정폭 정수, big-endian) + 경로 바이트
  2. `raw_sha256` 32바이트(고정폭, 길이 프리픽스 불필요)
  3. `size`(고정폭 `u64` big-endian)
- **길이 프리픽스 근거**: 경로 사이 구분자 없이 concat하면 서로 다른 엔트리 집합이 같은 바이트열을 만들 수 있다(예: `["ab","c"]` vs `["a","bc"]`). 길이 프리픽스가 이 모호성을 제거해 **injective(단사)** 인코딩을 보장한다.
- **CBOR 코덱과의 분리(D1)**: 이 프레이밍은 U0 CBOR 지속 코덱(`encode`/`decode`)과 **별개**다. 지속 코덱은 값 복원(round-trip)용이고, 다이제스트 프레이밍은 **버전 간 안정적 콘텐츠 지문**용이다 — 지속 스키마에 신규 필드가 추가돼도(NFR-13 진화) 다이제스트가 흔들리지 않아 no-op 판정(FR-10)이 유지된다.
- **결정성 계약**: 논리적으로 동일한 엔트리 집합은 발견/삽입 순서와 무관하게 항상 동일한 `manifest_digest`를 낸다(canonical 정렬 후 계산). 상세 속성은 `business-rules.md` PROP-U1-02.

---

## 4. 엔티티 관계 개요 (화살표 표기)

**합성/산출 관계 ("A -> B" = A가 B를 포함/산출):**

- `VaultSnapshot` -> `ScannedFile`(다수, canonical 정렬) + `root` + `captured_at`
- `ScannedFile` -> `RelativePath`(1) + `ByteCount`(1)
- `ContentAddressing.hash_stream(reader)` -> `Sha256Digest`(파일 콘텐츠, 표준 SHA-256)
- `ManifestBuilder.build` -- (scan + 파일별 hash + digest) --> `Manifest`(U0 타입)
- `Manifest` -> `ManifestEntry`(다수) + `ManifestDigest`(1) [U0 소유 형상]
- `ContentAddressing.manifest_digest(entries)` -> `ManifestDigest`(길이-프리픽스 프레이밍, §3)
- `ManifestDiffer.diff(last_committed, current)` -> `ChangeSet`(U0 타입; empty == no-op)
- `SafetyLimitsValidator.validate(manifest)` -> `LimitVerdict`(WithinLimits | Exceeded(LimitViolation))

**교차 단위 경계 (소유 방향):**

- 마지막 커밋 `Manifest` -- (owned/persist) --> **U4 `SyncStateStore`**; `ManifestDiffer.diff`에 **인자로 전달**(U1은 지속하지 않음)
- `ContentAddressing.hash_stream` -- (reuse) --> **U3 `UploadProtocolDriver`** 전송 시 재검증(Q8=A/노트1)
- `ScanError`/`LimitVerdict` -- (판정 값 반환) --> **U8 `SyncCycleCoordinator`** 가 로그/상태/헬스로 표면화(U1은 push 안 함, §0 노트2)
- SafetyLimits 값 -- (상수 참조) --> **U0 `config::model`** 상수(재정의 없음)

**텍스트 설명(다이어그램 대안)**: U1의 데이터 흐름은 "볼트 루트 -> `VaultSnapshot`(열거) -> 파일별 `Sha256Digest`(스트리밍 해시) -> `ManifestEntry` 집합 -> canonical 정렬 + `ManifestDigest` -> `Manifest`"의 단방향 빌드 파이프라인이다. 이렇게 만든 현재 `Manifest`와 U4가 넘겨준 마지막 커밋 `Manifest`를 `ManifestDiffer`가 비교해 `ChangeSet`을 낸다. 전송 전에는 `SafetyLimitsValidator`가 `Manifest`를 3한도로 검사한다. 어느 단계도 이벤트를 누적하지 않으며(FQ-2=A), 지속 상태는 U4가, 관측 표면화는 U8이 소유한다.
