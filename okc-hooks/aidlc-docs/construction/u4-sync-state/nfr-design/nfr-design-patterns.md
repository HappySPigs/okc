# U4 sync-state — NFR Design Patterns (NFR 실현 설계 패턴)

**단계**: CONSTRUCTION -> Per-Unit Loop -> **U4 Resilience & Retry** -> NFR Design -> 산출물 1/2 (`nfr-design-patterns.md`)
**작성일**: 2026-09-08
**크레이트**: `sync-state` (lib) · **소속 컴포넌트(공개 애그리게이트)**: `SyncStateStore`, `RetryBackoffController`
**입력 아티팩트**: `nfr-requirements/nfr-requirements.md`·`nfr-requirements/tech-stack-decisions.md`(U4 NFR Requirements 산출물) · `functional-design/`(domain-entities·business-rules·business-logic-model, FD 결정 D-01..D-14) · `inception/requirements/requirements.md`(§5 NFR-03/04/11/13 · §7 RISK-01) · U0 `nfr-design/`(nfr-design-patterns·logical-components, 스타일·추적표 템플릿) · 활성 확장 `property-based-testing.md`(ON, Full)·`resiliency-baseline.md`(ON)
**규칙**: `construction/nfr-design.md` Step 6 · `common/content-validation.md`(ASCII 화살표 `A -> B`, 박스 문자 미사용, Rust 제네릭 백틱) · `common/ascii-diagram-standards.md`

> **문서 성격**: 이 문서는 U4가 확정한 **품질 속성(NFR Requirements)** 을 **어떤 설계 패턴으로 실현하는가** 를 기록한다. 여기서 새로 결정하는 것은 없다 — NFR Requirements(카테고리별 NFR)·tech-stack-decisions(구체 크레이트)·Functional Design(규칙 ID R-*, 속성 ID PROP-U4-*)·FD 결정 D-01..D-14를 **설계 패턴으로 실현**한다. 각 패턴은 (a) 패턴/결정 진술, (b) 실현하는 NFR·규칙 근거, (c) 구조 노트/불변식, (d) 명시적 트레이드오프로 기술한다. 논리 컴포넌트 분해의 상세 맵은 자매 산출물 `logical-components.md`가 소유하며, 이 문서는 각 패턴이 어느 논리 컴포넌트에 안착하는지만 §10 추적표에서 참조한다.
>
> **표기 규약**: 화살표는 ASCII `A -> B`만 사용한다(유니코드 화살표 금지). 박스/선-그리기 문자를 쓰지 않는다. Rust 제네릭/타입/식별자(예: `std::fs::rename`, `File::sync_all`, `Instant`, `BTreeMap<Sha256Digest, ByteCount>`, `Result<T, StateError>`)는 백틱으로 감싼다.

---

## 0. U4 설계 성격 요약 (패턴 진입 전 맥락)

U4 `sync-state`는 의존이 **U0 `foundation` 하나뿐**인 Wave-2 얇은 lib이며, 상태를 공유하지 않는 두 애그리게이트로 나뉜다:

- `SyncStateStore` — 마지막 커밋 매니페스트 + dirty 신호 + 진행 중 재개 오프셋을 **crash-atomic**하게 디스크에 지속/복구하는 zero-loss 저장 계층(RESILIENCY-01 = High criticality). **유일한 I/O 소유자**.
- `RetryBackoffController` — 전송 오류를 재시도 정책으로 분류하고 지수 백오프·오프라인 판정·에스컬레이션을 계산하는 **순수·결정적 인메모리 상태머신**(I/O·네트워크·전역 시계 없음).

이 성격상 U4의 NFR 실현 패턴은 (1) **원자 파일 쓰기·복구 패턴군**(§1~§4)과 (2) **순수 결정적 계산 패턴군**(§5~§6), 그리고 (3) 두 축을 가로지르는 **오류·PBT·config-주입 패턴**(§7~§9)으로 분류된다.

---

## 1. crash-atomic 지속 파이프라인 (REL-01) — 같은 디렉터리 temp + `sync_all` fsync + `std::fs::rename` atomic replace [D-01/D-03]

**(a) 패턴/결정**: `SyncStateStore`의 모든 지속 쓰기(`mark_dirty`/`commit_manifest`/`persist_resume_offset`/`clear_resume_offsets`)는 단일 원자 파이프라인을 탄다. `std::fs`만으로 다음 순서를 고정한다:

```
persist(state):
  1) bytes = foundation::encode(&state)             // U0 코덱(ciborium 백엔드), U4 직접 의존 없음
  2) tmp = "<state_path>.tmp"  (최종 파일과 같은 디렉터리)
  3) File::create(tmp) -> write_all(bytes) -> File::sync_all()   // 임시 파일 내용 fsync
  4) std::fs::rename(tmp, state_path)               // 동일 파일시스템 atomic replace
  5) File::open(parent_dir) -> sync_all()           // 부모 디렉터리 메타데이터 fsync(best-effort)
```

