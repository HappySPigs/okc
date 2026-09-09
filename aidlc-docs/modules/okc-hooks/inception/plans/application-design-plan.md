# 애플리케이션 설계 계획 — okc-hooks "Watcher"

**단계**: INCEPTION → Application Design (계획 + 명확화 질문)
**역할**: Application Architect (애플리케이션 설계자)
**작성일**: 2026-09-07
**상태**: 아래 §4 질문의 `[Answer]:` 태그에 답해 주시면, 계획을 승인받은 뒤 설계 산출물을 생성합니다. **답변·승인 전에는 산출물을 생성하지 않습니다.**

> 이 계획은 승인된 requirements.md / stories.md / personas.md / execution-plan.md를 근거로, 병렬 분석 4건 + 완전성·일관성 크리틱 1건(workflow wf_b6ac560c-928, 5/5 에이전트, 오류 0)을 거쳐 종합했습니다. 크리틱 판정은 **CONCERNS**였고, 그 지적(컴포넌트 과분할·중복·네이밍 드리프트·이벤트 루프 아키텍처 상충)은 §1 권장안에 구조적으로 반영했으며, **사람이 결정해야 하는 사안만** §4 질문으로 남겼습니다.

---

## 1. 권장 설계 접근 (분석 결과 — §4 답변으로 변경 가능)

Application Design은 **높은 수준의 컴포넌트 식별 + 인터페이스 + 서비스(오케스트레이션) 계층 + 의존성 그래프**만 다룹니다. 상세 비즈니스 로직은 이후 단위별 Functional Design에서 확정합니다.

### 1.1 크리틱이 잡은 구조적 정리 사항 (권장 기준선)
초안(41개 컴포넌트)에는 4명의 분석가가 독립 작업하며 생긴 중복·상충이 있었습니다. 아래는 **질문이 아니라 이미 권장 기준선으로 확정**한 정리입니다(원하시면 §4 Other로 뒤집을 수 있습니다):

- **단일 조립 루트(composition root) 1개**: `WatcherDaemon` 하나만 존재 — 의존성 주입, 단일 인스턴스 락 획득, 큐 복구, 실행/종료, 상태 집계 담당. (중복이던 `DaemonSupervisor`는 병합)
- **공유 파운데이션 모듈 신설**: 모든 단위가 참조하는 `ConfigProvider`(설정) + 공유 타입/에러 분류 체계(전송 결과·오류 taxonomy, ChangeSet, 매니페스트/큐/히스토리 값 타입) + 무손실 직렬화 코덱을 한 곳에 두고 조립 루트가 소유해 **아래로 주입**. → 승인된 빌드 순서(U1→U4→U5→U2→U3)에서 U4가 U3/U5 타입을 역방향 참조하는 문제 해소.
- **감지 루프와 업로드 루프 분리(생산자/소비자)**: 감지→적재는 계속 돌고, 업로드는 큐를 독립적으로 배출. (오프라인/대용량 중에도 감지 지속 — US-E3-04) — 단, 확정은 **Q2**.
- **push-only 상태 모델**: 각 단위가 `StatusService`로 상태를 밀어 넣기만 함 → U6가 U1–U5를 역참조하지 않음(순환 방지).
- **네이밍 레지스트리 통일**: 초안의 이름 드리프트(예: `VaultDiff`=`ManifestDiffer`, `Logger`=`StructuredLogger`)를 하나의 정식 이름으로 통일해야 의존성 그래프 검증 가능.

### 1.2 컴포넌트 지형 (통합 후 목표 ~18–20개, 생성 단계에서 최종 확정)

