# 스토리 → 작업 단위 매핑 (Unit-of-Work Story Map) — okc-hooks "Watcher"

**단계**: INCEPTION → Units Generation (Part 2 — 생성) · 산출물 3/3
**작성일**: 2026-09-08
**역할**: Solution Architect (단위 분해 담당)
**출처(권위)**:
- `aidlc-docs/inception/user-stories/stories.md` — 44개 INVEST 스토리 + 요구사항 커버리지 매트릭스 (**스토리 ID의 유일 원천**)
- `aidlc-docs/inception/plans/execution-plan.md` §4 — U1..U7 스토리 매핑(8+5+8+8+6+5+4 = 44, 누락·중복 0 검증)
- `aidlc-docs/inception/application-design/application-design.md` — 30개 컴포넌트 지형 + FQ-1/2/3=A 결정 매핑
- `aidlc-docs/inception/plans/unit-of-work-plan.md` §1.3 — 매핑 정합(44/44 유지) + FQ-3 조정 3건
- 확정 10-단위 그룹핑(UQ-1=A / UQ-2=B / UQ-3=A) — **고정, 재분해 금지**

**검증 요약**: 스토리 44/44 전수 매핑, **미매핑 0**, 중복 소유 0, 총계 불변(=44). stories.md에 실재하는 ID만 사용(임의 생성 없음).

---

## 1. 매핑 원칙

1. **기준선 승계**: execution-plan §4의 논리 매핑(U1=8, U2=5, U3=8, U4=8, U5=6, U6=5, U7=4 = 44)을 1차 소유(primary owner)의 기준선으로 승계한다.
2. **UQ-1=A U7 분할**: 기존 U7(4개 스토리)를 성격에 맞게 두 단위로 분배한다.
   - **U7a Deploy & Update** — 바이너리 생명주기(배포·업데이트·제거): US-E6-01, US-E6-02, US-E6-04.
   - **U7b Operations Control** — 실행 중 데몬 제어(CLI·IPC·실행상태): US-E6-03.
3. **FQ-3=A 조정 3건 반영**(원문 대체가 아니라 오버레이 — 가치·수용 의도 불변, 소유/표현만 정정):
   - **US-E7-01**(해싱 결정성, NFR-08): "okc-core `raw_sha256` 재현" → **표준 `sha256sum` 일치**로 정정. **소유 U1 유지**(okc-core 코드 의존성 0, FQ-1=A).
   - **US-E3-01/US-E3-02**(지속 큐/coalesce): 이벤트별 지속 큐 → **`SyncStateStore` 상태머신**(최신-상태-대체, FQ-2=A). "newer supersedes older"는 매 사이클 폴더 재스냅샷으로 구조적 자동 성립. **소유 U4 유지**.
   - **US-E7-06**(직렬화 무손실 round-trip, NFR-13): 무손실 코덱은 **공유 타입**(`CoreTypes`)이므로 **소유를 U0 Foundation으로 통일**. execution-plan §4는 U1에 배정했었음 — **U1 인접성은 아래 표에 주석**으로 명시(경미한 차이, 44/44 총계 불변).
4. **교차/접착 단위(U0·U8)의 소유 원칙**(§7 상세):
   - **일반 원칙**: Foundation(U0)과 Orchestration(U8)은 특정 에픽 스토리 블록을 "소유"하지 않는 교차/접착 단위다. 모든 E1–E7 스토리가 이들을 관통(하향 주입 + 조립 루트 + 직렬 사이클)하지만, 1차 소유는 U1..U7b가 유지한다.
   - **단 하나의 문서화된 예외**: US-E7-06의 1차 소유는 U0이다 — 이 스토리가 검증하는 대상(무손실 직렬화 코덱)이 per-unit 관심사가 아니라 Foundation 계층의 **공유 `CoreTypes` 관심사**이기 때문. U8은 예외 없이 **스토리 소유 0**.
5. **E7 트랙 접힘**: E7(PBT/회복탄력성)은 독립 단위가 아니라 **검증 대상 코드를 소유한 단위로 접힌다**(PBT-ON per-unit 모델). 각 E7 스토리는 자신이 보호하는 컴포넌트의 소유 단위에 귀속된다.