WAL/증분 로그는 두지 않는다 — 상태가 소형(매니페스트 1개 + boolean + 소형 맵)이라 매 쓰기 **전체 재기록(buffered `Vec<u8>`)** 이 저렴하고, rename 원자성만으로 RPO=0을 충족한다.

**(b) NFR/규칙 실현**:
- **U4-NFR-REL-01**(원자 all-or-nothing 쓰기): 최종 상태 파일은 항상 **완결된 last-good** 또는 **완결된 새 상태** 중 하나만 관측되며, 부분 기록된 중간 상태는 존재할 수 없다.
- **규칙 R-STATE-01**(temp+rename 원자 쓰기)·**R-STATE-02**(단일 결합 문서)·**business-logic-model §2.1**(persist 파이프라인).
- **U0-NFR-REL-01 연계**: U0 무손실 코덱(`encode`/`decode`)이 복원값 정확성을 보장하고, U4 원자 쓰기가 부분 상태를 배제해 둘이 결합하면 RPO=0 지속이 완성된다.

**(c) 구조 노트/불변식**: (1) 임시 파일은 반드시 **최종 파일과 같은 디렉터리**에 생성한다 — cross-device rename(비원자적 복사 fallback)을 회피한다(AC-2). (2) 세 필드는 하나의 `PersistedState`로 함께 인코딩·교체되므로 필드 간 불일치가 구조적으로 불가능하다(R-STATE-02). (3) 부모 디렉터리 fsync는 rename 메타데이터 내구성 배리어이며 **Unix 한정 유효**, Windows는 best-effort로 처리하고 `MoveFileEx` replace-atomicity + 임시 파일 `sync_all`에 의존한다(§8 이식성 패턴).

**(d) 트레이드오프**: (1) WAL/증분 로그는 **미채택(MVP 트림 D-01)** — 소형 상태에 로그 재생·체크포인트 기계를 얹으면 구현이 커지고 이득이 없다. (2) `tempfile` 크레이트는 **미채택** — 시스템-temp 모델이 "같은 디렉터리" 요구와 어긋나고 부모 dir fsync를 제공하지 않아 `std` 직접 사용이 더 작고 정확하다(tech-stack §2). (3) 채택안의 비용은 매 쓰기 전체 재직렬화이나 상태가 소형이라 무시할 만하다.

---

## 2. last-good 롤백 / 손상 복구 non-crash (REL-02) — U0 panic-free `decode` 위의 total 복구 판정 [D-04]

**(a) 패턴/결정**: `open_and_recover(cfg)`는 로드 시 **패닉하지 않고** 항상 유효한 초기 상태로 진입하는 total 함수다. 복구 판정을 표로 고정한다:

```
open_and_recover(cfg)
  -> 경로 해소(state_path 부재 시 플랫폼 기본)
  -> 최종 파일 부재      : Ok({ None, dirty=false, {} })                       (최초 실행)
  -> 정상 decode         : 잔존 temp 삭제 -> Ok(last-good)                     (잔존 temp = 실패한 쓰기 흔적)
  -> decode 실패(손상)   : Err(StateError::CorruptRecovered{recovered_to: Empty}) + dirty=true 강제
```

**(b) NFR/규칙 실현**:
- **U4-NFR-REL-02**(non-crash 복구): 어떤 손상/절단/잔존-temp 조합에도 패닉 없이 `Ok`(빈/last-good) 또는 `Err(CorruptRecovered)`로 종결한다.
- **규칙 R-STATE-04**(복구 판정 표)·**business-logic-model §2.2**(복구 시퀀스)·**domain-entities §2.4**(`StateError`/`RecoveredState`).
- **U0-NFR-REL-02 의존**: U0 순수 표면이 panic-free total이라 `decode`가 적대적 바이트열에도 `Err(CodecError)`로만 표면화한다 -> U4 복구 핫 경로가 재-크래시하지 않는다. 이 의존이 U4 복구 계약의 전제다.

**(c) 구조 노트/불변식**: (1) "부분쓰기 롤백"의 실체는 손상 파일을 되돌리는 것이 아니라 **atomic rename 덕에 애초에 부분 상태가 커밋되지 않음**이다 — 잔존 temp는 무해하게 정리된다. (2) 디코드 실패 복구는 반드시 `dirty=true`를 남겨 다음 사이클 재조정(FR-04)을 강제한다 -> 마지막 커밋을 잃어도 관측/적재 변경이 재스냅샷으로 zero-loss 재발견(FQ-2=A). (3) `recovered_to`(Empty/LastGood)는 복구 목적지를 정확히 보고해 U8이 재조정 강도를 판단한다. (4) **경계 명시**: T_recon 타이머 자체는 U2/U8 소관이며 U4 계약은 "손상/부재 시 `dirty=true`로 재조정 트리거"까지다.