| 계층/단위 | 컴포넌트(권장) | 요지 |
|---|---|---|
| **Foundation** | `ConfigProvider`, `CoreTypes & Codec` | 설정 로드/검증/리로드, 공유 타입·에러 taxonomy·무손실 코덱 |
| **U1 Deterministic Content Core** (순수) | `ContentAddressing`, `VaultScanner`, `ManifestBuilder`, `ManifestDiffer`, `SafetyLimitsValidator` | 해시/매니페스트/일관 스냅샷/diff/프리플라이트. (마지막-커밋 매니페스트 *저장* 위치는 Q2) |
| **U4 Resilience/Queue** | `DurableQueue`, `RetryBackoffController` | 무손실 지속 큐(+coalesce 내장 Q7), 백오프/오프라인 배출 |
| **U5 Auth & Consent** | `AuthTransport`, `CredentialProvider`, `ConsentGate` | TLS+토큰+응답분류, 토큰 소스(config/env 우선), RISK-01 고지·동의·철회 |
| **U2 Change Detection & Trigger** | `FilesystemWatcher`, `ReconciliationScheduler`, `VaultAvailabilityGuard`, `SingleInstanceLock` | 크로스플랫폼 감시(+디바운스 내장), 재조정, 빈-볼트 가드, 단일 인스턴스 |
| **U3 Upload Protocol Client** | `UploadProtocolDriver` | have/want→재개 전송→커밋→진행률 (내부 하위 로직 포함) |
| **U6 Observability** | `StructuredLogger`, `StatusService`, `UploadHistoryStore`, `CriticalErrorNotifier`, `TrayIndicator`(선택) | 로그(주 표면)/상태·헬스/히스토리/중대오류/트레이(선택) |
| **U7 Lifecycle & CLI** | `ServiceManager`, `AutoUpdater`, `Uninstaller`, `RunStateController`, `OperatorCli`, `ControlPlane`(Q5) | 서비스 설치·자동시작, 자동 업데이트/롤백, 제거, 실행상태 제어, CLI + IPC 서버 |
| **Service/Orchestration** | `WatcherDaemon`, `SyncCycleCoordinator`, `UploadDrainWorker` | 조립 루트 + 생산자 루프 + 소비자(배출) 루프 (Q2) |

> 초안 41개 → 중복 병합(~6–8개) + 헬퍼를 상위 컴포넌트의 책임으로 접기(DebounceTrigger/CoalesceEngine/TransferProgressReporter 등) → **~18–20개 상위 컴포넌트** 목표. 정확한 병합 몇 건은 Q2/Q4/Q7 답변에 따라 최종 결정됩니다.

---

## 2. 실행 체크리스트 (계획 승인 후 수행)
- [x] §4 답변 + §1 기준선을 반영해 네이밍 레지스트리(정식 컴포넌트 이름) 확정 — 30개 정식 이름 + 별칭 통일 (`component-dependency.md` §5)
- [x] `components.md` 생성 — 컴포넌트별 목적/책임/인터페이스
- [x] `component-methods.md` 생성 — 메서드 시그니처 + 입출력 타입 (상세 비즈니스 규칙은 Functional Design)
- [x] `services.md` 생성 — 서비스(오케스트레이션) 정의/책임/상호작용
- [x] `component-dependency.md` 생성 — 의존성 매트릭스 + 통신 패턴 + 데이터 흐름 다이어그램(검증된 Mermaid + 텍스트 대안)
- [x] `application-design.md` 생성 — 위 문서 통합본
- [x] 설계 완전성·일관성 검증(순환 없음, 빌드 순서와 의존 방향 일치, U1 순수성 유지) — 크리틱 CONCERNS(blocking 0), 비블로킹 수정 2건 반영
- [x] 활성 확장 컴플라이언스 요약(Resiliency ON / PBT ON / Security OFF; PBT·Security는 Application Design에 대체로 N/A) 포함
- [x] aidlc-state.md 갱신 + 이 계획 단계 완료 표시 (+ FQ-3=A 경미 수정: requirements.md §12 / stories.md 부록)

