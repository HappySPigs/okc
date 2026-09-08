# U4 sync-state — Tech Stack Decisions (기술 스택 결정)

**단계**: CONSTRUCTION -> Per-Unit Loop -> **U4 Resilience & Retry** -> NFR Requirements -> 산출물 2/2 (`tech-stack-decisions.md`)
**작성일**: 2026-09-08
**크레이트**: `sync-state` (lib) · **소속 컴포넌트**: `SyncStateStore`, `RetryBackoffController`
**입력 아티팩트**: `domain-entities.md`·`business-rules.md`·`business-logic-model.md`(U4 FD) · `plans/u4-sync-state-functional-design-plan.md` §3 결정 D-01..D-14 · `requirements.md`(§5 NFR-03/04/11/13 · §7 RISK-01) · 활성 확장 `property-based-testing.md`(PBT-09)·`resiliency-baseline.md` · U0 `tech-stack-decisions.md`(전파 원칙·스타일·의존성 표 템플릿) · `crates/foundation/**`(FROZEN 코덱·타입·`proptest-support` feature 실물) · 워크스페이스 `Cargo.toml`(핀 실물)
**규칙**: `construction/nfr-requirements.md` Step 6 · `common/content-validation.md`(Unicode 박스 문자 미사용, 화살표 표기 `A -> B`, 표/코드블록 검증)

> **문서 성격**: 이 문서는 U4의 **구체 크레이트/기법 선택을 기록하는 유일한 산출물**이다. 자매 산출물 `nfr-requirements.md`가 카테고리별 NFR(기술중립)을 정의하고, 이 문서는 그 NFR을 실현할 **구체 기술 결정**과 각 결정의 **근거·MVP 트림·전파 범위**를 확정한다. English 식별자(크레이트명·타입명·config 키·rule/NFR ID)는 원문 그대로 유지하고 Rust 제네릭/타입은 백틱으로 감싼다(예: `std::fs::rename`, `BTreeMap<Sha256Digest, ByteCount>`).
>
> **WRITE SCOPE 주의**: 이 문서는 **설계 기록**이다. 실제 크레이트 Cargo.toml 배선·소스 작성은 U4 Code Generation 단계 소관이며 이 문서에서 수행하지 않는다. 워크스페이스/크레이트 매니페스트는 편집하지 않는다.

---

## 0. 전파·상속 원칙 (U4 = Wave-2, U0 상속 소비자)

U4 `sync-state`는 의존이 **U0 `foundation` 하나뿐**인 Wave-2 얇은 단위다. 따라서 U4는 U0 tech-stack이 확정한 워크스페이스 전역 결정(**Rust · Edition 2024 · MSRV 1.85 · `proptest` PBT 프레임워크 · 크레이트 major/minor 핀**)을 **그대로 상속**하며 새 워크스페이스 결정을 만들지 않는다. 외부 크레이트는 `[workspace.dependencies]`에서 상속(`serde.workspace = true` 등), 내부 크레이트는 `[workspace.package]` 상속 + `publish=false` + path 의존이다.

- **핵심 결과 — 신규 외부 크레이트 0건**: U4는 워크스페이스에 **새 외부 크레이트를 도입하지 않는다**. 지속 I/O는 `std::fs`, 백오프 타이밍은 `std::time`, full jitter 난수는 자체 소형 결정적 PRNG로 충당한다(§2~§4). 이는 MVP 지침(최소 크레이트, std 우선)과 정합한다.
- **전파 표기 규약**: 각 결정 절 말미에 전파 범위를 명시한다(대부분 `U4 내부 한정`).
- **버전 정책**: 정확한 patch 핀 + MSRV(1.85) 대비 CI 검증은 **Build-and-Test 이월**(U0와 동일 정책, 워크스페이스 `Cargo.toml` 주석과 정합).

---

## 1. 언어 / 툴체인 (상속 — 재결정 아님)

| 항목 | 결정 | 근거 |
|---|---|---|
| 언어 | **Rust** | NFR-17 확정(U0 상속). PBT-09 `proptest`의 전제. |
| Edition | **2024** (`edition.workspace = true`) | U0 §1 상속. `[workspace.package]` 상속. |
| MSRV | **1.85 고정** (`rust-version.workspace = true`) | U0 §1 상속. NFR-05 크로스플랫폼 재현 빌드. Edition 2024와 정합. |