**(d) 트레이드오프**: 방어적 손상 경로에서 "빈 상태 + dirty" 대신 "패닉/abort"를 택하는 대안은 **미채택** — 데몬(GUI-less 서비스)이 재시작마다 재-크래시해 keep-alive를 무력화한다. `CorruptRecovered`를 치명 오류가 아니라 **비치명 복구 신호**로 승격하는 대가로 U8이 이 신호를 소비해 재조정을 강제해야 하는 계약을 진다.

---

## 3. 무손실 round-trip + latest-state-wins (REL-03 / REL-04) — U0 코덱 재사용 + 단일 원자 쓰기 커밋 전이 [D-02/D-06]

**(a) 패턴/결정**: `PersistedState`의 직렬화는 **U0 `foundation::core_types::codec::{encode, decode}`** (FROZEN, ciborium 백엔드)를 호출만 한다 — U4는 `ciborium`을 직접 의존하지 않는다. `PersistedState`/`ResumeOffsetMap`는 `serde::Serialize`/`Deserialize`만 파생해 U0 코덱 시그니처(`encode<T: Serialize>`/`decode<T: DeserializeOwned>`)에 맞춘다. `commit_manifest(m)`는 `{last_committed=m, dirty=false, resume_offsets={}}`를 **한 번의 원자 쓰기**(§1 파이프라인)로 반영한다(clear 흡수).

**(b) NFR/규칙 실현**:
- **U4-NFR-REL-03**(무손실 round-trip): `decode(encode(s)) == s` — `None`/빈 매니페스트/다수 엔트리, `dirty` 양값, `resume_offsets` 빈/다수·경계 오프셋 모두 보존. `ResumeOffsetMap = BTreeMap<Sha256Digest, ByteCount>`의 **결정적 정렬**이 round-trip 안정성을 보장한다.
- **U4-NFR-REL-04**(latest-state-wins): `commit_manifest` 임의 순서에서 `last_committed_manifest()`는 항상 마지막 커밋과 같다(구조적 최신-상태-대체, 별도 coalesce 코드 없음).
- **규칙 R-STATE-05/06**(dirty 라이프사이클)·**R-RESUME-03**(commit이 offsets clear)·**business-logic-model §2.4** · **PROP-U4-03/04**.

**(c) 구조 노트/불변식**: (1) coalesce는 FQ-2=A상 다음 사이클 재스냅샷으로 **구조적으로** 성립하므로 U4 속성은 큐 인터리빙이 아니라 **커밋 지속 전이의 불변식**이다. (2) 커밋 원자 쓰기가 `resume_offsets`를 같은 트랜잭션에서 비워 stale 오프셋이 다음 사이클로 새지 않는다. (3) 인코딩/디코딩 실패는 `StateError::Serde`(= U0 `CodecError`, Fatal, `is_retryable()==false`)로 표면화되며 패닉하지 않는다(§2와 결합).

**(d) 트레이드오프**: (1) U4 자체 codec 신설은 **미채택** — U0 FROZEN 표면 뒤 캡슐화된 코덱 재사용으로 충분하며 별도 codec은 무손실 계약 중복이다(tech-stack §5). (2) 필드별 개별 파일/부분 저장은 **미채택**(D-02) — 필드 간 불일치를 원천 차단하는 단일 문서가 원자성 보증을 단순화한다. (3) 대가: 어떤 소규모 갱신(예: 오프셋 1건)도 전체 문서 재기록이나, 상태 소형성(§1)으로 무시할 만하다.

---

## 4. 지속 비용 유계 / non-streaming buffered 재기록 (PERF-01) — 원자성이 요구하는 완결 파일 [D-01]

**(a) 패턴/결정**: `SyncStateStore`는 **명시적으로 non-streaming**이다 — `encode`가 산출한 완결 `Vec<u8>`를 임시 파일에 통째로 기록한 뒤 rename한다. 스트리밍/청크 쓰기를 두지 않는다. `RetryBackoffController`의 `classify`/`on_failure`/`on_success`는 **O(1)** 산술·비교로 I/O가 없다.

**(b) NFR/규칙 실현**:
- **U4-NFR-PERF-01**(지속 비용 = 상태 크기 선형·유계 / 백오프 O(1); 수치 게이트 N/A): encode·전체 재기록 비용은 매니페스트 엔트리 수(SafetyLimits 상한 <= 100k, U0 R-LIMIT-01)에 선형이며 인메모리 버퍼드 변환이다.
- **규칙 R-STATE-02**(전체 재기록)·**business-logic-model §2.1**.