## 3. 필수 산출물 (Application Design 규칙)
- [x] `aidlc-docs/inception/application-design/components.md`
- [x] `aidlc-docs/inception/application-design/component-methods.md`
- [x] `aidlc-docs/inception/application-design/services.md`
- [x] `aidlc-docs/inception/application-design/component-dependency.md`
- [x] `aidlc-docs/inception/application-design/application-design.md` (통합본)

---

# 4. 결정이 필요한 질문

각 질문의 `[Answer]:` 뒤에 보기 문자를 적어 주세요. 질문마다 제 권장안을 표시해 두었습니다. 맞는 보기가 없으면 **X) 기타**를 고르고 설명을 적어 주세요. 한 번에 몰아서(예: "Q1=B, Q2=A, ...") 답하셔도 되고, **"전부 권장안대로"** 라고만 하셔도 됩니다.
(이미 확정된 것들 — 2 페르소나, nginx식 데몬 모델, Security OFF, 단일 바이너리, 목(mock) 서버, Rust 코어 가정 — 은 질문에서 제외했습니다.)

## Question 1 — okc-core 콘텐츠 지문을 어떻게 재현할까 (dependencies)
**왜 중요한가**: Watcher는 파일/볼트의 "지문(해시)"을 okc-core와 **바이트 단위로 똑같이** 만들어야 서버와 정합이 맞습니다(안 맞으면 have/want·커밋이 깨짐 — 최상위 정확성 위험). 그런데 okc-core는 이 계산기를 외부에 공개(export)하지 않고, 유일한 공개 진입점은 파일 전체를 메모리에 올리며 전체 파싱을 합니다(2 GiB 파일에서 NFR-02 위반).

A) Watcher 쪽 Rust 크레이트에 스킴을 **재구현**(4개 도메인 문자열·정규화·제외규칙·uleb128·canonical-JSON까지 동일하게) + 양쪽 레포에 골든 벡터 커밋 + CI 교차검증.

B) **okc-core(옆 레포 ../okc-core)에 최소 스트리밍 identity API를 추가**해 그 프리미티브만 노출하고, Watcher가 크레이트 의존으로 호출. **(권장)**

C) okc-core를 벤더링해 기존 `inspect_sources` 전체 파이프라인을 그대로 호출.

X) 기타 (아래 [Answer]: 태그 뒤에 설명)

**권장**: **B** (okc-core가 수정 가능한 옆 레포이므로 최소 스트리밍 API 추가가 가장 안전; 불가하면 A + 골든 벡터. C는 메모리 위반으로 기각). 이 결정은 U1 크레이트 구조와 "okc-core 변경이 임계 경로에 있는지"를 좌우합니다.

[Answer]: X watcher는 애초에 폴더 변경점만 감지해서 서버로 데이터 보내는 역할인데 core랑 의존성 자체가 없는데.

## Question 2 — 감지·업로드를 한 사이클로 vs 두 루프로 (service-layer)
**왜 중요한가**: 두 분석가가 서로 반대되는 구조를 만들어 하나로 정해야 합니다. "변경 감지→큐 적재"와 "큐 배출→업로드"를 **분리된 두 루프**로 돌릴지, **트리거마다 한 번에 끝까지** 도는 단일 사이클로 돌릴지. 여기에 "마지막-커밋 매니페스트를 1개만 둘지 2개(관찰본/커밋본) 둘지"가 딸려옵니다.

A) **분리(생산자/소비자)**: 감지 루프가 스냅샷·diff·적재; 업로드 루프가 백오프 게이트 하에 독립 배출. diff는 **단일 last-committed 매니페스트** 기준, 경로별 coalesce로 재관찰 흡수. **(권장)**

B) **단일 직렬 사이클**: 트리거마다 감지→적재→업로드→커밋을 끝까지 수행 후 다음 트리거.

C) 하이브리드: 온라인/유휴 시 단일 사이클, 오프라인/백오프 시에만 분리 배출.

X) 기타 (아래 [Answer]: 태그 뒤에 설명) 