---

## 2. 확정 10-단위 참조 (컴포넌트 30개 = 정확히 1회씩 배정)

| Unit ID | 이름 | 크레이트 | 컴포넌트 | 커버 에픽(1차) |
|---|---|---|---|---|
| **U0** | Foundation | `foundation` (lib) | CoreTypes, ConfigProvider | (교차 관심사) + E7 코덱 예외 1건 |
| **U1** | Deterministic Content Core | `content-core` (lib) | ContentAddressing, VaultScanner, ManifestBuilder, ManifestDiffer, SafetyLimitsValidator | E1(매니페스트/diff), E2(스냅샷/프리플라이트), E7(무상태 속성) |
| **U2** | Change Detection & Trigger | `change-detect` (lib) | FilesystemWatcher, ReconciliationScheduler, VaultAvailabilityGuard | E1(감지·트리거·재조정·가드·단일인스턴스) |
| **U3** | Upload Protocol Client | `upload-client` (lib) | UploadProtocolDriver | E2(업로드 프로토콜), E7(프로토콜 속성) |
| **U4** | Resilience & Retry | `sync-state` (lib) | SyncStateStore, RetryBackoffController | E3(무손실 상태·백오프·오프라인), E7(큐/크래시 속성·회복탄력성) |
| **U5** | Auth & Consent | `auth-consent` (lib) | AuthTransport, CredentialProvider, ConsentGate | E4(인증·동의) |
| **U6** | Observability | `observability` (lib) | StructuredLogger, StatusService, UploadHistoryStore, CriticalErrorNotifier, TrayIndicator(선택) | E5(관측) |
| **U7a** | Deploy & Update | `lifecycle-deploy` (lib) | ServiceManager, AutoUpdater, Uninstaller | E6(설치·자동시작·자동업데이트/롤백·제거) |
| **U7b** | Operations Control | `ops-control` (lib) | OperatorCli, ControlPlane, RunStateController | E6(실행상태 제어: pause/resume·sync-now·stop) |
| **U8** | Orchestration | `watcher-bin` (binary) | WatcherDaemon, SyncCycleCoordinator, SingleInstanceLock | (조립/접착 — 스토리 소유 0) |

> 단일 배포 바이너리(RESILIENCY-01): 모든 lib 크레이트를 `watcher-bin`이 링크. Foundation은 조립 루트(U8)가 소유해 아래로 주입(하향 주입). 로그/상태 싱크 엣지는 U6 런타임 의존이 아니라 Foundation(U0)의 트레이트 계약 의존으로 처리(Fix1 [Foundation-contract]). U6는 push-only, U1은 파일 읽기 전용(영속화는 U4). TrayIndicator는 선택(nullable no-op 싱크, FR-18).

---

## 3. 단위별 스토리 목록

### U0 — Foundation (`foundation`)
1차 소유 스토리: **1개**(문서화된 예외 — §7 참조)

| 스토리 ID | 제목(요약) | 귀속 근거 |
|---|---|---|
| US-E7-06 | 직렬화 무손실 round-trip 속성 (NFR-13) | 무손실 코덱 = 공유 `CoreTypes` 관심사(FQ-3). **U1 인접 주석**: execution-plan §4는 U1에 배정했으나 산출물에서 Foundation 소유로 통일. |

교차 관통(비소유): 모든 E1–E7 스토리가 `CoreTypes`(공유 타입·에러 taxonomy·무손실 코덱·상태 어휘·싱크 계약 트레이트)와 `ConfigProvider`(단일 JSON 설정)를 참조.

### U1 — Deterministic Content Core (`content-core`)
1차 소유 스토리: **7개** (execution-plan 8개 중 US-E7-06이 U0으로 이동 → 7)