**전파 범위**: 상속만 — U4는 워크스페이스 값을 물려받아 드리프트 없이 재사용한다.

---

## 2. crash-atomic 지속 I/O = `std::fs` temp+rename+fsync (D-01/D-03, MVP)

U4-NFR-REL-01(crash-atomic 원자 쓰기)의 실현 메커니즘이다. FD 결정 D-01은 **temp+rename**(WAL 아님)을, D-03은 **fsync 배리어 순서**를 확정했다.

**결정**: 표준 라이브러리 `std::fs`만으로 원자 쓰기 파이프라인을 구현한다.

```
persist(state):
  1) bytes = foundation::encode(&state)          // U0 코덱, ciborium 백엔드(§5)
  2) tmp = "<state_path>.tmp"  (최종 파일과 같은 디렉터리)
  3) File::create(tmp) -> write_all(bytes) -> f.sync_all()   // 임시 파일 내용 fsync
  4) std::fs::rename(tmp, state_path)            // 동일 파일시스템 atomic replace
  5) File::open(parent_dir) -> dir.sync_all()    // 부모 디렉터리 메타데이터 fsync(best-effort)
```

**채택 근거**:
- `std::fs::rename`은 동일 파일시스템 내에서 **atomic replace**를 제공한다 — Unix(`rename(2)`)와 Windows(`MoveFileExW` + `MOVEFILE_REPLACE_EXISTING`) 모두에서 기존 최종 파일을 원자적으로 대체하므로 부분 상태가 커밋되지 않는다(R-STATE-01 불변식 직접 충족).
- 임시 파일을 **최종 파일과 같은 디렉터리**에 두어 cross-device rename(비원자적 복사 fallback)을 회피한다.
- 상태가 소형(FQ-2=A: 매니페스트 1개 + boolean + 소형 맵)이라 매 쓰기 전체 재기록이 저렴하다 -> WAL/증분 로그 불필요(D-01 MVP 트림).

**`tempfile` 크레이트 미채택(MVP 트림)**: `tempfile`의 주 가치(시스템 temp 디렉터리 보안 임시 파일 + 자동 정리)는 U4 요구와 어긋난다 — 원자 rename을 위해 임시 파일이 **최종 파일과 같은 디렉터리**에 있어야 하고, 잔존 임시 파일 정리는 `open_and_recover`가 명시적으로 수행한다(R-STATE-04). `NamedTempFile::persist`도 부모 디렉터리 fsync는 제공하지 않는다. 따라서 `std::fs` 직접 사용이 더 작고 요구에 정확히 맞으며 외부 의존을 0으로 유지한다.

**플랫폼 주의(문서화)**: 부모 디렉터리 fsync는 Unix에서 rename 메타데이터 내구성을 보강하는 배리어다. Windows는 디렉터리 핸들 fsync 의미가 다르므로 **best-effort**로 처리하고, 내구성은 `MoveFileEx`의 replace-atomicity + 임시 파일 `sync_all`에 의존한다. 이 플랫폼 차이는 조건부 컴파일로 흡수하며 상세는 Code Generation 이월.

**전파 범위**: **U4 `SyncStateStore` 내부 한정**. `std`만 사용하므로 워크스페이스 의존 변화 없음.

---

## 3. 오류 처리 = `thiserror`(운영) + 순수 serde(값 타입) (U0 관례 상속)

U0-NFR-MNT-01의 오류 파생 관례를 U4가 그대로 따른다.

| 부류 | 타입 | 파생 전략 | 근거 |
|---|---|---|---|
| **운영 오류(반환용)** | `StateError` (`CorruptRecovered`/`Io`/`Serde`) | **`thiserror`** 파생(`Display`/`std::error::Error`) | `Result`로 반환되는 라이브러리 오류. U0 §5 관례 상속. `Serde` 변형은 U0 `CodecError`(Fatal)에 대응. |
| **분류 값 타입** | `RetryClass`·`RetryDecision` | **순수 enum/struct**(직렬화·throw 아님) | 지속·전송되지 않는 인메모리 판정 값. |
| **지속 값 타입(CBOR round-trip 대상)** | `PersistedState`·`ResumeOffsetMap` | **`serde::Serialize`/`Deserialize` 파생만** | U0 코덱 round-trip 대상(U4-NFR-REL-03). `StateConfig`·`BackoffConfig`는 U8이 파싱해 주입하므로 U4에서 config 파싱 파생 불필요. |