**권장**: **A** — 오프라인/대용량 업로드 중에도 감지가 멈추지 않아야 하고(US-E3-04), 무손실(NFR-03)에 유리. 커밋 전까지 변경을 "선택된 상태"로 유지해 크래시로도 유실 안 됨. 매니페스트는 1개면 충분(2개는 상태·실패모드만 증가). C는 코드 경로 2개로 복잡.

[Answer]: B

## Question 3 — 설정(config) 로더는 어디에 둘까 (component-identification)
**왜 중요한가**: 모든 단위가 설정을 읽는데, 두 분석가가 `ConfigProvider`를 각각(U5 인증 / 서비스 계층) 만들었습니다. 중복 제거 + 위치 확정이 필요합니다.

A) U5(인증)가 `ConfigProvider` 소유 → 나머지 U1..U4/U6/U7이 설정을 위해 U5에 의존.

B) **공유 파운데이션의 `ConfigProvider` 1개**를 조립 루트가 최초 생성해 아래로 주입; U5는 토큰/인증(CredentialProvider·AuthTransport·ConsentGate)만 소유. **(권장)**

C) 스키마는 U1 코어에, 로딩/리로드 배선은 U7에, U5는 토큰 필드만 해석.

X) 기타 (아래 [Answer]: 태그 뒤에 설명)

**권장**: **B** — 설정을 인증 단위에 두면 그래프가 뒤집힙니다(U1..U4/U6가 U5에 의존). 파운데이션에 두고 주입하면 비순환 유지 + 리로드 팬아웃(토큰 변경→ConsentGate 재확인, 로그레벨→StructuredLogger)이 깔끔.

[Answer]: X 간단하게 json 파일로 nginx 처럼 해줘.

## Question 4 — TLS/토큰이 붙는 HTTP 통신 소유자 (dependencies)
**왜 중요한가**: `OkcUploadClient`(U3)와 `AuthTransport`(U5)가 **둘 다** TLS+토큰부착+응답분류를 소유해 충돌합니다. 또한 재시도 판단에 쓰는 "전송 결과/오류 분류 타입"을, U4가 U3/U5보다 **먼저** 빌드되는데 어디에 둘지도 정해야 합니다.

A) **U5가 얇은 `AuthTransport`**(TLS + 매 요청 토큰 + 단일 오류 taxonomy) 소유; U3는 프로토콜 의미만; U4는 그 taxonomy로 재시도 — 단 **taxonomy 타입은 공유 파운데이션**에 둬 U4가 역방향 참조 없이 사용. **(권장)**

B) U3가 HTTP 클라이언트 소유; U5는 토큰 인터셉터+응답분류만; TLS/타임아웃은 U3에서 설정.

C) U5·U3 밖의 중립 전송 컴포넌트를 두 곳에 주입.

X) 기타 (아래 [Answer]: 태그 뒤에 설명)

**권장**: **A** — TLS(NFR-06)·토큰(FR-13)·401 감지(US-E4-03)는 단일 인증 관심사라 U3가 재구현하면 안 됨. 오류 taxonomy 1개면 U4(재시도 vs 인증중단 vs 백프레셔)와 ConsentGate 훅이 단일 계약을 씀. 빌드 순서상 타입은 파운데이션에 있어야 함.

[Answer]: A

## Question 5 — CLI 명령이 실행 중인 데몬에 닿는 통로 (service-layer)
**왜 중요한가**: GUI가 없으니 `status/health/pause/resume/sync-now/stop/history/consent/reload` 같은 명령이 **돌고 있는 데몬**에 전달될 통로가 필요합니다.

A) **로컬 IPC** — Unix 도메인 소켓(mac/Linux) + 명명 파이프(Windows), 소유 사용자 권한으로 제한, 작은 버전드 요청/응답 프로토콜 + `--json`. **(권장)**

B) 루프백 HTTP/JSON 서버(설정 가능한 127.0.0.1 포트).

C) 라이브 채널 없음 — CLI가 명령 파일 + OS 시그널을 쓰고, 데몬이 감시하며 상태 파일을 씀.