| 스토리 ID | 제목(요약) | 비고 |
|---|---|---|
| US-E1-02 | 볼트 매니페스트 재계산 + 마지막 커밋 대비 diff | FQ-1: "okc-core 스킴" → 표준 SHA-256으로 읽음. `vault_content_id` 권위 계산은 서버, 클라이언트는 `manifest_digest` 유지. |
| US-E2-01 | 일관된 시점 스냅샷 및 확장 가능한 매니페스트 해싱 | FQ-1: 표준 SHA-256(sha256sum). |
| US-E2-02 | 업로드 전 SafetyLimit 사전검사 (Preflight) | 볼트 단위 3한도(≤20GiB/≤2GiB/≤100k). |
| US-E7-01 | 매니페스트/스냅샷 해싱 결정성 속성 (NFR-08) | **FQ-3 조정**: 표준 `sha256sum` 일치로 정정, 소유 U1 유지. |
| US-E7-05 | 재조정 diff 정확 집합 속성 (NFR-12) | ManifestDiffer 보호. |
| US-E7-07 | SafetyLimits 경계 정확·단조성 속성 (NFR-14) | SafetyLimitsValidator 보호. |
| US-E7-10 | 속성 프레임워크/기술 스택 가정 확정 (NFR-17) | FQ-1: "okc-core 프리미티브 재사용" 삭제 — 표준 SHA-256 Rust 직접 구현, proptest 채택. |

### U2 — Change Detection & Trigger (`change-detect`)
1차 소유 스토리: **5개**

| 스토리 ID | 제목(요약) | 비고 |
|---|---|---|
| US-E1-01 | 파일시스템 이벤트 감시 + 디바운스 자동 트리거 | 네이티브 감시(FSEvents/inotify/ReadDirectoryChangesW). |
| US-E1-03 | 시작 시 재조정(reconciliation) 스캔 | FQ-2: "지속 큐" 표현은 `SyncStateStore` 모델로 읽음. |
| US-E1-04 | T_recon 주기적 재조정 스캔 (이벤트 손실 백스톱) | |
| US-E1-05 | 단일 인스턴스 잠금 (큐/매니페스트 보호) | **컴포넌트-소유 주석**: 구현 컴포넌트 `SingleInstanceLock`은 UQ-1=A로 U8(Orchestration)에 재배치됨(응집도↑). 1차 **스토리** 소유는 execution-plan §4 승계로 U2 유지(설치 단위 보호 = E1 관심사). |
| US-E1-06 | 빈 볼트 / 볼트 사용 불가 안전 가드 | VaultAvailabilityGuard. |

### U3 — Upload Protocol Client (`upload-client`)
1차 소유 스토리: **8개**

| 스토리 ID | 제목(요약) | 비고 |
|---|---|---|
| US-E2-03 | 콘텐츠 주소 기반 have/want 협상 | [MVP] |
| US-E2-04 | 재개 가능한 청크 전송 (tus 스타일) | [MVP] |
| US-E2-05 | 원시 볼트 커밋 및 서버 바인딩 | [MVP] |
| US-E2-06 | 멱등적 콘텐츠 주소 업로드 (no-op) | |
| US-E2-07 | 대용량/초기 동기화 진행률 및 "재개 중" 상태 | |
| US-E2-08 | 런타임 SafetyLimit 초과 처리 | FR-19 케이스 3 원천. |
| US-E7-02 | have/want 정확성·멱등성 속성 (NFR-09) | 프로토콜 속성. |
| US-E7-03 | 재개 청크 재조립 바이트 일치 속성 (NFR-10) | 프로토콜 속성. |

### U4 — Resilience & Retry (`sync-state`)
1차 소유 스토리: **8개**