**`anyhow` 미채택**: U0 §5와 동일 근거 — `StateError::CorruptRecovered{recovered_to}`의 구조화 변이가 호출부(U8)의 재조정 판정에 필요하므로 타입소거는 부적합.

**전파 범위**: **U4 내부 한정**. `thiserror`·`serde`는 워크스페이스 상속 핀(신규 아님).

---

## 4. 백오프 타이밍 + full jitter 난수 = `std::time` + 자체 결정적 PRNG (D-09/D-13, MVP)

U4-NFR-REL-05(백오프)·U4-NFR-MNT-01(결정성)의 실현이다. FD 결정 D-09는 full jitter 단일 공식, D-13은 주입 시계 + 시드 결정성을 확정했다.

**결정**:
- **타이밍**: `std::time::{Duration, Instant}` 산술만 사용한다 — `base_delay = min(cap, initial * mult^attempt)`, `retry_after = jitter(0, base_delay)`. `now: Instant`는 `on_failure`에 **주입**한다(전역 시계 미사용).
- **full jitter 난수**: `[0, base_delay]` 균등 난수를 **자체 구현 소형 결정적 PRNG**(splitmix64 계열, ~5~10줄)로 생성한다. 시드는 컨트롤러 생성 시 주입되어 재현 가능하다.

**채택 근거**:
- full jitter의 난수 품질 요구는 자명하다 — 재시도 지연을 `[0, base]`에 대략 균등 분산해 thundering-herd를 완화하는 것이 전부이며 암호학적 품질이 불필요하다. splitmix64는 짧고 통계적으로 충분하다.
- **주입 시드 결정성**을 직접 실현한다(D-13/U4-NFR-MNT-01) — 같은 (상태, `now`, seed)이면 같은 `RetryDecision`이 나와 PROP-U4-05 상태머신 PBT의 정확 재현·shrinking이 가능하다.
- 외부 의존 0 — MVP 지침(std 우선) 정합. U0가 redaction newtype을 15~20줄로 자체 구현한 패턴과 동형.

**`rand` 크레이트 미채택(MVP 트림)**: `rand`는 잘 확립된 크레이트이나 (a) `rand`/`rand_core`/`getrandom` 등 의존 트리를 워크스페이스에 새로 도입하고, (b) 결정성을 위해 `StdRng::seed_from_u64`를 별도로 배선해야 하며 그 이점이 splitmix64 자체 구현 대비 미미하다. 정책 엔진 없는 단일 공식(D-09)에는 자체 PRNG가 더 작고 결정성 요구에 직접 맞는다.

**전파 범위**: **U4 `RetryBackoffController` 내부 한정**. std만 사용.

---

## 5. 직렬화 = U0 코덱 재사용(`foundation::encode`/`decode`) — `ciborium` 직접 의존 없음 (FROZEN 상속)

U4-NFR-REL-03(무손실 round-trip)의 실현이다.

**결정**: `PersistedState`의 직렬화는 **U0가 소유·구현한 무손실 코덱** `foundation::core_types::codec::{encode, decode}`를 **호출**한다. 이 함수는 이미 `pub`이며(FROZEN, `crates/foundation/src/core_types/codec.rs`) 내부 백엔드가 `ciborium`이다.

- **U4는 `ciborium`을 직접 의존하지 않는다** — 코덱 함수가 U0 표면 뒤에 캡슐화돼 있으므로 U4는 `foundation`만 의존하면 된다(U0 tech-stack §2.1이 예상한 "U4로 ciborium 전파"는 **U0 코덱 표면 재사용으로 충족**되며, 별도 ciborium 직접 핀이 불필요하다).
- U4가 소유하는 것은 **파일 I/O(§2)뿐**이며 바이트 변환은 U0가 담당(I/O 없는 순수 코덱).
- `PersistedState`/`ResumeOffsetMap`는 `serde::Serialize`/`Deserialize`를 파생해 U0 코덱의 `encode<T: Serialize>`/`decode<T: DeserializeOwned>` 시그니처에 맞춘다. `ResumeOffsetMap = BTreeMap<Sha256Digest, ByteCount>`의 결정적 정렬이 round-trip 안정성을 보장한다(PROP-U4-03).

**전파 범위**: 코덱은 **U0 소유**(FROZEN). U4는 `foundation` path 의존을 통해 소비. `serde`는 워크스페이스 상속 핀.

---