X) 기타 (아래 [Answer]: 태그 뒤에 설명)

**권장**: **A** — 로컬 전용·저지연·포트 불필요(포트충돌/방화벽/공격면 없음)·파일·파이프 권한 + OS peer 자격증명으로 접근 통제. `status --json`·종료코드 헬스 계약(US-E5-02)에 적합. B는 이득 없이 리스닝 포트 노출, C는 반쯤 쓰인 파일 경쟁으로 취약.

[Answer]: A

## Question 6 — have/want를 "내용"으로 vs "경로"로 (component-methods)
**왜 중요한가**: 요구사항이 서로 어긋납니다 — FR-07은 "raw_sha256(내용)으로 협상", NFR-09는 "want = 매니페스트 경로들 ∖ 서버 보유". 어느 쪽을 키로 삼느냐가 프로토콜 페이로드 형태를 정합니다.

A) 내용 주소: want를 raw_sha256로 — 내용 같은 파일은 한 번만 업로드; 커밋에 경로→해시 맵 포함.

B) 경로 키: want를 경로로 — 바뀐 경로마다 해당 blob 업로드(교차 중복제거 없음, NFR-09 문자 그대로).

C) **전송은 raw_sha256로 중복제거하되, 매니페스트/커밋은 권위 있는 경로→해시 맵**(NFR-09를 "참조된 서로 다른 blob 해시 ∖ 서버 보유"로 해석). **(권장)**

X) 기타 (아래 [Answer]: 태그 뒤에 설명)

**권장**: **C** — 내용 주소 협상 + 전체 파일 중복제거 + 중복 미디어 재전송 방지를 모두 만족하면서, 서버가 트리를 재구성할 경로 정보도 넘김. FR-07/NFR-09 문구 충돌을 정합적으로 해소.

[Answer]: C

## Question 7 — 지속 큐가 저장하는 "작업 단위"와 coalesce 시점 (component-methods)
**왜 중요한가**: 같은 파일이 짧은 시간에 여러 번 바뀔 때 큐에 무엇을 쌓고 언제 "최신 상태 하나로 합칠지"가 큐 인터페이스와 PBT 참조 모델을 정합니다.

A) **파일별 변경 엔트리**(경로 키), 적재 시 즉시 coalesce(항상 파일별 최신 상태 유지); 커밋의 vault_content_id는 배출 시점 매니페스트에서 도출. **(권장)**

B) 사이클별 ChangeSet 엔트리(트리거당 1개), 배출 직전 지연 coalesce.

C) 2계층: 파일별 blob 엔트리 + vault_content_id를 참조하는 별도 커밋-의도 엔트리.

X) 기타 (아래 [Answer]: 태그 뒤에 설명)

**권장**: **A** — US-E3-02가 경로별 coalesce를 원자적으로 요구. 즉시 coalesce로 큐 최소화 + "최신 상태 승리" 불변식을 PBT-06(US-E7-08)·NFR-11(US-E7-04)이 바로 검증. B는 낡은 중간 사이클을 디스크에 남김, C는 엔트리 타입·순서 제약 추가.

[Answer]: X 합체보단 대체에 가까운데 watcher는

## Question 8 — 전송 도중 파일이 또 바뀌면? (component-methods)
**왜 중요한가**: FR-22는 두 방식을 허용합니다 — 매니페스트 시점에 바이트를 얼려두거나(freeze), 전송 시 각 blob 해시를 재검증하고 불일치면 다시 스냅샷. 전송 중 볼트 지문이 바뀌는 상황 처리를 정해야 합니다.

A) **전송 시 재검증만**: 업로드 때 재-읽기·재-해시 → 매니페스트와 일치하면 전송, 불일치면 그 커밋 중단하고 다음 (디바운스+coalesce된) 사이클이 새 상태를 가져감. 추가 디스크 없음. **(권장)**

