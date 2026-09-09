# 작업 단위 정의(Unit of Work) — okc-hooks "Watcher"

**단계**: INCEPTION → Units Generation (Part 2, 산출물 1/3)
**작성일**: 2026-09-08
**입력 아티팩트**: `components.md`(30개 컴포넌트), `component-methods.md`(메서드 시그니처), `unit-of-work-plan.md`(§4 UQ-1/2/3 확정 답변), `requirements.md`(FR/NFR/RISK/DEP + FQ 부록), `execution-plan.md`(§4 U1..U7 분해), `services.md`(오케스트레이션 계층)
**확정 결정**: UQ-1=A(10단위) · UQ-2=B(Cargo 워크스페이스) · UQ-3=A(웨이브 DAG)
**관련 산출물**: 단위 간 의존성·빌드 순서·통신 패턴 → `unit-of-work-dependency.md`(2/3); 스토리 매핑(44/44) → `unit-of-work-story-map.md`(3/3)

---

## 1. 개요 및 결정 요약

### 1.1 이 문서의 범위

okc-hooks "Watcher"는 단일 배포 바이너리(RESILIENCY-01) 내부에서 동작하는 GUI 없는 크로스플랫폼 백그라운드 데몬이다(nginx 방식). Application Design에서 확정한 **30개 컴포넌트**를 빌드·설계·테스트의 단위로 묶어, 각 단위의 크레이트명·소속 컴포넌트·책임을 확정한다. 이 문서는 `unit-of-work-plan.md` §4의 세 결정(UQ-1=A / UQ-2=B / UQ-3=A)이 승인된 것으로 전제하고, 그 결정을 컴포넌트 배정 수준으로 구체화한다.

### 1.2 결정 요약

| 결정 | 답변 | 요지 |
|---|---|---|
| **UQ-1 (단위 입도)** | **A — 10단위** | 기존 U7을 **U7a(배포/업데이트)** + **U7b(운영 제어)** 로 분할. Foundation → **U0** 승격, Orchestration → **U8** 승격. `SingleInstanceLock`을 U2 → U8로 이동. |
| **UQ-2 (코드 조직)** | **B — Cargo 워크스페이스** | 단위별 라이브러리 크레이트 + 얇은 `watcher-bin` 바이너리 크레이트. 크레이트 의존이 비순환 그래프를 컴파일타임에 강제. §3 참조. |
| **UQ-3 (빌드 순서)** | **A — 웨이브 DAG** | 선형 체인 폐기, 의존성 DAG 웨이브를 작업 순서 기준으로 채택. U7 분할로 웨이브 깊이 4 → 약 5. 상세 매트릭스는 `unit-of-work-dependency.md`(2/3). |

### 1.3 7단위 → 10단위 정합 근거

`execution-plan.md` §4는 논리 단위 **U1..U7(7개)** 로 44개 스토리를 매핑했다(8+5+8+8+6+5+4 = 44, 누락·중복 0). 그 이후 Application Design과 Units Generation Part 1을 거치며 다음 세 조정이 누적되어 **10단위**가 되었다. 컴포넌트 총 30개는 각 단위에 정확히 1회씩 배정된다.

1. **Foundation → U0 승격 (신규 계층)**: Application Design에서 U1..U7 어디에도 속하지 않는 공유 파운데이션 계층(`CoreTypes`, `ConfigProvider`)이 신설됐다. 모든 단위가 참조하지만 특정 스토리를 "소유"하지 않는 교차 관심사이며, 조립 루트가 소유해 아래로 주입해 비순환을 유지한다. → 독립 단위 **U0**.

2. **Orchestration → U8 승격 (접착 계층)**: `WatcherDaemon`(조립 루트/생명주기) + `SyncCycleCoordinator`(트리거당 1회 직렬 사이클)는 U1..U7에 없는 "접착(glue)" 컴포넌트다(`services.md`가 정의한 유일한 2개 서비스). → 독립 단위 **U8**.

3. **U7 분할 (패널 권장 결함 해소)**: 독립 검토 패널은 기존 U7이 서로 다르게 변경·설계되는 **두 일**을 겸한다고 판정했다 — `{ServiceManager, AutoUpdater, Uninstaller}` = 바이너리 생명주기(설치·업데이트·제거) vs `{OperatorCli, ControlPlane, RunStateController}` = 실행 중 데몬 제어. 이를 **U7a**(배포/업데이트)와 **U7b**(운영 제어)로 분리했다. 각 게이트가 작고 응집도 높은 단위로 독립한다.