## 6. 속성 기반 테스트 = `proptest` via `proptest-support` feature (PBT-09 상속, PBT-07 미러링)

> **PBT-09(프레임워크) 상속**: 프레임워크 선택은 U0 tech-stack §7에서 워크스페이스 전역 `proptest`로 이미 확정된 blocking 의무다. U4는 이를 **상속**하며 재선택하지 않는다. 이 절은 U4 크레이트에서의 **feature 게이팅·제너레이터 재사용 방식**을 확정한다.

### 6.1 프레임워크 = `proptest` (상속)

워크스페이스 공용 dev-dependency `proptest`(핀 `1`)를 U4 PBT(PROP-U4-01~07)에 사용한다. custom generators·automatic shrinking·seed reproducibility·`cargo test` 통합 요구를 충족한다(U0 §7.1 근거 상속).

### 6.2 제너레이터 노출·재사용 = `proptest-support` 비기본 cargo feature (U0 패턴 미러링, Q10 상속)

**결정**: U4 크레이트에 **비기본(non-default) cargo feature `proptest-support`** 를 두어 `proptest`를 **optional dependency**로 게이트하고(U0 `foundation/Cargo.toml`의 `proptest-support = ["dep:proptest"]` 패턴을 동일하게 미러링), 그 뒤에 U4 자기 제너레이터를 노출한다.

- **U0 제너레이터 재사용**: U4는 `foundation`을 **dev-dependency로 `features = ["proptest-support"]`** 와 함께 두어 U0 도메인 제너레이터(`Manifest`·`Sha256Digest`·`ByteCount`)를 재사용한다 — 재정의 없음(U4-NFR-MNT-02, 드리프트 방지).
- **U4 자기 제너레이터**(PBT-07 요구, `business-logic-model.md` §5 총괄): `PersistedState` 제너레이터, 명령 시퀀스 제너레이터(`mark_dirty`/`commit_manifest`/`persist_resume_offset`/`clear_resume_offsets` 임의 교차), 크래시 주입점 제너레이터(직렬화/temp/fsync/rename 전후 + 손상 바이트), `TransportErrorClass` 시퀀스 + 주입 시계(단조 `Instant`)/시드 제너레이터.
- **feature 게이트 효과**: non-default이므로 `proptest`가 **프로덕션 빌드 그래프에 유입되지 않는다**(U0 MNT-02/PBT-07 정합).

**참고 Cargo 형상(Code Generation 배선 대상, 이 문서는 배선하지 않음)**:

```
[dependencies]
foundation = { path = "../foundation" }
serde = { workspace = true, features = ["derive"] }
thiserror.workspace = true
proptest = { workspace = true, optional = true }   # proptest-support 에서만 유입

[dev-dependencies]
proptest.workspace = true
foundation = { path = "../foundation", features = ["proptest-support"] }  # U0 제너레이터 재사용

[features]
default = []
proptest-support = ["dep:proptest"]
```

### 6.3 이월(PBT-08)

케이스 수·shrink 튜닝·고정 시드 vs 시드 로깅·`proptest-regressions` 정책·CI 통합, 그리고 **크래시 주입 하니스(프로세스 kill·전원 손실 모사)의 구체 구현**은 **Code Generation / Build-and-Test 및 NFR Design 이월**이다(property-based-testing.md Enforcement Integration; RESILIENCY-14). 이 문서는 프레임워크 상속(PBT-09) + 제너레이터 노출 방식(PBT-07)만 확정한다.

**전파 범위**: **U4 내부**(dev-dependency + 자기 제너레이터). U4 제너레이터를 소비하는 하위 단위는 없다(U4는 DAG 상 소비자 말단).

---

## 7. AUTOPILOT 결정 (topic / chosen / MVP-trim? / rationale)

이 단계에서 저자가 확정한 기술 결정(질문 없이 권장안 + MVP 편향). FD 결정 D-01..D-14는 재확정 대상이 아니며, 여기서는 그 실현 기술을 확정한다.

