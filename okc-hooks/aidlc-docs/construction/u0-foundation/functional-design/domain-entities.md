# U0 Foundation — Domain Entities (도메인 엔티티 / 값 타입)

**단계**: CONSTRUCTION → Per-Unit Loop → **U0 Foundation** → Functional Design → 산출물 1/3 (`domain-entities.md`)
**작성일**: 2026-09-08
**크레이트**: `foundation` (lib) · **소속 컴포넌트**: `CoreTypes`, `ConfigProvider`
**전제(확정 답변)**: Q1=B(내부 지속 상태 = CBOR) · Q2=A(SafetyLimits = 컴파일타임 상수) · Q3=A(리로드 실패 = keep-last-good, 최초 로드 실패 = abort) · Q4=B(알 수 없는 config 키 = strict reject) · Q5=A(리로드 트리거 = CLI `reload`만) · Q6=A(발견 = 플랫폼 기본 경로 + `--config` + `OKC_WATCHER_CONFIG`) · Q7=A(토큰 우선순위 = secure-store > config > env) · Q8=A(SyncState = 선형 + dirty 재진입) · FQ-1=A(표준 SHA-256, 권위 `vault_content_id`는 서버) · FQ-2=A(최신 상태 대체) · FQ-3=A(NFR-13 오버레이: US-E7-06 queue→sync-state 정정, requirements §12.2)

> **문서 성격**: 이 문서는 U0가 소유하는 **타입 파운데이션**이다. `business-rules.md`(검증·결정 규칙)와 `business-logic-model.md`(코덱·상태전이·데이터 흐름)가 이 문서의 타입 정의를 참조한다. 여기서는 **각 타입의 목적·필드 스키마·불변식·정규화/검증 규칙·동등성/정렬 의미**를 정의하고, 규칙의 상세 매핑 표와 흐름/시퀀스는 자매 산출물로 교차 참조한다.
>
> **표기 규약**: 비즈니스 의미 중심의 **기술중립 설계**다. Rust스러운 타입 시그니처는 **참고용(reference-only)** 이며 인프라·스레딩·I/O 메커니즘이 아니라 개념적 형상을 표현한다. 다이어그램은 ASCII 박스 없이 **화살표 표기(A -> B)** 와 표/목록으로 기술하고, 복잡 요소에는 텍스트 설명을 첨부한다. English 식별자명은 원문 그대로 유지한다.

---

## 1. CoreTypes 값/엔티티 카탈로그

CoreTypes는 실행 로직이 거의 없는 **선언적 값 타입 모듈**이다(코덱과 소수의 순수 헬퍼만 로직 보유). 아래 각 타입에 대해 목적·필드 스키마·불변식·정규화/검증 규칙·동등성/정렬 의미를 정의한다.

### 1.1 프리미티브(Primitives)

#### RelativePath

- **목적**: 볼트 루트 기준의 **정규화된 POSIX 상대경로**. 매니페스트 엔트리 식별자이자 `ChangeSet.deleted`의 원소이며, 서버 커밋 페이로드의 경로→해시 맵 키다. 경로가 정규화되지 않으면 매니페스트 정렬·diff·`manifest_digest`의 결정성이 깨지므로 U0에서 정규화를 강제한다.
- **필드 스키마**

  | 필드 | 개념 타입 | 의미 |
  |---|---|---|
  | (내부값) | 문자열(정규화된 POSIX 상대경로) | 볼트 루트로부터의 상대 위치. 예: `notes/daily/2026-09-08.md` |

  참고 형상: `struct RelativePath(String)`

- **정규화 규칙(정규화는 값 구성 시점에 1회 적용, 멱등)** — 상세 알고리즘·오류 문구는 `business-rules.md` 참조:
  1. **구분자**: 경로 구분자는 **항상 `/`(POSIX 슬래시)**. Windows 백슬래시 `\`는 구성 시 `/`로 치환한다. 저장·직렬화·해시 입력 모두 `/`만 사용한다.
  2. **상대성 강제**: **절대경로 금지**(선행 `/`, 드라이브 접두 `C:` 등), **볼트 루트 이탈 금지**(`..` 세그먼트 금지). 위반은 값 구성 실패(정규화 오류)로 거부한다 — 볼트 경계를 벗어난 경로는 도메인상 무효.
  3. **잉여 세그먼트 제거**: `.`(현재 디렉터리) 세그먼트, 중복 슬래시(`//`), 선행/후행 슬래시를 제거한다.
  4. **대소문자(case) 취급 결정**: **바이트 그대로 보존(case-sensitive, 변형 없음)**. macOS/Windows 파일시스템이 대소문자 비구분일 수 있으나, RelativePath는 파일시스템이 반환한 이름을 **있는 그대로** 담는다(소문자화·폴딩 없음). 근거: (a) 서버 dedup은 `raw_sha256` 콘텐츠 기준이므로 경로 대소문자에 의존하지 않고, (b) 임의 폴딩은 무손실 round-trip(NFR-13)을 깨뜨린다.
  5. **Unicode 정규화 입장 결정**: **정규화하지 않는다(NFC/NFD 변형 금지). 파일시스템이 제공한 코드포인트 시퀀스를 바이트 그대로 보존**한다. 근거: (a) 무손실 직렬화 round-trip(US-E7-06/NFR-13)이 최우선 불변식이며 어떤 Unicode 폴딩도 이를 위반할 수 있고, (b) 재스냅샷 모델(FQ-2=A)에서 경로는 폴더가 진실의 원천이므로 관측된 바이트를 그대로 반영해야 한다. (macOS의 NFD 화면표시 vs 저장형 차이는 여기서 정규화하지 않으며, 필요 시 상위 단위가 스캔 계층에서 다룰 관심사다.)
- **불변식**: 정규화된 값만 존재한다(구성에 성공한 RelativePath는 항상 POSIX 구분자 · 상대 · `..`/`.` 없음). 정규화는 멱등(`normalize(normalize(p)) == normalize(p)`).
- **동등성/정렬**: **바이트 단위(UTF-8) 사전식(lexicographic) 정렬 및 동등성**. 대소문자 무시 없음, 로캘 비의존. 이 정렬이 Manifest 정렬 키의 1차 요소이며 `manifest_digest` 결정성의 기반이다.

#### Sha256Digest

