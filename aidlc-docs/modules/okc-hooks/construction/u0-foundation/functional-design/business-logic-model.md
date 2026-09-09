# U0 Foundation — Business Logic Model (핵심 로직 · 알고리즘 · 데이터 흐름)

**단계**: CONSTRUCTION → Per-Unit Loop → **U0 Foundation** → Functional Design → 산출물 3/3 (`business-logic-model.md`)
**작성일**: 2026-09-08
**크레이트**: `foundation` (lib) · **소속 컴포넌트**: `CoreTypes`, `ConfigProvider`
**전제(확정 답변)**: Q1=B(내부 지속 상태 = CBOR) · Q2=A(SafetyLimits = 컴파일타임 상수) · Q3=A(리로드 실패 = keep-last-good, 최초 로드 실패 = abort) · Q4=B(알 수 없는 config 키 = strict reject) · Q5=A(리로드 트리거 = CLI `reload`만) · Q6=A(발견 = 플랫폼 기본 경로 + `--config` + `OKC_WATCHER_CONFIG`) · Q7=A(토큰 우선순위 = secure-store > config > env) · Q8=A(SyncState = 선형 + dirty 재진입) · FQ-1=A(표준 SHA-256, 권위 `vault_content_id`는 서버) · FQ-2=A(최신 상태 대체, 폴더가 진실의 원천) · FQ-3=A(NFR-13 오버레이: US-E7-06 queue→sync-state 정정, requirements §12.2) · Q2(사이클)=B(트리거당 1회 직렬 사이클)

> **문서 성격**: 이 문서는 U0가 소유하는 타입(`domain-entities.md` 정의)과 규칙(`business-rules.md` 정의) 위에서 동작하는 **핵심 로직·알고리즘·데이터 흐름**을 기술한다. 타입의 필드/불변식은 `domain-entities.md`, 결정·검증 규칙(매핑 표·검증 규칙·우선순위 규칙)은 `business-rules.md`가 소유하며 이 문서는 그 타입/규칙을 **참조**한다(중복 정의 회피).
>
> **표기 규약**: 비즈니스 의미 중심의 **기술중립 설계**다. Rust스러운 시그니처는 **참고용(reference-only)** 이며 인프라·스레딩·I/O 메커니즘이 아니라 개념적 흐름을 표현한다. 다이어그램은 ASCII 박스 없이 **화살표 표기(A -> B)** 와 표/순서 목록으로 기술하고, 복잡 요소에는 텍스트 설명을 첨부한다. English 식별자명은 원문 그대로 유지한다.

---

## 1. encode/decode 코덱 흐름 (Q1=B: CBOR)

### 1.1 두 직렬화 경로의 분리 (혼동 금지)

U0에는 **두 개의 완전히 별개인 직렬화 경로**가 있으며 서로 섞이지 않는다.

| 경로 | 포맷 | 방향 | 소유 | 대상 데이터 |
|---|---|---|---|---|
| **A. 사람이 편집하는 설정** | **JSON**(고정, nginx식) | 사람 -> `ConfigProvider.load` -> 타입드 `WatcherConfig` | `ConfigProvider` (§4) | 설정 파일 1건 |
| **B. 데몬 내부 지속 상태** | **CBOR**(serde 호환 바이너리) | 값 -> `CoreTypes.encode` -> 바이트 -> (파일 I/O) -> `CoreTypes.decode` -> 값 | `CoreTypes` 코덱 (이 절) | 아래 §1.2 목록 |

- **CBOR는 오직 경로 B(내부 상태)에만 쓰인다.** 설정 파일은 사람이 읽고 편집해야 하므로 CBOR가 아니라 JSON을 유지한다(경로 A). 이 둘을 혼용하지 않는 것이 핵심 계약이다.

### 1.2 CBOR 코덱을 통과하는 CoreTypes (내부 지속 상태)

`CoreTypes.encode`/`decode`가 무손실 왕복을 보장하는 값들(모두 `domain-entities.md`에 정의):

| 값 타입 | 지속 소유 단위 | 지속 대상 개념 |
|---|---|---|
| `Manifest` (+ `ManifestEntry`, `RelativePath`, `Sha256Digest`, `ManifestDigest`) | U4 `SyncStateStore` | 마지막 커밋 매니페스트(diff 기준선) |
| `SyncState` | U4 `SyncStateStore` | 동기화 사이클 상태(+ dirty 플래그) — 모델은 §2 |
| `Timestamp` | 여러 단위 | 이벤트 시각(스냅샷·동의·히스토리) |
| `ClassifiedError` / `TransportError` / `TransferResult` | U4/U6 | 분류된 오류·전송 결과(진단 상세 포함) |
| `ConsentGrant` (U5 정의) | U5 `ConsentGate` | 동의 부여 참조 |
| `UploadHistoryRecord` (U6 정의) | U6 `UploadHistoryStore` | 업로드 히스토리 append 레코드 |
| 재개 오프셋(`resume_offset`) | U4 `SyncStateStore` | 진행 중 업로드 재개 위치 |