| 스토리 ID | 제목(요약) | 비고 |
|---|---|---|
| US-E3-01 | 크래시/재시작에도 살아남는 지속 상태 | **FQ-3 조정**: 큐 → `SyncStateStore`(마지막커밋+dirty+재개오프셋). 무손실 가치 불변. [MVP] |
| US-E3-02 | 대기 변경의 최신 스냅샷 합치기(구조적 최신-상태-대체) | **FQ-3 조정**: 별도 coalesce 단계 삭제 — 매 사이클 재스냅샷이 곧 최신. |
| US-E3-03 | 실패·타임아웃·서버 백프레셔 시 지수 백오프 재시도 | RetryBackoffController(401 제외). |
| US-E3-04 | 오프라인 우아한 저하와 재연결 시 배출 | FQ-2: 별도 drain 루프 없이 "다음 사이클이 곧 drain". |
| US-E7-04 | 합치기 latest-state-wins 속성 (NFR-11) | **FQ-3 조정**: 대상이 큐 인터리빙 → `SyncStateStore` 상태 전이 불변식. |
| US-E7-08 | 지속 큐 상태 기반 명령 시퀀스 속성 (PBT-06) | **FQ-3 조정**: 모델이 큐 → `SyncStateStore` 상태머신(commit_manifest/mark_dirty/set_resume_offset/persist/recover). |
| US-E7-09 | 크래시/재시작 zero-loss + T_recon 검출 지연 속성 (NFR-03) | 무손실/≤T_recon 보증 불변. |
| US-E7-11 | 회복탄력성 테스트 전략 (RESILIENCY-14) | 크래시 주입·오프라인/재개·이벤트 드롭. NFR Design/Operations로 상세 이월. |

### U5 — Auth & Consent (`auth-consent`)
1차 소유 스토리: **6개** (P2 주담당 스토리 소재지)

| 스토리 ID | 제목(요약) | 비고 |
|---|---|---|
| US-E4-01 | config 기반 API 토큰 인증 (매 요청, TLS) | [MVP] AuthTransport(TLS)+CredentialProvider. |
| US-E4-02 | OS 보안 저장소에 토큰 저장 (선택적 강화) | [optional — confirm-or-drop] |
| US-E4-03 | 인증 실패 처리 및 토큰 재입력 | FR-19 케이스 1 원천. |
| US-E4-04 | 최초 실행 RISK-01 공개 고지 확인 (informed consent 기록) | ConsentGate. P2. |
| US-E4-05 | 상시 동의 부여 및 참조 저장 | ConsentGate. |
| US-E4-06 | 동의 조회/철회 및 철회 시 로컬 업로드 차단 | ConsentGate. P2. |

### U6 — Observability (`observability`)
1차 소유 스토리: **5개**

| 스토리 ID | 제목(요약) | 비고 |
|---|---|---|
| US-E5-01 | 구조화 로그 (PRIMARY 상태 표면) | StructuredLogger. push-only. |
| US-E5-02 | CLI status 명령 + 헬스 체크 | StatusService(2축: update_probe/health_check). |
| US-E5-03 | 로컬 append-only 업로드 히스토리 (CLI 조회) | UploadHistoryStore. |
| US-E5-04 | 중대 오류 활성 표면화 (닫힌 집합) | CriticalErrorNotifier. FR-19 4케이스 표면화. |
| US-E5-05 | 선택적 시스템 트레이/메뉴바 상태 아이콘 | TrayIndicator(선택, nullable no-op). [optional — confirm-or-drop] |

### U7a — Deploy & Update (`lifecycle-deploy`)
1차 소유 스토리: **3개** (바이너리 생명주기 — 배포·업데이트·제거)

| 스토리 ID | 제목(요약) | 컴포넌트 |
|---|---|---|
| US-E6-01 | OS 서비스로 설치 및 자동시작 | ServiceManager (launchd/systemd/Windows Service). |
| US-E6-02 | 자동 업데이트 + 헬스 체크 실패 시 자동 롤백 | AutoUpdater. FR-19 케이스 4 원천. |
| US-E6-04 | 제거/폐기 정리 | Uninstaller (로컬 평문 산출물 + 토큰 삭제, 서비스 등록 해제). |

### U7b — Operations Control (`ops-control`)
1차 소유 스토리: **1개** (실행 중 데몬 제어 — CLI·IPC·실행상태)

| 스토리 ID | 제목(요약) | 컴포넌트 |
|---|---|---|
| US-E6-03 | 실행 상태 제어: 일시정지/재개 + 지금 동기화 + 중지 | RunStateController + OperatorCli + ControlPlane(로컬 IPC: UDS/명명 파이프). |

### U8 — Orchestration (`watcher-bin`)
1차 소유 스토리: **0개** (조립/접착 단위 — §7 참조)