- **목적**: 파일 바이트에 대한 **표준 SHA-256 해시(= `sha256sum` 결과)**. FQ-1=A에 따라 okc-core 콘텐츠 주소 스킴을 재현하지 않는 표준 해시이며, 서버의 콘텐츠 dedup 정렬과 값이 일치한다. 계산 로직(스트리밍 해시)은 U1 `ContentAddressing` 소유이고, U0는 **값 타입만** 정의한다.
- **필드 스키마**

  | 필드 | 개념 타입 | 의미 |
  |---|---|---|
  | (내부값) | 고정 32바이트 옥텟 배열 | 파일 바이트의 SHA-256 다이제스트(256비트) |

  참고 형상: `struct Sha256Digest([u8; 32])`

- **불변식**: 정확히 32바이트. 동일 바이트열 -> 동일 다이제스트(결정성; 실제 계산은 U1). 표준 sha256sum과 값 일치.
- **동등성/정렬**: 32바이트 내용에 대한 바이트 단위 동등성. 정렬이 필요하면 옥텟 사전식(주로 want 집합 등에서 안정적 순서 확보용); 매니페스트 정렬 키의 tie-break 보조로만 쓰인다(§1.3 ManifestEntry 정렬 참조).
- **텍스트 표현(선택)**: 로그·CLI 표면화 시 소문자 16진(hex) 문자열이 관례이나, 이는 표현 관심사이며 저장/해시 입력은 raw 32바이트다.

#### ManifestDigest

- **목적**: **하나의 Manifest 전체에 대한 로컬 다이제스트**. 용도는 **로컬 멱등/no-op 판정 discriminator** 다 — "현재 매니페스트가 마지막 커밋 매니페스트와 논리적으로 동일한가?"를 O(1) 비교로 판정(FR-10 no-op 조기 종료, US-E2-06). **권위 있는 `vault_content_id`가 아니다**(FQ-1=A) — 그 값은 서버가 소유·계산한다. ManifestDigest는 서버로 보내는 커밋 페이로드에 참조로 포함될 수 있으나(경로→해시 맵과 함께) 서버의 콘텐츠 아이덴티티를 대체하지 않는다.
- **필드 스키마**

  | 필드 | 개념 타입 | 의미 |
  |---|---|---|
  | (내부값) | 고정 32바이트 옥텟 배열 | 정규화·정렬된 Manifest 엔트리 시퀀스에 대한 결정적 해시 |

  참고 형상: `struct ManifestDigest([u8; 32])`

- **무엇에 대해 계산되는가**: **정렬된 `(relative_path, raw_sha256, size)` 3-튜플 시퀀스**에 대한 결정적 해시. 계산 순서: (1) 엔트리를 canonical 정렬 키(§1.4 Manifest)로 정렬, (2) 각 엔트리를 결정적 인코딩으로 직렬화, (3) 시퀀스 전체를 SHA-256으로 해시. **계산 로직은 U1 `ContentAddressing::manifest_digest` 소유**이고, U0는 값 타입과 결정성 계약만 정의한다. 정확한 도메인 인코딩(길이 프레이밍 등)은 U1 Functional Design 이월.
- **결정성 요구(불변식)**: **동일한 논리적 매니페스트(같은 엔트리 집합)는 엔트리 삽입/발견 순서와 무관하게 항상 동일한 ManifestDigest를 산출한다.** 이는 canonical 정렬(§1.4)에 의존한다. 정렬이 없으면 동일 볼트가 스캔 순서에 따라 다른 다이제스트를 내어 no-op 판정이 오작동한다.
- **동등성/정렬**: 32바이트 내용 동등성. 정렬 의미 없음(비교/판정용).
- **Sha256Digest와의 구분(중요)**: 둘 다 32바이트지만 **의미가 다르다** — `Sha256Digest`는 **한 파일의 콘텐츠** 해시(dedup 단위), `ManifestDigest`는 **매니페스트 전체**에 대한 no-op discriminator. 타입 시스템상 구별되는 별개 타입으로 유지해 혼용을 컴파일타임에 차단한다.

#### Timestamp

- **목적**: 도메인 이벤트의 시각 표시(볼트 스냅샷 캡처 시각, 동의 부여 시각, 업로드 히스토리 레코드 시각 등).
- **필드 스키마**

  | 필드 | 개념 타입 | 의미 |
  |---|---|---|
  | (내부값) | UTC 기준 시점(instant) | 벽시계(wall-clock) UTC 순간. 직렬화 시 결정적 표현(예: UTC epoch 나노초 또는 RFC3339) |

  참고 형상: `struct Timestamp(/* UTC instant */)`

- **불변식·결정성**: **직렬화 표현은 결정적·무손실**이어야 한다(round-trip 시 동일 시점 복원, NFR-13). 저장·비교의 기준시는 **UTC**로 통일한다(로컬 타임존은 표현 관심사이며 저장하지 않는다). 단조 클록(monotonic)은 지속 상태가 아니라 스케줄링 관심사이므로 여기서 다루지 않는다(상위 단위 소관).
- **동등성/정렬**: 시점의 자연 순서(과거 -> 미래). 히스토리 질의(`--since`)와 정렬에 사용.

#### ByteCount

- **목적**: 바이트 수를 나타내는 파운데이션 프리미티브(전송량·재개 오프셋·크기 표면화). component-methods.md §CoreTypes에 CoreTypes 타입으로 열거된다.
- **필드 스키마**

  | 필드 | 개념 타입 | 의미 |
  |---|---|---|
  | (내부값) | 비음 정수(`u64` 기반) | 바이트 수(64비트) |

  참고 형상: `struct ByteCount(u64)`(또는 `type ByteCount = u64`)

- **불변식·표현 일원화**: 비음. **바이트 수를 표현하는 모든 필드는 개념상 `ByteCount`다** — 본 문서 참고 형상에서 `u64`로 표기된 곳(`ManifestEntry.size`, `TransferResult`의 `bytes`/`resume_offset`, `StatusSnapshot.resume`의 (전송, 전체) 쌍 등)은 동일 의미의 바이트 수이며 `ByteCount`로 일원화한다. round-trip 무손실 대상(NFR-13).
- **동등성/정렬**: 자연수 순서.

> **`LogRecord` (참조 타입 — Logger 계약 도메인 레코드)**: `Logger.log(record)`의 `record`는 구조화 로그 레코드(`LogRecord`)다. 이는 관측 계약(§1.6 `Logger`)이 push-only로 받는 도메인 레코드이며, 필드 확정(레벨·이벤트·`cycle_id`·구조화 필드맵)은 **U6 `StructuredLogger` Functional Design 이월**이다. U0는 `Logger` 트레이트가 이 레코드를 받는다는 **계약**만 정의한다(값 스키마는 U6 소유).