> 값 타입과 코덱은 U0가 소유하고, **실제 저장 파일 I/O(파일 열기·원자적 쓰기·복구)는 소비 단위(U4/U5/U6)** 가 수행한다. U0는 순수 바이트 변환만 담당한다(I/O 없음).

### 1.3 코덱 변환 파이프라인 (화살표 표기)

**저장(persist) 방향 — `encode`:**

```
도메인 값 v (예: Manifest)
  -> [CoreTypes.encode<T: Serialize>(v)]
  -> CBOR 바이트열 Vec<u8>
  -> (소비 단위의 원자적 파일 쓰기: temp+rename 또는 WAL — U4/U5/U6 소관)
  -> 디스크
```

**적재(load) 방향 — `decode`:**

```
디스크
  -> (소비 단위의 파일 읽기 — U4/U5/U6 소관)
  -> CBOR 바이트열 &[u8]
  -> [CoreTypes.decode<T: DeserializeOwned>(bytes)]
  -> 도메인 값 v' (T 타입)
```

- **참고 시그니처**: `fn encode<T: Serialize>(value: &T) -> Result<Vec<u8>, CodecError>` · `fn decode<T: DeserializeOwned>(bytes: &[u8]) -> Result<T, CodecError>`.
- **오류 처리**: 직렬화/역직렬화 실패는 `CodecError`(Fatal 계열 — `ErrorClass::Fatal`에 대응, `is_retryable()==false`)로 표면화한다. 손상된 상태 파일의 복구(부분 기록 롤백 등)는 U4 `SyncStateStore`의 관심사이며, U0 코덱은 "이 바이트열이 유효 CBOR인가 / 대상 타입으로 역직렬화되는가"만 판정한다.

### 1.4 무손실 보장 (US-E7-06 / NFR-13)

- **핵심 불변식**: CBOR 코덱을 통과하는 **모든** 값 `v`에 대해 `decode(encode(v)) == v`. 즉 자유형 오류 문자열(`detail`)의 유니코드·개행·빈 문자열, `Option` 부재(`None`), 빈 컬렉션, `size`/오프셋의 경계값(0·대값)까지 **정보 손실 없이** 복원된다.
- **CBOR 선택 근거(Q1=B)**: 자기기술적(self-describing) 바이너리라 **자동 업데이트 후 상태 파일 스키마 진화**(새 필드 추가/무시)에 강하고, 컴팩트·결정적이다. 사람 가독성은 포기하나(디버깅은 로그로 대체), 내부 상태에는 가독성이 요구되지 않는다.
- **U4 무손실 지속의 기반(RESILIENCY 연계)**: 이 무손실 코덱이 U4 `SyncStateStore`의 zero-loss 지속(RPO=0, NFR-03)의 토대다. 크래시/재시작 후 디스크에서 복구한 상태가 원래 값과 정확히 같아야 미반영 편집·재개 오프셋이 유실되지 않는다.

---

## 2. SyncState 전이 모델 (Q8=A: 선형 + dirty 재진입)

### 2.1 모델 소유·실행 위치 (명시)

- **모델 정의 = 여기 U0.** `SyncState` 타입(`Idle`/`Dirty`/`Uploading`/`Committed`, `domain-entities.md` §1.5), dirty-플래그 개념, 아래 전이 표가 U0가 정의하는 **참조 모델**이다.
- **지속·복구 = U4 `SyncStateStore`.** 상태와 dirty 플래그의 디스크 지속(원자적 쓰기), 크래시 후 복구는 U4 소관이다.
- **stateful PBT(PBT-06) 실행 = U4.** U4가 명령 시퀀스(`mark_dirty`/`commit_manifest`/`persist_resume_offset`/persist/recover)를 **이 모델**을 참조 오라클로 삼아 상태 기반 속성 테스트한다(US-E7-08). U0에는 상태 실행 로직이 없으므로 U0 계층에는 stateful 실행 속성이 없다(§6.4 참조).

### 2.2 전제 (FQ-2=A + Q2=B)

- **최신 상태 대체(FQ-2=A)**: 폴더가 **진실의 원천**이다. 사이클은 이벤트를 재생(replay)하지 않고 매번 볼트를 **재스냅샷**해 `Manifest`를 재계산한 뒤 마지막 커밋과 diff한다. 따라서 dirty는 **누적 이벤트 큐가 아니라 단일 boolean 신호**("다음 사이클이 재스냅샷해야 함")다.
- **트리거당 1회 직렬 사이클(Q2=B)**: 한 트리거는 정확히 하나의 사이클을 끝까지 직렬로 돈다. 사이클은 동시 실행되지 않으며(U8 `SyncCycleCoordinator`의 단일-사이클 잠금이 강제), 진행 중 새 트리거/변경은 **dirty 플래그 세팅**으로 흡수되어 다음 사이클로 미뤄진다.

### 2.3 전이 표 (from-state | event | to-state | side-effect / notes)

