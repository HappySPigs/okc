# U0 Foundation — Business Rules (결정 규칙 / 검증 로직 / 제약)

**단계**: CONSTRUCTION → Per-Unit Loop → **U0 Foundation** → Functional Design → 산출물 2/3 (`business-rules.md`)
**작성일**: 2026-09-08
**크레이트**: `foundation` (lib) · **소속 컴포넌트**: `CoreTypes`, `ConfigProvider`
**전제(확정 답변)**: Q1=B(내부 지속 상태 = CBOR) · Q2=A(SafetyLimits = 컴파일타임 상수) · Q3=A(리로드 실패 = keep-last-good, 최초 로드 실패 = abort) · Q4=B(알 수 없는 config 키 = strict reject) · Q5=A(리로드 트리거 = CLI `reload`만) · Q6=A(발견 = 플랫폼 기본 경로 + `--config` + `OKC_WATCHER_CONFIG`) · Q7=A(토큰 우선순위 = secure-store > config > env) · Q8=A(SyncState = 선형 + dirty 재진입) · FQ-1=A · FQ-2=A · FQ-3=A(NFR-13 오버레이: US-E7-06 queue→sync-state 정정, requirements §12.2)

> **문서 성격**: 이 문서는 U0가 소유하는 **결정 규칙·검증 로직·제약**을 정의한다. 타입 정의는 자매 산출물 `domain-entities.md`가 소유하며 이 문서는 그 타입명·필드명·`WatcherConfig` 스키마를 **그대로 재사용**한다(재정의·모순 없음). 코덱·상태전이 흐름/시퀀스는 `business-logic-model.md`가 소유한다.
>
> **표기 규약**: 비즈니스 의미 중심의 **기술중립 설계**다. Rust스러운 시그니처는 **참고용**이며 인프라·스레딩·I/O 메커니즘이 아니라 규칙의 개념 형상을 표현한다. 다이어그램은 ASCII 박스 없이 **화살표 표기(A -> B)** 와 표/목록으로 기술한다. English 식별자명은 원문 그대로 유지한다.

---

## 1. 코덱 무손실 불변식 (US-E7-06 / NFR-13)

**규칙 R-CODEC-01 (구속력 있는 불변식)**: `CoreTypes`의 CBOR 코덱(`encode`/`decode`, Q1=B)을 통과하는 **모든** 지속 대상 값 `v`에 대해 다음이 **반드시 성립한다**:

```
decode(encode(v)) == v
```

- **적용 대상 타입**(내부 지속 상태): `Manifest`, `ManifestEntry`, `RelativePath`, `Sha256Digest`, `ManifestDigest`, `Timestamp`, `ChangeSet`, `SyncState`, `ErrorClass`, `TransportError`, `ClassifiedError`, `TransferResult`, 그리고 하위 단위 지속 레코드(`ConsentGrant`[U5] / `UploadHistoryRecord`[U6]).
- **보존 대상(무손실 범위)**: 자유형 오류 문자열(`detail`), 유니코드 코드포인트·개행, `Option` 부재(`None`), 빈 컬렉션, 경계 수치(0·최대값). 어떤 정규화·폴딩·손실 변환도 이 round-trip을 위반해서는 안 된다.
- **오류 처리**: 인코딩/디코딩 실패는 `CodecError`(Fatal 계열, `ErrorClass::Fatal`)로 표면화하며 재시도 대상이 아니다(§3). 흐름 상세는 `business-logic-model.md`.

**US-E7-06 수용기준 정합**(stories.md §US-E7-06):
- "모든 매니페스트 값에 대해 `deserialize(serialize(m)) == m`" → `Manifest`/`ManifestEntry`에 R-CODEC-01 적용.
- "모든 큐 항목(FR-03)에 대해 `deserialize(serialize(e)) == e`" → **FQ-3=A 정정으로 queue entries 대상이 제거**되고 **sync-state(마지막 커밋 매니페스트 + dirty 플래그 + 재개 오프셋)** 로 대체됨(requirements.md §12.2, NFR-13 오버레이). 따라서 `SyncState`/재개 오프셋에 R-CODEC-01 적용.
- "모든 히스토리 레코드(해시/타임스탬프/상태/바이트/오류상세)에 대해 `deserialize(serialize(r)) == r`(자유형 오류 문자열·유니코드 포함)" → `ClassifiedError.detail`·`Timestamp`·바이트 카운트 등 무손실 보존.