### 1.2 매니페스트 값 타입

#### ManifestEntry

- **목적**: 볼트 내 **파일 1개**의 콘텐츠 지문 레코드. Manifest의 원소이며 ChangeSet의 added/modified 원소.
- **필드 스키마**

  | 필드 | 개념 타입 | 의미 | 비고 |
  |---|---|---|---|
  | `relative_path` | `RelativePath` | 볼트 루트 기준 정규화 경로 | 엔트리 식별자(파일 아이덴티티) |
  | `raw_sha256` | `Sha256Digest` | 파일 콘텐츠의 표준 SHA-256 | dedup·수정 판정 기준 |
  | `size` | 바이트 수(비음 정수, `u64`) | 파일 바이트 크기 | SafetyLimits 검사 및 diff 보조 판정 |

  참고 형상: `struct ManifestEntry { relative_path: RelativePath, raw_sha256: Sha256Digest, size: u64 }`

- **필드 결정 근거(mtime 미포함)**: 수정시각(mtime)은 **필드에 포함하지 않는다**. 근거: (a) 콘텐츠 아이덴티티는 `raw_sha256`가 결정하며 mtime은 콘텐츠와 무관하게 변동(touch)해 결정성·no-op 판정을 해칠 수 있고, (b) FQ-2=A 재스냅샷 모델에서 수정 판정 기준은 **`size` + `raw_sha256`** 로 충분하다(수정 판정 상세는 U1 `ManifestDiffer` Functional Design 이월). mtime 기반 최적화가 향후 필요하면 스캔 계층(U1)의 비영속 힌트로 다루며 매니페스트 지문에는 넣지 않는다.
- **불변식**: `relative_path`는 정규화되어 있음(RelativePath 불변식 상속). `raw_sha256`는 해당 파일 콘텐츠와 일치(생성자 계약; 실제 재검증은 U3 전송 시점). `size`는 비음.
- **정렬/동등성 키**: **canonical 정렬 키 = `relative_path` 오름차순**(바이트 사전식). 정상 매니페스트에서 경로는 유일하므로 경로만으로 전순서가 정해진다. 이론적 tie-break(동일 경로 방어)는 `(raw_sha256, size)` 순으로 확장하나 정상 입력에서는 발생하지 않는다. 동등성은 세 필드 전체의 값 동등성.

#### Manifest

- **목적**: 한 사이클에서 재계산된 **볼트 전체의 콘텐츠 스냅샷 지문** — 엔트리들의 순서 있는 집합 + 그에 대한 `manifest_digest`. FQ-1=A에 따라 권위 `vault_content_id`는 담지 않는다(서버 소유).
- **필드 스키마**

  | 필드 | 개념 타입 | 의미 |
  |---|---|---|
  | `entries` | `ManifestEntry`의 canonical 정렬 시퀀스 | 볼트 파일별 지문 목록(경로 오름차순) |
  | `manifest_digest` | `ManifestDigest` | 정렬된 엔트리 시퀀스에 대한 로컬 no-op discriminator |

  참고 형상: `struct Manifest { entries: Vec<ManifestEntry>, manifest_digest: ManifestDigest }`

- **canonical 정렬(결정성의 근거)**: `entries`는 **`relative_path` 오름차순(바이트 사전식)으로 정렬된 상태를 canonical 형상으로 한다.** `manifest_digest`는 이 canonical 정렬을 전제로 계산되므로, **논리적으로 동일한 매니페스트는 원 엔트리 삽입 순서와 무관하게 동일한 정렬·동일한 다이제스트**를 갖는다. 이것이 no-op 판정(FR-10)과 diff(NFR-12)의 결정성을 보장한다.
- **불변식**:
  - `entries`는 canonical 정렬되어 있고, `relative_path`는 집합 내 유일(중복 경로 없음).
  - `manifest_digest == digest(canonical(entries))` (다이제스트는 엔트리와 일관 — 파생값 불변식). 계산은 U1 소유, U0는 계약 정의.
  - 0-엔트리 매니페스트(빈 볼트)는 유효한 값이다(정규 표현). 단, 빈 매니페스트를 "전부 삭제"로 해석해 파괴적 커밋을 내는 판정은 **U0의 관심사가 아니다** — U2 `VaultAvailabilityGuard` 소관.
- **동등성**: `manifest_digest` 일치 = 논리적 동일(no-op 판정의 O(1) 근거). 완전 동등은 `entries` 시퀀스 전체 일치.

### 1.3 변경 집합(Change Set)

#### ChangeSet

- **목적**: **마지막 커밋 매니페스트(prior) 대비 현재 매니페스트(current)의 차이(diff)**. FQ-2=A 최신 상태 대체 모델에서 매 트리거마다 폴더를 **재스냅샷 후 diff 한 산출물**이다 — **이벤트 큐/변경 로그가 아니다**(누적되지 않으며, 다음 사이클은 다시 재스냅샷해 새 ChangeSet을 만든다). 계산 로직은 U1 `ManifestDiffer::diff(last_committed, current)` 소유.
- **필드 스키마**

  | 필드 | 개념 타입 | 의미 |
  |---|---|---|
  | `added` | `ManifestEntry` 시퀀스 | current에만 존재하는 파일(신규) |
  | `modified` | `ManifestEntry` 시퀀스 | 양쪽에 존재하나 콘텐츠(`raw_sha256`)/`size`가 달라진 파일(current 값) |
  | `deleted` | `RelativePath` 시퀀스 | prior에만 존재하고 current에 없는 파일(삭제) |

  참고 형상: `struct ChangeSet { added: Vec<ManifestEntry>, modified: Vec<ManifestEntry>, deleted: Vec<RelativePath> }`

- **`is_empty()` 의미**: `added`·`modified`·`deleted`가 **모두 비었을 때** true. 이는 `current`와 `last_committed`가 논리적으로 동일(= `manifest_digest` 일치)함과 동치이며, **no-op 사이클 판정**(NFR-01, 업로드 미생성)의 근거다. `is_empty()` 규칙 상세와 `manifest_digest` 동치성은 `business-rules.md` 참조.
- **불변식**:
  - 정확성(NFR-12): 누락 0·오탐 0 — 세 목록의 합집합은 prior와 current의 정확한 차집합/대칭차를 반영한다(정확 규칙은 `business-rules.md`, 계산은 U1).
  - `deleted`는 경로만 담는다(삭제는 콘텐츠 지문이 없으므로). `added`/`modified`는 current 상태의 엔트리를 담는다.
  - 재스냅샷 산출물이므로 **상태가 없다(stateless diff)** — ChangeSet 자체는 지속되지 않는다(지속 상태는 U4의 마지막 커밋 매니페스트 + dirty 플래그).