> "dirty 플래그"는 `SyncState` 값과 **별개**로 유지되는 boolean이다. 상태 값 자체는 아래 4개로 폐쇄되며, dirty는 "커밋 후 재스냅샷 필요" 여부를 표시한다.

| # | from-state | event(트리거·결과) | to-state | dirty 플래그 | side-effect / notes |
|---|---|---|---|---|---|
| T1 | `Idle` | 변경 감지(FS 이벤트 디바운스 / 재조정 tick / sync-now) | `Dirty` | (해당 없음) | 미커밋 변경 존재 표시. 다음 사이클이 재스냅샷 대상 |
| T2 | `Dirty` | 사이클 시작(직렬 잠금 획득) | `Uploading` | 진입 시 **clear** | 재스냅샷 -> `Manifest` 재계산 -> diff -> negotiate -> transfer -> commit 수행(협상/전송은 상위 단위) |
| T3 | `Uploading` | 업로드 **성공** | `Committed` | 불변 | 서버 커밋 성공. 마지막 커밋 매니페스트 지속 직전/직후 |
| T4 | `Committed` | 커밋 지속 완료 & **dirty 미설정** | `Idle` | (false) | 마지막 커밋 = 현재 볼트 상태. 대기 진입 |
| T5 | `Uploading` | 사이클 진행 중 **새 변경 감지** | `Uploading` (유지) | **set** | 이벤트 큐 아님 — 단일 dirty 신호만 세팅. 현재 사이클은 계속 진행 |
| T6 | `Committed` | 사이클 마무리 중 **새 변경 감지** | `Committed` (유지) | **set** | 동일 — dirty만 세팅 |
| T7 | `Committed` | 커밋 지속 완료 & **dirty 설정됨** | `Dirty` | 소비(재진입) | `Idle`로 가지 않고 즉시 `Dirty` 재진입 -> 다음 사이클이 재스냅샷(T5/T6에서 세팅된 변경 반영) |
| T8 | `Uploading` | 업로드 **실패**(재시도 가능 오류) | `Dirty` | (실질 유지) | **별도 `Failed` 상태 없음.** `Dirty`로 되돌려 재시도/백오프(U4 `RetryBackoffController`)를 대기. 다음 사이클이 재스냅샷 |

**텍스트 요약(다이어그램 대안)**: 정상 경로는 `Idle -> Dirty -> Uploading -> Committed -> Idle`의 선형 순환이다. 여기에 두 종류의 "재진입"이 겹친다 — (1) 사이클 진행 중 새 변경은 `Uploading`/`Committed` 상태를 바꾸지 않고 **dirty 플래그만** 세팅하며(T5/T6), 커밋 지속 완료 시점에 dirty가 세팅돼 있으면 `Idle` 대신 `Dirty`로 재진입해(T7) 다음 사이클이 재스냅샷한다. (2) 업로드 실패는 `Failed` 상태를 만들지 않고 `Dirty`로 되돌려(T8) 재시도 대기로 둔다. 폴더가 진실의 원천이므로(FQ-2=A) "무엇이 바뀌었는지"를 기억할 필요가 없다 — 다음 재스냅샷이 최신 상태를 다시 계산한다.

### 2.4 불변식 (전이 모델)

- 상태 집합은 `{ Idle, Dirty, Uploading, Committed }`로 폐쇄. `Failed`·`Paused` 등 추가 상태 없음(운영 pause는 별개 축 — `OperationalState`, §5.3).
- 사이클 시작(T2) 시 dirty를 clear하고 재스냅샷하므로, 사이클 진행 중 세팅된 dirty(T5/T6)만 다음 사이클을 유발한다 — **변경이 유실되지 않으면서 무한 재스냅샷 루프도 없다**(변경이 멈추면 T4로 `Idle` 안착).
- 지속 표현은 무손실 왕복 대상(§1, NFR-13). 전이 합법성의 실제 검증(불법 전이 거부)은 U4가 이 모델 위에서 수행한다.

---

## 3. Manifest 빌드 + diff 데이터 흐름

> U0는 **타입(`Manifest`/`ManifestEntry`/`ChangeSet`/`ManifestDigest`)과 논리적 알고리즘 계약**을 정의한다. 실제 스캔·해시·diff **계산 로직은 U1**(`VaultScanner`/`ContentAddressing`/`ManifestBuilder`/`ManifestDiffer`)가 소유한다. 이 절은 그 데이터 흐름을 U0 관점에서 기술한다.

### 3.1 빌드 흐름 (scan -> Manifest -> ManifestDigest)

```
볼트 루트(디스크)
  -> [scan] 일관 시점 파일 열거              (U1 VaultScanner)
  -> ScannedFile 집합 { relative_path, size }
  -> [각 파일 스트리밍 해시] raw_sha256 계산   (U1 ContentAddressing.hash_stream, 표준 sha256sum, FQ-1=A)
  -> ManifestEntry 집합 { relative_path, raw_sha256, size }
  -> [canonical 정렬: relative_path 오름차순(바이트 사전식)]
  -> [manifest_digest 계산: 정렬된 (path, raw_sha256, size) 시퀀스에 대한 결정적 해시]  (U1 ContentAddressing.manifest_digest)
  -> Manifest { entries(정렬됨), manifest_digest }
```