4. **`SingleInstanceLock` 이동 (U2 → U8)**: 단일 인스턴스 락은 조립 루트 `WatcherDaemon`이 기동 시 `acquire()`하는 컴포넌트다(`services.md` `WatcherDaemon` 2단계). 논리적으로 U2(변경 감지)가 아니라 데몬 생명주기(조립 루트)에 속하므로 U8로 이동했다 — 응집도 상향이며 추가 비용은 없다("무료").

**결과**: U0 + U1 + U2 + U3 + U4 + U5 + U6 + U7a + U7b + U8 = **10단위**, 컴포넌트 **30개** 배정.

### 1.4 확정 10단위 개요표

| Unit ID | 이름 | 크레이트 | 유형 | 컴포넌트 수 | 대표 에픽 |
|---|---|---|---|---|---|
| **U0** | Foundation | `foundation` | lib | 2 | (교차 관심사) |
| **U1** | Deterministic Content Core | `content-core` | lib | 5 | E7 (결정적 코어 속성) |
| **U2** | Change Detection & Trigger | `change-detect` | lib | 3 | E1 (모니터링·트리거링) |
| **U3** | Upload Protocol Client | `upload-client` | lib | 1 | E2 (업로드 프로토콜) |
| **U4** | Resilience & Retry | `sync-state` | lib | 2 | E3 (오프라인·백프레셔·재시도) |
| **U5** | Auth & Consent | `auth-consent` | lib | 3 | E4 (인증·동의) |
| **U6** | Observability | `observability` | lib | 5 | E5 (히스토리·관측성) |
| **U7a** | Deploy & Update | `lifecycle-deploy` | lib | 3 | E6 (수명주기·업데이트) |
| **U7b** | Operations Control | `ops-control` | lib | 3 | E6 (수명주기·업데이트) |
| **U8** | Orchestration | `watcher-bin` | binary | 3 | (접착/조립) |

> **에픽 번호 정합 주석**: 대표 에픽 번호는 권위 있는 `stories.md`의 에픽 정의(E1=모니터링·트리거링 US-E1-xx, E2=업로드 프로토콜 US-E2-xx, E3=오프라인·백프레셔·재시도 US-E3-xx, E4=인증·동의 US-E4-xx, E5=히스토리·관측성 US-E5-xx, E6=수명주기·업데이트 US-E6-xx, E7=결정적 코어 속성 검증·회복탄력성 US-E7-xx)를 따른다. `unit-of-work-plan.md` §1.2 표의 괄호 라벨(예: "E2(감지·트리거)", "E1(업로드)")은 주제 서술로서 E-번호가 `stories.md`와 어긋나 있어, 본 문서는 `stories.md`/`execution-plan.md` §4의 번호 체계로 통일한다. 스토리별 정밀 매핑(44/44)은 `unit-of-work-story-map.md`(3/3)에서 확정한다.

---

## 2. 10개 단위 정의

각 단위의 소속 컴포넌트·책임·대표 에픽은 `components.md`/`component-methods.md`에서 확인된 계약에 근거한다.

### U0 — Foundation

- **크레이트**: `foundation` (lib)
- **소속 컴포넌트**: `CoreTypes`, `ConfigProvider`
- **대표 에픽**: 교차 관심사(대표 에픽 없음) — 직렬화 라운드트립 스토리 US-E7-06을 담고, 전 에픽을 관통
- **얇은 단위 근거**: 임계경로 루트이자 **싱크 계약 역전**을 보존하는 단위이므로 원칙적으로 얇다. 여기에 다른 로직을 병합하면 모든 단위가 참조하는 루트에 관심사가 섞여 비순환 강제 효과가 훼손된다.

**책임**: 모든 단위가 공유하는 값 타입·오류 taxonomy·동기화 상태 타입·무손실 직렬화 코덱을 한 곳(`CoreTypes`)에 정의한다. 매니페스트 값 타입(`ManifestEntry`/`Manifest`), 변경 집합(`ChangeSet`), 전송 결과·오류 taxonomy(`TransferResult`/`ErrorClass`/`ClassifiedError`, Q4=A), 동기화 상태 머신 타입(`SyncState`), 공유 프리미티브(`RelativePath`/`Sha256Digest`/`ManifestDigest`/`Timestamp`)를 소유한다. `encode`/`decode` 코덱은 모든 타입에 대해 `decode(encode(v)) == v` 라운드트립을 보장한다(NFR-13).

`components.md` §0 수정1에 따라 **관측 싱크 계약 트레이트**(`Logger`/`StatusSink`/`HistorySink`/`CriticalEventSink`)와 Q9 2축 상태 어휘(`OperationalState`/`ActiveCondition`/`LivenessSignal`/`StatusSnapshot` + `update_probe()`/`health_check()` 시그니처)도 U0가 소유한다. 이로써 U1~U5가 로그·상태 push를 위해 U6(구현)을 역참조하지 않고 U0의 트레이트만 의존하게 되어, 빌드 순서 역전이 사라진다.