- **정렬/동등성**: 세 목록은 각각 `relative_path` 오름차순 정렬을 권장 형상으로 한다(결정적 표면화·테스트 재현성). 동등성은 세 목록 내용 일치.

### 1.4 전송 결과 · 오류 taxonomy (Q4=A: 파운데이션 소유)

> U0는 **타입/변이(variant)만** 정의한다. `is_retryable()`의 전체 매핑 표와 상태코드→클래스 규칙은 `business-rules.md`가 소유하며, U4 `RetryBackoffController`와 U5 `AuthTransport`가 이 타입을 소비/산출한다.
>
> **명명 정정(collision 해소)**: 전송 계층 오류 enum은 component-methods.md §CoreTypes의 `ErrorClass` 표기를 그대로 쓰지 않고 **`TransportErrorClass`로 명명**한다 — component-methods에는 상위 재시도용 `ErrorClass`(Retryable/AuthAborted/Backpressure/Fatal)와 전송용 `ErrorClass`(AuthFailed/ServerError/Backpressure/Timeout/Network)가 **같은 이름으로 중복 정의**되어 있어 이를 두 개의 별개 타입(`ErrorClass` = 재시도 분류, `TransportErrorClass` = 전송 분류)으로 분리한다. 하위 소비자(U4 `RetryBackoffController` / U5 `AuthTransport`)는 이 정정된 이름을 채택한다.

#### ErrorClass

- **목적**: 재시도 게이트(U4)가 소비하는 **의미 분류**. 파운데이션에 두어 U4가 U3/U5를 역참조 없이 분류 판단할 수 있게 한다(Q4=A).
- **변이(variants)**

  | 변이 | 의미(요약) |
  |---|---|
  | `Retryable` | 일시 오류 — 백오프 후 재시도 대상 |
  | `AuthAborted` | 인증 실패(예: 401) — 재시도 제외, U5 인증 흐름 위임 |
  | `Backpressure` | 서버 혼잡(PROJECT_BUSY/queue-full) — 오류 아닌 정상 지연으로 백오프 |
  | `Fatal` | 영구/치명 — 재시도 무의미(예: 코덱 실패, 프로토콜 위반) |

  참고 형상: `enum ErrorClass { Retryable, AuthAborted, Backpressure, Fatal }`

- **`is_retryable()` 의도**: `ErrorClass`가 재시도 가능한지 여부를 판정하는 순수 헬퍼. 개략 의도는 `Retryable`·`Backpressure`는 재시도 경로(단 Backpressure는 지연), `AuthAborted`·`Fatal`은 비재시도. **전체 매핑 표는 `business-rules.md` 소유** — 여기서는 헬퍼의 존재와 분류 의도만 정의.

#### TransportError

- **목적**: `AuthTransport`(U5)가 산출하는 **전송 계층 오류**의 세부 taxonomy. HTTP 상태·전송 실패를 도메인 클래스로 표현한다. (component-methods 참조 타입과 정합)
- **필드/변이**

  | 필드 | 개념 타입 | 의미 |
  |---|---|---|
  | `class` | 아래 전송 오류 클래스 enum | 전송 실패의 의미 분류 |
  | `http_status` | 선택적 정수(`Option<u16>`) | 존재 시 HTTP 상태코드 |
  | `detail` | 자유형 문자열 | 진단용 상세(유니코드 포함 가능) |

  전송 오류 클래스(variants): `AuthFailed` · `ServerError` · `Backpressure` · `Timeout` · `Network`

  참고 형상:
  ```
  enum TransportErrorClass { AuthFailed, ServerError, Backpressure, Timeout, Network }
  struct TransportError { class: TransportErrorClass, http_status: Option<u16>, detail: String }
  ```

- **변이 의미(요약; 상태코드→클래스 전체 표는 `business-rules.md`)**: `AuthFailed`(401) · `ServerError`(5xx) · `Backpressure`(PROJECT_BUSY/queue-full, 429 등) · `Timeout`(요청 타임아웃) · `Network`(연결 실패).
- **불변식**: `detail`은 자유형이며 **무손실 round-trip 대상**(유니코드·개행 포함 가능, NFR-13). `http_status`는 있을 수도 없을 수도 있다(Network/Timeout는 상태 없음).

#### ClassifiedError

- **목적**: 전송 무관하게 **분류된 오류의 일반 형상**. `ErrorClass` + 코드 + 상세를 담아 상위 계층(U4 재시도, U6 히스토리)이 공통 소비한다.
- **필드 스키마**

  | 필드 | 개념 타입 | 의미 |
  |---|---|---|
  | `class` | `ErrorClass` | 재시도 판정용 의미 분류 |
  | `code` | 선택적 문자열(`Option<String>`) | 도메인/프로토콜 코드(있으면) |
  | `detail` | 자유형 문자열 | 진단용 상세(유니코드 포함) |

  참고 형상: `struct ClassifiedError { class: ErrorClass, code: Option<String>, detail: String }`

- **불변식**: `detail`·`code`는 무손실 round-trip 대상(NFR-13). `class`는 `ErrorClass` 변이 중 하나.

#### TransferResult

- **목적**: 한 blob/사이클 전송의 **결과 요약**. 성공·부분(재개 가능)·실패를 표현.
- **변이**

  | 변이 | 필드 | 의미 |
  |---|---|---|
  | `Success` | `bytes: u64` | 전송 완료, 전송 바이트 수 |
  | `Partial` | `bytes: u64`, `resume_offset: u64` | 부분 전송 — 마지막 ack 오프셋에서 재개 가능 |
  | `Failed` | `error: ClassifiedError` | 실패 — 분류된 오류 동반 |

  참고 형상:
  ```
  enum TransferResult {
      Success { bytes: u64 },
      Partial { bytes: u64, resume_offset: u64 },
      Failed  { error: ClassifiedError },
  }
  ```

- **불변식**: `Partial.resume_offset <= Partial.bytes`가 개념적으로 성립(ack된 오프셋은 전송량 이내). 재개 오프셋의 지속은 U4 `SyncStateStore` 소유이며 U0는 결과 형상만 정의.

### 1.5 동기화 상태 머신 타입 (Q8=A)

#### SyncState