B) 매니페스트 시점 freeze: 바뀐 blob을 CoW 복제/스테이징(최대 ~20 GiB 추가 디스크, 플랫폼 의존).

C) 하이브리드: 기본 재검증, 초대용량/저속 전송만 CoW freeze 옵트인.

X) 기타 (아래 [Answer]: 태그 뒤에 설명)

**권장**: **A** — 백그라운드 데몬에서 최대 20 GiB freeze는 비현실적·플랫폼 의존. 재검증은 디스크 0·크로스플랫폼·내용주소와 정합(불일치 blob은 자연히 무효). 중단-후-재coalesce로 커밋을 단일 content-id에 원자적으로 유지.

[Answer]: A

## Question 9 — 데몬 상태를 어떻게 표현할까 (component-methods)
**왜 중요한가**: US-E5-02는 {idle, syncing, offline, error, over-limit}를 나열하지만, 실제로는 동시에 성립하는 상태(paused, auth-failed, consent-blocked, vault-unavailable)가 있고, 자동 업데이트 롤백 게이트가 운영 헬스와 겹칩니다.

A) 모든 상태를 담은 평면 enum 1개 + 운영/업데이트-롤백 공용 헬스 신호 1개.

B) **2축 모델** — 운영 라이프사이클(idle|syncing|offline|paused) + 활성 조건 집합(AuthFailed, ConsentBlocked, OverLimit, VaultUnavailable, UpdateRolledBack) + **업데이트용 순수 liveness `update_probe()`를 운영 `health_check()`와 분리**. **(권장)**

C) 우선순위로 동시 조건을 하나 상태로 접는 평면 enum + 별도 업데이터 체크.

X) 기타 (아래 [Answer]: 태그 뒤에 설명)

**권장**: **B** — 상태들이 상호 배타적이지 않음("idle인데 auth-failed", "offline이고 over-limit"). 평면 enum은 동시 조건을 감춤(위험). 조건 집합이 CriticalErrorNotifier의 닫힌 집합과 1:1. `update_probe()`를 `health_check()`와 분리하면 좋은 새 버전이 단지 그 순간 auth-failed/over-limit이라는 이유로 잘못 롤백되는 것(롤백 루프, 계획 위험 #5)을 방지.

[Answer]: B

---

---

# 5. 후속 확인 질문 (답변 분석 결과 — Steps 8-9)

**확정된 답변**: Q4=A, Q5=A, Q6=C, Q8=A, Q9=B.

**재확인 필요**: Q1(X)·Q2(B)·Q7(X)을 종합하면 원래 요구사항보다 **더 단순한 아키텍처**(= 원래 비전 "원시 볼트를 서버로 업로드, 서버가 okc-core로 컴파일"과 부합)를 가리킵니다. 이는 승인된 requirements.md/stories.md 일부와 어긋나므로, 산출물 생성 전에 아래 3가지만 확정합니다.

## FQ-1 — okc-core 의존성 (Q1=X 반영)
확인: Watcher는 okc-core에 **코드 의존 없이**, 표준 SHA-256으로 파일 지문만 계산하고 **원시 바이트를 서버로 업로드**하며, `vault_content_id` 등 okc 콘텐츠 주소 계산은 **전부 서버**가 담당한다.

A) 맞다 — okc-core 의존성 0. Watcher는 표준 SHA-256 + 경로 매니페스트만. **(권장 — 원래 "원시 볼트 업로드" 비전과 일치)**
   - 영향: U1이 대폭 축소(표준 해시 + 매니페스트 + diff + SafetyLimit만), okc-interop/US-E7-01/US-E7-05(okc 적합성) 제거, FR-02/NFR-08을 "표준 해시 + 경로 맵"으로 재해석.

B) 아니다 — Watcher도 `vault_content_id`를 서버와 바이트 동일하게 계산해야 한다(일부 재현 유지).

X) 기타 (아래 [Answer]: 태그 뒤에 설명)

