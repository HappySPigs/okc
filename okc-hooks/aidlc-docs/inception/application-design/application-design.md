## 애플리케이션 설계 (통합본) — okc-hooks "Watcher"

**단계**: INCEPTION → Application Design
**작성일**: 2026-09-08
**역할**: Application Architect (애플리케이션 설계자)
**근거**: 승인된 `requirements.md` / `stories.md` / `personas.md` / `execution-plan.md` + `application-design-plan.md`의 확정 답변(Q1..Q9, FQ-1/2/3) + 병렬 설계 4건 + 통합 1건 + 완전성 크리틱 1건(workflow, 6/6 에이전트, 오류 0, 669,586 토큰).

> 이 문서는 아래 4개 세부 산출물의 **통합 개요**다. 상세는 각 문서를 참조한다.
> - `components.md` — 30개 컴포넌트 목적/책임/공개 인터페이스
> - `component-methods.md` — 메서드 시그니처 + 입출력 타입 + 오류 taxonomy
> - `services.md` — 오케스트레이션 2개(`WatcherDaemon`, `SyncCycleCoordinator`)
> - `component-dependency.md` — 의존성 매트릭스 + 통신 패턴 + 데이터 흐름 + 네이밍 레지스트리 + 검증

---

## 1. 아키텍처 개요

Watcher는 Obsidian 볼트를 감시해 변경 시 **원시(raw) 볼트 바이트**를 미래 OKC 웹서비스로 업로드하는, **GUI 없는 크로스플랫폼 백그라운드 데몬(nginx 방식)**이다. 단일 배포 바이너리 안에 논리 모듈 U1..U7 + 공유 파운데이션 + 오케스트레이션이 들어간다. 결정적 코어는 Rust 가정(NFR-17).

확정 답변으로 아키텍처가 **원래보다 단순**해졌다:
- **FQ-1=A**: okc-core 코드 의존성 **0**. Watcher는 표준 SHA-256(= sha256sum)으로 파일 지문 + 경로 매니페스트만 계산하고 **원시 바이트를 서버로 업로드**한다. 권위 있는 `vault_content_id` 계산은 **전부 서버**. Watcher는 로컬 멱등/no-op 판정용 `manifest_digest`만 유지.
- **FQ-2=A**: **최신 상태 대체(latest-state-replacement)** 모델. 이벤트별 지속 큐 없음. 지속 상태 = 마지막 커밋 매니페스트 1개 + dirty 표시 + 재개 오프셋. 매 트리거마다 폴더를 재스냅샷→diff(폴더가 진실의 원천). 오프라인이면 사이클이 백오프로 재시도하고 그 사이 편집은 다음 스냅샷이 자동 포함(무손실).
- **Q2=B**: **단일 직렬 사이클** — 트리거마다 감지→업로드→커밋을 끝까지 1회. 별도 생산자/소비자 루프 없음.

## 2. 컴포넌트 지형 (30개)

| 계층/단위 | 컴포넌트 |
|---|---|
| **Foundation** (2) | CoreTypes, ConfigProvider |
| **U1 Deterministic Content Core** (5, 순수) | ContentAddressing, VaultScanner, ManifestBuilder, ManifestDiffer, SafetyLimitsValidator |
| **U4 Resilience / Sync State** (2) | SyncStateStore, RetryBackoffController |
| **U5 Auth & Consent** (3) | AuthTransport, CredentialProvider, ConsentGate |
| **U2 Change Detection & Trigger** (4) | FilesystemWatcher, ReconciliationScheduler, VaultAvailabilityGuard, SingleInstanceLock |
| **U3 Upload Protocol Client** (1) | UploadProtocolDriver |
| **U6 Observability** (5) | StructuredLogger, StatusService, UploadHistoryStore, CriticalErrorNotifier, TrayIndicator(선택) |
| **U7 Lifecycle & CLI** (6) | ServiceManager, AutoUpdater, Uninstaller, RunStateController, OperatorCli, ControlPlane |
| **Orchestration** (2) | WatcherDaemon(조립 루트), SyncCycleCoordinator(단일 직렬 사이클) |

빌드 순서: **Foundation → U1 → U4 → U5 → U2 → U3 → U6 → U7 → Orchestration**.

## 3. 핵심 설계 결정 (답변 매핑)