- **목적**: 최신 상태 대체 모델의 **동기화 사이클 상태**를 나타내는 enum. **전이 모델(다이어그램)은 `business-logic-model.md`, 전이 규칙(가드·실패 처리)은 `business-rules.md`에 정의**하고, U0는 **타입과 dirty-플래그 개념만** 정의한다. **지속/복구(persist/recover)는 U4 `SyncStateStore` 소유이며, U4가 이 모델로 stateful PBT(PBT-06)를 실행**한다(모델 정의는 여기 U0, 실행은 U4).
- **변이**

  | 변이 | 의미 |
  |---|---|
  | `Idle` | 대기 — 미처리 변경 없음(마지막 커밋 = 현재 볼트 상태) |
  | `Dirty` | 미커밋 변경 존재 — 다음 사이클이 재스냅샷해야 함 |
  | `Uploading` | 사이클 진행 중(협상/전송/커밋 수행) |
  | `Committed` | 업로드 성공, 커밋 지속 직전/직후 |

  참고 형상: `enum SyncState { Idle, Dirty, Uploading, Committed }`

- **dirty-플래그 개념(Q8=A 핵심)**: 상태와 **별개로**, 사이클 진행 중(`Uploading`/`Committed`) 새 변경이 감지되면 **dirty 플래그를 세팅**해 "다음 사이클이 재스냅샷해야 함"을 표시한다. 이는 이벤트 큐가 아니라 **단일 boolean 신호**다(FQ-2=A: 폴더가 진실의 원천). 업로드 **실패**는 별도 `Failed` 상태를 만들지 않고 상태를 `Dirty`로 유지해 재시도/백오프(U4) 대기로 둔다.
- **전이 요약(모델 정의는 U0; 정본 전이 표 T1~T8은 `business-logic-model.md` §2.3 — 단일 출처)**: `Idle -> Dirty`(변경 감지, T1) · `Dirty -> Uploading`(사이클 시작, 진입 시 dirty **clear**, T2) · `Uploading -> Committed`(업로드 성공, T3) · `Committed -> Idle`(**커밋 지속 완료 & dirty 미설정 시에만**, T4). **사이클 진행 중 새 변경**은 상태를 바꾸지 않고 **dirty 플래그만 세팅**한다 — `Uploading -> Uploading`(유지, T5) / `Committed -> Committed`(유지, T6). `Committed -> Dirty`(**커밋 지속 완료 시점에 dirty가 설정돼 있으면** `Idle` 대신 재진입, T7). `Uploading -> Dirty`(업로드 **실패** 시 Dirty로 되돌려 재시도 대기, T8). **명시적 `Failed` 상태 없음.** (주의: 새 변경 감지는 `Uploading`/`Committed`에서 상태 전이를 일으키지 않는다 — dirty 세팅만 하며, 실제 `Dirty` 재진입은 T7의 커밋 완료 시점 또는 T8의 업로드 실패에서만 일어난다.)
- **불변식**: 상태 집합은 위 4개로 폐쇄. 지속 표현은 무손실 round-trip 대상(NFR-13). 전이의 합법성 검증은 U4가 이 타입 위에서 수행.

### 1.6 관측 싱크 계약 트레이트 + Q9 2축 상태 어휘 (components.md §0 수정1: 파운데이션 소유)

> **배치 근거**: 로그·상태 push의 **계약(트레이트)** 과 상태 값 타입을 U0에 두어, U1~U5가 관측을 위해 U6(구현)을 **역참조하지 않고** U0 트레이트만 의존하게 한다(빌드 순서 역전 해소). **U0가 인터페이스(계약)를 정의하고, U6가 구현하며, 조립 루트(U8)가 하위 단위에 하향 주입**한다. **U0는 U6를 역참조하지 않는다.** 이들은 모두 **push-only 다운스트림 주입 계약**이다(하위 단위가 값을 밀어넣기만 하며 U6에서 U0로 돌아오는 참조가 없다). 트레이트 메서드의 push-only 상호작용 모델은 `business-logic-model.md` 참조.

#### 싱크 트레이트(계약)

| 트레이트 | 목적 | 대표 메서드(개념) | U6 구현체 |
|---|---|---|---|
| `Logger` | 구조화 로그 파사드 | `log(record)` / `event(level, event, cycle_id, fields)` | `StructuredLogger` |
| `StatusSink` | 상태 push(뮤테이터) | `set_operational` / `raise_condition` / `clear_condition` / `record_sync_success` / `set_dirty` / `set_resume_progress` / `set_liveness` | `StatusService` |
| `HistorySink` | 업로드 히스토리 append | `append(record)` | `UploadHistoryStore` |
| `CriticalEventSink` | 중대 이벤트 보고 | `report_auth_failure` / `report_cycle_result` / `report_preflight_exceeded` / `report_update_rollback` | `CriticalErrorNotifier` |

- **push-only 계약 의미**: 위 트레이트는 하위 단위가 **주입받아 호출(push)만** 한다. U0는 트레이트 시그니처(계약)만 정의하고 구현은 U6가 제공한다. 최종 메서드 시그니처의 확정 형상은 `business-logic-model.md`(상호작용 모델)에서 다루며, 여기서는 계약의 소유·방향만 명시한다.
- **infallible 표면**: 관측 push는 best-effort·비치명적(관측 실패가 동기화 코어를 막지 않음)으로 계약한다(상세 규칙은 `business-rules.md`).

#### Q9 2축 상태 어휘 값 타입

2축 모델: **축1 = 운영 라이프사이클(단일 값)**, **축2 = 활성 조건집합(동시 성립 가능)**.

| 타입 | 종류 | 값/필드 | 의미 |
|---|---|---|---|
| `OperationalState` | enum(축1) | `Idle` · `Syncing` · `Offline` · `Paused` | 운영 라이프사이클 단일 상태 |
| `ActiveCondition` | enum(축2 원소) | `AuthFailed` · `ConsentBlocked` · `OverLimit` · `VaultUnavailable` · `UpdateRolledBack` | 동시 성립 가능한 활성 조건(집합 원소) |
| `LivenessSignal` | enum | `IdleReached` · `CredentialReadable` | startup liveness 신호(각 단위가 push) |
| `StatusSnapshot` | struct | `operational: OperationalState`, `conditions: Set<ActiveCondition>`, `last_success: Option<Timestamp>`, `dirty: bool`, `resume: Option<(ByteCount, ByteCount)>`, `consent: ConsentState`, `offline: bool` | CLI status 표면용 현재 2축 상태 + 부가 필드 스냅샷 |