> **연계**: 이 불변식은 U4 무손실 지속(RPO=0/zero-loss, Resiliency)의 기반이다 — 크래시/재시작 후 디스크에서 복구된 상태가 원값과 정확히 동일함을 코덱 계층이 보장한다.

---

## 2. ChangeSet no-op 규칙 (NFR-01)

**규칙 R-NOOP-01**: `ChangeSet::is_empty()` 가 `true` 이면(= `added`·`modified`·`deleted` 세 목록이 **모두 비었을 때**), 해당 동기화 사이클은 **no-op**이다:

- **업로드 미생성** — 서버로의 협상/전송이 발생하지 않는다.
- **상태 변경 없음** — 사이클은 부수효과 없이 종료하고 `SyncState`는 `Idle`로 복귀한다(재스냅샷했으나 diff가 없으면 커밋할 것이 없음).

**규칙 R-NOOP-02 (ManifestDigest 동치성)**: `is_empty()` 가 `true` 인 것은 `current` 매니페스트와 `last_committed` 매니페스트가 **논리적으로 동일**함과 **동치**이며, 이는 두 매니페스트의 `manifest_digest` 일치로 O(1) 판정된다:

```
current.manifest_digest == last_committed.manifest_digest
  <=>  diff(last_committed, current).is_empty()  == true
```

- `manifest_digest`는 로컬 no-op discriminator이며 권위 `vault_content_id`가 아니다(FQ-1=A). 서버는 이 판정에 관여하지 않는다.
- 이 규칙은 FR-10 no-op 조기 종료(US-E2-06)의 근거다. `manifest_digest` 계산 로직은 U1 `ContentAddressing::manifest_digest` 소유, 정렬 canonical 형상은 `domain-entities.md` §1.2.

---

## 3. ErrorClass::is_retryable 매핑 (재시도 분류)

**소유 경계**: U0는 오류를 **분류(classify)** 만 한다. 백오프 스케줄·재시도 횟수·지연 계산은 **U4 `RetryBackoffController` 소유**이며 U0는 관여하지 않는다. `TransportError`(U5 `AuthTransport` 산출) → `ErrorClass` 매핑과 `ErrorClass::is_retryable()` 판정만 여기서 확정한다.

**규칙 R-CLASS-01 (전송 오류 → 클래스 → 재시도 판정, 전체 매핑)**:

| `TransportErrorClass` 변이 | 대표 신호 | → `ErrorClass` | `is_retryable()` | 근거 |
|---|---|---|---|---|
| `AuthFailed` | HTTP 401 | `AuthAborted` | **아니오(no)** | 인증 실패 — 재시도해도 동일 실패. 새 자격증명 필요(U5 인증 흐름 위임, `ActiveCondition::AuthFailed`) |
| `ServerError` | HTTP 5xx | `Retryable` | **예(yes)** | 서버측 일시 오류 — 백오프 후 재시도 시 성공 가능 |
| `Backpressure` | PROJECT_BUSY / queue-full / 429 | `Backpressure` | **예(yes)** | 오류가 아닌 정상 지연 신호 — 백오프 후 재시도(단, U4가 지연 강조) |
| `Timeout` | 요청 타임아웃 | `Retryable` | **예(yes)** | 일시 지연 — 재시도 대상 |
| `Network` | 연결 실패 | `Retryable` | **예(yes)** | 일시 네트워크 단절 — 재시도 대상(오프라인 큐잉/재시도) |

**규칙 R-CLASS-02 (`ErrorClass::is_retryable()` 판정 표)**: `TransportError` 무관하게 `ErrorClass` 단독으로도 판정이 확정된다(전역 total function):