- **canonical 정렬이 결정성의 근거**: `relative_path` 오름차순 정렬을 canonical 형상으로 하므로, **논리적으로 동일한 볼트는 스캔/발견 순서와 무관하게 동일한 정렬·동일한 `manifest_digest`** 를 낸다(`domain-entities.md` §1.2 Manifest 불변식).
- **`ManifestDigest`는 로컬 no-op discriminator**이지 권위 `vault_content_id`가 아니다(FQ-1=A) — 후자는 서버가 소유·계산한다.

### 3.2 diff 흐름 (prev/current Manifest -> ChangeSet)

```
diff(prev: Manifest(마지막 커밋), current: Manifest(방금 재스냅샷))   (U1 ManifestDiffer.diff)
  -> ChangeSet {
       added:    current 에만 있는 경로의 ManifestEntry,
       modified: 양쪽에 있으나 (raw_sha256 또는 size) 다른 경로의 current ManifestEntry,
       deleted:  prev 에만 있는 경로의 RelativePath
     }
```

- **정확성(NFR-12)**: 누락 0·오탐 0 — 세 목록의 합집합은 prev/current의 정확한 차집합/대칭차를 반영한다. `modified` 판정 기준(size + raw_sha256)의 상세는 U1 이월.
- **재스냅샷 산출물(stateless)**: `ChangeSet`은 지속되지 않는다. 다음 사이클은 다시 재스냅샷해 새 `ChangeSet`을 만든다 — 이벤트 큐가 아니다(FQ-2=A). 지속 상태는 U4의 "마지막 커밋 매니페스트 1개 + dirty 플래그 + 재개 오프셋"뿐이다.

### 3.3 ManifestDigest 동등성 단락(short-circuit) => no-op

```
if current.manifest_digest == prev.manifest_digest:
     ChangeSet 은 empty (added/modified/deleted 모두 빈 목록) 와 동치
     -> ChangeSet.is_empty() == true
     -> no-op 사이클: 협상·전송·커밋 미생성(FR-10, NFR-01), 업로드 안 함
else:
     실제 diff 수행 -> 비어있지 않은 ChangeSet -> 업로드 진행
```

- **O(1) 판정 근거**: `manifest_digest` 일치는 "두 매니페스트가 논리적으로 동일"과 동치이므로, 전체 엔트리 비교 없이 32바이트 비교로 no-op을 조기 판정한다. `is_empty()`의 정확한 규칙과 `manifest_digest` 동치성 증명은 `business-rules.md` 소유.

**텍스트 설명**: 빌드는 "폴더 -> 정렬된 지문 목록 -> 다이제스트"의 단방향 파이프라인이고, diff는 "두 지문 목록(과거/현재)을 비교 -> 변경 집합"이다. 다이제스트가 같으면 diff를 실제로 수행하지 않고 곧장 no-op으로 끝낸다. 어느 단계도 이벤트를 누적하지 않으며, 매 사이클이 현재 폴더 상태로부터 처음부터 다시 계산한다.

---

## 4. Config 생명주기 흐름 (load / current / reload / subscribe)

> 참고 시그니처: `load(path)` · `current() -> ConfigSnapshot` · `reload() -> Result<(), ConfigError>` · `subscribe(observer: &dyn ConfigReloadObserver)`. 필드 스키마·검증 규칙(필수/선택/범위/Q4 strict)은 `domain-entities.md` §2 및 `business-rules.md` 소유. 이 절은 **흐름**을 기술한다.

### 4.1 경로 해소 (Q6=A, 로드타임 관심사)

`load`의 입력 경로는 config **내용**이 아니라 로드타임에 해소된다. 우선순위(높음 -> 낮음):

```
1) --config CLI 플래그        (최우선)
2) OKC_WATCHER_CONFIG 환경변수
3) 플랫폼 기본 경로            (fallback)
```

- 셋 다 없고 기본 경로에도 파일이 없으면 = 최초 로드 실패(§4.4 abort).

### 4.2 최초 load 흐름 (기동 시)

순서 목록:

1. **경로 해소**(§4.1) -> 대상 파일 경로 확정
2. **파일 읽기 + JSON 파싱** -> 파싱 실패 시 실패(§4.4)
3. **스키마 검증**(Q4=B strict):
   - 필수 필드 존재/형식 검증(`vault_path` 비어있지 않음, `server_endpoint`는 `https`만 등 — `domain-entities.md` §2)
   - **알 수 없는/여분 키 = 즉시 reject**(오타 조기 발견, nginx식). lenient 무시 아님.
4. **타입드 `WatcherConfig` 구성** -> 유효하면 `current()`가 반환할 스냅샷으로 확정
5. `current()`가 이 스냅샷을 노출

```
경로 해소 -> 파일 읽기 -> JSON 파싱 -> 스키마 검증(strict) -> WatcherConfig(타입드) -> current()
```