`ConfigProvider`는 nginx 방식의 **단일 JSON 설정 파일**(Q3=X)을 로드·검증·리로드하고 타입드 `WatcherConfig`로 노출한다. 토큰의 1차 저장 경로는 config 평문 필드(+env 폴백, §13 확정)이며, 리로드 시 관찰자 구독 패턴으로 팬아웃한다(토큰 변경 → U5 `ConsentGate`, 로그레벨 → U6 `StructuredLogger`) — 구독자가 등록하는 구조라 U0는 U5/U6 타입을 참조하지 않는다.

---

### U1 — Deterministic Content Core

- **크레이트**: `content-core` (lib)
- **소속 컴포넌트**: `ContentAddressing`, `VaultScanner`, `ManifestBuilder`, `ManifestDiffer`, `SafetyLimitsValidator`
- **대표 에픽**: E7(결정적 코어 속성 검증) — 또한 E1(매니페스트 재계산·diff)·E2(스냅샷·프리플라이트) 담당

**책임**: 볼트 콘텐츠를 결정적으로 지문화하고 매니페스트를 재계산·비교하는 **순수 코어**다. `ContentAddressing`은 표준 SHA-256으로 파일별 `raw_sha256`(= `sha256sum`)을 스트리밍 계산하고 정렬된 엔트리에 대한 `manifest_digest`를 산출한다 — FQ-1=A에 따라 okc-core 콘텐츠 주소 스킴은 재현하지 않으며, 권위 있는 `vault_content_id`는 서버 소유다. `VaultScanner`는 설정된 볼트 루트를 일관 시점으로 열거하고 파일별 스트리밍 리더를 제공한다(U1 내 유일한 파일 읽기 관심사). `ManifestBuilder`는 스캔 결과와 해시를 결합해 사이클마다 `Manifest`를 재계산한다.

`ManifestDiffer`는 마지막 커밋 매니페스트와 현재 매니페스트를 비교해 정확한 `ChangeSet`(누락 0·오탐 0, NFR-12)을 산출하며, 동일하면 빈 집합을 반환한다(no-op). `SafetyLimitsValidator`는 전송 전 세 한도(총량 ≤20 GiB, 파일당 ≤2 GiB, 파일 수 ≤100k)를 경계-정확·단조로 검사한다(NFR-14). 이 단위는 `VaultScanner`를 통한 파일 읽기를 제외하면 지속/네트워크 I/O가 없고, 마지막 커밋 매니페스트의 저장은 U4가 소유한다(호출 시 인자 전달). 무상태 결정적 로직은 proptest 속성(NFR-08/12/13/14)의 대상이다.

---

### U2 — Change Detection & Trigger

- **크레이트**: `change-detect` (lib)
- **소속 컴포넌트**: `FilesystemWatcher`, `ReconciliationScheduler`, `VaultAvailabilityGuard`
- **대표 에픽**: E1(모니터링 & 트리거링)

**책임**: 볼트 변경을 감지해 동기화 사이클 트리거를 발행하고, 이벤트 손실 백스톱과 파괴적 커밋 가드를 제공한다. `FilesystemWatcher`는 FSEvents/inotify/ReadDirectoryChangesW 어댑터로 생성·수정·삭제·이름변경을 구독하고, 디바운스 정적 구간(T_debounce) 후 편집 버스트당 정확히 1회만 트리거한다(NFR-05 이식성). `ReconciliationScheduler`는 시작 시 1회 전체 재조정과 T_recon 주기 재조정을 트리거해 런타임 이벤트 손실의 백스톱을 제공하고, 미관측 변경의 검출 지연을 ≤ T_recon으로 상한한다(NFR-03).

`VaultAvailabilityGuard`는 빈 볼트/루트 부재/언마운트/접근 불가를 vault-unavailable로 분류하고, 0-파일 매니페스트가 "전부 삭제"로 diff되어 서버 콘텐츠를 비가역 삭제하는 것을 보류한다(US-E1-06). 세 컴포넌트는 §0 노트2에 따라 **의도적으로** `StatusService`/`StructuredLogger`를 의존하지 않는다(U2 순수성) — 판정만 반환하고, 실제 로그·상태 push는 U8 `SyncCycleCoordinator`가 수행한다. (기존 U7의 `RunStateController`가 pause/resume을 호출하며, 이는 U7b 소관이다.)

---

### U3 — Upload Protocol Client