교차 관통(비소유): `WatcherDaemon`(조립 루트/생명주기), `SyncCycleCoordinator`(트리거당 1회 직렬 사이클), `SingleInstanceLock`(설치 단위 잠금). 모든 E1–E7 스토리의 런타임 경로가 조립 루트와 직렬 사이클을 관통한다. 특히 US-E1-05의 잠금 **컴포넌트**는 여기 위치하나 **스토리** 1차 소유는 U2(§3 참조).

---

## 4. 전수 커버리지 표 (44행 — 스토리 ID → 소유 단위)

| # | 스토리 ID | 제목(요약) | 에픽 | 소유 단위 | 비고 |
|---|---|---|---|---|---|
| 1 | US-E1-01 | 파일시스템 이벤트 감시 + 디바운스 자동 트리거 | E1 | U2 | |
| 2 | US-E1-02 | 볼트 매니페스트 재계산 + diff | E1 | U1 | FQ-1(표준 SHA-256) |
| 3 | US-E1-03 | 시작 시 재조정 스캔 | E1 | U2 | FQ-2(SyncStateStore) |
| 4 | US-E1-04 | T_recon 주기적 재조정 스캔 | E1 | U2 | |
| 5 | US-E1-05 | 단일 인스턴스 잠금 | E1 | U2 | 컴포넌트는 U8, 스토리 소유 U2 |
| 6 | US-E1-06 | 빈 볼트/사용 불가 안전 가드 | E1 | U2 | |
| 7 | US-E2-01 | 일관 시점 스냅샷 + 확장 해싱 | E2 | U1 | FQ-1(표준 SHA-256) |
| 8 | US-E2-02 | 업로드 전 SafetyLimit 사전검사 | E2 | U1 | |
| 9 | US-E2-03 | 콘텐츠 주소 have/want 협상 | E2 | U3 | MVP |
| 10 | US-E2-04 | 재개 가능한 청크 전송 | E2 | U3 | MVP |
| 11 | US-E2-05 | 원시 볼트 커밋 및 서버 바인딩 | E2 | U3 | MVP |
| 12 | US-E2-06 | 멱등적 콘텐츠 주소 업로드 (no-op) | E2 | U3 | |
| 13 | US-E2-07 | 대용량/초기 동기화 진행률·"재개 중" | E2 | U3 | |
| 14 | US-E2-08 | 런타임 SafetyLimit 초과 처리 | E2 | U3 | FR-19 케이스 3 |
| 15 | US-E3-01 | 크래시/재시작 지속 상태 | E3 | U4 | **FQ-3(SyncStateStore)**, MVP |
| 16 | US-E3-02 | 최신 스냅샷 합치기(구조적 대체) | E3 | U4 | **FQ-3(latest-state-replacement)** |
| 17 | US-E3-03 | 지수 백오프 재시도 | E3 | U4 | |
| 18 | US-E3-04 | 오프라인 저하 + 재연결 배출 | E3 | U4 | |
| 19 | US-E4-01 | config 토큰 인증 (TLS) | E4 | U5 | MVP |
| 20 | US-E4-02 | OS 보안 저장소 토큰 저장 | E4 | U5 | optional |
| 21 | US-E4-03 | 인증 실패 처리 및 토큰 재입력 | E4 | U5 | FR-19 케이스 1 |
| 22 | US-E4-04 | 최초 실행 RISK-01 고지 확인 | E4 | U5 | P2 |
| 23 | US-E4-05 | 상시 동의 부여 및 참조 저장 | E4 | U5 | |
| 24 | US-E4-06 | 동의 조회/철회 + 업로드 차단 | E4 | U5 | P2 |
| 25 | US-E5-01 | 구조화 로그 (PRIMARY 표면) | E5 | U6 | |
| 26 | US-E5-02 | CLI status + 헬스 체크 | E5 | U6 | |
| 27 | US-E5-03 | append-only 업로드 히스토리 | E5 | U6 | |
| 28 | US-E5-04 | 중대 오류 활성 표면화 | E5 | U6 | FR-19 표면화 |
| 29 | US-E5-05 | 선택적 트레이/메뉴바 아이콘 | E5 | U6 | optional |
| 30 | US-E6-01 | OS 서비스 설치·자동시작 | E6 | **U7a** | U7 분할 |
| 31 | US-E6-02 | 자동 업데이트 + 자동 롤백 | E6 | **U7a** | U7 분할, FR-19 케이스 4 |
| 32 | US-E6-03 | 실행 상태 제어(pause/resume·sync-now·stop) | E6 | **U7b** | U7 분할 |
| 33 | US-E6-04 | 제거/폐기 정리 | E6 | **U7a** | U7 분할 |
| 34 | US-E7-01 | 해싱 결정성 속성 (NFR-08) | E7 | U1 | **FQ-3(표준 SHA-256)** |
| 35 | US-E7-02 | have/want 정확성·멱등성 속성 (NFR-09) | E7 | U3 | |
| 36 | US-E7-03 | 재개 청크 재조립 바이트 일치 속성 (NFR-10) | E7 | U3 | |
| 37 | US-E7-04 | 합치기 latest-state-wins 속성 (NFR-11) | E7 | U4 | **FQ-3(SyncStateStore 전이)** |
| 38 | US-E7-05 | 재조정 diff 정확 집합 속성 (NFR-12) | E7 | U1 | |
| 39 | US-E7-06 | 직렬화 무손실 round-trip 속성 (NFR-13) | E7 | **U0** | **FQ-3(Foundation 코덱)**; U1 인접 |
| 40 | US-E7-07 | SafetyLimits 경계·단조성 속성 (NFR-14) | E7 | U1 | |
| 41 | US-E7-08 | 상태 기반 명령 시퀀스 속성 (PBT-06) | E7 | U4 | **FQ-3(SyncStateStore 상태머신)** |
| 42 | US-E7-09 | zero-loss + T_recon 검출 지연 속성 (NFR-03) | E7 | U4 | |
| 43 | US-E7-10 | 속성 프레임워크/기술 스택 확정 (NFR-17) | E7 | U1 | FQ-1(okc 재사용 삭제) |
| 44 | US-E7-11 | 회복탄력성 테스트 전략 (RESILIENCY-14) | E7 | U4 | NFR Design/Operations 이월 |