| 결정 | 답변 | 설계 반영 |
|---|---|---|
| okc-core 지문 재현 | FQ-1=A | U1 축소 — 표준 SHA-256 + 매니페스트 + diff + SafetyLimit만. `vault_content_id`는 서버. |
| 지속 상태 모델 | FQ-2=A | `SyncStateStore` = 마지막커밋 매니페스트 + dirty + 재개 오프셋(이벤트 큐 없음). |
| 감지·업로드 구조 | Q2=B | `SyncCycleCoordinator` 단일 직렬 사이클. `UploadDrainWorker` 제거. |
| config 로더 | Q3=X | 파운데이션 `ConfigProvider`, 단일 JSON(nginx식), 토큰 평문 필드. |
| HTTP/TLS 소유자 | Q4=A | `AuthTransport`(U5)가 TLS+토큰+응답분류 단독 소유; 오류 taxonomy는 `CoreTypes`. |
| CLI→데몬 통로 | Q5=A | `ControlPlane` 로컬 IPC(UDS + 명명 파이프), 소유자 권한 제한. |
| have/want 키 | Q6=C | 전송은 raw_sha256 중복제거, 커밋은 권위 있는 경로→해시 맵 + 매니페스트 다이제스트. |
| 전송 중 변경 처리 | Q8=A | 전송 시 재-읽기·재-해시 재검증, 불일치 시 커밋 중단·다음 사이클 재스냅샷(freeze 없음). |
| 데몬 상태 표현 | Q9=B | `StatusService` 2축 모델 + `update_probe()`(순수 liveness) / `health_check()`(운영) 분리. |
| 요구사항 불일치 처리 | FQ-3=A | 영향 요구사항/스토리를 단순 모델에 맞게 경미 수정(requirements/stories 부록). |

## 4. 데이터 흐름 요약 (단일 사이클)

트리거(U2 디바운스/재조정) → `SyncCycleCoordinator` 사이클 시작 → 가용성 가드(U2) → 재스냅샷/해시/매니페스트(U1) → diff(U1, 마지막커밋 대비) → 프리플라이트 한도(U1) → 동의 게이트(U5) → have/want·재개 전송·전송 시 재검증·커밋(U3→U5→서버 목계약) → 상태 지속(U4) + 히스토리 append(U6) + 상태/로그 push(U6). 실패/오프라인은 `RetryBackoffController`(U4) 백오프 → 전체 사이클 재시도, 그 사이 편집은 다음 재스냅샷이 자동 흡수. (상세 Mermaid/텍스트: `component-dependency.md` §3·§4)

## 5. 설계 검증 결과

- **순환 의존 없음** ✅ — 30노드 DAG, 위상 정렬 가능.
- **빌드 시퀀스 정합** ✅ — 원래 5건의 순서 역전 엣지(U3/U5→U6 관측 싱크)는 로깅·상태 계약을 `CoreTypes`에 두는 **의존성 역전(수정1)**으로 해소.
- **U1 순수성 유지** ✅ — U1은 `VaultScanner` 파일읽기 외 I/O 없음, 지속 저장은 U4.
- **파운데이션 하향 주입 + U6 push-only** ✅ — U6는 U1–U5 역참조 없음.
- **크리틱 판정**: CONCERNS(blocking 0, 미커버 요구사항 0, 순환 0, 네이밍 불일치 0, divergence OK). 비블로킹 수정 2건(순서 역전 계약화·선택적 컴포넌트 no-op 주입)은 산출물에 반영 완료.

## 6. 확장(Extension) 컴플라이언스 요약

| 확장 | 상태 | Application Design 단계 평가 |
|---|---|---|
| Security Baseline | OFF | 미로딩. RISK-01(원시 볼트 연속 업로드, 클라이언트 필터 없음)/RISK-02(재승인 부담) 수용. 토큰 평문 저장 수용. — **N/A** (opt-out) |
| Resiliency Baseline | ON (블로킹) | **정합** — 무손실(`SyncStateStore` 원자적 쓰기 + 크래시 복구), 백오프(`RetryBackoffController`), 자동 업데이트/롤백 게이팅(`AutoUpdater`↔`update_probe()`), 관측(U6 push-only)을 명시적 컴포넌트로 배치. 위반 없음. |
| Property-Based Testing | ON (블로킹) | **N/A (이 단계)** — PBT 속성/제너레이터는 Functional Design / Code Generation에서 적용. Application Design은 대상 컴포넌트(ContentAddressing 결정성, ManifestDiffer 정확성, SafetyLimitsValidator 경계, SyncStateStore 상태머신, CoreTypes 코덱 라운드트립)를 식별만 함. |

## 7. Functional Design 이월 항목 (다음 단계)

- 파운데이션 관측 싱크 계약(trait) 최종 시그니처 + wire 포맷(canonical JSON vs CBOR).
- U1: 일관 시점 스냅샷·정렬 도메인·diff 판정 기준.
- U4: 원자적 쓰기 메커니즘(temp+rename vs WAL)·fsync 순서·상태머신 전이 + PBT-06 모델.
- U5: 오류 상태코드→클래스 매핑표, 동의 고지 최종 문안·저장 포맷.
- U3: 청크 크기·청크별 무결성 스킴·재조립 라운드트립 속성(PBT-01).
- U6/U7: 로그 로테이션, 헬스/liveness 판정 임계, 플랫폼별 서비스 유닛·트레이 백엔드, IPC 프레이밍·버전 협상.

---

> **다음 AIDLC 단계**: Units Generation (시스템을 단위별 작업으로 분해). 본 설계의 U1..U7 + Foundation + Orchestration 경계와 빌드 순서를 근거로 단위를 확정한다.