- **크레이트**: `upload-client` (lib)
- **소속 컴포넌트**: `UploadProtocolDriver`
- **대표 에픽**: E2(업로드 프로토콜) — 또한 E7(프로토콜 PBT US-E7-02/03)
- **얇은 단위 근거**: 컴포넌트는 1개지만 **의존 합류점의 복잡한 프로토콜 상태머신**을 담기에 원칙적으로 얇다. have/want → 재개 청크 전송 → 전송 시 재검증 → 커밋 → 멱등의 다단계 상태를 하나의 응집 단위로 묶고, U1·U4·U5가 모이는 합류점이므로 다른 단위와 병합하면 프로토콜 상태머신의 PBT 격리가 희석된다.

**책임**: 업로드 프로토콜 3~6단계를 **한 트리거당 끝까지 도는 단일 직렬 사이클**(Q2=B)로 수행한다. HTTP를 직접 소유하지 않고 U5 `AuthTransport`를 통해서만 전송하며, 재개 오프셋은 U4 `SyncStateStore`를 사용하고, 진행률·over-limit은 U6 `StatusService`로 push한다. have/want 협상(Q6=C, `raw_sha256` 기준)으로 서버가 결여한 blob만 전송하고(FR-07, NFR-09), config 임계값 S 초과 blob 또는 실패/타임아웃 blob은 재개 가능 청크로 전송한다(FR-08, NFR-10).

전송 직전 각 blob 바이트를 **스스로 재-읽기**하여 순수 `ContentAddressing`으로 재-해시하고 매니페스트 해시와 일치할 때만 전송한다(Q8=A/§0 노트1); 불일치 시 그 커밋을 중단하고 다음 사이클이 재스냅샷한다. 모든 want blob이 존재하면 권위 있는 경로→해시 맵 + `manifest_digest`를 참조하는 커밋을 전송하며(FR-06/09), 다이제스트가 마지막 커밋과 같으면 no-op으로 조기 종료한다(FR-10). 매 사이클 시작 시 `SafetyLimitsValidator`(U1)를 재사용해 런타임 한도를 재검사한다(US-E2-08). 재시도 스케줄링과 동의 게이트는 소유하지 않는다(U8/U4·U5 위임).

---

### U4 — Resilience & Retry

- **크레이트**: `sync-state` (lib)
- **소속 컴포넌트**: `SyncStateStore`, `RetryBackoffController`
- **대표 에픽**: E3(오프라인·백프레셔·재시도) — 또한 E7(상태머신·크래시 PBT US-E7-08/09/11)
- **얇은 단위 근거**: 단일 **"장애 복구(failure recovery)"** 테마로 응집된 단위라 원칙적으로 얇다. 무손실 지속(`SyncStateStore`)과 백오프·오프라인 판정(`RetryBackoffController`)은 서로 다른 PBT 프로파일(크래시 주입 상태머신 vs 백오프 상태머신)을 가지므로, 다른 단위로 흩으면 회복탄력성 테스트 격리를 잃는다.

**책임**: **최신 상태 대체(latest-state-replacement) 모델**(FQ-2=A)의 지속 상태를 소유한다 — 이벤트별 지속 큐가 아니라 마지막 커밋 매니페스트 1개 + dirty 표시 + 진행 중 업로드 재개 오프셋. `SyncStateStore`는 권위 있는 경로→해시 맵 + 다이제스트(Q6=C)를 지속하고 사이클 시작 시 diff 기준으로 읽어 주며, 모든 쓰기를 원자적(temp+rename 또는 WAL)으로 수행해 쓰기 도중 크래시가 상태를 손상시키지 않게 하고(NFR-03), 로드 시 부분 기록을 마지막 정상 상태로 롤백한다. 직렬화는 U0 `CoreTypes` 무손실 코덱을 사용한다(NFR-13).

`RetryBackoffController`는 U0 오류 taxonomy로 전송 결과를 분류해(Transient/Offline/Backpressure/AuthFailed/Permanent) 실패 시 지수 백오프 + 상한으로 다음 재시도 시각을 계산하고, 오프라인을 별도 서버 왕복 없이 판정한다(US-E3-04). 401(AuthFailed)은 재시도에서 제외해 U5 인증 흐름에 위임하고, 연속 실패 횟수를 추적해 FR-19 케이스 2 에스컬레이션 신호를 낸다(실제 알림은 U6 소관).

---

### U5 — Auth & Consent

- **크레이트**: `auth-consent` (lib)
- **소속 컴포넌트**: `AuthTransport`, `CredentialProvider`, `ConsentGate`
- **대표 에픽**: E4(인증 & 동의) — P2(프라이버시 민감 사용자) 주담당 스토리 소재지