| `ErrorClass` | `is_retryable()` | 근거 |
|---|---|---|
| `Retryable` | **예(yes)** | 일시 오류 — 백오프 후 재시도 경로 |
| `Backpressure` | **예(yes)** | 정상 지연 — 백오프 후 재시도(지연 강조) |
| `AuthAborted` | **아니오(no)** | 인증 실패 — 재시도 제외, U5 인증 흐름 |
| `Fatal` | **아니오(no)** | 영구/치명(예: `CodecError`, 프로토콜 위반) — 재시도 무의미 |

**규칙 R-CLASS-03 (전역성/total function)**: `is_retryable()`는 **모든** `ErrorClass` 변이에 대해 정확히 하나의 boolean 판정을 반환하는 **전(total) 함수**다 — 미정의·예외 경로 없음. 마찬가지로 모든 `TransportErrorClass` 변이는 R-CLASS-01에 의해 정확히 하나의 `ErrorClass`로 매핑된다(누락·중복 없음).

> `http_status`는 있을 수도(HTTP 응답) 없을 수도(Network/Timeout) 있으며, 판정은 클래스에 의해 결정되고 상태코드는 진단 보조다.

---

## 4. config 스키마 검증 규칙

소스는 **사람이 편집하는 단일 JSON 파일**(nginx식). `ConfigProvider.load(path)`가 파싱 후 아래 규칙으로 검증하고 타입드 `WatcherConfig`(`ConfigSnapshot`)로 노출한다. 필드 스키마·기본값은 `domain-entities.md` §2 표를 그대로 재사용한다.

### 4.1 필수 / 선택 필드 및 per-field 검증

| 필드 | 필수? | 검증 규칙(형식·범위) | 위반 시 |
|---|---|---|---|
| `vault_path` | 예 | 비어있지 않은 문자열. **절대 파일시스템 경로**여야 함(상대경로·빈 값 거부). 경로 **존재 여부는 검증하지 않음**(런타임 가용성 = U2 `VaultAvailabilityGuard` 관심사) | 검증 실패 |
| `server_endpoint` | 예 | 유효 URL 형식. **스킴은 `https`만 허용**(TLS 강제, NFR-06). `http`/무스킴 거부 | 검증 실패 |
| `token` | 아니오 | 존재 시 비어있지 않은 문자열. 부재 허용(env/secure-store 폴백). 값 형식 검증 없음(권위 유효성은 서버 401이 결정) | 존재하되 공백이면 실패 |
| `secure_store_enabled` | 아니오 | 불리언(기본 `false`) | 불리언 아니면 실패 |
| `log_level` | 아니오 | `trace`/`debug`/`info`/`warn`/`error` 중 하나(기본 `info`) | 그 외 값 실패 |
| `notify_consecutive_failures` | 아니오 | 양의 정수(>= 1)(기본 `3`) | 0·음수·비정수 실패 |

> 다른 단위가 소비하는 섹션(`debounce_ms`, `reconciliation_interval_s`, `exclude_patterns`, `chunk_threshold_bytes`, `backoff`, `request_timeout_s`, `log_rotation`, `service`, `tray_enabled`, `confirm_empty` 등)의 per-field 규칙은 해당 단위 Functional Design이 확정한다. 단, **알 수 없는 키 판정(§4.2)은 전체 통합 스키마 기준**으로 적용된다.

### 4.2 알 수 없는 / 여분 키 = STRICT reject (Q4=B)

**규칙 R-CFG-STRICT-01**: 스키마에 정의되지 않은 키(또는 여분 키)가 **하나라도** 있으면 **검증 실패(reject)** 로 처리한다 — lenient 무시가 **아니다**.

- **근거**: 평문 JSON을 사람이 편집하므로 오타(예: `vault_paht`, `serer_endpoint`)를 조기에 잡는다(nginx식 엄격성).
- **오류 리포트 규칙**: 검증 오류는 **발견된 모든 알 수 없는 키를 나열**한다(첫 키에서 중단하지 않음) — 사용자가 한 번에 여러 오타를 교정할 수 있게 한다. 리포트는 키 이름과 위치(가능 시)를 포함한다.