**미매핑 검증**: stories.md의 44개 스토리(E1×6, E2×8, E3×4, E4×6, E5×5, E6×4, E7×11 = 44)가 위 표에 정확히 1행씩 존재 → **미매핑 0, 중복 소유 0**.

---

## 5. 총계 재확인

| 소유 단위 | 스토리 수 | 스토리 ID |
|---|---|---|
| U0 Foundation | **1** | US-E7-06 |
| U1 Deterministic Content Core | **7** | US-E1-02, US-E2-01, US-E2-02, US-E7-01, US-E7-05, US-E7-07, US-E7-10 |
| U2 Change Detection & Trigger | **5** | US-E1-01, US-E1-03, US-E1-04, US-E1-05, US-E1-06 |
| U3 Upload Protocol Client | **8** | US-E2-03, US-E2-04, US-E2-05, US-E2-06, US-E2-07, US-E2-08, US-E7-02, US-E7-03 |
| U4 Resilience & Retry | **8** | US-E3-01, US-E3-02, US-E3-03, US-E3-04, US-E7-04, US-E7-08, US-E7-09, US-E7-11 |
| U5 Auth & Consent | **6** | US-E4-01, US-E4-02, US-E4-03, US-E4-04, US-E4-05, US-E4-06 |
| U6 Observability | **5** | US-E5-01, US-E5-02, US-E5-03, US-E5-04, US-E5-05 |
| U7a Deploy & Update | **3** | US-E6-01, US-E6-02, US-E6-04 |
| U7b Operations Control | **1** | US-E6-03 |
| U8 Orchestration | **0** | (스토리 소유 없음 — 접착 단위) |
| **합계** | **44** | — |