- 위 파이프라인 중 **어느 단계라도 실패**하면 -> §4.4 최초 로드 = abort(non-zero exit).

### 4.3 reload 흐름 (Q5=A CLI `reload`만 + Q3=A keep-last-good)

**트리거 표면(Q5=A)**: `reload`는 **CLI `reload` 명령만**으로 유발된다.

```
OperatorCli(`reload`) -> ControlPlane(IPC) -> ConfigProvider.reload()
```

SIGHUP·파일 변경 자동 감지는 U0 범위 밖(시그널 핸들링은 U8 이월). 리로드 시퀀스:

1. **NEW config load + validate**: §4.2의 2~4단계를 **새 파일**에 대해 수행(경로는 최초와 동일 해소 규칙)
2. **분기(Q3=A)**:
   - **검증 성공** -> **원자적 swap**: `current()`가 가리키는 스냅샷을 NEW로 교체 -> 구독자 팬아웃(§4.5)
   - **검증 실패** -> **keep-last-good**: 마지막 정상 config 유지 + 오류 로그 + **계속 실행**(데몬 정지 안 함). `reload()`는 `ConfigError` 반환(호출한 CLI에 실패 표면화). 구독자 팬아웃 **없음**.

```
reload() -> [NEW load+validate] -> (성공 분기 | 실패 분기)
```

- **성공 분기**: 원자적 swap(current := NEW) -> 구독자 fan-out.
- **실패 분기**: keep-last-good(current 불변) + 오류 로그 + `Err(ConfigError)` (계속 실행).

- **원자성**: swap은 원자적이어야 하며(부분 적용 없음), 구독자는 완결된 NEW 스냅샷만 관측한다. 실패 시 어떤 구독자도 부분/무효 config를 보지 않는다.

### 4.4 실패 동작 요약 (Q3=A)

| 시점 | 실패 종류 | 동작 |
|---|---|---|
| **최초 기동 load** | 파일 부재 / 파싱 오류 / 스키마 위반 / 알 수 없는 키 | **비정상 종료(non-zero exit)** — degraded/paused 진입 없음 |
| **실행 중 reload** | 위와 동일 | **keep-last-good + 오류 로그 + 계속 실행**(nginx식 fail-safe) |

### 4.5 구독자 팬아웃 (subscribe, Q5 CLI-트리거 시에만)

- **패턴**: `subscribe(observer)`로 등록한 관찰자에게, **성공한 reload swap 직후에만** 새 스냅샷을 fan-out한다. U0는 관찰자 인터페이스(`ConfigReloadObserver`)만 정의하고 구독자를 역참조하지 않는다 — **구독자가 등록하는 구조**이므로 U0는 U5/U6 타입을 import하지 않는다(비순환 유지).
- **대표 팬아웃 대상**(구독은 하위 단위가 수행):
  - 토큰/`secure_store_enabled` 변경 -> U5 `CredentialProvider`/`ConsentGate`(재해석, `on_config_reload`)
  - `log_level`/로테이션 변경 -> U6 `StructuredLogger`(`reload`)

```
reload 성공 swap -> fan-out -> [U5 CredentialProvider.on_config_reload] + [U6 StructuredLogger.reload] + ...
```

- **호출 방향**: `ConfigProvider`(U0) -> observer 트레이트 -> 하위 단위 구현. U0에서 하위 단위로의 push이며, 하위 단위가 U0를 역참조하지 않는다.

---

## 5. 싱크 계약(sink-contract) 상호작용 모델 (push-only, 하향 주입)

### 5.1 방향·소유 (U6 역참조 없음)

- **U0가 계약(트레이트)을 정의**하고, **U6가 구현**하며, **조립 루트 U8이 하위 단위에 하향 주입(downward injection)** 한다.
- 모든 싱크는 **push-only**다 — 하위 단위(caller)가 값을 밀어넣기만 하고, U6 -> U0 방향의 되돌아오는 참조가 없다. 따라서 U0는 **U6를 절대 역참조하지 않는다**(빌드 순서 역전 해소).

**호출 방향(화살표 표기):**

```
호출자(U1~U5, U8 코디네이터)  --(push, U0 트레이트 타입으로 주입받음)-->  싱크 트레이트(U0 정의)
U6 구현체(StructuredLogger 등)  --implements-->  싱크 트레이트(U0 정의)
U8(조립 루트)  --injects(하향 주입)-->  U6 구현체를 U0 계약 타입으로 하위 단위에 전달
```

- 즉 U1~U5는 관측을 위해 U6가 아니라 **U0 트레이트에만** 의존한다. U8이 런타임에 U6 구현을 꽂아 push-only가 성립한다.

### 5.2 싱크 트레이트(계약) 카탈로그

`domain-entities.md` §1.6과 정합. U0는 계약(방향·시그니처 형상)만 정의하고 판정 로직은 U6 이월.