### 4.3 SafetyLimits = 고정 컴파일타임 상수 (Q2=A, NFR-14)

**규칙 R-LIMIT-01**: `SafetyLimits`는 `WatcherConfig` 필드가 **아니며**, 컴파일타임 고정 상수다. 대응 config 키가 존재하면 §4.2에 의해 **알 수 없는 키로 거부**된다.

| 한도 | 값 |
|---|---|
| 총 볼트 크기 | `<= 20 GiB` |
| 파일당 크기 | `<= 2 GiB` |
| 파일 수 | `<= 100,000` |

- **근거**: okc-core 캡과 일치하며, **서버가 권위 있게 재검증(DEP-04)** 하므로 클라이언트 한도는 프리플라이트 가드다. 클라이언트에서 config로 변경 가능하게 하면 서버 캡과 어긋날 위험이 있어 상수로 고정한다.
- **경계·단조성**(US-E7-07/NFR-14): 경계에서 정확히 accept/reject를 가르고(off-by-one 없음), 파일/바이트를 더해도 reject가 accept로 뒤집히지 않는다. **실제 검사 로직은 U1 `SafetyLimitsValidator` 소유**이며, U0는 상수 값과 경계 계약만 정의한다.

### 4.4 config 파일 발견 우선순위 (Q6=A)

**규칙 R-DISCOVER-01**: `load(path)`의 경로 해소는 **로드타임 관심사**(config 내용 아님)이며 다음 우선순위로 첫 유효값을 채택한다:

```
1) --config <path>  CLI 플래그        (최우선)
2) OKC_WATCHER_CONFIG  환경변수
3) 플랫폼별 표준 기본 경로            (최후순위)
```

- `--config` 플래그가 있으면 env·기본 경로를 무시한다. 플래그 부재 시 `OKC_WATCHER_CONFIG`가 있으면 그것을, 없으면 플랫폼 기본 경로를 사용한다.
- 해소된 경로의 파일이 부재·파싱 실패·스키마 위반이면 최초 로드 실패로 §5의 abort 규칙이 적용된다.

---

## 5. config 리로드 / 실패 동작 (Q3=A)

**규칙 R-RELOAD-01 (선검증 후 원자적 스왑)**: `reload()`는 **새 config를 §4 규칙으로 전부 검증한 뒤에만** 활성 config를 교체한다. 검증은 스왑 이전에 완료된다(load → parse → validate → swap 순).

**규칙 R-RELOAD-02 (원자성 / all-or-nothing)**: 스왑은 **원자적**이다 — 전체 새 config가 통째로 적용되거나(성공) 전혀 적용되지 않는다(실패). **부분 적용 금지**(일부 필드만 반영되는 중간 상태 없음).

**규칙 R-RELOAD-03 (실행 중 리로드 실패 = keep-last-good, nginx식 fail-safe)**: 실행 중 `reload()`가 검증에 실패하면:

- **마지막 정상 config(last-good)를 그대로 유지**한다(활성 config 불변).
- **오류를 로그**로 남긴다(어떤 검증 규칙 위반인지 포함).
- **데몬은 계속 실행**한다 — degraded/paused 상태 진입 없음, 정지 없음.

**규칙 R-RELOAD-04 (최초 기동 로드 실패 = abort)**: **최초 기동 시** `load(path)`가 실패(파일 부재/파싱 오류/스키마 위반/알 수 없는 키)하면 **비정상 종료(non-zero exit)** 한다. 기동 시점에는 유지할 last-good이 없으므로 keep-last-good이 성립하지 않는다.

**규칙 R-RELOAD-05 (관찰자 통지는 성공 후에만)**: 관찰자(subscribers) 팬아웃은 **성공적 스왑 이후에만** 발생한다(§8). 리로드 실패 시 관찰자는 통지받지 않으며 기존 값이 유효하게 유지된다.