| 주제 | 채택안 | MVP 트림? | 근거 |
|---|---|---|---|
| crash-atomic 지속 I/O | `std::fs` 임시 파일 + `std::fs::rename`(atomic replace) + `sync_all` fsync 배리어 | **예** | WAL/증분 로그 미채택(D-01). 소형 상태 전체 재기록이 저렴, rename 원자성이 NFR-03 충족. |
| `tempfile` 크레이트 | **미채택**(std 직접) | **예** | 임시 파일이 최종 파일과 같은 디렉터리에 있어야 원자 rename 가능 — tempfile의 시스템-temp 모델과 어긋남, 부모 dir fsync 미제공. std가 더 작고 정확. |
| 직렬화 코덱 | U0 `foundation::encode`/`decode` 재사용(ciborium 백엔드), **U4 ciborium 직접 의존 없음** | **예** | 코덱이 U0 FROZEN 표면 뒤 캡슐화 — 별도 codec/WAL 신설 불필요. U4는 파일 I/O만 소유. |
| full jitter 난수원 | 자체 소형 결정적 PRNG(splitmix64 계열, 주입 시드) | **예** | 주입 시드 결정성(D-13, PBT 재현) 직접 실현 + 외부 의존 0. jitter 품질 요구 자명. |
| `rand` 크레이트 | **미채택** | **예** | 의존 트리 도입 대비 이점 미미. 단일 공식(D-09)에 자체 PRNG가 결정성 요구에 직접 맞음. |
| 백오프 타이밍 | `std::time::{Duration, Instant}` 산술 + 주입 `now` | 아니오 | 순수·결정적 계산(D-13). 전역 시계 미사용. |
| 백오프 수치 기본값(D-10 이월 해소) | `initial_delay=1s`, `multiplier=2.0`, `cap=300s`, `jitter=full`; `N=notify_consecutive_failures`(U0, 기본 3) | 아니오(이월 해소) | FR-11 무한증가 방지(상한), NFR-04. 값은 federated 키 `backoff`로 U8이 타입드 주입. |
| federated config 주입 | `StateConfig`(`state_path: PathBuf`)·`BackoffConfig`(`Duration` 필드)를 U8 watcher-bin이 생성자 주입, U4는 `ConfigProvider` 미독취 | 아니오(웨이브 정책) | DEC-FEDERATED-KEYS Q6=A 하향 주입. live-reload는 MVP 밖(재시작). |
| 오류 파생 | `StateError` = `thiserror`; `PersistedState`/`ResumeOffsetMap` = 순수 serde | 아니오 | U0 §5 관례 상속. anyhow 미채택(구조화 변이 필요). |
| PBT feature | `proptest` optional + `proptest-support` 비기본 feature(U0 미러링) + `foundation` dev-dep `proptest-support`로 제너레이터 재사용 | 아니오 | PBT-09 상속 + PBT-07(재사용·드리프트 방지). 프로덕션 빌드 유출 방지. |
| 신규 외부 크레이트 | **0건** | **예** | std + U0(foundation) + 워크스페이스 상속 핀(serde/thiserror/proptest)만. MVP 최소 스택. |
| patch-pin + MSRV(1.85) CI 검증 | **Build-and-Test 이월** | 아니오(이월) | U0와 동일 정책. 이 문서는 major/minor 상속만 확정. |

---

## 8. 의존성 요약표

**범례**: version-policy = 외부 크레이트는 `[workspace.dependencies]` 상속, 내부는 path + `[workspace.package]` 상속. kind = normal(런타임) / dev(테스트). **U4는 신규 외부 크레이트를 도입하지 않는다** — 아래는 모두 워크스페이스 상속 또는 std.

| crate / 기법 | version-policy | kind | feature | 신규? | grounding |
|---|---|---|---|---|---|
| `foundation` (U0) | path 의존, `[workspace.package]` 상속, `publish=false` | normal + dev | dev에서 `proptest-support`(제너레이터 재사용) | 아니오(내부, FROZEN) | 코덱·`CoreTypes`·오류 taxonomy 소비(§5) · U4-NFR-MNT-02 |
| `serde` | workspace-inherited (핀 `1`) | normal | `derive` | 아니오 | `PersistedState`/`ResumeOffsetMap` 파생(§3/§5) |
| `thiserror` | workspace-inherited (핀 `2`) | normal | — | 아니오 | `StateError` 운영 오류 파생(§3, U0 관례) |
| `proptest` | workspace-inherited (핀 `1`) | **dev** | `proptest-support`(비기본, optional dep) | 아니오 | PBT-09 상속(§6) · PROP-U4-01~07 |
| `std::fs` (rename/File/sync_all) | 표준 라이브러리 | — | — | 아니오(std) | crash-atomic 원자 쓰기(§2) |
| `std::time` (Duration/Instant) | 표준 라이브러리 | — | — | 아니오(std) | 백오프 타이밍(§4) |
| 자체 결정적 PRNG(splitmix64) | U4 소스(~5~10줄) | — | — | 아니오(자체 구현) | full jitter 결정성(§4) |
| `ciborium` | (U0 내부 백엔드, U4 직접 의존 아님) | — | — | 아니오 | U4는 U0 코덱 표면만 호출(§5) |
| `tempfile` / `rand` | **의도적 미채택** | — | — | — | §2 / §4 트림 근거 |