**정합 확인 (execution-plan §4 대비)**:
- U1: 8 → **7** (US-E7-06이 U0으로 이동)
- U0: 0 → **1** (US-E7-06 수용) — "U1(7) + U0(1)" 합은 원래 U1(8)과 동일
- U7(4) → **U7a(3) + U7b(1) = 4** (분할, 총계 불변)
- U2·U3·U4·U5·U6: **변동 없음**
- 전체: 8+5+8+8+6+5+4 = 44 → **7+5+8+8+6+5+3+1 + (U0)1 + (U8)0 = 44** ✅ **총계 불변**

---

## 6. FQ-3=A 조정 반영 요약

| 조정 | 영향 스토리 | 소유 결정 | 표현 정정 |
|---|---|---|---|
| FQ-1 표준 SHA-256 (okc-core 재현 제거) | US-E7-01, US-E1-02, US-E2-01, US-E7-10 | 전부 U1 유지 | "okc-core `raw_sha256`/스킴" → 표준 `sha256sum`; `vault_content_id` 권위 계산은 서버, 클라이언트는 `manifest_digest` |
| FQ-2 최신-상태-대체 (지속 이벤트 큐 제거) | US-E3-01, US-E3-02, US-E3-03, US-E3-04, US-E7-04, US-E7-08 | 전부 U4 유지 | 큐 → `SyncStateStore`(마지막커밋+dirty+재개오프셋); coalesce/drain은 재스냅샷으로 구조적 자동 성립 |
| FQ-3 무손실 코덱 소유 통일 | US-E7-06 | **U1 → U0 Foundation** | 무손실 코덱은 공유 `CoreTypes` 타입 → Foundation 소유; U1 인접성 주석 유지 |

> 위 조정은 원문 스토리를 대체하지 않는 오버레이다. 각 스토리의 가치(so that)·수용 의도는 불변이며, 소유/달성 메커니즘 표현만 단순화 아키텍처에 맞춰 정정한다(requirements.md §12 / stories.md FQ-3 부록 근거).

---

## 7. 교차/접착 단위(U0·U8) 소유 주석

- **U0 Foundation** — `CoreTypes`(공유 타입·에러 taxonomy·무손실 코덱·상태 어휘·싱크 계약 트레이트) + `ConfigProvider`(단일 JSON 설정)는 모든 E1–E7 스토리가 관통하는 교차 관심사다. 조립 루트(U8)가 소유해 아래로 하향 주입하므로 비순환이 유지된다. **1차 스토리 소유는 원칙적으로 없다.** 유일한 문서화된 예외는 **US-E7-06**(직렬화 무손실 round-trip) — 검증 대상인 무손실 코덱이 per-unit 관심사가 아니라 `CoreTypes`의 공유 타입이기 때문에 Foundation이 1차 소유한다(execution-plan §4의 U1 배정과의 인접성은 표에 주석).

- **U8 Orchestration** — `WatcherDaemon`(조립 루트/생명주기), `SyncCycleCoordinator`(트리거당 1회 직렬 사이클), `SingleInstanceLock`(설치 단위 잠금)은 전 단위를 묶는 접착 컴포넌트다. 모든 E1–E7 스토리의 런타임 경로가 이들을 관통하지만 **스토리 1차 소유는 0**이다. 특히 UQ-1=A로 `SingleInstanceLock` **컴포넌트**가 U2→U8로 이동했으나(응집도↑), 대응 **스토리** US-E1-05의 1차 소유는 execution-plan §4 승계에 따라 **U2**로 유지된다(설치 단위 보호는 E1 감지·트리거 관심사) — 컴포넌트 위치와 스토리 소유가 의도적으로 분리된 유일 사례.

- **E7 트랙** — 독립 단위가 아니라 검증 대상 코드를 소유한 단위로 접힌다(PBT-ON per-unit): 무상태 속성(US-E7-01/05/07/10)→U1, 프로토콜 속성(US-E7-02/03)→U3, 상태·회복탄력성 속성(US-E7-04/08/09/11)→U4, 무손실 코덱 속성(US-E7-06)→U0.

- **총계 불변 보증** — 교차/접착 단위 처리(U8=0, U0=예외 1건) 이후에도 스토리 총계는 **44 = 44**로 보존된다(§5 정합 확인).