**(c) 구조 노트/불변식**: 원자 temp+rename는 **완결 파일**을 요구한다 — 스트리밍 쓰기는 부분 파일 순간을 만들어 원자성 계약(§1)과 상충하므로 buffered 전체 쓰기가 원자성의 필연적 귀결이다. 즉 U4의 non-streaming은 성능 태만이 아니라 **원자성이 강제하는 설계**다. 절대 처리량/지연/피크메모리 게이트는 U4에 설정하지 않는다(입력 크기 종속·상위 직렬 사이클이 쓰기 빈도 지배).

**(d) 트레이드오프**: 대용량 스트리밍 해시 메모리 바운드(NFR-02)는 **U1 소관**이지 U4 값 지속의 관심사가 아니다 — U4에 스트리밍 codec fork를 도입하지 않는다(U0 §11 buffered 확정 상속).

---

## 5. 순수·결정적 백오프 상태머신 (REL-05 / MNT-01) — 주입 `now: Instant` + 주입 시드 splitmix64 자체 PRNG [D-09/D-13]

**(a) 패턴/결정**: `RetryBackoffController`는 부수효과가 없는 순수 상태머신이다. 타이밍은 `std::time::{Duration, Instant}` 산술만 쓰고, `now: Instant`는 `on_failure(err, now)`에 **주입**한다(전역 시계·`Instant::now()` 내부 호출 없음). full jitter의 `[0, base_delay]` 균등 난수는 **자체 구현 소형 결정적 PRNG(splitmix64 계열, ~5~10줄)** 로 생성하며, 시드는 컨트롤러 생성 시 주입한다.

```
base_delay(attempt) = min(cap, initial_delay * multiplier ^ attempt)     // std::time 산술, O(1)
retry_after         = splitmix64(seed_state).uniform(0, base_delay)      // full jitter, 결정적
next_retry_at       = now + retry_after                                  // 주입 now
```

**(b) NFR/규칙 실현**:
- **U4-NFR-REL-05**(백오프 graceful degradation): 지수 + full jitter + 상한. 불변식 `0 <= retry_after <= cap`(무한 증가 방지, FR-11). `on_success()`가 `attempt`·`consecutive_failures`·`next_retry_at`을 모두 리셋.
- **U4-NFR-MNT-01**(순수·결정적): 같은 (상태, `now`, seed)이면 같은 `RetryDecision` -> PROP-U4-05 상태머신 PBT의 정확 재현·shrinking 가능. 결정성이 없으면 PBT가 무의미해지므로 "구현 관례"가 아니라 **명시적 테스트가능성 계약**으로 승격.
- **규칙 R-BACKOFF-01/02/03** · **business-logic-model §3.2** · **PROP-U4-05**.

**(c) 구조 노트/불변식**: (1) 주입 경계 불변식 — 컨트롤러 표면에 파일/네트워크/전역 시계 접근이 없다. (2) `base_delay`(지터 전)는 `attempt` 누적에 대해 `cap`까지 **비감소(non-decreasing)**. (3) `now` 주입은 단조 증가 시퀀스를 전제하되 U4는 값을 소비만 하고 시계를 소유하지 않는다.

**(d) 트레이드오프**: (1) `rand` 크레이트는 **미채택(MVP 트림)** — jitter 품질 요구가 자명(암호학적 불필요)하고, 결정성을 위해 `StdRng::seed_from_u64`를 배선하는 이점이 splitmix64 자체 구현 대비 미미하며 의존 트리(`rand`/`rand_core`/`getrandom`)를 새로 도입한다. (2) 시계 추상화를 위한 `Clock` 트레이트 도입은 **미채택** — `now: Instant` 파라미터 주입이 트레이트 seam보다 작고 결정성 요구를 동일하게 충족한다(MVP 트림). (3) 정책 엔진/서킷브레이커/재시도 예산은 **미채택**(D-09) — 단일 full jitter 공식만 지원한다.

---

## 6. classify 전역 매핑 + 에스컬레이션 게이트 + 프로브-없는 오프라인 판정 (REL-05) [D-08/D-11/D-12]

**(a) 패턴/결정**: `classify(TransportErrorClass) -> RetryClass`는 U0의 5개 전송 오류 변형에 **total 매핑**하는 순수 함수이며 `Permanent`를 산출하지 않는다(`Permanent`는 코디네이터가 Fatal 계열을 넘길 때만 도달하는 예약 변형). 오프라인은 **능동 프로브 없이** `Timeout`/`Network` 분류만으로 판정한다(`is_offline=true`, 서버 헬스체크 왕복 없음). `escalate`는 게이트로 고정한다:

```
classify:  AuthFailed->AuthFailed  ServerError->Transient  Backpressure->Backpressure  Timeout|Network->Offline
escalate = (RetryClass == Transient) AND (consecutive_failures >= N)     // N = U0 notify_consecutive_failures(기본 3)
불변식:    is_offline == true  =>  escalate == false                     // 오프라인 반복 알림 방지(US-E3-04)
           AuthFailed | Permanent  =>  retry_after == None               // 카운터 불변(R-AUTH-01)
```

**(b) NFR/규칙 실현**:
- **U4-NFR-REL-05 AC-2/3/4** · **규칙 R-RETRY-01/02**(classify 매핑·순수 오프라인 판정)·**R-ESCAL-01/02**·**R-AUTH-01** · **PROP-U4-06/07**.
- **NFR-04 graceful degradation**: 프로브-없는 오프라인 판정이 별도 서버 왕복을 발생시키지 않아 오프라인 기간의 자원 소모를 최소화한다.

**(c) 구조 노트/불변식**: (1) `classify`는 유한 도메인(5 변형) total이라 누락·중복이 PBT 전수 검증으로 배제된다. (2) 에스컬레이션 게이트의 세 제외(Offline·Backpressure·AuthFailed)는 임의 카운트에서도 `false`를 유지한다. (3) 오프라인 판정은 순수 분류 기반이라 별도 상태·타이머가 없고, 재연결 drain은 다음 사이클이 곧 drain이다(FQ-2=A, 별도 루프 없음).

**(d) 트레이드오프**: 능동 헬스체크/서버 ping으로 오프라인을 판정하는 대안은 **미채택**(R-RETRY-02, MVP) — 추가 네트워크 왕복·엔드포인트 계약·타임아웃 튜닝을 도입하고 NFR-04(자원 절약)와 상충한다. 대가: `Timeout`이 실제로는 서버 과부하일 때도 `Offline`로 표시될 수 있으나, 재시도 백오프가 동일하게 흡수하므로 실질 영향이 없다.

---

## 7. 오류 처리 = U0 taxonomy 상속 (`thiserror` 운영 오류 + 순수 serde 값 타입) [U0 §5 관례 상속]

**(a) 패턴/결정**: U0-NFR-MNT-01 오류 파생 관례를 그대로 따른다.

| 부류 | 타입 | 파생 | 근거 |
|---|---|---|---|
| 운영 오류(반환용) | `StateError`(`CorruptRecovered`/`Io`/`Serde`) | `thiserror`(`Display`/`std::error::Error`) | `Result<T, StateError>`로 반환. `Serde` = U0 `CodecError`(Fatal) 대응 |
| 분류 값 타입 | `RetryClass`·`RetryDecision` | 순수 enum/struct | 지속·전송 안 함 |
| 지속 값 타입 | `PersistedState`·`ResumeOffsetMap` | `serde` 파생만 | U0 코덱 round-trip 대상(§3) |

**(b) NFR/규칙 실현**: **U4-NFR-REL-02**(`CorruptRecovered` 구조화 신호)·**U4-NFR-REL-03**(`Serde` 표면화)·**domain-entities §2.4**.

**(c) 구조 노트/불변식**: `StateError::CorruptRecovered{recovered_to}`는 오류가 아니라 **복구 신호**로, 호출부(U8)가 구조화 변이를 읽어 재조정을 판단한다.

**(d) 트레이드오프**: `anyhow` 타입소거는 **미채택** — `CorruptRecovered{recovered_to}`의 구조화 변이가 U8 재조정 판정에 필요하므로 타입소거는 부적합(U0 §5 동형 근거).

---

## 8. 이식성/어댑터 패턴 (REL-01 보강) — 디렉터리 fsync의 플랫폼 조건부 컴파일

**(a) 패턴/결정**: 부모 디렉터리 fsync(§1 5단계)를 **플랫폼별 조건부 컴파일**로 흡수한다. Unix에서는 `File::open(parent_dir) -> sync_all()`로 rename 메타데이터 내구성을 보강하고, Windows에서는 디렉터리 핸들 fsync 의미가 다르므로 **best-effort/무연산**으로 처리하고 내구성을 `MoveFileEx`(`MOVEFILE_REPLACE_EXISTING`) replace-atomicity + 임시 파일 `sync_all`에 의존한다.

**(b) NFR/규칙 실현**: **U4-NFR-REL-01 AC-3**(부모 dir fsync 배리어, Windows best-effort 문서화) · **tech-stack §2.1** · NFR-05(크로스플랫폼 재현 빌드, 단일 배포 바이너리).

**(c) 구조 노트/불변식**: rename 자체의 replace-atomicity는 Unix(`rename(2)`)·Windows(`MoveFileExW`) 양쪽에서 보장되므로, 부모 dir fsync 유무와 무관하게 **부분 상태 미커밋 불변식(§1)은 전 플랫폼에서 성립**한다. 디렉터리 fsync는 전원 손실 내구성 배리어의 강도 차이일 뿐이다.