| 트레이트 | 상호작용 | 대표 메서드(개념) | U6 구현체 |
|---|---|---|---|
| `Logger` | 하위 단위 -> push 로그 레코드 | `log(record)` / `event(level, event, cycle_id, fields)` | `StructuredLogger` |
| `StatusSink` | 하위 단위 -> push 상태 뮤테이션 | `set_operational` / `raise_condition` / `clear_condition` / `record_sync_success` / `set_dirty` / `set_resume_progress` / `set_liveness` | `StatusService` |
| `HistorySink` | 하위 단위 -> push 히스토리 append | `append(record)` | `UploadHistoryStore` |
| `CriticalEventSink` | 하위 단위 -> push 중대 이벤트 | `report_auth_failure` / `report_cycle_result` / `report_preflight_exceeded` / `report_update_rollback` | `CriticalErrorNotifier` |

- **infallible 표면**: 관측 push는 best-effort·비치명적으로 계약한다 — 관측 실패(로그 쓰기 실패 등)가 동기화 코어를 막지 않는다. `StatusSink` 뮤테이터는 in-memory·infallible. 상세 규칙은 `business-rules.md`.
- **읽기 판정 분리(§0 노트3)**: `StatusSink` 구현(U6 `StatusService`)은 두 개의 분리된 읽기 판정을 제공한다 — `health_check()`(운영 헬스: `OperationalState` + `ActiveCondition` 집합에서 healthy/unhealthy 도출, CLI 종료코드 계약) vs `update_probe()`(순수 liveness: 기동 + `IdleReached` + `CredentialReadable`만 판정). U7a `AutoUpdater`는 `update_probe()`만 소비해 일시 조건(AuthFailed/OverLimit)에 의한 오판 롤백 루프를 방지한다. U0는 두 판정의 **의도·반환 형상 계약**만 정의(임계는 U6 이월).

### 5.3 2축 상태 어휘 모델 (Q9)

상태는 **두 개의 직교 축**으로 표현된다 — 축1은 단일 값(운영 라이프사이클), 축2는 동시 성립 가능한 조건 집합. 추가로 startup liveness 신호가 있으며, 이 모두가 `StatusSnapshot`으로 집계된다.

| 축/타입 | 종류 | 가능한 값 | 의미 |
|---|---|---|---|
| **축1**: `OperationalState` | enum(단일 값) | `Idle` | 대기(사이클 없음) |
| | | `Syncing` | 사이클 진행 중 |
| | | `Offline` | 오프라인(서버 도달 불가) |
| | | `Paused` | 운영자 pause(감시 상주, 사이클 보류) |
| **축2**: `ActiveCondition` | enum(집합 원소, 동시 성립) | `AuthFailed` | 인증 실패(401/토큰 거부) 조건 활성 |
| | | `ConsentBlocked` | 동의 미부여/철회로 업로드 차단 조건 활성 |
| | | `OverLimit` | SafetyLimits 초과 조건 활성 |
| | | `VaultUnavailable` | 볼트 루트 부재/언마운트 조건 활성 |
| | | `UpdateRolledBack` | 자동 업데이트 롤백 발생 조건 활성 |
| **신호**: `LivenessSignal` | enum(startup 신호) | `IdleReached` | 기동 후 최초 idle 도달 |
| | | `CredentialReadable` | 토큰 소스 해소 가능 확인 |
| **집계**: `StatusSnapshot` | struct | `operational` + `conditions:Set<ActiveCondition>` + `last_success:Option<Timestamp>` + `dirty:bool` + `resume:Option<(ByteCount,ByteCount)>` + `consent:ConsentState` + `offline:bool` | CLI `status` 표면용 현재 2축 상태 + 부가 필드 스냅샷 |

- **2축 근거**: 운영 라이프사이클(축1)은 한 시점에 하나의 값이지만, 오류/차단 조건(축2)은 여러 개가 동시에 성립할 수 있다(예: `Syncing` 중 `OverLimit` + `AuthFailed`). 단일 enum으로는 이 조합을 표현할 수 없으므로 두 축으로 분리한다.
- **push 흐름**: 하위 단위가 `raise_condition`/`clear_condition`로 축2 조건을 켜고 끄며, `set_operational`로 축1을 갱신하고, `set_liveness`로 startup 신호를 push한다. `StatusService`가 이를 단일 지점에 집계해 `snapshot()`으로 CLI에 노출한다. 조건 -> 운영상태 결합 규칙(예: `VaultUnavailable`이 운영상태에 미치는 영향)은 U6 이월.

---

## 6. Testable Properties (PBT-01) — 로직/알고리즘 계층

> **확장 강제(PBT-01, Full)**: 이 산출물은 **로직·흐름 계층**의 속성을 식별한다. 각 속성에 카테고리 라벨 {Round-trip, Invariant, Idempotence, Commutativity, Oracle, Induction, Easy verification}과 도메인 제너레이터(PBT-07) 요구를 기재한다. 프레임워크 선택(PBT-09)은 NFR Requirements 이월(Rust=proptest 유력). 타입 계층 속성(정규화 멱등 등)은 `domain-entities.md` §5, 규칙 계층 속성(`is_retryable` 매핑·config 검증·`is_empty` 규칙)은 `business-rules.md`의 PBT 섹션에서 다룬다(중복 회피).