참고 형상:
```
enum  OperationalState { Idle, Syncing, Offline, Paused }
enum  ActiveCondition  { AuthFailed, ConsentBlocked, OverLimit, VaultUnavailable, UpdateRolledBack }
enum  LivenessSignal   { IdleReached, CredentialReadable }
struct StatusSnapshot  { operational, conditions, last_success, dirty, resume, consent, offline }
```

- **`update_probe()` vs `health_check()` 의도(§0 노트3)**: 둘 다 `StatusSink` 구현(U6 `StatusService`)이 제공하는 **읽기 판정**이나 **의미가 분리**된다 — `health_check()`는 **운영 헬스**(운영상태 + 활성조건집합에서 healthy/unhealthy 도출, CLI 종료코드 계약 근거, US-E5-02), `update_probe()`는 **순수 liveness**(기동 + `IdleReached` + `CredentialReadable` 신호만 판정). 분리 이유: 일시적 `AuthFailed`/`OverLimit` 같은 활성 조건이 좋은 새 버전을 오판 롤백시키는 **롤백 루프를 방지**(U7a `AutoUpdater`는 `update_probe()`만 소비). U0는 이 두 판정의 **의도와 반환 형상 계약**만 정의하고, 판정 임계·조건→상태 결합 규칙은 U6 Functional Design 이월.

---

## 2. WatcherConfig 필드 스키마 + 기본값 표

`ConfigProvider`가 로드·검증 후 노출하는 **타입드 config**다. 소스는 **사람이 편집하는 단일 JSON 파일**(Q3=X). 아래 필드 표는 U0가 소유하는 **파운데이션 공통 스키마**이며, 각 하위 단위가 소비하는 세부 섹션(디바운스 T_debounce, T_recon, 백오프 스케줄, 청크 임계값 S, 로그 로테이션, 서비스/트레이 등)은 **해당 단위 Functional Design이 자기 섹션 스키마를 확정**한다(component-methods 이월 규약). 여기서는 **U0 수준에서 계약이 확정된 핵심 필드**를 정의한다.

**범례**: 필수? = 최초 로드 시 반드시 존재해야 하는가. 검증 실패 시 동작은 §2.2 참조.

| 필드 | 타입 | 필수? | 기본값 | 의미 | 검증 규칙 |
|---|---|---|---|---|---|
| `vault_path` | 문자열(절대 파일시스템 경로) | 예 | 없음 | 감시 대상 Obsidian 볼트 루트 절대경로 | 비어있지 않음. 존재 여부는 config 스키마 검증이 아니라 런타임 가용성(U2 `VaultAvailabilityGuard`) 관심사. 상대경로/빈 값 = 검증 실패 |
| `server_endpoint` | 문자열(URL) | 예 | 없음 | okc 서버 업로드 엔드포인트 base URL | 유효 URL 형식. **스킴은 `https`만 허용**(TLS 강제, NFR-06). `http`/무스킴 = 검증 실패 |
| `token` | 문자열(평문) | 아니오 | 없음(미설정) | API 토큰 **1차·기본 저장 위치**(평문, §13/Q7) | 존재 시 비어있지 않은 문자열. 부재 허용(env/secure-store 폴백 가능). 값 형식 검증 없음(권위 유효성은 서버 401이 결정) |
| `secure_store_enabled` | 불리언 | 아니오 | `false` | OS secure-store opt-in 토글(Q7) | 불리언. `true`라도 세션 불가 시 config/env로 안전 폴백(U5) |
| `log_level` | 열거 문자열 | 아니오 | `info` | 최소 로그 레벨 | `trace`/`debug`/`info`/`warn`/`error` 중 하나. 그 외 = 검증 실패 |
| `notify_consecutive_failures` | 정수 | 아니오 | `3` | N회 연속 실패 알림 임계(FR-19 케이스2) | 양의 정수(>= 1) |

> **다른 단위 소비 섹션(U0 표에는 형상만 예고, 스키마 확정은 해당 단위)**: `debounce_ms`(T_debounce, U2) · `reconciliation_interval_s`(T_recon, U2) · `exclude_patterns`(U1 VaultScanner) · `chunk_threshold_bytes`(S, U3) · `backoff`(초기지연/배수/상한/지터, U4) · `request_timeout_s`(NFR-04, U5) · `log_rotation`(U6) · `service`(자동시작/계정, U7a) · `tray_enabled`(U6, 선택) · `confirm_empty`(U2). 이들은 각 단위가 자기 섹션 스키마·기본값을 확정하며, 알 수 없는 키 판정(§2.1 Q4=B)은 **전체 통합 스키마** 기준으로 적용된다.

### 2.1 알 수 없는/여분 키 처리 (Q4=B: STRICT reject)

알 수 없는(스키마에 없는) 키 또는 여분 키가 있으면 **검증 실패(reject)** 로 처리한다 — lenient 무시가 아니다. 근거: 평문 JSON을 사람이 편집하므로 오타(예: `vault_paht`)를 조기에 잡는다(nginx식 엄격성). 판정 상세·오류 리포트 형식은 `business-rules.md`.

### 2.2 검증/로드 실패 시 동작 (Q3=A)

- **최초 기동 로드 실패**(파일 부재/파싱 오류/스키마 위반/알 수 없는 키): **비정상 종료(non-zero exit)** — degraded/paused 상태 진입 없음.
- **실행 중 `reload()` 실패**: **마지막 정상 config 유지(keep-last-good) + 오류 로그 + 계속 실행**(nginx식 fail-safe).
- 상세 규칙·원자성은 `business-rules.md`(검증 규칙) 및 `business-logic-model.md`(load/reload 흐름) 참조.

### 2.3 SafetyLimits는 config 필드가 아님 (Q2=A)

`SafetyLimits`(총 <=20 GiB / 파일당 <=2 GiB / 파일 수 <=100k)는 **config에 노출되지 않는 컴파일타임 고정 상수**다 — okc-core 캡과 일치하며 서버가 권위 있게 재검증한다(DEP-04). WatcherConfig에 대응 필드가 **없음**을 명시한다. 상수 값·경계 규칙은 `business-rules.md`, 소비 컴포넌트는 U1 `SafetyLimitsValidator`.

### 2.4 발견·토큰 소스는 로드타임 관심사 (Q6=A / Q7=A)