**책임**: Watcher에서 서버로 나가는 유일한 HTTP 경로와 동의 게이트를 소유한다. `AuthTransport`는 **얇은 전송 소유자**(Q4=A)로 TLS만 사용하고(NFR-06), 매 요청 토큰을 첨부하며(FR-13), 응답을 U0 오류 taxonomy로 분류하고(401→AuthFailed, 5xx→ServerError, PROJECT_BUSY/queue-full→Backpressure, 타임아웃→Timeout, 연결실패→Network) config 타임아웃을 적용한다(NFR-04). 프로토콜 의미(U3)·재시도(U4)는 소유하지 않으며 2xx 본문은 상위(U3)에 반환한다.

`CredentialProvider`는 토큰 소스를 결정한다 — 1차 경로는 config JSON `token` 필드(또는 env 폴백, keyring-less 평문, §13 확정), 선택적 강화로 OS secure-store(US-E4-02); secure-store 불가(헤드리스/데몬) 시 config/env로 안전 폴백한다. `ConsentGate`는 RISK-01 최초 실행 고지 확인(US-E4-04) + 상시 동의 부여/참조 저장(US-E4-05) + 조회/철회(US-E4-06)를 소유하며, 유효 동의가 없으면 U8 `SyncCycleCoordinator`의 업로드/커밋 단계를 차단한다(감시·상태는 유지). 철회는 전진 방향(forward-only)이며, DEP-01/02/05/06/07의 서버측 동작은 [blocked-on-server] 목 계약으로 검증한다.

---

### U6 — Observability

- **크레이트**: `observability` (lib)
- **소속 컴포넌트**: `StructuredLogger`, `StatusService`, `UploadHistoryStore`, `CriticalErrorNotifier`, `TrayIndicator`(선택/optional)
- **대표 에픽**: E5(히스토리 & 관측성)

**책임**: 데몬의 활동·상태·이력·중대 오류를 **push-only**로 표면화한다. U6 컴포넌트는 U0가 소유한 싱크 트레이트(`Logger`/`StatusSink`/`HistorySink`/`CriticalEventSink`)의 **구체 구현**만 제공하며, 조립 루트가 하위 단위에 하향 주입한다 — 따라서 U6는 U1~U5를 역참조하지 않는다. `StructuredLogger`는 JSON-line 구조화 로그의 주 표면이고(`cycle_id` 상관관계), `StatusService`는 2축 상태 모델(Q9=B)의 단일 집계 지점으로 `snapshot()`/`health_check()`(운영 헬스)/`update_probe()`(순수 liveness)를 제공한다.

`update_probe()`는 운영 조건집합과 **분리**되어(§0 노트3), 일시적 AuthFailed/OverLimit 때문에 좋은 새 버전이 오판 롤백되는 루프를 방지한다 — U7a `AutoUpdater`가 이 메서드만 소비한다. `UploadHistoryStore`는 append-only 로컬 히스토리(수정/삭제 불가, 무손실 라운드트립)를 CLI 질의로 노출한다. `CriticalErrorNotifier`는 닫힌 4조건(401, N회 연속 실패, 프리플라이트 초과, 자동 업데이트 롤백)만 능동 표면화한다(FR-19). `TrayIndicator`는 선택 항목이다(§4 참조).

---

### U7a — Deploy & Update

- **크레이트**: `lifecycle-deploy` (lib)
- **소속 컴포넌트**: `ServiceManager`, `AutoUpdater`, `Uninstaller`
- **대표 에픽**: E6(수명주기 & 업데이트) — 배포/업데이트/제거 슬라이스(US-E6-01/02/04)
- **얇은 단위 근거**: 기존 U7 분할의 **바이너리 생명주기** 절반이다. 설치·자동시작·자동 업데이트·롤백·제거는 하나의 응집된 "배포 아티팩트 관리" 관심사이며, 실행 중 데몬 제어(U7b)와는 변경·설계 주기가 다르다. 따라서 별도의 작고 응집된 단위로 유지한다.

**책임**: Watcher를 OS 네이티브 백그라운드 서비스로 설치·운영·업데이트·제거한다. `ServiceManager`는 launchd/systemd/Windows Service에 대해 `install/uninstall/start/stop/restart/status`를 3-OS 동일 계약으로 제공하고 부팅/로그인 자동시작을 구성한다(FR-21, US-E6-01). `AutoUpdater`는 채널 config에 따라 업데이트를 스테이징 후 `ServiceManager.restart()`로 재기동하고, bounded time 내 `StatusService.update_probe()` 통과 여부로 헬스 게이트를 판정해 실패/타임아웃 시 직전 정상 버전으로 자동 롤백한다(FR-20/NFR-16); 롤백 시 `CriticalErrorNotifier.report_update_rollback`(FR-19 케이스 4)을 호출한다.