**(d) 트레이드오프**: 크로스플랫폼 디렉터리 fsync 추상화 크레이트 도입은 **미채택** — 조건부 컴파일 두 갈래로 충분하며 외부 의존 0을 유지한다. 구체 `cfg` 배선은 Code Generation 이월.

---

## 9. federated config 주입 패턴 (wave 정책) — U8 하향 주입 타입드 값, U4는 `ConfigProvider` 미독취

**(a) 패턴/결정**: U4는 자기 config 값을 **생성자 주입으로만** 받는다 — `ConfigProvider`를 읽지 않는다. U8 `watcher-bin`(composition root)이 raw config 파일을 파싱해 **타입드 값**으로 해소한 뒤 U4 생성자에 주입한다:

```
watcher-bin(U8)  raw config 파싱/검증
   -> StateConfig { state_path: PathBuf }         -> SyncStateStore::open_and_recover(&cfg)
   -> BackoffConfig { initial_delay: Duration, multiplier: f64, cap: Duration, jitter }
                                                  -> RetryBackoffController::new(&cfg, seed)
   -> N (= U0 core field notify_consecutive_failures 의 해소값)  -> 컨트롤러 생성 시 주입
```

**(b) NFR/규칙 실현**: **DEC-FEDERATED-KEYS Q6=A**(하향 주입) · **domain-entities §2.3/§3.3**(`StateConfig`/`BackoffConfig` 투영) · **U4-NFR-REL-05 백오프 수치**(federated 키 `backoff`) · **tech-stack §7**(federated config 주입 결정).

**(c) 구조 노트/불변식**: (1) U4가 소비하는 6 CORE 필드 외 값(`state_path`·`backoff` 파라미터)은 **U0 `ConfigSnapshot`에서 읽지 않는다** — U8이 raw 파싱해 `PathBuf`·`Duration` 타입드로 주입한다(U0 FROZEN 유지). (2) 에스컬레이션 임계 `N`은 U0 core field `notify_consecutive_failures`(기본 3)를 **단일 출처**로 하며, U8이 해소값을 컨트롤러에 주입한다 — `BackoffConfig`에 중복 정의하지 않는다. (3) federated 파라미터의 **live-reload는 MVP 밖**(재시작으로 변경) — U0 core-field(token 등) 리로드만 U0 observer로 fan-out하며, U4는 어떤 U0 observer 계약도 구현하지 않는다(재시작 재구성 모델). (4) `StateConfig`/`BackoffConfig`는 U4에서 config 파싱 파생(serde)을 두지 않는다 — 이미 파싱된 타입드 값이 주입된다.

**(d) 트레이드오프**: U4가 `ConfigProvider`에서 federated 키를 직접 읽는 대안은 **미채택**(wave 정책) — U0가 6 CORE 필드만 타입드로 노출(FROZEN)하므로 non-core 키를 U0에 추가하면 U0 재설계가 필요하다. live-reload 지원도 **미채택**(MVP) — 관찰자 배선·재구성 원자성 부담이 크고 데몬 재시작이 값싼 대안이다.

---

## 10. 추적표 (패턴 -> NFR ID -> 규칙 ID -> 논리 컴포넌트)

| 설계 패턴 | NFR ID | 규칙 ID | 안착 논리 컴포넌트 |
|---|---|---|---|
| §1 crash-atomic 파이프라인(temp+rename+fsync) | U4-NFR-REL-01 | R-STATE-01/02/03 | `SyncStateStore` -> AtomicWriter |
| §2 last-good 롤백 / non-crash 복구 | U4-NFR-REL-02 | R-STATE-04 | `SyncStateStore` -> StateRecoverer · StatePathResolver |
| §3 무손실 round-trip + latest-state-wins | U4-NFR-REL-03, U4-NFR-REL-04 | R-STATE-05/06, R-RESUME-01/02/03 | `SyncStateStore` -> StateCodecAdapter · ResumeOffsetTracker |
| §4 지속 유계 / non-streaming buffered | U4-NFR-PERF-01 | R-STATE-02, R-LIMIT-01(U0) | `SyncStateStore` -> AtomicWriter |
| §5 순수 결정적 백오프 상태머신 | U4-NFR-REL-05, U4-NFR-MNT-01 | R-BACKOFF-01/02/03 | `RetryBackoffController` -> BackoffScheduler · JitterSource |
| §6 classify total + 에스컬레이션 게이트 + 프로브-없는 오프라인 | U4-NFR-REL-05 | R-RETRY-01/02, R-ESCAL-01/02, R-AUTH-01 | `RetryBackoffController` -> ErrorClassifier · EscalationGate |
| §7 오류 처리(U0 taxonomy) | U4-NFR-REL-02, U4-NFR-REL-03 | (StateError 계약) | `SyncStateStore` -> StateRecoverer(StateError) |
| §8 이식성(디렉터리 fsync 조건부) | U4-NFR-REL-01 | R-STATE-01 | `SyncStateStore` -> AtomicWriter |
| §9 federated config 주입 | U4-NFR-REL-05, (config 경계) | (주입 계약) | 생성자 표면(`open_and_recover`·`new`) |
| PBT feature 게이트 + 제너레이터 재사용 | U4-NFR-MNT-02, U4-NFR-MNT-03 | PBT-07/09/10 | ProptestGenerators(test-support, 런타임 그래프 밖) |