- **config 파일 발견(Q6=A)**: `WatcherConfig`의 필드가 아니라 **로드타임 경로 해소** 관심사다. 우선순위: `--config` 플래그 > `OKC_WATCHER_CONFIG` 환경변수 > 플랫폼 기본 경로. 이 값들은 `ConfigProvider.load(path)`의 입력이며 config 내용에 담기지 않는다(발견 흐름은 `business-logic-model.md`).
- **토큰 소스 우선순위(Q7=A)**: 위 표의 `token`(config 평문)은 **1차·기본 소스**이나, 최종 토큰 해소 우선순위는 **secure-store(opt-in 활성 & 세션 가용 시) > config `token` > env 폴백**이다. 이 해소 로직은 U5 `CredentialProvider` 소유이며 config는 그중 한 소스를 제공할 뿐이다(정확한 우선순위 규칙은 `business-rules.md`).

---

## 3. 인코딩 노트 (Q1=B)

두 가지 별개의 직렬화 경로가 있으며 **혼동 금지**:

| 데이터 | 포맷 | 소유/방향 |
|---|---|---|
| **사람이 편집하는 config 파일** | **JSON**(고정, nginx식) | 사람 -> `ConfigProvider.load` -> 타입드 `WatcherConfig` |
| **데몬이 내부적으로 저장하는 지속 상태** | **CBOR**(serde 호환 바이너리) | `CoreTypes` `encode`/`decode` 코덱 |

- **CBOR 코덱을 통과하는 타입(내부 지속 상태)**: 마지막 커밋 `Manifest`, `SyncState`(지속은 U4 `SyncStateStore`), 동의 부여 참조(`ConsentGrant`, U5), 업로드 히스토리 레코드(`UploadHistoryRecord`, U6), 재개 오프셋 등. 이들 값 타입은 U0에 정의되고 코덱도 U0가 소유하며, 실제 저장 파일 I/O는 소비 단위(U4/U5/U6)가 수행한다.
- **불변식(NFR-13/US-E7-06)**: CBOR 코덱을 통과하는 모든 값 `v`에 대해 `decode(encode(v)) == v`(무손실 round-trip). 자유형 오류 문자열·유니코드·`Option` 부재 등 모두 보존.
- **CBOR 선택 근거(Q1=B)**: 컴팩트·자기기술적이라 자동 업데이트 후 상태파일 스키마 진화에 강함. **config 파일은 CBOR가 아니다** — 사람 가독성 위해 JSON 유지.
- 코덱 흐름·오류(`CodecError`, Fatal 계열) 상세는 `business-logic-model.md`.

---

## 4. 엔티티 관계 개요 (화살표 표기 — ASCII 박스 미사용)

아래는 타입 간 합성(composition)/참조 관계를 화살표 표기와 표로 기술한 것이다(텍스트 다이어그램).

**합성(포함) 관계 — "A -> B" 는 A가 B를 필드로 포함/참조함을 뜻한다:**

- `Manifest` -> `ManifestEntry`(다수, canonical 정렬 시퀀스) + `ManifestDigest`(1)
- `ManifestEntry` -> `RelativePath`(1) + `Sha256Digest`(1) + `size:u64`(1)
- `ManifestDigest` -- (계산 대상) --> `Manifest`의 정렬된 엔트리 시퀀스 (파생값; 로컬 no-op discriminator, `vault_content_id` 아님)
- `ChangeSet` -> `added: [ManifestEntry]` + `modified: [ManifestEntry]` + `deleted: [RelativePath]`
- `ChangeSet` -- (diff 산출) --> `diff(prior: Manifest, current: Manifest)` (재스냅샷 diff, 이벤트 큐 아님)
- `ClassifiedError` -> `ErrorClass`(1) + `code: Option<String>` + `detail: String`
- `TransportError` -> `TransportErrorClass`(1) + `http_status: Option<u16>` + `detail: String`
- `TransferResult::Failed` -> `ClassifiedError`(1)
- `StatusSnapshot` -> `OperationalState`(1) + `Set<ActiveCondition>` + `last_success: Option<Timestamp>` + `ConsentState`(U5 정의) + `resume/dirty/offline`

**소유/구현/주입 방향 (빌드 순서 역전 해소):**

- `CoreTypes`(U0) == 값 타입 + 오류 taxonomy + `SyncState` + 싱크 트레이트(계약) + 2축 상태 어휘 **정의**
- 하위 단위(U1~U5) -- depends-on --> `CoreTypes` 트레이트(계약)만 (U6 역참조 없음)
- U6 구현체(`StructuredLogger`/`StatusService`/`UploadHistoryStore`/`CriticalErrorNotifier`) -- implements --> `CoreTypes` 싱크 트레이트
- 조립 루트(U8) -- injects(하향 주입) --> U6 구현체를 하위 단위에 `CoreTypes` 계약 타입으로 주입 (push-only 성립)

**포맷 경로 구분:**

- 사람 -> JSON config 파일 -> `ConfigProvider.load` -> `WatcherConfig`(타입드)
- 내부 상태 값 타입(`Manifest`/`SyncState`/`ConsentGrant`/`UploadHistoryRecord` 등) -> `CoreTypes.encode` -> CBOR 바이트 -> (U4/U5/U6 파일 I/O) -> `CoreTypes.decode` -> 원 값 (round-trip 무손실)

**텍스트 설명(다이어그램 대안)**: 최상위 도메인 엔티티는 `Manifest`이며, 이는 다수의 `ManifestEntry`로 구성되고 각 엔트리는 프리미티브(`RelativePath`+`Sha256Digest`+size)로 분해된다. `ManifestDigest`는 정렬된 매니페스트에서 파생되는 로컬 판정값이다. `ChangeSet`은 두 `Manifest`(prior/current)의 diff 결과로, 엔트리와 삭제 경로를 담되 상태를 지속하지 않는다. 오류 계열(`ErrorClass`/`TransportError`/`ClassifiedError`/`TransferResult`)은 전송 결과를 분류해 재시도(U4)와 히스토리(U6)에 공급한다. 싱크 트레이트와 2축 상태 어휘는 U0가 계약으로 정의하고 U6가 구현하며 U8이 하향 주입해 push-only 관측을 성립시킨다.

---

## 5. Testable Properties (PBT-01) — 엔티티/타입 계층

> **확장 강제(PBT-01, Full)**: 이 산출물은 엔티티/타입 계층의 속성을 식별한다. 각 속성에 카테고리 라벨 {Round-trip, Invariant, Idempotence, Commutativity, Oracle, Induction, Easy verification}과 도메인 제너레이터(PBT-07) 요구를 기재한다. 프레임워크 선택(PBT-09)은 NFR Requirements 이월(Rust=proptest 유력). 검증 규칙 계층의 속성(`is_retryable` 매핑, config 검증, `is_empty` no-op)은 `business-rules.md`의 PBT 섹션에서, 코덱·상태전이 흐름 속성은 `business-logic-model.md`의 PBT 섹션에서 각각 다룬다(중복 회피).