`Uninstaller`는 서비스 등록 해제 + 로컬 평문 산출물(히스토리·SyncState·로그) 경로 기반 삭제 + 토큰 제거를 idempotent하게 수행하며, 데몬이 꺼진 상태에서도 동작하고 볼트 원본은 절대 미접촉한다(US-E6-04, RISK-01). 산출물 정리 경로는 U0 `ConfigProvider`로 해소하고, 토큰의 secure-store 삭제는 U5 `CredentialProvider`에 위임한다.

---

### U7b — Operations Control

- **크레이트**: `ops-control` (lib)
- **소속 컴포넌트**: `OperatorCli`, `ControlPlane`, `RunStateController`
- **대표 에픽**: E6(수명주기 & 업데이트) — 운영 제어 슬라이스(US-E6-03); 또한 E5의 CLI 표면(US-E5-02/03) 서비스
- **얇은 단위 근거**: 기존 U7 분할의 **실행 중 데몬 제어** 절반이다. 운영자 CLI(사실상의 운영자 API) + 로컬 IPC + run-state 보유는 하나의 응집된 "런타임 제어" 관심사이며, 배포 생명주기(U7a)와 분리해 각 게이트를 작게 유지한다.

**책임**: GUI 없는 데몬의 운영자 제어 표면을 소유한다. `OperatorCli`는 서브커맨드(`status`/`health`/`pause`/`resume`/`sync-now`/`stop`/`history`/`consent`/`reload`/`install`/`uninstall`)를 파싱·디스패치하고, `health` 결과를 프로세스 종료코드로 매핑한다(0 healthy / 비정상 non-zero, US-E5-02 계약); 실행 중 데몬 대상 명령은 `ControlPlane` 클라이언트로 IPC 전송하고, 서비스 수명주기 명령은 U7a `ServiceManager`/`Uninstaller`를 직접 호출한다.

`ControlPlane`은 로컬 IPC 서버/클라이언트(Q5=A)로 Unix 도메인 소켓(mac/Linux) + 명명 파이프(Windows)를 소유 사용자 권한으로 바인딩하고, 버전드 요청/응답 프로토콜로 status/health → `StatusService`, history → `UploadHistoryStore`, pause/resume/sync-now/stop → `RunStateController`, consent → U5 `ConsentGate`, reload → U0 `ConfigProvider`로 디스패치한다. `RunStateController`는 데몬의 권위 있는 run-state 보유자(running|paused, sync_requested, stop_requested)로, paused를 재시작 후에도 지속하고 U8 오케스트레이션이 이 상태를 **읽어** 동작한다(역참조 회피, US-E6-03).

---

### U8 — Orchestration

- **크레이트**: `watcher-bin` (binary)
- **소속 컴포넌트**: `WatcherDaemon`, `SyncCycleCoordinator`, `SingleInstanceLock`
- **대표 에픽**: 접착/조립(대표 에픽 없음) — 전 에픽 E1~E7을 관통

**책임**: 단일 배포 바이너리의 **조립 루트(Composition Root)** 다(§4 상세). `WatcherDaemon`은 config 로드, `SingleInstanceLock::acquire()`(FR-23 — 이미 실행 중이면 활성 PID/소유자 보고 후 비정상 종료, stale이면 회수 후 진행), 지속 상태 복구, 전 단위 컴포넌트 생성·주입(DI), 실행/graceful 종료, 상태 집계를 담당한다. 상태·로그 싱크는 파운데이션(U0) 추상을 통해 하향 주입되어 push-only가 성립하고 U6 역참조가 없다.

`SyncCycleCoordinator`는 트리거당 1회 **단일 직렬 사이클**(Q2=B)을 실행한다. 트리거 소스(`FilesystemWatcher` 디바운스, `ReconciliationScheduler` 주기, `ControlPlane` sync-now)를 배선받아 사이클을 직렬화하고, U2 검사기의 판정(트리거 로그·vault-unavailable, §0 노트2)과 `ConsentGate` 업로드 게이트를 받아 U6 싱크로 push하는 실제 표면화를 소유한다. `SingleInstanceLock`은 U2가 아니라 이 조립 루트에서 획득되므로 U8에 배정한다(UQ-1=A 이동 결정).

---

## 3. 그린필드 코드 조직 전략 (UQ-2=B)

그린필드 프로젝트이므로 코드 조직 전략을 명시한다. 결정은 **UQ-2=B — Cargo 워크스페이스 + 단위별 라이브러리 크레이트 + 얇은 바이너리 크레이트**다. 단위와 크레이트가 1:1로 대응하여 Units Generation 산출물과 코드 구조가 정확히 일치한다.

### 3.1 워크스페이스 디렉토리 트리