### 6.1 코덱 무손실 round-trip (핵심)

- **PROP-BL-01 — `decode(encode(v)) == v`** (카테고리: **Round-trip**; PBT-02; US-E7-06/NFR-13)
  - **대상**: §1.2의 모든 CBOR 통과 값 — `Manifest`(+ 하위 타입), `SyncState`, `Timestamp`, `ClassifiedError`/`TransportError`/`TransferResult`, 그리고 하위 단위 지속 레코드(`ConsentGrant`[U5]/`UploadHistoryRecord`[U6]는 각 단위에서 자기 제너레이터로 재확인).
  - **속성**: 임의 값 `v`에 대해 `decode(encode(v)) == v`. 자유형 `detail` 문자열의 유니코드·개행·빈 문자열, `Option` 부재(`None`), 빈/단일/다수 컬렉션, `size`/오프셋 경계(0·대값) 포함.
  - **제너레이터(PBT-07)**: 각 값 타입 도메인 제너레이터 — 특히 정규화 규칙을 만족하는 `RelativePath`, 유니코드/개행/빈 문자열 경계를 포함하는 `detail`, 0/경계/대값 정수, 빈/단일/다수 엔트리 `Manifest`.
  - **US-E7-06 정합**: 매니페스트·(지속) 상태·히스토리 레코드의 `deserialize(serialize(x)) == x`. (원문 "queue entries"는 FQ-2=A 최신상태 모델을 FQ-3=A 요구사항 정정으로 반영해 sync-state로 대체 — requirements.md §12.2, NFR-13 오버레이.)
  - **비고(실행 위치)**: 코덱 자체는 U0 소유이므로 U0 크레이트에서 실행. 하위 단위 지속 레코드는 각 단위가 자기 제너레이터로 재확인.

### 6.2 Manifest 빌드/diff 알고리즘 속성

- **PROP-BL-02 — ManifestDigest 순서 무관 결정성** (카테고리: **Invariant** + **Oracle**)
  - **속성(Invariant)**: 동일 논리 엔트리 집합은 삽입/발견 순서와 무관하게 동일 `manifest_digest` — `digest(shuffle(entries)) == digest(entries)`(canonical 정렬 후 계산).
  - **속성(Oracle)**: 참조 오라클 = canonical 정렬 후 표준 재계산과 일치.
  - **제너레이터(PBT-07)**: 임의 엔트리 집합 + 순열(permutation), 경로 유일성 보장, 경계(빈/단일/대량 축소본).
  - **비고(실행 위치)**: 계산 로직은 **U1 `ContentAddressing` 소유**이므로 실행 위치는 U1. U0는 결정성 **계약**을 정의(도메인-entities §5.2와 교차, 여기서는 흐름 관점 재확인).

- **PROP-BL-03 — diff 정확성 오라클: ChangeSet 적용 = 재구성** (카테고리: **Oracle** / **Easy verification**)
  - **속성**: 임의의 두 매니페스트 `prev`, `current`에 대해 `apply(prev, diff(prev, current)) == current`. 즉 `prev`에서 `deleted` 경로를 제거하고 `added`/`modified` 엔트리를 반영하면 `current`(canonical 형상)가 정확히 재구성된다 — 누락 0·오탐 0(NFR-12)의 검증하기 쉬운 형태.
  - **보조 속성(Invariant)**: `added`/`modified`/`deleted`는 서로소이며(같은 경로가 두 목록에 동시에 나오지 않음), 세 목록의 경로 합집합은 `prev`·`current` 경로 대칭차/교차와 정합.
  - **보조 속성(Round-trip 특수)**: `diff(m, m).is_empty() == true`(동일 매니페스트 diff는 no-op) 및 `current.manifest_digest == prev.manifest_digest <=> diff.is_empty()`(§3.3 단락 조건과 동치).
  - **제너레이터(PBT-07)**: 상관된 매니페스트 쌍 제너레이터 — 기저 매니페스트 + 파생(일부 add/modify/delete 적용), 빈 볼트 경계, 전량 삭제/전량 신규 경계.
  - **비고(실행 위치)**: `diff` 계산은 **U1 `ManifestDiffer` 소유**이므로 실행 위치는 U1. U0는 알고리즘 계약(위 오라클 관계)을 정의. `is_empty()`/동치성 규칙 상세는 `business-rules.md`.

### 6.3 Config 흐름 속성