---

## 6. 토큰 해소 우선순위 (Q7=A)

**소유 경계**: 최종 토큰 해소 로직은 **U5 `CredentialProvider::resolve_token()` 소유**다. U0는 config가 제공하는 소스(`token`·`secure_store_enabled`)의 스키마와 **우선순위 규칙**만 확정한다.

**규칙 R-TOKEN-01 (순서 있는 해소 알고리즘)**: 다음 순서로 첫 유효 토큰을 채택한다:

```
1) SecureStore   — secure_store_enabled == true  AND  데스크톱 세션 가용   (최우선)
2) config token  — WatcherConfig.token 필드(평문, 1차·기본 소스)
3) Env fallback   — 환경변수 토큰                                          (최후순위)
```

- **secure-store**: opt-in(`secure_store_enabled == true`)이 활성이고 세션이 가용할 때만 최우선으로 승리한다. **opt-in 비활성이거나 세션 불가**(헤드리스/데몬)이면 secure-store를 건너뛰고 **config `token`으로 안전 폴백**한다(`CredentialError::SecureStoreUnavailable` → 폴백).
- **config `token`**: §13 정정에 따른 **1차·기본 소스**. secure-store가 활성이 아닌 일반 배포의 기본 경로다.
- **Env fallback**: config에도 없을 때의 최후 폴백.

**규칙 R-TOKEN-02 (부재 처리)**: 세 소스 모두 토큰이 없으면 `CredentialError::Missing`, 존재하되 공백이면 `Empty`. 이 상태는 U0가 실패로 만들지 않으며(토큰은 config 선택 필드), **인증 시점(U5 `AuthTransport`/`ConsentGate`)에 표면화**된다(요청 시 401 또는 사전 `Missing` 판정).

> **RISK-01 / RISK-02 (수용 위험)**: config 평문 `token`은 **문서화된 수용 위험**이다(requirements.md §13, RISK-01 소스 노출 / RISK-02 재승인 thrash). **Security Baseline 확장 = OFF**이므로 별도 시크릿 강제 통제는 없다. secure-store는 선택적 강화 경로일 뿐이며, TLS 강제(NFR-06)만 잔존 통제로 유지된다.

---

## 7. 리로드 트리거 권한 (Q5=A)

**규칙 R-TRIGGER-01**: `ConfigProvider.reload()`를 호출할 수 있는 유일한 경로는 **CLI `reload` 명령**이다:

```
OperatorCli  ->  ControlPlane  ->  ConfigProvider.reload()
```

- **SIGHUP 등 시그널 트리거 없음** — 시그널 핸들링은 U8 소관이며 U0는 전제하지 않는다.
- **config 파일 변경 자동 감지(file-watch) 리로드 없음** — 편집 중 부분쓰기 리로드 위험을 회피한다.
- 크로스플랫폼(Windows엔 SIGHUP 없음) 이식성을 위해 CLI 경로만 전제한다.

---

## 8. 관찰자 팬아웃 규칙 (subscribe)

**규칙 R-OBSERVER-01 (성공 후 push-only 통지)**: 성공적 리로드(스왑) **직후에만** 등록된 관찰자(`ConfigReloadObserver`)에게 변경을 push한다. 팬아웃은 **push-only·단방향**이며 `ConfigProvider`는 관찰자에 대한 **역참조(back-reference)를 보유하지 않는다**(계약 타입으로만 하향 주입, U8 조립).

**규칙 R-OBSERVER-02 (팬아웃 대상 매핑)**:

| 변경된 config 요소 | 통지 대상(관찰자) | 후속 동작 |
|---|---|---|
| `token` / `secure_store_enabled` | U5 `ConsentGate` / `CredentialProvider` | `on_config_reload()` → 토큰 재해석(US-E4-03) |
| `log_level` | U6 `StructuredLogger` | 최소 로그 레벨 재적용 |