```text
okc-hooks/                          # 워크스페이스 루트
├── Cargo.toml                      # [workspace] members = 아래 10개 크레이트
├── Cargo.lock
├── crates/
│   ├── foundation/                 # U0  (lib)  — CoreTypes, ConfigProvider
│   │   ├── Cargo.toml              #   dependencies: (외부 크레이트만; 내부 의존 0 = DAG 루트)
│   │   └── src/lib.rs
│   ├── content-core/               # U1  (lib)  — ContentAddressing, VaultScanner,
│   │   ├── Cargo.toml              #     ManifestBuilder, ManifestDiffer, SafetyLimitsValidator
│   │   └── src/lib.rs              #   dependencies: foundation
│   ├── change-detect/              # U2  (lib)  — FilesystemWatcher, ReconciliationScheduler,
│   │   ├── Cargo.toml              #     VaultAvailabilityGuard
│   │   └── src/lib.rs              #   dependencies: foundation
│   ├── upload-client/              # U3  (lib)  — UploadProtocolDriver
│   │   ├── Cargo.toml              #   dependencies: foundation, content-core, sync-state, auth-consent
│   │   └── src/lib.rs
│   ├── sync-state/                 # U4  (lib)  — SyncStateStore, RetryBackoffController
│   │   ├── Cargo.toml              #   dependencies: foundation
│   │   └── src/lib.rs
│   ├── auth-consent/               # U5  (lib)  — AuthTransport, CredentialProvider, ConsentGate
│   │   ├── Cargo.toml              #   dependencies: foundation
│   │   └── src/lib.rs
│   ├── observability/              # U6  (lib)  — StructuredLogger, StatusService,
│   │   ├── Cargo.toml              #     UploadHistoryStore, CriticalErrorNotifier, TrayIndicator(선택)
│   │   └── src/lib.rs              #   dependencies: foundation
│   ├── lifecycle-deploy/           # U7a (lib)  — ServiceManager, AutoUpdater, Uninstaller
│   │   ├── Cargo.toml              #   dependencies: foundation, observability, auth-consent
│   │   └── src/lib.rs
│   ├── ops-control/                # U7b (lib)  — OperatorCli, ControlPlane, RunStateController
│   │   ├── Cargo.toml              #   dependencies: foundation, observability, auth-consent,
│   │   └── src/lib.rs              #     lifecycle-deploy
│   └── watcher-bin/                # U8  (binary) — WatcherDaemon, SyncCycleCoordinator,
│       ├── Cargo.toml              #     SingleInstanceLock
│       └── src/main.rs             #   dependencies: 전 lib 크레이트 (foundation..ops-control)
└── ...
```

> 위 크레이트별 `dependencies` 주석은 §2 컴포넌트 계약에서 확인된 의존 방향의 요약이며, 정밀한 의존성 매트릭스·통신 패턴·웨이브 배치는 `unit-of-work-dependency.md`(2/3)에서 확정한다.

### 3.2 비순환 그래프의 컴파일타임 강제

각 lib 크레이트의 `Cargo.toml` `[dependencies]` 섹션이 **의존성 방향을 컴파일 타임에 강제**한다. Cargo는 크레이트 간 순환 의존을 허용하지 않으므로, 단위 의존 그래프가 곧 크레이트 그래프가 되어 순환이 원천 봉쇄된다. 예를 들어 `observability`(U6)가 `content-core`(U1)를 import하려면 `observability/Cargo.toml`에 명시적 의존을 추가해야 하고, 그러면 역참조가 코드 리뷰에 눈에 보이는 변경으로 드러난다 — 이 프로젝트의 최대 리스크인 "의존성 순환·역참조"를 컴파일러가 막는다.

특히 `components.md` §0 수정1의 [Foundation-contract] 배치가 이 전략과 정합한다: 로그·상태 싱크 계약 트레이트가 `foundation`(U0)에 있으므로, `content-core`/`change-detect`/`sync-state`/`auth-consent`/`upload-client`는 로그·상태 push를 위해 `observability`(U6)에 의존하지 않고 **`foundation`에만** 의존한다. 즉 이 5개 엣지는 U6 런타임 의존이 아니라 U0 의존이며, `observability` 크레이트는 push-only(U1~U5 크레이트를 `[dependencies]`에 넣지 않음)로 유지된다.

### 3.3 단일 실행파일 산출 (RESILIENCY-01)

`watcher-bin`(U8) 바이너리 크레이트가 `[dependencies]`로 전 lib 크레이트(`foundation` .. `ops-control`)를 링크하여 **단일 실행파일**을 산출한다. 워크스페이스로 소스를 10개 크레이트로 나누어도 배포 산출물은 하나의 바이너리이므로, RESILIENCY-01(단일 배포 컴포넌트)이 충족된다. `watcher-bin`은 조립 루트 로직(§4) 외에는 로직을 담지 않는 **얇은 바이너리**다.