- **PROP-BL-04 — config 파싱 round-trip + reload 멱등성** (카테고리: **Round-trip** + **Idempotence**; PBT-04)
  - **속성(Round-trip)**: 유효한 `WatcherConfig`를 JSON으로 표현 후 다시 load하면 동일 타입드 값 복원(`load(render(cfg)) == cfg`) — 필드 손실 없음.
  - **속성(Idempotence)**: 동일 유효 config로 `reload()`를 **두 번 적용한 결과 = 한 번 적용한 결과**(`current()` 스냅샷 동일). 성공 reload의 재적용은 상태를 바꾸지 않는다.
  - **속성(keep-last-good 보존, Invariant)**: 검증 실패 config로 `reload()`하면 `current()`는 **직전 성공 스냅샷을 그대로 유지**하며(Q3=A), 실패는 `Err(ConfigError)`로만 표면화된다(부분 적용 없음).
  - **제너레이터(PBT-07)**: 유효 config 제너레이터(필수/선택 필드 조합, 경계값 — `log_level` 전 열거, `notify_consecutive_failures` >=1 경계), **무효** config 제너레이터(알 수 없는 키/오타, 무효 스킴, 상대 `vault_path`, 빈 필수 필드) — Q4=B strict reject 및 keep-last-good 검증용.
  - **비고**: 검증 규칙(필수/범위/strict)·오류 리포트 형식은 `business-rules.md` 소유. 이 속성은 그 규칙 위의 **흐름 불변식**(round-trip·멱등·keep-last-good)을 대상으로 한다.

### 6.4 SyncState 전이 모델 (U4 stateful PBT 대상 — 모델은 U0, 실행은 U4)

- **PROP-BL-05 — SyncState 전이 모델** (카테고리: **Induction** / 상태 기반; PBT-06)
  - **상태**: §2의 전이 표(Q8=A)는 **U4 stateful PBT(PBT-06)의 참조 모델**이다. 임의 명령 시퀀스(변경 감지/사이클 시작/업로드 성공/실패/커밋 지속/mid-cycle 변경 + persist/recover)에 대해 실제 U4 `SyncStateStore` 동작이 이 참조 모델과 관측적으로 동일해야 한다(US-E7-08). dirty 플래그 흡수(T5~T7)와 실패 시 `Dirty` 유지(T8), 변경 없으면 `Idle` 안착(T4)이 핵심 불변식.
  - **U0에서의 위치(명시)**: **모델(타입·전이·dirty-플래그)은 여기 U0에서 정의**하고, **명령 시퀀스 실행 속성은 U4가 수행**한다. U0는 상태 실행 로직이 없으므로 **U0 계층에는 stateful 실행 속성이 없다** — 여기서는 모델 정의 + U4 실행 위임을 명시한다.

### 6.5 속성 없음(No PBT properties identified) 판정

| 로직/흐름 요소 | 판정 | 근거 |
|---|---|---|
| 싱크 push 상호작용(`Logger`/`StatusSink`/`HistorySink`/`CriticalEventSink`) | **No PBT properties identified** (U0 계층) | 순수 계약(인터페이스) + best-effort infallible 동작 — 값 변환이 아니라 부수효과 push. 구현 속성(집계·판정)은 U6 소유 |
| 2축 상태 어휘(`OperationalState`/`ActiveCondition`/`LivenessSignal`/`StatusSnapshot`) | round-trip(PROP-BL-01)에 포함 외 **독립 흐름 속성 없음** | 값 어휘 — 조건 -> 운영상태 결합 판정 규칙은 U6 소유 |
| 경로 해소(§4.1) | **No PBT properties identified** (U0 로직 계층) | 우선순위 규칙은 결정적 소수 케이스 — 예제 기반 단위테스트가 적합(PBT 대상 아님). 규칙 상세는 `business-rules.md` |

> **제너레이터(PBT-07) 총괄**: 위 속성은 도메인 타입 제너레이터를 요구한다(특히 `RelativePath`·`Manifest`·상관된 매니페스트 쌍·유효/무효 `WatcherConfig`·자유형 `detail`). 제너레이터 구체 구현·shrinking·고정 시드·CI 통합(PBT-08)은 Code Generation/Build-and-Test 이월이며, 여기서는 **요구 사실과 대상**을 명시한다.

---

## 7. 확장 컴플라이언스 요약 (이 산출물 범위)

| 확장 | 활성 | 이 산출물 적용 | 판정 |
|---|---|---|---|
| **Property-Based Testing** | ON (Full) | **강제·준수** | §6에 로직/흐름 속성 식별(PROP-BL-01~05), 카테고리 라벨·제너레이터 요구 기재. 코덱 round-trip(PBT-02)·diff 오라클·config 멱등(PBT-04)·SyncState 모델(PBT-06, U4 실행) 명시. 속성 없는 요소는 §6.5에 판정. |
| **Resiliency Baseline** | ON | 코덱↔무손실 연계 문서화, 그 외 N/A | RESILIENCY-01: U0=Critical(전 단위 의존) 명시(§1.4). 무손실 CBOR 코덱이 U4 zero-loss 지속(RPO=0, NFR-03)의 기반임을 문서화. RTO/RPO 수치·배포/롤백·관측/HA/DR은 U0(순수 로직)에 **N/A**(상위/인프라 소관). |
| **Security Baseline** | OFF | N/A | 미로딩·미강제. config 평문 토큰(RISK-01)·RISK-02는 문서화된 수용 위험(requirements §12/§13). |