---

## 11. MANDATORY 카테고리 N/A 판정표

| 카테고리 | 판정 | 근거 |
|---|---|---|
| **Scalability** | N/A | U4는 얇은 lib으로 자체 런타임·스레드·처리량 축이 없음. 단일 writer는 U8/U2 보장(R-STATE-03). 100k/20 GiB 스트리밍 스케일(NFR-02)은 U1 소관. 신규 확장성 패턴 없음 |
| **Performance(수치)** | N/A(정성 계약만) | §4 지속 선형·유계 + 백오프 O(1) 정성 계약. throughput/latency/peak-memory 수치 게이트 근거 없음(입력 크기 종속·상위 사이클 지배) |
| **Availability** | N/A | lib 크레이트라 SLA 없음. 단일 사용자·사용자 재시작 로컬 데몬이라 RTO/availability-SLA는 requirements RESILIENCY-02에서 N/A 확정 |
| **Resilience** | **핵심 적용 + 일부 N/A** | RESILIENCY-01: U4 = **High**(zero-loss 저장 계층). §1~§3(crash-atomic·last-good·무손실·latest-state-wins)이 RPO=0(NFR-03)의 실행 지점, §5~§6(백오프+graceful offline)이 RESILIENCY-10(NFR-04). RESILIENCY-14 회복력 테스트의 씨앗 = 크래시 주입/오프라인/재개 하니스(§12 이월). RTO/HA/DR 수치·배포는 인프라 소관 N/A |
| **Security** | N/A(강제 통제) | Security Baseline OFF, RISK-01 수용(로컬 평문 상태 파일). U4는 네트워크·자격증명 표면 없음 -> 실현할 잔존 통제 없음(TLS=U5, 토큰 redaction=U0). 암호화 저장·키관리 신설 없음 |
| **Usability** | N/A | U4는 UI/CLI/트레이 표면 없는 lib. config 값은 U8이 타입드 주입(§9, `ConfigProvider` 미독취). `StateError`는 프로그램적 신호이지 사람 메시지 아님. 사용자 표면은 U6/U7 소관 |
| **Logical Components** | 다뤄짐(N/A 아님) | Application Design은 공개 2 컴포넌트만 확정, FD는 타입·규칙·흐름만 정의 — 내부 논리 분해는 NFR Design에서 처음 명명. 상세는 자매 산출물 `logical-components.md`가 소유(추적성 맵, 물리 모듈 증식 강제 아님) |

---

## 12. 후속 단계 이월 항목 (참고)

- **크래시 주입 하니스**(프로세스 kill·전원 손실 모사)·오프라인 시뮬레이션·재개 중단 테스트의 구체 구현 -> Code Generation / Build-and-Test(RESILIENCY-14, US-E7-11, PROP-U4-02/05 씨앗).
- 디렉터리 fsync 플랫폼 조건부 컴파일(`cfg(unix)`/`cfg(windows)`)의 구체 배선 -> Code Generation.
- splitmix64 PRNG 구현·시드 로깅 vs 고정 시드·`proptest-regressions` 정책·CI 통합(PBT-08) -> Code Generation / Build-and-Test.
- patch 버전 핀 + MSRV(1.85) 대비 CI 검증 -> Build-and-Test(U0 정책 상속).

---

## 13. 확장 컴플라이언스 요약 (완료 게이트용)