### 3.4 크레이트별 단위테스트·PBT 격리

각 lib 크레이트는 자기 단위의 단위테스트와 속성 기반 테스트(PBT, 확장 ON)를 자체 `#[cfg(test)]`/`tests/`로 격리 소유한다. 결정적 코어(`content-core`)의 stateless 속성(NFR-08/12/13/14), `sync-state`의 상태머신·크래시 복구 모델 속성(US-E7-08/09), `upload-client`의 프로토콜 속성(NFR-09/10)이 서로 다른 크레이트에 격리되어, 크레이트별 증분 컴파일·독립 테스트 실행이 가능하다. UQ-1=A가 U7을 분할하고 얇은 단위들을 병합하지 않은 이유(§2)가 이 테스트 격리와 웨이브 병렬성을 보존하는 데 있다.

---

## 4. TrayIndicator 선택 및 U8 조립 루트

### 4.1 TrayIndicator는 선택(optional)

`TrayIndicator`(U6, `observability` 크레이트)는 데스크톱 세션 편의용 **선택 컴포넌트**다(FR-18/US-E5-05). 헤드리스 데몬(nginx 방식)에서는 부재할 수 있으며, **어떤 스토리도 트레이 존재에 의존하지 않는다**. `components.md` §0 수정2에 따라 `CriticalErrorNotifier`(U6)의 `TrayIndicator` 의존은 **nullable no-op 싱크**로 정의되어, 조립 루트가 트레이 부재 시 no-op 싱크를 주입한다. 따라서 중대 오류의 표면화(로그 + 헬스 + CLI status, US-E5-04)는 트레이 부재와 무관하게 항상 성립한다. `TrayIndicator::start()`는 헤드리스/비활성 시 `Ok(None)`을 반환하고 모든 메서드가 비치명적으로 계약된다. 트레이의 최종 채택/제거(confirm-or-drop)는 U6 Functional Design에서 확정한다.

### 4.2 U8은 조립 루트(하향 주입)

U8(`watcher-bin`)은 전체 시스템의 **조립 루트**다. Foundation(U0의 `CoreTypes`/`ConfigProvider`)은 U8이 소유·구성하여 아래 단위로 **하향 주입(downward injection)** 한다 — 하위 단위는 U0가 정의한 값 타입·싱크 계약 트레이트에만 의존하고 상위 단위를 역참조하지 않으므로 비순환이 유지된다(의존성 역전). 구체적으로 `WatcherDaemon`은 U6 싱크 구현(`StructuredLogger`/`StatusService`/`UploadHistoryStore`/`CriticalErrorNotifier` + 선택적 `TrayIndicator` no-op)을 생성해 U0 계약 타입으로 하위 단위(U3/U5 등)에 주입한다 — 이로써 5개 U3/U5→U6 엣지가 파운데이션-계약 의존으로 성립해 빌드 순서 역전이 해소된다.

`SingleInstanceLock`은 이 조립 루트에서 `acquire()`되므로 U8에 속한다(UQ-1=A 이동 결정, `services.md` `WatcherDaemon` 2단계). `SyncCycleCoordinator`는 U8 내부에서 U2 검사기·U6 싱크·U3 드라이버·U5 게이트를 모두 주입받아 트리거당 1회 직렬 사이클을 구동하며, U2 순수성(§0 노트2)이 만든 push 공백을 코디네이터가 메운다.

---

## 5. 확장 컴플라이언스 (Units Generation 단계)

| 확장 | 활성 | 이 단계 적용 | 판정 근거 |
|---|---|---|---|
| **Resiliency Baseline** | ON | 부분 적용 | RESILIENCY-01(단일 배포 컴포넌트)이 §3.3의 단일 바이너리 산출(`watcher-bin`이 전 크레이트 링크)로 준수됨. 나머지 RESILIENCY 규칙(RPO/RTO, 크래시·오프라인 설계 등)은 Construction 단계 소관 → 이 단계 대체로 N/A. |
| **Property-Based Testing** | ON (Full) | N/A | PBT 속성 명세·제너레이터·CI는 Functional Design/Code Generation/Build-and-Test 소관. 단, §3.4의 크레이트별 PBT 격리가 후속 PBT 실행의 구조적 기반을 제공. |
| **Security Baseline** | OFF | N/A | 규칙 미로딩·미강제. RISK-01(config 평문 토큰 포함 로컬 평문 산출물)/RISK-02는 문서화된 수용 위험. |