**규칙 R-OBSERVER-03 (결정성)**: 동일한 성공 리로드는 동일한 관찰자 집합에 결정적으로 통지된다(순서·대상 재현 가능). 통지는 부수효과가 관찰자 측에 격리되며 `ConfigProvider` 상태를 되돌리지 않는다.

---

## 9. Testable Properties (PBT-01) — 규칙(RULES) 계층

> **확장 강제(PBT-01, Full)**: 이 문서는 **검증·결정 규칙 계층**의 속성을 식별한다. 각 속성에 카테고리 라벨 {Round-trip, Invariant, Idempotence, Commutativity, Oracle, Induction, Easy verification}과 도메인 제너레이터(PBT-07) 요구를 기재한다. 프레임워크 선택(PBT-09)은 NFR Requirements 이월(Rust=proptest 유력). 코덱·상태전이 흐름 속성은 `business-logic-model.md`, 타입/엔티티 속성은 `domain-entities.md`가 각각 소유한다(중복 회피).

### 9.1 코덱 무손실 round-trip

- **PROP-BR-01 — `decode(encode(v)) == v`** (카테고리: **Round-trip**; PBT-02; US-E7-06/NFR-13)
  - **속성**: §1 R-CODEC-01 적용 대상 모든 값 `v`에 대해 round-trip 무손실. 자유형 `detail`(유니코드·개행·빈 문자열)·`None`·빈 컬렉션·경계 수치 포함.
  - **제너레이터(PBT-07)**: 각 지속 타입의 도메인 제너레이터 — 특히 `RelativePath`(정규화 규칙 만족), 유니코드/개행 포함 `detail` 문자열, 0·경계·대값 `size`/오프셋, 빈/단일/다수 엔트리 `Manifest`.
  - **비고**: `domain-entities.md` PROP-DE-01과 동일 불변식을 규칙 관점(구속력)에서 재확인. 실행은 한 곳에서 공유.

### 9.2 config 파싱 round-trip + 리로드 멱등성

- **PROP-BR-02 — config 파싱 round-trip** (카테고리: **Round-trip**; PBT-04)
  - **속성**: 유효한 `WatcherConfig`를 직렬화 후 재파싱하면 동일 타입드 config를 얻는다(기본값 채움 포함해 안정). `parse(serialize(cfg)) == cfg`(정규화 후).
  - **제너레이터(PBT-07)**: 유효 필드 조합 제너레이터 — `vault_path`(절대경로), `server_endpoint`(https URL), `log_level` 열거, 선택 필드 존재/부재, `notify_consecutive_failures` 경계(1·대값).

- **PROP-BR-03 — 리로드 멱등성 "같은 config 두 번 적용 == 한 번"** (카테고리: **Idempotence**; PBT-04)
  - **속성**: 동일한 유효 config로 `reload()`를 연속 2회 적용한 결과 활성 상태와 관찰자 통지 후 상태는 1회 적용과 관측적으로 동일하다(멱등). 원자적 스왑(R-RELOAD-02)과 정합.
  - **제너레이터(PBT-07)**: 유효 config 제너레이터 + 반복 적용 시퀀스.

### 9.3 is_retryable 분류 전역성

- **PROP-BR-04 — `is_retryable` 는 전(total) 함수** (카테고리: **Invariant**; + Easy verification)
  - **속성**: 모든 `ErrorClass` 변이는 정확히 하나의 boolean 재시도 판정으로 매핑된다(미정의·예외 없음, R-CLASS-02/03). 또한 모든 `TransportErrorClass` 변이는 정확히 하나의 `ErrorClass`로 매핑된다(R-CLASS-01: 누락·중복 없음).
  - **제너레이터(PBT-07)**: `ErrorClass`·`TransportErrorClass` 전 변이 열거 제너레이터(유한 도메인 — 전수 검증 가능, Easy verification).
  - **오라클**: §3 매핑 표가 참조 오라클. 실제 판정이 표와 일치.

### 9.4 strict 알 수 없는 키 거부