### 5.1 코덱 무손실 round-trip (핵심)

- **PROP-DE-01 — `decode(encode(v)) == v`** (카테고리: **Round-trip**; PBT-02; US-E7-06/NFR-13)
  - **대상**: CBOR 코덱을 통과하는 모든 CoreTypes 값 — `Manifest`, `ManifestEntry`, `RelativePath`, `Sha256Digest`, `ManifestDigest`, `Timestamp`, `ChangeSet`, `SyncState`, `ErrorClass`, `TransportError`, `ClassifiedError`, `TransferResult`, 그리고 하위 단위 지속 레코드(`ConsentGrant`[U5]/`UploadHistoryRecord`[U6]는 각 단위에서 자기 제너레이터로 재확인).
  - **속성**: 임의 값 `v`에 대해 `decode(encode(v)) == v`. 자유형 오류 문자열·유니코드·`Option` 부재·빈 컬렉션 포함.
  - **제너레이터(PBT-07)**: 각 값 타입에 대한 도메인 제너레이터 필요 — 특히 `RelativePath`(정규화 규칙 만족하는 경로), `detail` 문자열(유니코드·개행·빈 문자열 경계), `size`/오프셋(0·경계·대값), 빈/단일/다수 엔트리 매니페스트.
  - **US-E7-06 수용기준 정합**: 매니페스트·(지속) 상태·히스토리 레코드의 `deserialize(serialize(x)) == x`. (원문 "queue entries"는 **FQ-2=A 최신상태 모델을 FQ-3=A 요구사항 정정으로 반영**해 sync-state[마지막 커밋 매니페스트 + dirty + 재개 오프셋]로 대체 — requirements.md §12.2.)

### 5.2 ManifestDigest 결정성

- **PROP-DE-02 — 순서 무관 다이제스트 결정성** (카테고리: **Invariant** + **Oracle**)
  - **속성(Invariant)**: 동일한 논리적 엔트리 집합은 **삽입/발견 순서와 무관하게** 동일한 `manifest_digest`를 산출한다 — `digest(shuffle(entries)) == digest(entries)`(canonical 정렬 후 계산이므로).
  - **속성(Oracle)**: 참조 오라클 = canonical 정렬 후 표준 재계산. 실제 계산이 오라클과 일치.
  - **제너레이터(PBT-07)**: 임의 엔트리 집합 + 그 순열(permutation). 경로 유일성 보장 제너레이터 + 경계(빈 집합, 단일, 대량 100k 근접 축소본).
  - **비고**: `manifest_digest` **계산 로직은 U1 `ContentAddressing` 소유**이므로 이 속성의 실행 위치는 U1이며, U0는 결정성 **계약**을 정의한다.

### 5.3 RelativePath 정규화 멱등성

- **PROP-DE-03 — 정규화 멱등** (카테고리: **Idempotence**)
  - **속성**: `normalize(normalize(p)) == normalize(p)`. 또한 정규화 결과는 항상 POSIX 구분자·상대·`..`/`.` 없음(Invariant 보조).
  - **제너레이터(PBT-07)**: 혼합 구분자(`\`/`/`), 중복 슬래시, `.` 세그먼트, 선행/후행 슬래시, 유니코드 경로, (거부 대상) `..`·절대경로를 포함하는 raw 경로 문자열 제너레이터.
  - **비고**: `..`/절대경로 입력은 정규화 실패(거부)를 반환하는 것이 올바른 동작이며, 이는 `business-rules.md`의 경로 검증 규칙과 교차한다.

### 5.4 SyncState 전이 (U4 stateful PBT 대상 — 모델은 U0, 실행은 U4)

- **PROP-DE-04 — SyncState 전이 모델** (카테고리: **Induction** / 상태 기반; PBT-06)
  - **상태**: `SyncState` 전이 모델(§1.5, Q8=A)은 **U4 stateful PBT(PBT-06)의 참조 모델**이다. **모델(타입·전이·dirty-플래그)은 여기 U0에서 정의**하고, **명령 시퀀스 속성(`commit_manifest`/`mark_dirty`/`set_resume_offset`/persist/recover vs 참조 모델)의 실행은 U4 `SyncStateStore`**가 수행한다(US-E7-08).
  - **U0에서의 위치**: U0는 타입과 전이 합법성 계약만 제공하므로 U0 계층에는 **stateful 실행 속성이 없다** — 여기서는 모델 정의 + U4 실행 위임을 명시한다.

### 5.5 속성 없음(No PBT properties identified) 판정

| 타입 | 판정 | 근거 |
|---|---|---|
| `Timestamp` | round-trip에 포함(PROP-DE-01) 외 **독립 속성 없음** | 값 래퍼 — 결정적 직렬화 외 도메인 규칙 없음(단조 클록은 지속 관심사 아님) |
| `ErrorClass` / `TransportError` / `ClassifiedError` / `TransferResult` | round-trip에 포함 외 U0 계층 **독립 속성 없음** | `is_retryable` 매핑 속성은 `business-rules.md` 소유(규칙 계층) |
| 싱크 트레이트(`Logger`/`StatusSink`/`HistorySink`/`CriticalEventSink`) | **No PBT properties identified** | 순수 계약(인터페이스) — 값이 아니라 push-only 동작. 상호작용 모델 속성은 `business-logic-model.md`, 구현 속성은 U6 |
| `OperationalState`/`ActiveCondition`/`LivenessSignal` | round-trip에 포함 외 **독립 속성 없음** | 유한 enum 값 어휘 — 판정 결합 규칙(조건->상태)은 U6 소유 |
| `StatusSnapshot` | round-trip 후보(직렬화 시) 외 **독립 속성 없음** | 집계 스냅샷 — 도출 규칙은 U6 |
| `WatcherConfig` | U0 엔티티 계층 **직접 속성 없음** | config 파싱 round-trip + 리로드 멱등성(PBT-04)은 `business-rules.md`/`business-logic-model.md` 소유 |

> **제너레이터(PBT-07) 총괄**: 위 모든 속성은 도메인 타입 제너레이터를 요구한다(특히 `RelativePath`·`Manifest`·자유형 `detail` 문자열). 제너레이터의 구체 구현·shrinking·고정 시드·CI 통합(PBT-08)은 Code Generation/Build-and-Test 이월이며, 여기서는 **요구 사실과 대상 타입**을 명시한다.