> **버전 핀 주석**: 위 major/minor 핀은 워크스페이스 `[workspace.dependencies]`에서 상속하며, 정확한 patch 핀과 MSRV(Edition 2024, Rust 1.85) 정합 검증은 **Build-and-Test**에서 수행한다(U0 정책 상속). `tempfile`·`rand`는 §2·§4 근거로 미채택.

---

## 9. 범위 절제 기록 (Scope Trims / Deferrals)

| 절제 항목 | 결정 | 근거 / 이월처 |
|---|---|---|
| WAL / write-ahead log | **미채택**(temp+rename만) | D-01. 소형 상태 전체 재기록 저렴, rename 원자성으로 NFR-03 충족(§2). |
| `tempfile` 크레이트 | **미채택**(std 직접) | 같은 디렉터리 임시 파일 + 부모 dir fsync 요구에 std가 더 정확(§2). |
| `rand` 크레이트 | **미채택**(자체 PRNG) | 결정성 요구에 자체 splitmix64가 직접 맞고 의존 0(§4). |
| 코덱 streaming API | **buffered 확정**(U0 상속) | 원자 temp+rename가 완결 파일을 요구 -> fork 부재(U0 §11 상속). |
| 백오프 정책 엔진 / 서킷브레이커 / 재시도 예산 | **미채택**(단일 full jitter 공식) | D-09 MVP. 오류 클래스별 상이 스케줄 없음. |
| federated 파라미터 live-reload | **MVP 밖**(재시작으로 변경) | 웨이브 정책(DEC-FEDERATED-KEYS). U0 core-field 리로드만 fan-out. |
| 크래시 주입 하니스 구체 구현 | **NFR Design/Operations 이월** | RESILIENCY-14, US-E7-11(§6.3). |
| PBT-08 상세(케이스/시드/CI) | **Code Generation / Build-and-Test 이월** | property-based-testing.md Enforcement Integration. |
| patch-pin + MSRV(1.85) CI 검증 | **Build-and-Test 이월** | U0 정책 상속(§0). |

---

## 10. 확장 컴플라이언스 요약 (완료 게이트용)

| 확장 | 활성 | 이 문서 적용 판정 | 근거 |
|---|---|---|---|
| **Property-Based Testing** | ON (Full) | **강제·준수 — PBT-09 상속 충족** | §6이 워크스페이스 `proptest`(PBT-09, U0에서 확정) 상속 + `proptest-support` 비기본 feature 게이팅(U0 미러링) + `foundation` dev-dep로 U0 제너레이터 재사용(PBT-07). PBT-08 상세·크래시 주입 하니스는 이월(명시). blocking 없음. |
| **Resiliency Baseline** | ON | **핵심 적용** | RESILIENCY-01: U4 = High(zero-loss 저장 계층). §2 `std::fs` temp+rename+fsync가 crash-atomic RPO=0 지속(NFR-03)의 실행 기술, §4 std 백오프 + graceful offline(NFR-04, RESILIENCY-10). RESILIENCY-14 하니스는 NFR Design/Operations 이월. RTO/RPO 수치·DR·HA·배포는 U4(얇은 lib)에 **N/A**(RESILIENCY-02). |
| **Security Baseline** | OFF | **N/A** | 미로딩·미강제. U4에 실현할 잔존 통제 없음(TLS=U5, 토큰=U0). 로컬 평문 상태 파일 = 수용 위험 RISK-01(requirements §7/§12.2). 암호화 저장·키관리 신설 없음. |

**N/A NFR 카테고리(기술 선택 없음)**: 확장성 · 가용성 · 성능 수치 목표(지속 선형·유계·백오프 O(1) 정성 계약만, `nfr-requirements.md` §3) · Resiliency DR/RTO · config 파일 외 사용성 · 보안 강제 통제. 근거 상세는 자매 산출물 `nfr-requirements.md` §7 N/A 판정표.
</content>