[Answer]: A — okc-core 코드 의존성 0. Watcher는 표준 SHA-256(= sha256sum)으로 파일 지문 + 경로 매니페스트만 계산하고 원시 바이트를 서버로 업로드하며, 권위 있는 `vault_content_id` 계산은 전부 서버가 담당한다. (Watcher는 로컬 멱등/no-op 판정용 매니페스트 다이제스트만 유지 — okc 스킴 재현 불필요)

## FQ-2 — 지속 상태 모델 (Q2=B + Q7=X "대체" 반영)
확인: 지속 상태는 **"마지막 커밋 매니페스트" 하나**뿐이고, 매 트리거마다 폴더 현재 상태를 **다시 스냅샷 → diff**로 보낸다(폴더 자체가 진실의 원천 = **이벤트별 지속 큐 없음**; "합체"가 아니라 "최신 상태로 대체"). 오프라인이면 사이클이 백오프로 재시도하고, 그 사이 편집은 다음 스냅샷이 자동 포함(무손실).

A) 맞다 — 이벤트 큐 없이 "최신 상태 대체" 모델. **(권장 — Q2=B·Q7과 정합)**
   - 영향: U4 `DurableQueue` → "마지막커밋 매니페스트 + 변경(dirty) 표시 + 재개 오프셋"으로 축소, FR-03/US-E3-01/02·PBT-06(큐 상태머신)을 스냅샷-diff 모델로 재해석.

B) 아니다 — 오프라인 중 개별 변경 이벤트를 무손실 지속 큐에 계속 쌓는다(원래 FR-03 유지). ※ 이 경우 Q2를 A(분리 루프)로 되돌리는 게 자연스럽습니다.

X) 기타 (아래 [Answer]: 태그 뒤에 설명)

[Answer]: A — 최신 상태 대체 모델. 이벤트별 지속 큐 없음. 지속 상태 = "마지막 커밋 매니페스트 1개 + dirty 표시 + 재개 오프셋". 매 트리거마다 폴더를 재스냅샷→diff(폴더가 진실의 원천). 오프라인이면 사이클이 백오프로 재시도하고 그 사이 편집은 다음 스냅샷이 자동 포함(무손실).

## FQ-3 — 승인된 요구사항과의 불일치 처리
FQ-1·FQ-2를 단순 모델로 확정하면 requirements.md/stories.md 일부(FR-02, FR-03, NFR-08, NFR-13, US-E3-01/02, US-E7-01/05/06/08, DEP okc-interop 등)와 어긋납니다.

A) 해당 요구사항/스토리를 이 단순 모델에 맞게 **가볍게 수정**하고 audit에 기록. **(권장)**

B) 요구사항은 그대로 두고 Application Design 문서에 "의도된 divergence"로만 명시.

X) 기타 (아래 [Answer]: 태그 뒤에 설명)

[Answer]: A — 영향받는 요구사항/스토리(FR-02, FR-03, FR-07/NFR-09, FR-12/NFR-11, NFR-08, NFR-13, NFR-17, PBT-06, US-E3-01/02, US-E7-01/08, DEP okc-interop 등)를 이 단순 모델에 맞게 경미 수정하고 audit에 기록한다.

## 참고 — Q3(config) 해석
"간단하게 json 파일로 nginx처럼" → **단일 JSON 설정 파일**을 공유 파운데이션 `ConfigProvider`가 로드(배치는 권장 B와 동일). 토큰도 이 JSON의 한 필드(평문, execution-plan §6-B). 이 해석으로 진행합니다 — 다르면 알려주세요.

---

> **다음 단계**: FQ-1/2/3을 받으면 §1 기준선 + 전체 답변으로 §3 산출물 5종을 생성하고(필요 시 requirements/stories 경미 수정 반영), 표준 완료 메시지(변경 요청 / 승인)로 게이트를 제시합니다. **FQ 확정 전에는 생성하지 않습니다.**