- **PROP-BR-05 — 알 수 없는 키 STRICT reject** (카테고리: **Invariant**; Q4=B)
  - **속성**: 스키마에 없는 키가 하나라도 포함된 모든 config 입력은 검증 실패로 판정되며(수락 불가), 오류 리포트는 삽입된 모든 알 수 없는 키를 나열한다(R-CFG-STRICT-01).
  - **제너레이터(PBT-07)**: 유효 config에 무작위 알 수 없는 키(1개·다수)를 주입하는 제너레이터 + 오타 변형(알려진 키의 인접 편집 거리) 제너레이터.

### 9.5 SyncState 전이 (U4 stateful PBT 대상)

- **PROP-BR-06 — SyncState 전이 규칙 모델** (카테고리: **Induction** / 상태 기반; PBT-06)
  - **상태**: 전이 규칙(가드·실패 처리·dirty 재진입, Q8=A)의 **모델은 U0에서 정의**(§`domain-entities.md` §1.5 + `business-logic-model.md`)하고, **명령 시퀀스 속성의 실행은 U4 `SyncStateStore`(PBT-06, US-E7-08)** 가 수행한다. U0 규칙 계층에는 stateful 실행 속성이 **없다** — 모델 정의 + U4 실행 위임만 명시.

### 9.6 속성 없음(No PBT properties identified) 판정

| 규칙/요소 | 판정 | 근거 |
|---|---|---|
| §2 ChangeSet no-op 규칙 | PROP-BR-01(round-trip) 및 `manifest_digest` 결정성(PROP-DE-02, `domain-entities.md`)에 종속 | `is_empty() <=> digest 일치`는 U1 diff 정확성 속성(US-E7-01, U1 소유)으로 검증되며 U0 규칙 계층 독립 속성 없음 |
| §4.3 SafetyLimits 경계/단조성 | **U1 소유**(US-E7-07/NFR-14) | 검사 로직은 U1 `SafetyLimitsValidator` — U0는 상수·경계 계약만 정의, 실행 속성 없음 |
| §7 리로드 트리거 권한 | **No PBT properties identified** | 호출 경로 제약(구성 규칙) — 속성 테스트 대상 아님 |
| §8 관찰자 팬아웃 | 상호작용 모델은 `business-logic-model.md` | push-only 동작 — 값 속성 아님. 결정성은 리로드 멱등성(PROP-BR-03)에 흡수 |

> **제너레이터(PBT-07) 총괄**: 위 속성은 도메인 타입 제너레이터를 요구한다(특히 유효/무효 `WatcherConfig`, 유니코드 `detail`, `ErrorClass`/`TransportErrorClass` 전 변이). 구체 구현·shrinking·고정 시드·CI 통합(PBT-08)은 Code Generation/Build-and-Test 이월이며, 여기서는 **요구 사실과 대상**을 명시한다.

---

## 10. 확장 컴플라이언스 요약 (완료 게이트용)

| 확장 | 활성 | 이 문서 적용 판정 | 근거 |
|---|---|---|---|
| **Property-Based Testing** | ON (Full) | **준수** | §9 Testable Properties 섹션 제공 — 코덱 round-trip(PBT-02), config round-trip+리로드 멱등성(PBT-04), `is_retryable` 전역성/strict-key 거부(Invariant), 제너레이터(PBT-07) 요구 기재. SyncState는 U4 실행 위임 명시(PBT-06). |
| **Resiliency Baseline** | ON | **부분 적용 + 대체로 N/A** | RESILIENCY-01: U0 = Critical(전 단위 의존). §1 무손실 코덱 불변식이 U4 zero-loss 지속(RPO=0)의 기반임을 문서화. §5 keep-last-good/abort는 config 회복력 규칙. RTO/RPO 수치·배포/롤백·관측/HA/DR은 U0(순수 규칙)에 **N/A**(상위 단계/인프라 소관). |
| **Security Baseline** | OFF | **N/A** | 미로딩·미강제. config 평문 토큰(RISK-01)·RISK-02는 §6에 명시된 문서화된 수용 위험. TLS 강제(NFR-06, §4.1 `https`)만 잔존 통제. |