| 확장 | 활성 | 이 단계 판정 | 근거 |
|---|---|---|---|
| **Property-Based Testing** | ON (Full) | **준수 — blocking 없음** | PBT-09(프레임워크)는 NFR-Req에서 `proptest`로 이미 SATISFIED(워크스페이스 상속) — 이 단계 신규 blocking PBT 결정 없음. 각 패턴이 확정 속성에 정렬: §1/§2/§3 -> PROP-U4-01/02/03/04, §5 -> PROP-U4-05, §6 -> PROP-U4-06/07. PBT-07(제너레이터 재사용)은 `logical-components.md` ProptestGenerators가 문서화. PBT-08·크래시 주입 하니스는 §12 이월 |
| **Resiliency Baseline** | ON | **핵심 적용 + 일부 N/A — blocking 없음** | RESILIENCY-01: U4 = High(zero-loss 저장 계층). §1~§3이 crash-atomic RPO=0 지속(NFR-03)의 설계 실현, §5~§6이 타임아웃+백오프 graceful offline(RESILIENCY-10, NFR-04). RESILIENCY-14 하니스는 §12 이월. RTO/HA/DR 수치·배포는 얇은 lib에 N/A(RESILIENCY-02) |
| **Security Baseline** | OFF | **N/A — 미로딩·미강제** | 확장 OFF. U4에 실현할 잔존 통제 없음(TLS=U5, 토큰=U0). 로컬 평문 상태 파일 = 문서화된 수용 위험 RISK-01(requirements §7/§12.2). 암호화 저장·키관리 신설 없음 |

**블로킹 판정**: 이 단계에 blocking finding 없음. PBT-09는 NFR-Req에서 충족(워크스페이스 `proptest` 상속), Resiliency는 핵심 적용(수치 N/A), Security Baseline은 OFF로 N/A다.

---

## 14. Autopilot Decisions (이 단계 확정 — topic / chosen / MVP-trim? / rationale)

이 NFR Design 단계에서 저자가 확정한 **설계 패턴 수준** 결정(질문 없이 권장안 + MVP 편향). FD 결정 D-01..D-14와 NFR Requirements의 기술 결정은 재오픈 대상이 아니며, 여기서는 그 실현 **설계 패턴**을 확정한다.

| 주제 | 채택안 | MVP 트림? | 근거 |
|---|---|---|---|
| 원자 쓰기 파이프라인 설계 | 같은 디렉터리 temp -> `sync_all` -> `rename` -> 부모 dir fsync 단일 파이프라인(모든 쓰기 경유) | 예 | WAL/증분 로그 미채택(D-01). 소형 상태 전체 재기록 + rename 원자성으로 REL-01 충족. 파이프라인 1개로 모든 mutator 통일 |
| 복구 함수 total 계약 | `open_and_recover` = panic-free total, 손상 시 빈 상태+dirty+`CorruptRecovered` 신호 | 아니오 | 데몬 재-크래시 방지(REL-02). U0 panic-free `decode` 전제 위에 구축 |
| streaming vs buffered 지속 | non-streaming buffered 전체 재기록(원자성이 완결 파일 요구) | 예 | 스트리밍은 부분 파일 순간을 만들어 원자성과 상충. 소형 상태라 buffered가 정확하고 단순 |
| 시계/난수 주입 seam | `now: Instant` 파라미터 주입 + 생성 시 시드 주입(트레이트 seam 없음) | 예 | `Clock`/`Rng` 트레이트보다 파라미터 주입이 작고 결정성(MNT-01) 동일 충족. splitmix64 자체 PRNG |
| 오프라인 판정 방식 | 순수 분류(`Timeout`/`Network`)만으로 판정, 능동 프로브 없음 | 예 | R-RETRY-02. 추가 네트워크 왕복 회피(NFR-04 graceful degradation) |
| 에스컬레이션 게이트 | `Transient && consecutive_failures >= N`만 escalate, `is_offline => !escalate` 불변식 | 아니오 | US-E3-04 오프라인 반복 알림 방지. N은 U0 core field 단일 출처 |
| 디렉터리 fsync 이식성 | `cfg(unix)` fsync / Windows best-effort(MoveFileEx replace-atomicity 의존) | 예 | 크로스플랫폼 추상화 크레이트 미도입. rename replace-atomicity가 전 플랫폼 부분-미커밋 불변식 보장 |
| 오류 파생 패턴 | `StateError`=`thiserror`; 값 타입=순수 serde; `anyhow` 미채택 | 아니오 | U0 §5 관례 상속. `CorruptRecovered{recovered_to}` 구조화 변이가 U8 재조정 판정에 필요 |
| federated config 실현 | `StateConfig`/`BackoffConfig`/N을 U8이 타입드 주입, U4 `ConfigProvider` 미독취, live-reload MVP 밖 | 예 | DEC-FEDERATED-KEYS Q6=A. U0 FROZEN 유지. 재시작 재구성이 값싼 대안 |
| PBT feature 게이트 | `proptest-support` 비기본 feature + `foundation` dev-dep로 U0 제너레이터 재사용 | 아니오 | PBT-07/09 상속(U0 미러링). 프로덕션 빌드 유출 차단 |
| U0 observer 구현 여부 | U4는 어떤 U0 sink/observer 계약도 구현하지 않음(순수 소비자 말단) | 예 | U4는 network·log·config-reload 표면 없음. 불필요한 계약 구현 회피 |
