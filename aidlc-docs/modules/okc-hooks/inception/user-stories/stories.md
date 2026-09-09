# 사용자 스토리 — okc-hooks "Watcher"

**단계**: INCEPTION → User Stories (Part 2 — 생성)
**작성일**: 2026-09-07
**출처**: 승인된 `aidlc-docs/inception/requirements/requirements.md` (FR-01..23, NFR-01..17, RISK-01/02, DEP-01..07, PBT/§4 URP) + 승인된 스토리 생성 계획(Q1..Q13)
**검증**: 적대적 완결성·추적성·INVEST 크리틱 통과(PASS) — 미커버 요구사항 0, 누락 파생 스토리 0, 데몬 모델 위배 0.

## 실행 모델 (권위 있는 확정)

Watcher는 사용자 **본인의 개인 데스크톱**(macOS / Windows / Linux)에 설치되어 **GUI 없이 백그라운드 데몬/서비스**로 상주한다(nginx 방식). 전용 GUI 창은 없으며 원격 서버도 아니다. 모든 상호작용은 **편집 가능한 config 파일 + 동반 CLI(start/stop/status/`sync now`/view-history/pause/resume) + 구조화 로그 + 헬스 체크**로 표현한다. 시스템 트레이/메뉴바 아이콘(FR-18)은 데스크톱 세션이 있을 때만 쓰는 **선택 항목**이며, 어떤 스토리도 트레이 존재에 의존하지 않는다.

**페르소나**: 정확히 2개 — **P1 Vault Owner(데몬 운영자)**, **P2 프라이버시 민감 Vault Owner(동일 인물의 관점 변형)**. 별도의 "Headless Operator / 서버 관리자" 페르소나는 두지 않는다(데몬 운영이 P1의 정상 모드). 미래 웹 서비스는 페르소나가 아니라 DEP-01..07 외부 계약으로 표현한다. 상세는 `personas.md` 참조.

## 태그 범례

- **[MVP Must-have]** — 전송 파이프라인 골격(자동 트리거 FR-01 / 원시 볼트 업로드 FR-06 / have-want FR-07 / 재개 전송 FR-08 / 지속 큐 FR-03). 그 외 스토리는 의도적으로 우선순위를 부여하지 않고 **Workflow Planning으로 이월**한다.
  - **주의(확인 필요)**: MVP 플래그는 전송 골격만 표시한다. 최소 동작 경로가 실제로 성립하려면 매니페스트+diff(US-E1-02/FR-02), 일관 스냅샷 해싱(US-E2-01/FR-22), 토큰 인증(US-E4-01/FR-13)이 **전제 의존**이지만 이들은 별도 우선순위를 매기지 않았다. 최종 MVP 집합 확정은 Workflow Planning에서 한다. → 아래 "미결/확인 항목" 참조.
- **[assumption-based (mock contract)]** — 미구축 서버의 문서화된 목(mock) 계약 기준으로 검증하는 스토리.
- **[blocked-on-server]** — 서버가 권위적으로 강제/수행하는 항목(실서버 전까지 클라이언트에서 강제 불가).
- **[optional — confirm-or-drop]** — 채택/제거를 Functional Design에서 최종 확정할 선택 항목.
- **[derived — not from an original FR]** — 원래 FR가 아니라 승인된 Q-답변에서 파생된 스토리.

---

## E1 모니터링 & 트리거링 (Monitoring & Triggering)

볼트 변경을 OS 파일시스템 이벤트로 감지해 디바운스 후 동기화 사이클을 자동으로 트리거하고, 프로세스 재시작·런타임 이벤트 손실·볼트 사용 불가 상황에서도 변경을 놓치거나 파괴적 커밋을 내지 않도록 보장한다. (Watcher는 nginx처럼 GUI 없는 백그라운드 데몬/서비스로 동작하므로, 이 에픽의 모든 상호작용은 config 파일 · CLI · 구조화 로그 · 헬스체크로 표현한다.)

### US-E1-01 - 파일시스템 이벤트 감시 + 디바운스 자동 트리거

As a P1 Vault Owner (백그라운드 데몬 운영자), I want the Watcher to watch the vault via OS filesystem events and auto-trigger a sync cycle after a debounce quiet period, so that my edits sync automatically without any manual action.

**Acceptance Criteria:**
- Given 데몬이 실행 중이고 설정된 볼트를 감시하고 있을 때, When 파일이 생성/수정/삭제/이름변경되면, Then 파일시스템 변경 이벤트가 관측되고 디바운스 타이머(T_debounce)가 시작/재설정된다.
- Given 하나 이상의 변경 이벤트가 관측된 상태에서, When T_debounce 동안 추가 이벤트가 없으면, Then 업로드 사이클이 트리거되어 E2 업로드 파이프라인으로 인계된다.
- Given 편집이 계속되어 이벤트가 T_debounce 안에 계속 도착할 때, When 정적 구간(quiet period)이 도달하지 못하면, Then 볼트가 잠잠해질 때까지 사이클은 트리거되지 않는다(버스트 합치기).
- [ ] T_debounce는 config 파일 설정값이며 문서화된 기본값을 가진다. 음수/유효하지 않은 값은 시작 시 명확한 로그/CLI 오류로 거부한다.
- [ ] 플랫폼 네이티브 감시 API를 사용한다: FSEvents(macOS), ReadDirectoryChangesW(Windows), inotify(Linux). [NFR-05 이식성 AC]
- [ ] `watcher status` CLI가 현재 감시 상태(watching / triggered / idle)와 마지막 이벤트 이후 경과 시간을 보고한다.
- [ ] 사이클 트리거 시 원인 요약을 담은 구조화 로그 항목을 남긴다. [FR-17 상호참조]
- property-style: 모든 편집 버스트에 대해 정적 구간당 정확히 한 번의 사이클이 트리거되며(for all edit bursts), 변경 감지 지연은 T_debounce로 상한이 정해진다. [NFR-01]

**Traceability:** FR-01, NFR-01, NFR-05 (파일시스템 감시 이식성 AC); 상호참조: FR-06 (자동 트리거 골격만 — 원시 볼트 업로드 자체는 US-E2-05 소유)

**Persona:** P1

**[MVP Must-have]**

### US-E1-02 - 볼트 매니페스트 재계산 + 마지막 커밋 대비 diff

As a P1 Vault Owner, I want the Watcher to recompute a content-addressed manifest each cycle and diff it against the last committed manifest, so that only genuinely changed files are selected for upload and unchanged files are never re-transferred.

**Acceptance Criteria:**
- Given 트리거된 사이클에서, When Watcher가 매니페스트를 재계산하면, Then 각 파일은 `{relative_path, raw_sha256, size}`를, 볼트 전체는 `vault_content_id`를 okc-core와 동일한 SHA-256 스킴으로 산출한다.
- Given 새로 계산된 매니페스트와 저장된 마지막 커밋 매니페스트가 있을 때, When 둘을 diff하면, Then 변경 집합(추가/수정/삭제)을 정확히 그것만 산출한다.
- Given 마지막 커밋 이후 어떤 파일 내용도 바뀌지 않은 상태에서, When 사이클이 실행되면, Then diff는 비어 있고 업로드 작업이 생성되지 않는다(no-op). [NFR-01]
- [ ] 매니페스트 스키마는 파일별 정확히 `{relative_path, raw_sha256, size}` + 볼트당 `vault_content_id`이다.
- [ ] 마지막 커밋 매니페스트는 로컬에 지속 저장되고 사이클 시작 시 읽힌다.
- [ ] 해싱은 증분(스트리밍)으로 수행해 okc-core 상한(100k 파일 / 20 GiB)까지 비제한 메모리 없이 확장된다. [NFR-02 상호참조]
- [ ] `watcher status`가 마지막 사이클의 `vault_content_id`와 변경 파일 수를 보고할 수 있다.
- property-style (E7 기술 트랙에서 보호, 상호연결): 임의 파일 내용에 대해 해싱은 결정적이며 파일 해시는 `sha256sum` / okc-core `raw_sha256`와 일치한다(NFR-08, E7 소유); `diff(last_committed, current)`는 임의의 이전/이후 상태에 대해 정확한 변경 집합을 반환한다(NFR-12, E7 소유).

**Traceability:** FR-02, NFR-01 (미변경 파일 미재전송 AC); 상호참조: NFR-02, NFR-08 (E7), NFR-12 (E7)

**Persona:** P1

### US-E1-03 - 시작 시 재조정(reconciliation) 스캔

As a P1 Vault Owner, I want the Watcher to run a full reconciliation scan on startup, so that any changes made while the daemon was stopped are detected and synced.

**Acceptance Criteria:**
- Given 데몬이 시작되면(부트 자동시작 또는 수동 CLI `watcher start`), When 초기화가 완료되면, Then 정상 이벤트 감시 상태로 진입하기 전에 볼트 전체를 재해시하고 마지막 커밋 매니페스트와 diff한다.
- Given 프로세스가 내려가 있는 동안 변경이 발생했을 때, When 시작 스캔이 실행되면, Then 그 변경이 변경 집합에 나타나 동기화 사이클을 트리거한다.
- Given 내려가 있는 동안 변경이 없었을 때, When 시작 스캔이 실행되면, Then diff는 비어 있고 업로드가 생성되지 않는다.
- [ ] 시작 스캔은 모든 데몬 시작 시(launchd/systemd/Windows Service 자동시작 포함) 실행된다. [FR-21 상호참조]
- [ ] 스캔 진행/결과는 구조화 로그로 기록되고 `watcher status`에 반영된다.

**Traceability:** FR-04 (시작 스캔); 상호참조: NFR-03 (재시작 시 무손실 복구 — 회복탄력성 트랙 소유)

**Persona:** P1

**INVEST note:** US-E1-04(주기 스캔)와 FR-04를 공유하지만 독립적이다 — "정지 상태에서 어긋난 볼트로 시작" 시나리오는 별개의 트리거 지점이며 별도로 테스트 가능하다.

### US-E1-04 - T_recon 주기적 재조정 스캔 (이벤트 손실 백스톱)

As a P1 Vault Owner, I want the Watcher to periodically re-scan the vault at a configurable maximum interval, so that changes missed due to filesystem-event loss are still detected within a bounded window.

**Acceptance Criteria:**
- Given 데몬이 실행 중일 때, When 마지막 재조정 이후 T_recon이 경과하면, Then 이벤트 발생 여부와 무관하게 백스톱으로 볼트 전체 재해시 + diff를 수행한다.
- Given 파일시스템 이벤트가 유실되었을 때(inotify `IN_Q_OVERFLOW` / FSEvents 병합 / ReadDirectoryChangesW 버퍼 오버플로 / 이벤트를 주지 않는 네트워크 마운트), When 다음 주기 스캔이 실행되면, Then 놓쳤던 변경이 감지되어 동기화된다.
- [ ] T_recon은 config 파일 설정값이며 문서화된 기본값을 가진다. 미관측 변경의 최대 미감지 창을 ≤ T_recon으로 상한한다.
- [ ] 주기 스캔은 진행 중인 사이클과 동시 실행되지 않는다(직렬화).
- [ ] `watcher status`가 다음 재조정까지 남은 시간과 마지막 재조정 결과를 보고한다.
- property-style: 임의의 이전/이후 볼트 상태에 대해 재조정 diff는 정확한 변경 집합과 같다(NFR-12, E7 소유).

**Traceability:** FR-04 (주기적 백스톱); 상호참조: NFR-03 (미관측 변경 감지 지연 ≤ T_recon)

**Persona:** P1

**INVEST note:** US-E1-03과 FR-04를 공유하나 독립적으로 가치가 있고(런타임 이벤트 손실 백스톱) 이벤트 드롭 시뮬레이션으로 별도 테스트 가능하다.

### US-E1-05 - 단일 인스턴스 잠금 (큐/매니페스트 보호)

As a P1 Vault Owner, I want the Watcher to allow only one running instance per install, so that two processes never corrupt the durable queue or last-committed manifest or produce duplicate uploads.

**Acceptance Criteria:**
- Given 하나의 Watcher 인스턴스가 이미 실행 중일 때, When 두 번째 인스턴스가 기동되면(예: 서비스가 이미 떠 있는데 수동 CLI로 `watcher start`), Then 두 번째 인스턴스는 잠금을 감지하고 시작을 거부하며 비정상 종료 코드로 종료하고 "이미 실행 중" 메시지를 명확히 로그로 남긴다.
- Given 실행 중이던 인스턴스가 종료되거나 크래시했을 때, When 새 인스턴스가 시작되면, Then 오래된(stale) 잠금을 회수하고 정상적으로 시작을 진행한다.
- [ ] OS 수준 잠금(예: lock 파일 / named mutex)이 지속 큐(FR-03)와 마지막 커밋 매니페스트를 보호한다.
- [ ] 두 번째 기동에서의 `watcher status`는 경쟁 데몬을 띄우는 대신 활성 인스턴스의 PID/소유자를 보고한다.
- [ ] 어느 시점에도 큐/매니페스트 상태를 변경할 수 있는 인스턴스는 정확히 하나다(불변식).

**Traceability:** FR-23 (FR-03 지속 큐 및 FR-02 마지막 커밋 매니페스트 보호)

**Persona:** P1

**INVEST note:** 단일 인스턴스 잠금은 설치 단위 보호다. Q2=A에 따라 클라우드 동기화된 하나의 볼트를 두 데몬이 감시하는 경우는 범위 밖(문서화된 제한사항)이며 FR-10 멱등성에 의존한 최선노력 조정으로만 다룬다.

### US-E1-06 - 빈 볼트 / 볼트 사용 불가 안전 가드

As a P1 Vault Owner, I want the Watcher to refuse to treat a suddenly-empty or unreachable vault as "delete everything", so that an unmounted/deleted/moved vault root never produces a destructive empty commit that wipes my server-side content.

**Acceptance Criteria:**
- Given 볼트에 이전엔 파일이 있었을 때, When 사이클/재조정이 파일 0개인 매니페스트를 계산하거나 볼트 루트가 없음/언마운트/접근 불가이면, Then Watcher는 이를 vault-unavailable로 분류하고 보류하며(전부-삭제 diff를 만들지 않음), 커밋(FR-09)을 내지 않는다.
- Given vault-unavailable 조건이 감지되면, When 이 상태에 진입하면, Then 지속적인 "vault-unavailable" 상태를 구조화 로그 · `watcher status` · 헬스체크로 표면화하고 조용히 무시하지 않는다.
- Given 볼트가 파일과 함께 다시 사용 가능해졌을 때, When 다음 사이클이 실행되면, Then 확인 없이 정상 diff가 자동으로 재개된다.
- Given 사용자가 진짜로 빈 볼트를 의도할 때, When CLI(예: `watcher confirm-empty`)나 config 플래그로 명시적으로 확인하면, Then 빈 상태가 수용되어 진행이 허용된다.
- [ ] 파일 0개 / 루트 없음 매니페스트는 절대 전부-삭제 변경 집합을 자동 생성하지 않는다.
- [ ] vault-unavailable 동안 헬스체크는 unhealthy/held로 보고한다.
- [ ] 진짜 빈 볼트를 수용하는 확인은 명시적이며 로그로 남긴다.

**Traceability:** FR-09 (파괴적 빈 커밋 방지 — 커밋 자체는 E2 소유); 관련: FR-02 (0개 파일 매니페스트), FR-04 (스캔이 0개를 봄), FR-17/헬스체크로 표면화. Q13=A에서 파생.

**Persona:** P1

**[derived — not from an original FR]**

---

## US-E2 업로드 프로토콜 (Upload Protocol)

원시 볼트를 okc-core의 콘텐츠 주소(content-addressed) 모델과 전체 스냅샷 수집 방식에 맞춰, §4 URP 단계(스냅샷·매니페스트 → 사전검사 → have/want → 재개 전송 → 커밋 → 멱등성)를 통해 대용량 볼트에도 효율적이고 회복탄력적으로 업로드한다. 데몬 모델에서 모든 상호작용은 config 파일 설정 / CLI 명령 / 구조화 로그 / 헬스체크로 표현하며, GUI·트레이에 의존하지 않는다.

### US-E2-01 - 일관된 시점 스냅샷 및 확장 가능한 매니페스트 해싱

As a P1 (Vault Owner / 데몬 운영자), I want 볼트를 일관된 시점(point-in-time) 스냅샷으로 고정하고 스트리밍 방식으로 해싱하기를 원하며, so that 사이클 도중 볼트가 바뀌어도 전송 바이트가 매니페스트 해시와 항상 일치하고, okc-core 상한에서도 메모리를 무한정 쓰지 않는다.

**Acceptance Criteria:**
- Given 매니페스트 계산 이후 전송 전에 파일이 변경(TOCTOU)되었을 때, When 각 blob 전송 시 raw_sha256을 재검증, Then 불일치 시 재스냅샷/재해시 후 전송하여 매니페스트의 해시와 전송 바이트가 일치한다.
- 불변식/설정 체크리스트:
  - [ ] 전송되는 각 파일 바이트는 그 사이클 매니페스트의 raw_sha256과 정확히 일치 (FR-22)
  - [ ] 해싱은 스트리밍/증분 방식으로 100k 파일 / 20 GiB 상한에서도 무한정 메모리를 쓰지 않음 (NFR-02)
  - [ ] okc-core와 동일한 SHA-256 스킴으로 vault_content_id 산출 → client/server 콘텐츠 id 정렬
- 속성형 참조 (E7 기술 트랙과 교차연결): 임의의 파일 내용에 대해 해시가 결정적이고 sha256sum / okc-core raw_sha256과 동일 (NFR-08).

**Traceability:** FR-22, NFR-02

**Persona:** P1

**INVEST note:** 스냅샷 일관성(FR-22)과 해싱 확장성(NFR-02)은 모두 §4 1단계(스냅샷·매니페스트)에 속하는 단일 관심사로, E1의 매니페스트 재계산(FR-02) 위에 얹혀 다른 프로토콜 단계와 독립적으로(로컬 전용) 테스트 가능하다.

### US-E2-02 - 업로드 전 SafetyLimit 사전검사 (Preflight)

As a P1 (Vault Owner / 데몬 운영자), I want 전송 전에 클라이언트가 검사 가능한 SafetyLimit으로 볼트를 사전검사하고 초과 시 조기에 거부하기를 원하며, so that 서버가 어차피 거부할 볼트에 대용량 전송을 낭비하지 않는다.

**Acceptance Criteria:**
- Given 볼트가 총 20 GiB, 파일당 2 GiB, 100k 파일 중 하나라도 초과할 때, When 전송 전 사전검사가 실행, Then 로컬에서 거부하고 어떤 한도를 얼마나 초과했는지 실행 가능한 리포트를 로그와 CLI status로 출력하며 전송을 시작하지 않는다.
- 불변식/설정 체크리스트:
  - [ ] 검사 한도: ≤20 GiB 총량, ≤2 GiB/파일, ≤100k 파일 (FR-05)
  - [ ] 한도 이내이면 통과하여 have/want 단계로 진행
  - [ ] 리포트는 CLI status 명령과 구조화 로그로 확인 가능 (데몬 모델; GUI 비의존)
- **[blocked-on-server]** 권위 있는 재검증(프로젝트 단위 ≤10 sources 상한 포함)은 서버가 수행 — 목 계약에 명시하며 실서버 전까지 강제 불가 (DEP-04).
- 속성형 참조 (E7 기술 트랙과 교차연결): SafetyLimit 검증은 경계-정확·단조적이어서 파일/바이트 추가가 reject를 accept로 되돌리지 않는다 (NFR-14).

**Traceability:** FR-05, DEP-04

**Persona:** P1

### US-E2-03 - 콘텐츠 주소 기반 have/want 협상 **[MVP Must-have]**

As a P1 (Vault Owner / 데몬 운영자), I want 매니페스트를 서버에 보내고 서버가 결여한 blob만 업로드하기를 원하며, so that 변경되지 않은 파일(예: 대용량 미디어)을 다시 전송하지 않는다.

**Acceptance Criteria:**
- Given 매니페스트를 서버에 POST했을 때, When 서버가 결여 blob 집합("want", raw_sha256 기준)을 응답, Then 그 want 집합의 blob만 전송하고 서버가 이미 보유한 blob은 건너뛴다.
- 불변식/설정 체크리스트:
  - [ ] 전송 단위는 전체 파일(whole-file) 단위 — 서브파일 델타 없음 (§4, okc-core 제약)
  - [ ] want 집합 = 매니페스트 경로 \ 서버 보유분
  - [ ] 유효한 API 토큰으로 인증된 세션 가정 (토큰은 config 파일로 입력; 인증 처리는 E4)
- 속성형 참조 (E7 기술 트랙과 교차연결): want == manifest_paths \ server_has 이며, 서버가 want를 보유한 뒤 재실행하면 want는 공집합이 된다 (NFR-09).

**Traceability:** FR-07, DEP-03

**Persona:** P1

**Tags:** **[MVP Must-have]** **[assumption-based (mock contract)]**

### US-E2-04 - 재개 가능한 청크 전송 (tus 스타일) **[MVP Must-have]**

As a P1 (Vault Owner / 데몬 운영자), I want 대용량이거나 중단된 blob 업로드가 처음부터 다시 시작하지 않고 마지막 확인된 오프셋부터 재개되기를 원하며, so that 불안정한 연결에서도 큰 볼트가 안정적으로 동기화된다.

**Acceptance Criteria:**
- Given 임계값 S를 초과하는 blob 또는 단일 요청 전송이 실패/타임아웃한 blob에 대해, When 전송이 중단되었다가 재개될 때, Then 마지막으로 확인(ack)된 오프셋부터 이어서 전송하며 처음부터 다시 보내지 않는다.
- 불변식/설정 체크리스트:
  - [ ] 임계값 S는 config 파일 설정값; S 이하 blob은 단일 요청 허용, S 초과 blob(최대 2 GiB) 또는 단일 요청 실패/타임아웃 blob은 청크 전송 (FR-08)
  - [ ] 청크별 무결성 검증(per-chunk integrity) 수행
  - [ ] 재개는 durable 상태에 기록된 마지막 ack 오프셋 기준
- 속성형 참조 (E7 기술 트랙과 교차연결): 청크 재조립은 원본 blob을 바이트 단위로 복원하며, 유효한 임의 오프셋에서 재개해도 동일 결과를 낸다 (NFR-10).

**Traceability:** FR-08, DEP-03

**Persona:** P1

**Tags:** **[MVP Must-have]** **[assumption-based (mock contract)]**

### US-E2-05 - 원시 볼트 커밋 및 서버 바인딩 **[MVP Must-have]**

As a P1 (Vault Owner / 데몬 운영자), I want 컴파일 산출물이 아닌 원시 볼트를 업로드하고, want 집합의 모든 blob이 서버에 존재하면 vault_content_id + 매니페스트를 참조하는 커밋을 보내기를 원하며, so that 서버가 스냅샷을 디스크로 구체화하고 okc-core 수집용 경로로 바인딩할 수 있다.

**Acceptance Criteria:**
- Given want 집합의 모든 blob이 서버에 존재할 때, When vault_content_id + 매니페스트를 참조하는 commit을 전송, Then 서버가 바이트를 디스크로 구체화(materialize)하고 okc-core 수집용 경로로 바인딩한다.
- 불변식/설정 체크리스트:
  - [ ] 업로드 대상은 원시 볼트(raw vault)이며 컴파일 산출물이 아님 (FR-06)
  - [ ] commit은 vault_content_id + 매니페스트를 참조 (FR-09)
- **[blocked-on-server]** 바이트 구체화 및 경로 바인딩은 서버 책임 — 목 계약 기준으로 commit 요청/응답만 클라이언트에서 검증 (DEP-03).

**Traceability:** FR-06, FR-09, DEP-03

**Persona:** P1

**Tags:** **[MVP Must-have]** **[assumption-based (mock contract)]**

**INVEST note:** FR-06("무엇을" = 원시 볼트)와 FR-09(커밋으로 "확정")는 하나의 커밋 단계로 응집되며, MVP 골격의 FR-01/FR-06 자동 동기화 축 중 E2 측 절반(FR-06)을 실현한다.

### US-E2-06 - 멱등적 콘텐츠 주소 업로드 (no-op)

As a P1 (Vault Owner / 데몬 운영자), I want 변경되지 않은 파일과 반복 커밋이 no-op으로 처리되기를 원하며, so that 불필요한 사이클이 비용을 들이지 않고 재시도가 안전하다.

**Acceptance Criteria:**
- Given 직전 커밋 이후 변경되지 않은 파일에 대해, When 다음 사이클이 실행될 때, Then 해당 파일은 재전송 없이 no-op으로 처리된다.
- Given 이미 존재하는 vault_content_id에 대한 commit을 재전송할 때, When 서버가 이를 수신, Then no-op으로 처리되어 중복 커밋이 안전하다.
- 불변식/설정 체크리스트:
  - [ ] 모든 단계가 콘텐츠 해시로 키잉(content-addressed)됨
  - [ ] 재시도/중복 요청이 상태를 손상시키지 않음(idempotent)
- 속성형 참조 (E7 기술 트랙과 교차연결): have/want 재실행 멱등성 (NFR-09).

**Traceability:** FR-10

**Persona:** P1

**Tags:** **[assumption-based (mock contract)]**

### US-E2-07 - 대용량/초기 동기화 진행률 및 "재개 중" 상태 **[derived — not from an original FR]**

As a P1 (Vault Owner / 데몬 운영자), I want 임계값 S를 초과하는 대용량/초기 동기화에 대해 CLI status와 로그로 진행률과 "재개 중" 표시를 보기를 원하며, so that GUI 없이도 긴 동기화가 멈춘 것이 아니라 진행 중임을 확인할 수 있다.

**Acceptance Criteria:**
- Given 전송 대상이 임계값 S를 초과하는 대용량/초기 동기화일 때, When 전송이 진행 중, Then CLI status 명령과 구조화 로그에 진행률(바이트/퍼센트)을 표시한다.
- Given 중단 후 재개될 때, When 전송이 이어질 때, Then "재개 중(resuming)" 상태를 CLI status와 로그에 표시한다.
- 불변식/설정 체크리스트:
  - [ ] S 이하의 일반 증분 사이클에는 진행률 오버레이 없음 — 프로토콜은 단일(증분과 동일), 표시기만 대용량에서 추가 (Q9=C)
  - [ ] 진행률/상태는 CLI status + 로그로만 노출; GUI/트레이 비의존 (데몬 모델)

**Traceability:** FR-08

**Persona:** P1

**Tags:** **[derived — not from an original FR]** **[assumption-based (mock contract)]**

**INVEST note:** 프로토콜 메커니즘(FR-08, US-E2-04)과 독립된 "관측 표면" 스토리로, 전송 로직 변경 없이 상태 표시만 추가·테스트할 수 있다.

### US-E2-08 - 런타임 SafetyLimit 초과 처리 **[derived — not from an original FR]**

As a P1 (Vault Owner / 데몬 운영자), I want 이전엔 유효했던 볼트가 런타임 중 한도를 초과하면 동기화를 중단하되 마지막 정상 커밋을 유지하고 한도 이내로 돌아오면 자동 재개하기를 원하며, so that 과도한 증가가 상태를 손상시키거나 잘못된 업로드를 만들지 않고 나에게 통지된다.

**Acceptance Criteria:**
- Given 이전엔 유효했던 볼트가 런타임 중 클라이언트 검사 가능한 SafetyLimit(FR-05)을 초과하도록 증가했을 때, When 사이클이 사전검사에서 초과를 감지, Then 동기화를 중단(halt)하고 마지막 정상 커밋 상태를 유지하며, FR-19 케이스 3 알림을 발생(로그/CLI/헬스체크로 표면화)하고 지속적인 "용량 초과(over-limit)" 상태를 표시한다.
- Given "용량 초과" 상태일 때, When 매 사이클 자동 재검사에서 한도 이내로 복귀가 확인, Then 동기화를 자동 재개하고 "용량 초과" 상태를 해제한다.
- 불변식/설정 체크리스트:
  - [ ] 마지막 정상 커밋(last good commit)은 초과 기간 동안 보존
  - [ ] "용량 초과" 상태는 CLI status + 헬스체크 + 로그로 확인 (데몬 모델; GUI 비의존)
  - [ ] 매 사이클 자동 재검사 (사용자 개입 불필요)

**Traceability:** FR-05, FR-19

**Persona:** P1

**Tags:** **[derived — not from an original FR]**

---

## E3 오프라인·백프레셔·재시도 (Offline, Backpressure & Retry)

네트워크가 끊기거나 서버가 백프레셔를 걸거나 데몬 프로세스가 중단되어도, 이미 관측·적재된 변경을 잃지 않고 최신 상태로 합쳐 사람 개입 없이 자동으로 재개·전송한다.

### US-E3-01 - 크래시/재시작에도 살아남는 지속(durable) 온디스크 큐
As a P1 (Vault Owner / 데몬 운영자), I want 대기 중인 변경을 디스크에 지속 저장하는 큐, so that 데몬/서비스가 크래시하거나 재부팅되어도 이미 관측·적재된 변경을 손실 없이 그대로 이어서 처리할 수 있다.

**Acceptance Criteria:**
- 행위/이벤트 흐름 (Given-When-Then):
  - Given 변경이 감지되어 큐에 적재된 상태, When 데몬 프로세스가 비정상 종료(크래시)한 뒤 재시작되면, Then 큐의 대기 항목이 디스크에서 복원되고 그 변경들은 손실 없이 다시 처리 대상이 된다.
  - Given 큐에 항목을 기록하는 도중 프로세스가 중단된 경우, When 재시작 시 큐를 로드하면, Then 부분 기록된 항목은 원자적으로 무시/롤백되어 큐가 손상되지 않고 마지막 정상 상태로 복원된다.
  - Given 서비스가 OS 서비스(launchd / systemd / Windows Service)로 자동 시작되도록 설치된 상태, When 데스크톱/서버가 재부팅되면, Then 데몬이 자동 재기동하며 지속 큐를 그대로 이어받는다.
- 불변식/설정 체크리스트:
  - [ ] 큐 저장 위치가 config 파일에서 확인/설정 가능하다.
  - [ ] 큐 쓰기는 원자적(임시 파일 + rename 또는 WAL 등)이어서 크래시 시 손상되지 않는다.
  - [ ] `watcher status` 및 헬스 체크가 현재 큐 깊이(대기 항목 수)를 노출한다.
  - [ ] 로컬 큐 파일이 OS 보안 저장소 밖 평문으로 저장됨을 문서화한다(RISK-01 로컬 평문 산출물).
- 속성형(property-style, 상호연결 E7): for all 명령 시퀀스(enqueue / dequeue / coalesce / persist / recover)에 대해 큐 상태는 참조 모델과 일치하고, recover 후에도 관측·적재된 변경이 보존된다 — PBT-06 (E7).

**Traceability:** FR-03, cross-link PBT-06 (E7), NFR-03 (무손실 목표 — E1의 FR-04와 공동), FR-23 (단일 인스턴스 잠금이 큐를 보호 — E1)

**Persona:** P1

**[MVP Must-have]**

### US-E3-02 - 대기 변경의 최신 스냅샷 합치기(coalesce)
As a P1 (Vault Owner / 데몬 운영자), I want 오프라인이거나 백오프 중일 때 같은 파일에 쌓인 대기 변경이 최신 상태 하나로 합쳐지는 것, so that 재연결 시 중간 상태를 낭비 전송하지 않고 최신 스냅샷만 업로드한다.

**Acceptance Criteria:**
- 행위/이벤트 흐름 (Given-When-Then):
  - Given 파일 A가 오프라인/백오프 중 여러 번 편집되어 큐에 여러 상태가 쌓인 상황, When 다음 업로드 사이클이 준비되면, Then 파일 A에 대해서는 최신 스냅샷(가장 최근 raw_sha256)만 남고 이전 대기 상태는 폐기된다("newer supersedes older").
  - Given 오프라인 중 파일이 생성되었다가 다시 삭제된 경우, When 재연결되면, Then 최종 상태(부재)만 반영되어 불필요한 생성→삭제 왕복 전송이 발생하지 않는다.
  - Given 커밋되지 않은 대기 변경만 존재하는 경우, When coalesce가 수행되면, Then 이미 성공적으로 커밋된 상태에는 영향을 주지 않는다(FR-10 멱등성과 정렬).
- 불변식/설정 체크리스트:
  - [ ] coalesce는 파일별(relative_path) 단위로 최신 상태만 유지한다.
  - [ ] coalesce는 큐 지속성(FR-03)과 함께 원자적으로 반영되어 크래시 시 중간 상태가 남지 않는다.
  - [ ] coalesce로 인한 큐 깊이 감소가 `watcher status` / 구조적 로그로 관측 가능하다.
- 속성형(property-style, 상호연결 E7): for all 큐 변경의 임의 인터리빙에 대해 coalesce는 "latest state wins" 불변식을 보존한다 — NFR-11 / PBT-06 (E7).

**Traceability:** FR-12, cross-link NFR-11 (E7), PBT-06 (E7), FR-10 (멱등성 정렬 — E2)

**Persona:** P1

**INVEST note:** coalesce는 결정적 코어 로직으로, 목 큐/스텁 상태에 대해 US-E3-01의 큐 구현과 독립적으로 협상·구현·테스트할 수 있다.

### US-E3-03 - 실패·타임아웃·서버 백프레셔 시 지수 백오프 재시도
As a P1 (Vault Owner / 데몬 운영자), I want 업로드가 실패/타임아웃하거나 서버가 백프레셔(PROJECT_BUSY, queue-full)를 반환하면 지속 큐를 통해 지수 백오프로 재시도하는 것, so that 일시적 장애나 서버 과부하에서도 수동 개입 없이 최종적으로 동기화가 완료된다.

**Acceptance Criteria:**
- 행위/이벤트 흐름 (Given-When-Then):
  - Given 업로드 요청이 타임아웃되거나 5xx/네트워크 오류로 실패한 경우, When 재시도 로직이 동작하면, Then 해당 항목은 큐에 유지된 채 지수 백오프(설정된 기본 스케줄) 간격으로 재시도된다.
  - Given (목 계약) 서버가 `PROJECT_BUSY` 또는 queue-full 백프레셔 응답을 반환한 경우, When 재시도 로직이 동작하면, Then 즉시 재시도하지 않고 백오프 후 재시도하며, 백프레셔는 오류가 아닌 정상적 지연으로 로그에 기록된다.
  - Given 재시도가 성공해 커밋이 완료된 경우, When 사이클이 종료되면, Then 백오프 간격이 초기화되고 항목이 큐에서 제거된다.
  - Given 모든 네트워크 호출, When 호출이 발생하면, Then 각 호출에 타임아웃이 적용되어 무한 대기하지 않는다.
- 불변식/설정 체크리스트:
  - [ ] 타임아웃 값과 백오프 스케줄(초기 지연/배수/상한)이 config 파일에서 설정 가능하며 합리적 기본값을 가진다(구체 기본값은 Functional/NFR Design로 이월).
  - [ ] 백오프에는 상한(max backoff)이 있어 무한 증가하지 않는다.
  - [ ] 재시도/백오프 상태(다음 재시도 예정 시각, 연속 실패 횟수)가 구조적 로그와 `watcher status`로 관측 가능하다.
  - [ ] 인증 실패(HTTP 401)는 재시도 대상이 아니라 인증 실패 흐름(E4)으로 위임한다.
  - [ ] N회 연속 실패 시의 능동 알림(FR-19 케이스 2)은 E5 관측성 스토리로 연결된다(본 스토리는 재시도 메커니즘만 소유).

**Traceability:** FR-11, NFR-04, cross-link FR-03 (재시도 매체 = 지속 큐), DEP-03 (업로드 엔드포인트 목 계약), FR-19 (연속 실패 알림 — E5)

**Persona:** P1

**[assumption-based (mock contract)]** (서버 백프레셔 신호 PROJECT_BUSY / queue-full은 미구축 서버의 문서화된 목 계약 기준)

**INVEST note:** 재시도/백오프는 목 서버가 주입한 실패·타임아웃·백프레셔 응답에 대해 독립적으로 테스트 가능하며, 오프라인 배출(US-E3-04)이나 coalesce(US-E3-02)와 분리해 구현할 수 있다.

### US-E3-04 - 오프라인 우아한 저하(graceful degradation)와 재연결 시 배출(drain)
As a P1 (Vault Owner / 데몬 운영자), I want 네트워크가 끊겨도 Watcher가 조용히 감시·적재를 계속하고 재연결되면 자동으로 큐를 배출하는 것, so that 오프라인 상태에서도 데이터 손실 없이 동작하고 연결 복구 시 사람 개입 없이 동기화가 재개된다.

**Acceptance Criteria:**
- 행위/이벤트 흐름 (Given-When-Then):
  - Given 네트워크가 도달 불가한 상태, When 볼트가 변경되면, Then 업로드는 시도하되 실패를 오프라인으로 인식하고 감시(FR-01)와 큐 적재(FR-03)를 계속하며, 반복적인 오류 알림을 발생시키지 않고 오프라인 상태를 로그 / `watcher status` / 헬스 체크로만 표시한다.
  - Given 오프라인 동안 여러 변경이 큐에 쌓인 상태, When 네트워크가 복구되면, Then Watcher는 사람 개입 없이 자동으로 큐 배출을 시작하고, 배출 전 coalesce(US-E3-02)를 적용해 최신 상태만 전송한다.
  - Given 배출 중 다시 오프라인이 된 경우, When 연결이 끊기면, Then 진행 중이던 재개 가능 업로드(FR-08, E2)의 오프셋을 유지한 채 백오프 상태로 되돌아가고, 재연결 시 이어서 배출한다.
- 불변식/설정 체크리스트:
  - [ ] 오프라인 판정은 타임아웃/연결 오류(NFR-04)에 기반하며 별도 서버 왕복 없이 결정된다.
  - [ ] 오프라인 상태에서도 데몬은 종료되지 않고 지속 실행된다(FR-21, E6).
  - [ ] 현재 연결 상태(online/offline)와 마지막 성공 동기화 시각이 `watcher status` / 헬스 체크로 노출된다.
- 속성형(property-style, 상호연결 E7): recover/재연결을 포함한 임의 명령 시퀀스에서도 관측·적재된 변경은 보존된다 — PBT-06 (E7).

**Traceability:** NFR-04, cross-link FR-01 (감시 — E1), FR-03 (지속 큐), FR-08 (재개 가능 업로드 — E2), FR-21 (지속 실행 데몬 — E6), PBT-06 (E7)

**Persona:** P1

**[assumption-based (mock contract)]** (재연결 후 실제 배출은 서버 업로드 엔드포인트의 문서화된 목 계약 기준; 오프라인 감지·계속 적재·백오프는 네트워크 차단만으로 로컬 테스트 가능)

**INVEST note:** 오프라인 감지·계속 적재·백오프는 네트워크 차단만으로 로컬에서 독립 테스트 가능하며, 서버(목)가 필요한 부분은 재연결 배출 경로뿐이다.

---

## US-E4 인증 & 동의 (Authentication & Consent)

Watcher가 미래 웹 서비스에 API 토큰으로 인증하고, 사용자의 정보에 입각한(informed) 상시 동의(standing consent) 아래에서만 raw 볼트를 업로드하도록 보장한다 — 데몬 모델에서 GUI/트레이 없이 config 파일 · CLI · 구조화 로그 · 헬스 체크만으로.

> **에픽 전반 전제 (Q3-CUSTOM 목 계약)**: 미래 웹 서비스가 API 토큰을 발급한다고 가정한다. 사용자는 그 토큰을 **config 파일**에 입력하며, config에 입력된 토큰이 발급된 토큰과 **일치하면** 인증이 성공하고 동기화가 진행된다. 실서버가 없으므로 모든 서버 의존 동작은 **문서화된 목(mock) 계약** 기준으로 검증한다. 데몬 모델에서 모든 상호작용 인수 조건은 **config 설정 · CLI 명령 · 로그 출력 · 헬스 체크**로 표현하며, 어떤 스토리도 트레이/GUI 존재에 의존하지 않는다.

### US-E4-01 - config 기반 API 토큰 인증 (매 요청 전송, TLS)
P1 Vault Owner로서, config 파일에 웹 서비스가 발급한 API 토큰을 입력하고 모든 요청에 그 토큰을 실어 보내고 싶다. 그래야 데몬이 GUI 없이도 서비스에 인증되어 동기화를 수행할 수 있다.

**Acceptance Criteria:**
- Given config 파일의 `token` 필드에 입력된 토큰이 (목 계약상) 발급된 토큰과 일치할 때, When Watcher가 서비스에 아무 요청을 보내면, Then 요청 헤더에 토큰이 포함되고 인증이 성공하며 업로드 사이클이 진행된다.
- Given config 토큰이 발급된 토큰과 불일치할 때, When Watcher가 요청을 보내면, Then (목 계약이) HTTP 401을 반환하고 Watcher는 인증 실패 흐름(US-E4-03)으로 진입한다.
- Given `token` 필드가 비었거나 없을 때, When 데몬이 사이클을 시작하려 하면, Then 업로드를 시작하지 않고 CLI `status` 명령과 로그에 실행 가능한(actionable) 오류를 표시한다.
- 불변식/설정 체크리스트:
  - [ ] 토큰의 **1차(primary) 저장 경로는 config 파일(또는 환경 변수)** — 키링 없는(keyring-less) 경로 (데몬 모델).
  - [ ] 서비스로 가는 **모든 요청**에 토큰을 첨부한다 (FR-13).
  - [ ] 모든 전송은 **TLS** 위에서만 이루어진다 (NFR-06).
  - [ ] 토큰의 존재/유효성 판정 결과는 CLI `status` + 헬스 체크로 관측 가능하다.
- 서버 계약 노트 **[blocked-on-server]**: 토큰의 발급/회전/폐기는 서버 소유(DEP-01); 사용자별 격리(멀티테넌시)는 서버 소유이며 토큰이 테넌트를 스코핑한다고 가정(DEP-05). 실서버 전까지 목 계약으로만 검증.

**Traceability:** FR-13, NFR-06, DEP-01, DEP-05

**Persona:** P1, P2

**[assumption-based (mock contract)]**

### US-E4-02 - OS 보안 저장소에 토큰 저장 (선택적 강화)
P1 Vault Owner로서, 선택적으로 토큰을 config 평문 대신 OS 보안 저장소(Keychain / Credential Manager / Secret Service)에 두고 싶다. 그래야 데스크톱 세션에서 자격 증명 노출면을 줄일 수 있다.

**Acceptance Criteria:**
- Given config에서 `secure_store` 옵션이 켜져 있고 데스크톱 세션의 보안 저장소가 사용 가능할 때, When 데몬이 토큰을 읽으면, Then 평문 config가 아니라 OS 보안 저장소에서 토큰을 조회한다.
- Given 보안 저장소를 사용할 수 없는(헤드리스/데몬) 환경일 때, When 데몬이 시작하면, Then config/환경 변수 경로로 안전하게 폴백하며 실행이 중단되지 않는다.
- 불변식 체크리스트:
  - [ ] **1차 경로는 config/환경 변수**이고, 보안 저장소는 opt-in 강화 옵션이다.
  - [ ] **어떤 스토리도 보안 저장소 존재에 의존하지 않는다** (US-E4-01 인증은 config만으로 성립).

**Traceability:** FR-13, NFR-06

**Persona:** P1, P2

**[optional — confirm-or-drop]**

**INVEST note:** US-E4-01(config 1차 경로)과 독립적으로 켜고 끌 수 있는 순수 강화 옵션이므로, 이 스토리를 드롭해도 인증 골격은 온전하다.

### US-E4-03 - 인증 실패 처리 및 토큰 재입력
P1 Vault Owner로서, 토큰이 거부되면 즉시 알림을 받고 config에서 새 토큰을 재입력해 동기화를 재개하고 싶다. 그래야 서버가 토큰을 회전/폐기했을 때도 데몬을 CLI로 복구할 수 있다.

**Acceptance Criteria:**
- Given 서비스가 토큰을 거부할 때(토큰 거부 / HTTP 401), When 이 조건이 발생하면, Then Watcher는 **FR-19 케이스 1** 알림을 발생시키고(1차 표면 = 로그 + CLI `status` + 헬스 체크; 데스크톱 알림은 선택적), 업로드를 중단하되 감시와 큐 적재(FR-03)는 유지한다.
- Given 서버가 토큰을 회전/폐기(DEP-01)하여 사용자가 config의 토큰을 새로 발급된 값으로 수정할 때, When Watcher가 config를 다시 읽고(CLI `reload` 또는 재시작) 토큰이 이제 일치하면, Then 인증이 성공하고 대기 중이던 업로드가 재개된다.
- 불변식 체크리스트:
  - [ ] 인증 실패 시 큐/감시는 유지, **업로드만 차단**한다.
  - [ ] 토큰 재입력(Q13)은 별도 UI 없이 **config 편집 → CLI `reload`/재시작**으로 반영된다 (인증 실패 여정에 흡수).
- 노트: FR-19의 알림 표면(트레이/데스크톱 등) 자체는 US-E5 소관이며, 여기서는 케이스 1(인증 실패)의 복구 흐름만 다룬다.

**Traceability:** FR-13, FR-19, DEP-01

**Persona:** P1

**[assumption-based (mock contract)]**

**INVEST note:** US-E4-01에 논리적으로 이어지지만, 목 계약의 401 응답만으로 독립 테스트가 가능하므로 별도 스토리로 유지한다.

### US-E4-04 - 최초 실행 RISK-01 공개 고지 확인 (informed consent 기록)
P2 프라이버시 민감 Vault Owner로서, 첫 업로드 전에 RISK-01 공개(disclosure) 고지를 명시적으로 확인하도록 요구받고 싶다. 그래야 업로드가 연속적·비가역적이며 서버 전까지 실효적 철회가 없다는 점을 이해한 상태에서만 동기화가 시작된다.

**Acceptance Criteria:**
- Given 최초 실행이고 기록된 고지 확인이 없을 때, When 사용자가 동기화를 시작하려 하거나 설정 CLI를 실행하면, Then Watcher는 RISK-01 고지 텍스트를 제시하고, config 플래그 또는 CLI `acknowledge` 명령으로 확인이 기록되기 전까지 업로드를 거부한다.
- 고지 내용 불변식 체크리스트:
  - [ ] **연속성(continuous)**: 상시 동의 + 자동 동기화 하에서 이후의 모든 볼트 변경(나중에 추가된 비밀 포함)이 추가 확인 없이 자동 업로드된다 (RISK-01).
  - [ ] **비가역성(irreversible) + 실효적 철회 부재**: 업로드된 콘텐츠는 회수 불가이며, 서버가 생기기 전까지 FR-14 철회는 전진 방향(forward-only)에 그친다 (RISK-01).
  - [ ] **클라이언트 측 민감 콘텐츠 필터링/사전검사 없음** — raw 볼트 전체(비밀·개인정보 포함)가 전송된다 (FR-15, NFR-07).
  - [ ] **로컬 평문 산출물**(히스토리·매니페스트·큐·로그)이 OS 보안 저장소 밖에 존재함을 고지한다 (RISK-01).
  - [ ] 확인은 config/CLI로 기록되며, 확인 전에는 업로드가 시작되지 않는다.
- RISK-02 인지 노트 **[blocked-on-server]**: 잦은 편집은 매 새 `vault_content_id`마다 서버의 이전 `ApprovedIntegrationPlan`을 무효화해 재승인 부담(re-approval thrash)을 유발한다. 완화는 대부분 서버 몫(고승인 감내 — DEP-07); Watcher 측 throttle/batch는 선택적이며 부분 완화(디바운스 FR-01, 합치기 FR-12, no-op 커밋 FR-10)만 이번 범위다.

**Traceability:** RISK-01, RISK-02, FR-15, NFR-07, DEP-07

**Persona:** P2

**[derived — not from an original FR]** **[blocked-on-server]**

**INVEST note:** 고지 확인은 목 계약 없이도 순수 클라이언트 상태(확인 플래그 → 업로드 게이트)로 독립 검증 가능하므로 상시 동의 부여(US-E4-05)와 분리한다.

### US-E4-05 - 상시 동의(standing consent) 부여 및 참조 저장
P1 Vault Owner로서, 설정 시 CLI/config로 상시 동의를 한 번 부여하고 데몬이 그 부여에 대한 참조를 저장하게 하고 싶다. 그래야 이후 업로드가 매번 확인 없이 자동으로 진행된다.

**Acceptance Criteria:**
- Given 고지 확인(US-E4-04)이 기록되어 있고 아직 동의 부여가 없을 때, When 사용자가 CLI `consent grant`(또는 config 플래그)로 동의를 부여하면, Then Watcher는 동의 부여 참조(부여 식별자 + 시각)를 로컬에 저장하고 이후 업로드는 상시 동의 하에 자동 진행된다.
- 불변식 체크리스트:
  - [ ] 동의는 설정 시 **1회 부여**(standing consent)된다 (FR-14).
  - [ ] 동의 부여 **참조**를 로컬에 저장한다 (FR-14).
  - [ ] 유효한 동의 부여가 없으면 업로드/커밋을 차단한다.
- 서버 계약 노트: 동의의 persist/scope/**enforce**는 서버 소유이며(DEP-02), 실서버 전까지는 로컬 캡처만 목 계약으로 검증한다.

**Traceability:** FR-14, DEP-02

**Persona:** P1, P2

**[assumption-based (mock contract)]**

### US-E4-06 - 동의 조회/철회 및 철회 시 로컬 업로드 차단
P2 프라이버시 민감 Vault Owner로서, CLI로 상시 동의를 조회·철회하고, 철회 시 감시·큐는 유지하되 재동의 전까지 모든 업로드/커밋이 차단되게 하고 싶다. 그래야 서버가 아직 강제하지 못하더라도 로컬에서 공개를 즉시 멈출 수 있다.

**Acceptance Criteria:**
- Given 동의 부여가 존재할 때, When 사용자가 CLI `consent view`를 실행하면, Then 부여 참조·부여 시각·현재 상태를 출력한다.
- Given 동의 부여가 존재할 때, When 사용자가 CLI `consent withdraw`를 실행하면, Then Watcher는 감시(FR-01)와 변경 큐 적재(FR-03)를 **유지**하되 재동의 전까지 **모든 업로드/커밋을 차단**한다 (Q6=B).
- Given 동의가 철회된 상태에서 이후 재동의할 때, When 사용자가 CLI로 재동의하면, Then 대기 중이던 큐의 업로드가 재개된다.
- 불변식 체크리스트:
  - [ ] 철회는 감시/큐를 유지, **업로드/커밋만 차단**한다.
  - [ ] 철회는 **전진 방향(forward-only)** — 이미 업로드된 콘텐츠는 회수 불가하다 (RISK-01).
- 서버 계약 노트 **[blocked-on-server]**: 서버 측 동의 강제·철회 반영은 서버 소유(DEP-02); integrate + 승인 + compile 파이프라인은 서버 소유이며 Watcher는 raw 업로드만 수행하므로(DEP-06), 실효적인 서버 측 철회/삭제는 서버 대기다.

**Traceability:** FR-14, DEP-02, DEP-06, RISK-01

**Persona:** P2, P1

**[blocked-on-server]**

---

## E5 히스토리 & 관측성 (History & Observability)

**에픽 가치**: GUI 없이 데몬으로 도는 Watcher에서, P1(데몬 운영자)과 P2(프라이버시 민감 사용자)가 **구조화 로그 + CLI status 명령 + 헬스 체크**를 1차 표면으로 삼아 현재 상태·업로드 이력·중대 오류를 관측하고 감사할 수 있게 한다. 시스템 트레이 아이콘(FR-18)은 데스크톱 세션 편의용 선택 항목일 뿐 어떤 기능도 이에 의존하지 않는다.

### US-E5-01 - 구조화 로그 (PRIMARY 상태 표면)
As a P1 Vault Owner (데몬 운영자), I want the Watcher to emit structured logs of its activity and errors, so that GUI 없이도 데몬의 동작·오류를 한 곳에서 관측하고 사후 분석할 수 있다.

**Acceptance Criteria:**
- Given 데몬이 실행 중이고, When 감시 트리거·매니페스트 계산·프리플라이트·have/want·전송·커밋·재시도 등 수명주기 이벤트가 발생하면, Then 각 이벤트가 타임스탬프·이벤트종류·심각도(severity)·상관관계(cycle) 필드를 가진 **구조화(예: JSON line) 로그 레코드**로 기록된다.
- Given 사이클 처리 중 오류가 발생하고, When 그것이 FR-19의 닫힌 집합에 속하지 않는 비목록(non-listed) 일시적 오류이면, Then 활성 알림 없이 로그로만 기록된다(FR-19와 정합; 실제 알림 동작은 US-E5-04에서 검증).
- 불변식/설정(체크리스트):
  - [ ] 로그는 파일 경로·최소 로그레벨·로테이션(크기/보존 기간)을 config 파일로 설정할 수 있다.
  - [ ] 각 레코드는 기계 판독 가능한 구조화 포맷이며 최소 {timestamp, level, event, cycle_id, message} 필드를 포함한다.
  - [ ] 로그 표면은 GUI/트레이에 의존하지 않는다(헤드리스 데몬에서 동일하게 동작).
  - [ ] 로그는 OS 보안 저장소 밖의 **로컬 평문** 산출물이며 실패 시 오류 상세에 민감정보가 섞일 수 있음이 알려진 수용 위험임을 명시한다(RISK-01 인지; 로컬 산출물 정리는 E6 소관).

**Traceability:** FR-17, NFR-15
**Persona:** P1

### US-E5-02 - CLI status 명령 + 헬스 체크 (데몬 상태 표면)
As a P1 Vault Owner (데몬 운영자), I want a CLI `status` command and a machine-checkable health check, so that GUI/트레이 없이도 데몬의 현재 상태(idle / syncing / offline / error / over-limit)를 즉시 조회하고 서비스 관리자(launchd/systemd/Windows Service)·모니터링에서 헬스를 판정할 수 있다.

**Acceptance Criteria:**
- Given 데몬이 실행 중이고, When 사용자가 `watcher status`를 실행하면, Then 현재 상태, 마지막 성공 동기화 시각, 지속 큐 깊이, 동의 상태, over-limit 여부(Q10=A / FR-19 케이스 3의 지속 상태 반영), 오프라인/백오프 여부를 출력한다.
- Given 모니터링 또는 서비스 관리자가 헬스를 판정해야 하고, When 헬스 체크를 호출하면, Then 정상은 종료코드 0(및 healthy 상태), 비정상은 0이 아닌 종료코드(및 사유)를 반환한다.
- 불변식/설정(체크리스트):
  - [ ] 상태 집합 {idle, syncing, offline, error, over-limit}가 열거·문서화된다.
  - [ ] `--json` 등 기계 판독 가능한 출력 옵션을 제공한다.
  - [ ] status·헬스 체크는 트레이/GUI가 없어도 동작한다(트레이 부재가 상태 관측을 막지 않는다).
  - [ ] over-limit 상태는 한도 이내로 돌아오면 다음 사이클에 자동 해제되어 status에 반영된다(halt/resume 동작 자체는 E2 업로드 프로토콜 소관, 여기서는 그 상태를 표면화만).

**Traceability:** NFR-15
**Persona:** P1
**INVEST note:** status/헬스 체크는 상태를 **표면화**할 뿐이며 상태 전이(over-limit halt/resume 등)는 다른 에픽이 소유하므로, 이 스토리는 표면 계약만으로 독립 개발·테스트(각 상태를 주입해 출력 검증)가 가능하다.
**[derived — not from an original FR]** (Q1=C/Q5=A 데몬 모델에서 NFR-15의 "status indicator"를 CLI+헬스체크로 구체화)

### US-E5-03 - 로컬 append-only 업로드 히스토리 (CLI 조회)
As a P1 Vault Owner (데몬 운영자) and a P2 Privacy-Conscious variant (감사 관점), I want a local, append-only upload history that I can query via CLI, so that 모든 업로드 시도의 결과를 사후 검토·감사하고 무엇이 언제 서버로 나갔는지 추적할 수 있다.

**Acceptance Criteria:**
- Given 업로드 사이클이 성공·실패·부분(partial) 중 하나로 종료되고, When 사이클이 끝나면, Then 히스토리에 한 건이 **추가(append)** 되며 기존 레코드는 절대 수정/삭제되지 않는다.
- Given 사용자가 이력을 보려 하고, When `watcher history`(예: `--since`, `--status`, `--content-id` 필터 포함)를 실행하면, Then 조건에 맞는 레코드가 반환된다.
- 불변식/설정(체크리스트):
  - [ ] 각 레코드는 {vault_content_id/스냅샷 해시, timestamp, status(success/failure/partial), 전송 바이트 수, error 상세}를 모두 담는다.
  - [ ] append-only 불변식: 신규 추가만 허용, 기존 레코드 변경 불가.
  - [ ] 시간·상태·content-id 기준으로 질의 가능하다.
  - [ ] 히스토리는 OS 보안 저장소 밖의 **로컬 평문**이며 error 상세에 민감정보가 섞일 수 있음이 수용 위험임을 명시한다(RISK-01 인지; 정리는 E6 소관).
- 속성형 참조: 히스토리 레코드 직렬화 라운드트립은 무손실이어야 한다("for all history records, deserialize(serialize(r)) == r"). 해당 결정적 속성은 **E7의 NFR-13**에서 검증하며 본 스토리는 이를 소비한다.

**Traceability:** FR-16
**Persona:** P1, P2
**INVEST note:** P1(운영 조회)과 P2(감사 관점)는 같은 append-only 조회 표면을 공유하므로 하나의 굵은 스토리로 유지해도 독립·테스트 가능하다(요구사항의 "얇은 표면은 굵게" 방침).

### US-E5-04 - 중대 오류 활성 표면화 (닫힌 집합)
As a P1 Vault Owner (데몬 운영자), I want the Watcher to actively surface only a closed set of critical conditions while all other transient errors stay log-only, so that 정말로 주의가 필요한 상황만 눈에 띄고 사소한 일시 오류로 인한 알림 피로가 없다.

**Acceptance Criteria:**
- Given 데몬이 실행 중이고, When 다음 닫힌 집합 중 하나가 발생하면 — (1) 인증 실패 / HTTP 401, (2) **N회 연속** 업로드 사이클 실패(N 설정 가능, 기본 3), (3) 로컬 프리플라이트 limit-exceeded(FR-05), (4) 자동 업데이트 롤백(FR-20) — Then 데몬 모델의 활성 표면인 **상향 심각도 구조화 로그 + 헬스 체크 unhealthy 전환 + CLI status의 해당 상태 반영**으로 표면화한다.
- Given 비목록(non-listed) 일시적 오류가 발생하고, When 그것이 위 4개 조건에 속하지 않으면, Then 활성 표면화 없이 로그로만 기록되고 헬스 체크는 healthy를 유지한다.
- Given 데스크톱 세션에 선택적 트레이(US-E5-05)가 존재하고, When 위 조건이 표면화되면, Then 데스크톱 알림을 **추가로** 띄울 수 있으나, 트레이/알림 부재가 로그·헬스·CLI를 통한 표면화를 막지 않는다(트레이 비의존).
- 불변식/설정(체크리스트):
  - [ ] 표면화 조건 집합은 위 4개로 **닫혀** 있으며 그 밖은 로그 전용이다.
  - [ ] 케이스 2의 연속 실패 임계 N은 config로 설정 가능(기본 3)하다.
  - [ ] 데스크톱 팝업은 선택 사항이며 어떤 표면화도 트레이 존재에 의존하지 않는다.

**Traceability:** FR-19, NFR-15
**Persona:** P1
**INVEST note:** 4개 조건의 발생 원천(401은 E4/목 인증 계약, 케이스 3은 E1/E2 프리플라이트, 케이스 4는 E6 롤백)은 각기 다른 에픽 소관이지만, 이 스토리는 그 조건들을 **주입해 표면화 동작만** 검증하므로 독립 개발·테스트가 가능하다(인증 401은 Q3 목 계약으로 시뮬레이션).

### US-E5-05 - 선택적 시스템 트레이/메뉴바 상태 아이콘
As a P1 Vault Owner (데몬 운영자), I want an optional system-tray/menu-bar status icon when I run in a desktop session, so that 데스크톱 환경에서는 상태를 눈으로 힐끗 확인할 수 있다 — 다만 데몬은 이것 없이도 완전히 동작한다.

**Acceptance Criteria:**
- Given 데스크톱 세션과 트레이/메뉴바가 사용 가능하고, When 데몬이 실행되면, Then 현재 상태(idle / syncing / offline / error)를 반영하는 아이콘을 **선택적으로** 표시할 수 있다.
- Given 헤드리스이거나 트레이가 없는 환경이고, When 데몬이 실행되면, Then 모든 상태·관측 정보는 로그(US-E5-01) + CLI status/헬스 체크(US-E5-02) + 중대 오류 표면화(US-E5-04)로 온전히 제공되며 데몬은 정상 동작한다.
- 불변식/설정(체크리스트):
  - [ ] 트레이는 데스크톱 세션 편의 기능이며 **어떤 다른 스토리도 트레이 존재에 의존하지 않는다**(nginx식 데몬은 트레이가 없어도 동작).
  - [ ] 트레이 활성화 여부는 config로 켜고 끌 수 있다.
  - [ ] 이 항목의 채택/제거는 Functional Design에서 최종 확정한다(confirm-or-drop).

**Traceability:** FR-18, NFR-15
**Persona:** P1
**INVEST note:** 트레이는 순수 부가 표면으로 다른 모든 관측 스토리와 독립이며, 제거되어도 나머지 스토리에 영향이 없다.
**[optional — confirm-or-drop]**

---

## E6 수명주기 & 업데이트 (Lifecycle & Updates)

Watcher를 nginx처럼 GUI 없는 OS 백그라운드 서비스로 설치·자동시작하고, 헬스 체크 기반 자동 롤백이 붙은 자동 업데이트·실행 상태 제어·안전한 폐기까지 데몬의 전체 수명주기를 관리하여 방치형 데몬이 안정적으로 오래 살아 있게 한다.

### US-E6-01 - OS 서비스로 설치 및 자동시작

P1 Vault Owner(데몬 운영자)로서, 나는 Watcher를 OS 서비스(launchd / systemd / Windows Service)로 설치하고 부팅/로그인 시 자동으로 기동되게 하고 싶다. 그래야 nginx처럼 GUI 없이 백그라운드에서 항상 볼트를 감시·동기화할 수 있다.

**Acceptance Criteria:**

행위/이벤트 흐름 (Given-When-Then):
- Given 설치 CLI가 있는 상태에서, When `watcher install`(또는 동등 명령)을 실행하면, Then 현재 OS에 맞는 서비스 유닛이 등록된다 — macOS는 launchd, Linux는 systemd 유닛, Windows는 Windows Service.
- Given 서비스가 등록된 상태에서, When 머신을 재부팅하거나 사용자가 로그인하면, Then Watcher 데몬이 자동으로 기동되어 idle 상태에 도달한다.
- Given 데몬이 실행 중일 때, When `watcher status`를 실행하거나 헬스 체크를 조회하면, Then 현재 실행 상태(idle / syncing / offline / error)와 서비스 등록 상태가 반환된다.
- Given GUI/트레이가 없는 세션(헤드리스 서버 포함)에서, When 데몬이 기동하면, Then 트레이 없이도 정상 동작하고 상태는 구조적 로그 + CLI status + 헬스 체크로만 관측된다.

불변식/설정 (체크리스트):
- [ ] config 파일에서 서비스 실행 계정/작업 디렉터리/자동시작 여부를 설정할 수 있다.
- [ ] 어떤 스토리도 트레이(FR-18) 존재에 의존하지 않는다 — 트레이는 선택적 편의 기능일 뿐, primary 관측 표면은 로그 + CLI status + 헬스 체크다.
- [ ] 데몬은 장기 실행 프로세스로서 재시작 후에도 config·상태를 이어받는다.

NFR-05 이식성 (체크리스트):
- [ ] 서비스 매니저 통합: macOS launchd / Linux systemd / Windows Service 각각 네이티브 메커니즘 사용.
- [ ] 네이티브 fs-watch: FSEvents / inotify / ReadDirectoryChangesW.
- [ ] 세 OS 모두에서 동일한 CLI 계약(install / uninstall / start / stop / status)을 제공.

**Traceability:** FR-21, NFR-05
**Persona:** P1
**INVEST note:** 설치·자동시작은 자동 업데이트(US-E6-02)·실행 상태 제어(US-E6-03)와 분리 가능하며 각각 단독으로 가치를 제공한다.

### US-E6-02 - 자동 업데이트 + 헬스 체크 실패 시 자동 롤백

P1 Vault Owner(데몬 운영자)로서, 나는 Watcher가 새 버전으로 자동 업데이트하되 업데이트 후 헬스 체크에 실패하면 직전 정상 버전으로 자동 롤백하기를 원한다. 그래야 방치형 데몬이 업데이트로 인해 조용히 죽지 않는다.

**Acceptance Criteria:**

행위/이벤트 흐름 (Given-When-Then):
- Given 새 버전이 배포되어 업데이트가 적용되면, When 새 버전이 기동을 시도하면, Then 데몬은 업데이트 후 헬스 체크를 수행한다: (1) 새 버전이 시작되고, (2) idle 상태에 도달하며, (3) 자격증명/토큰 소스(config/env의 토큰)를 bounded time 내에 읽을 수 있는지 확인한다.
- Given 헬스 체크의 세 조건을 모두 제한 시간 내에 통과하면, When 검증이 끝나면, Then 새 버전이 정상 버전으로 확정되고 이전 버전 아티팩트는 롤백 지점으로 보존된다.
- Given 헬스 체크 중 하나라도 실패하거나 제한 시간을 초과하면, When 실패가 확정되면, Then 직전 정상 버전으로 자동 롤백하고 데몬은 이전 버전으로 다시 idle에 도달한다 (NFR-16).
- Given 롤백이 발생하면, When 롤백이 완료되면, Then 이 사건이 구조적 로그에 기록되고 CLI status/헬스 체크로 노출되며, FR-19 케이스 4 알림 경로로 전달된다(알림 자체는 관측성 에픽 소유 — 상호참조).

불변식/설정 (체크리스트):
- [ ] 헬스 체크 제한 시간(bounded time)은 config로 설정 가능(구체 기본값은 Functional/NFR Design로 이월).
- [ ] 롤백 후 데몬은 자동 업데이트를 다시 시도하기 전 백오프/보류 상태를 유지해 롤백 루프를 피한다.
- [ ] 자동 업데이트 채널/활성 여부는 config로 제어 가능.

NFR-05 이식성 (체크리스트):
- [ ] 업데이트 메커니즘이 macOS/Windows/Linux 각각 네이티브 방식과 호환되며, 서비스 재시작을 서비스 매니저와 연동한다.
- [ ] 자격증명/토큰 소스 읽기 검증이 각 OS의 primary 경로(config/env, keyring-less)에서 동작하고 OS 보안 저장소는 선택적 강화 경로로 취급한다.

**Traceability:** FR-20, NFR-16, NFR-05
**Persona:** P1

### US-E6-03 - 실행 상태 제어: 일시정지/재개 + 지금 동기화 + 중지

P1 Vault Owner(데몬 운영자)로서, 나는 CLI와 config로 감시를 일시정지/재개하고, 수동으로 "지금 동기화"를 트리거하고, 서비스를 중지/종료할 수 있기를 원한다. 그래야 GUI 없이도 데몬의 실행 상태를 직접 제어할 수 있다. **[derived — not from an original FR]**

**Acceptance Criteria:**

행위/이벤트 흐름 (Given-When-Then):
- Given 데몬이 감시 중일 때, When `watcher pause`를 실행하면, Then 파일시스템 감시(FR-01)가 일시정지되고 상태가 paused로 표시된다; When `watcher resume`을 실행하면, Then 감시가 재개된다.
- Given 데몬이 idle/paused 상태일 때, When `watcher sync-now`를 실행하면, Then 디바운스 대기 없이 한 번의 업로드 사이클(FR-06 경로)이 즉시 트리거된다.
- Given 서비스가 실행 중일 때, When `watcher stop`(또는 서비스 매니저의 stop)을 실행하면, Then 데몬이 진행 중 작업을 안전하게 마무리하거나 지속 큐에 보존한 뒤 종료한다.
- Given pause/resume/stop 상태를 config로도 지정하면, When 데몬이 config를 다시 읽으면, Then 설정된 실행 상태를 반영한다.

불변식/설정 (체크리스트):
- [ ] 일시정지 상태는 재시작 후에도 보존된다(config/상태 파일에 기록).
- [ ] 모든 제어는 CLI + config로만 노출된다 — 트레이 의존 없음.
- [ ] `sync-now`는 멱등(FR-10)하므로 변경이 없으면 no-op 커밋으로 끝난다.

**Traceability:** FR-21, FR-01, FR-06 (derived per Q11=D)
**Persona:** P1
**INVEST note:** 세 제어(일시정지/재개, sync-now, 중지)는 하나의 "실행 상태 제어" 표면으로 묶여 단독 배포·테스트가 가능하며, FR-01 감시나 FR-06 동기화 스토리와 독립적으로 시연할 수 있다.

### US-E6-04 - 제거/폐기 정리

P2 프라이버시 민감 Vault Owner로서, 나는 Watcher를 제거/폐기할 때 로컬 평문 산출물(업로드 히스토리 FR-16, 매니페스트 FR-02, 지속 큐 FR-03, 로그 FR-17)과 저장된 토큰(FR-13)을 삭제하고 OS 서비스 등록을 해제할 수 있기를 원한다. 그래야 디스크에 민감한 평문 흔적(RISK-01)을 남기지 않고 데몬을 깨끗이 내릴 수 있다. **[derived — not from an original FR]**

**Acceptance Criteria:**

행위/이벤트 흐름 (Given-When-Then):
- Given Watcher가 서비스로 설치되어 로컬 상태를 축적한 상태에서, When `watcher uninstall`(또는 decommission)을 실행하면, Then OS 서비스 등록이 해제되고(launchd / systemd / Windows Service) 데몬이 중지된다.
- Given 제거 절차가 진행되면, When 정리 단계가 실행되면, Then 로컬 평문 산출물 — 업로드 히스토리(FR-16), 매니페스트(FR-02), 지속 큐(FR-03), 구조적 로그(FR-17) — 가 삭제된다.
- Given 저장된 API 토큰(FR-13)이 있으면, When 제거가 완료되면, Then config/env의 토큰과 (선택적 경로 사용 시) OS 보안 저장소의 토큰이 함께 제거된다.
- Given 제거가 완료되면, When 사용자가 결과를 확인하면, Then 어떤 아티팩트가 삭제되었는지 요약이 로그/CLI 출력으로 제공된다.

불변식/설정 (체크리스트):
- [ ] 정리는 이 스토리가 소유한 로컬 산출물에 한정되며 볼트 원본 파일은 절대 건드리지 않는다.
- [ ] 제거는 데몬이 실행 중이 아닐 때도 안전하게 동작(idempotent)하며, 이미 삭제된 항목은 no-op다.
- [ ] 부분 실패 시(권한 등) 어떤 항목이 남았는지 명확히 보고한다.

NFR-05 이식성 (체크리스트):
- [ ] 서비스 등록 해제가 macOS/Windows/Linux 각각 네이티브 서비스 매니저로 수행된다.
- [ ] 토큰/보안 저장소 정리가 각 OS 경로(config/env primary, OS 보안 저장소 선택적)에서 동작한다.

**Traceability:** FR-21, FR-16, FR-02, FR-03, FR-17, FR-13, RISK-01, NFR-05 (derived per Q13=A)
**Persona:** P1, P2
**INVEST note:** 폐기 정리는 스크래치 설치 픽스처로 독립 검증 가능하며, 다른 수명주기 스토리와 분리해 시연·테스트할 수 있다.

---

## E7 결정적 코어 속성 검증 & 회복탄력성 트랙 (Resilience & Property-Based Testing)

결정적 코어(해싱·매니페스트·have/want·재개·합치기·재조정·직렬화·용량검증·지속 큐)가 **임의 입력에 대해** 증명 가능하게 올바르고 손실 없이 동작함을 속성 기반 테스트로 보증하여, Watcher를 데몬으로 운영하는 P1의 볼트 데이터가 크래시·재시작·오프라인·재개를 겪어도 절대 유실/오염되지 않게 한다. [Q8=D 별도 기술 스토리 트랙]

**트랙 수준 참고 (Track-level notes):**
- 이 트랙의 모든 스토리는 자신이 보호하는 기능 스토리(E1~E6)와 상호 연결(Cross-links)되며, 인수 조건은 속성형("모든 … 에 대해 …")으로 표현한다. 데몬 모델이므로 관측은 GUI/트레이가 아니라 결정적 코어의 테스트 하니스·구조화 로그·CLI status·헬스체크로 이루어진다. [Q8=D, Q12=C]
- **PBT-01**: 각 속성의 최종 형태(생성기/불변식 포함)는 **Functional Design** 단계에서 확정한다. 이 트랙의 스토리는 속성의 의도와 경계를 고정하고, 구체 생성기는 Functional Design으로 이월한다.
- **PBT-08**: 모든 속성 하니스는 **축소(shrinking)**, **고정 시드(fixed seeds) 재현성**, **CI 통합**을 갖춘다.
- **PBT-09**: 속성 프레임워크 선택은 NFR Requirements 단계의 기술 스택 결정(NFR-17)에 종속된다 — 코어가 Rust로 확정되면 **proptest**를 채택한다(확정 대상, US-E7-10 참조).
- **RESILIENCY-14** 회복탄력성 테스트(크래시 주입 / 오프라인·재개 시뮬레이션 / 이벤트 드롭 시뮬레이션)는 상세 형태를 **NFR Design / Operations**로 이월한다(US-E7-11).

### US-E7-01 - 매니페스트/스냅샷 해싱 결정성 속성 (NFR-08)
As a Vault Owner (P1), I want 매니페스트/스냅샷 해싱이 임의의 파일 내용에 대해 결정적이며 파일별 해시가 `sha256sum` / okc-core `raw_sha256`과 정확히 일치함을 속성 테스트로 보장받기를, so that 클라이언트와 서버의 content-id/dedup 정렬(FR-02, FR-07)이 신뢰 가능하고 내 볼트가 정확히 주소지정된다.

**Acceptance Criteria:**
- 속성형: 모든 파일 내용(임의 바이트열, 빈 파일 포함)에 대해, 해시를 반복 계산하면 항상 동일한 값을 낸다(결정성).
- 속성형: 모든 파일 내용에 대해, Watcher가 계산한 `raw_sha256` == 표준 `sha256sum` 결과 == okc-core `raw_sha256`.
- 속성형: 모든 볼트 상태(임의 파일 집합/경로)에 대해, `vault_content_id`는 okc-core 스킴과 동일하게 산출된다.
- [ ] 하니스는 shrinking·고정 시드·CI 통합을 갖춘다 (PBT-08).
- [ ] 최종 속성/생성기 형태는 Functional Design에서 확정 (PBT-01).

**Cross-links:** E2 업로드 프로토콜의 FR-02 매니페스트 계산 스토리(및 FR-07 have/want 정렬).
**Traceability:** NFR-08, FR-02, PBT-01, PBT-08
**Persona:** P1

### US-E7-02 - have/want 정확성·멱등성 속성 (NFR-09)
As a Vault Owner (P1), I want have/want 계산이 `want == manifest_paths \ server_has`로 정확하고 재실행 시 빈 want를 내는 멱등성을 속성 테스트로 보장받기를, so that 변경되지 않은 파일은 재전송되지 않고(NFR-01) 데몬이 대용량 볼트를 반복 사이클에서 낭비 없이 동기화한다.

**Acceptance Criteria:**
- 속성형: 모든 (manifest_paths, server_has) 쌍에 대해, `want == manifest_paths \ server_has` (집합 차집합과 정확히 일치).
- 속성형: 모든 입력에 대해, 서버가 want 집합을 보유한 상태로 have/want를 재실행하면 결과 want는 공집합이다(멱등성).
- 속성형: server_has ⊇ manifest_paths인 모든 경우에 want는 공집합이다(전부 보유 시 no-op, FR-10 정합).
- [ ] shrinking·고정 시드·CI 통합 (PBT-08); 최종 속성 형태는 Functional Design 확정 (PBT-01).

**Cross-links:** E2 업로드 프로토콜의 FR-07 have/want 협상 스토리.
**Traceability:** NFR-09, FR-07, PBT-01, PBT-08
**Persona:** P1

### US-E7-03 - 재개 청크 재조립 바이트 일치 속성 (NFR-10)
As a Vault Owner (P1), I want 재개 가능(resumable) 청크 재조립이 임의 오프셋에서 재개해도 원본 blob과 바이트 단위로 완전히 일치함을 속성 테스트로 보장받기를, so that 대용량/초기 업로드가 중단 후 재개되어도(FR-08) 서버가 받는 바이트가 매니페스트 해시와 어긋나지 않는다.

**Acceptance Criteria:**
- 속성형: 모든 blob 내용과 모든 청크 분할에 대해, 청크를 순서대로 연결하면 원본 blob을 **바이트 단위로** 재구성한다(round-trip).
- 속성형: 모든 유효한 재개 오프셋에 대해, 그 오프셋에서 재개해 재조립한 결과는 중단 없이 전송한 결과와 동일하다.
- 속성형: 재조립된 blob의 `raw_sha256`은 매니페스트에 기록된 해시(FR-22)와 일치한다.
- [ ] shrinking·고정 시드·CI 통합 (PBT-08); 최종 속성 형태는 Functional Design 확정 (PBT-01).

**Cross-links:** E2 업로드 프로토콜의 FR-08 재개 가능 청크 업로드 스토리(및 FR-22 스냅샷 일관성).
**Traceability:** NFR-10, FR-08, PBT-01, PBT-08
**Persona:** P1

### US-E7-04 - 합치기 latest-state-wins 속성 (NFR-11)
As a Vault Owner (P1), I want 오프라인/백오프 중 대기 변경 합치기(coalescing)가 임의의 교차(interleaving)에 대해 "최신 상태 우선"을 보존함을 속성 테스트로 보장받기를, so that 재연결 후 서버로 배출되는 상태가 항상 각 파일의 가장 최신 스냅샷이다(FR-12).

**Acceptance Criteria:**
- 속성형: 같은 파일에 대한 모든 대기 변경의 임의 교차 순서에 대해, 합치기 결과는 그 파일의 가장 최신 상태 하나만 남긴다.
- 속성형: 서로 다른 파일들이 섞인 모든 대기 시퀀스에 대해, 합치기 후 각 파일 항목은 정확히 그 파일의 최신 상태를 가진다(누락/중복 없음).
- 속성형: 삭제 후 재생성, 재생성 후 삭제 등 모든 상태 전이 순서에 대해, 최종 결과는 마지막으로 발생한 상태와 일치한다.
- [ ] shrinking·고정 시드·CI 통합 (PBT-08); 최종 속성 형태는 Functional Design 확정 (PBT-01).

**Cross-links:** E3 오프라인·백프레셔·재시도의 FR-12 합치기 스토리; US-E7-08(상태 기반 큐)과 정합.
**Traceability:** NFR-11, FR-12, PBT-01, PBT-08
**Persona:** P1

### US-E7-05 - 재조정 diff 정확 집합 속성 (NFR-12)
As a Vault Owner (P1), I want 재조정 `diff(last_committed, current)`가 임의의 before/after 볼트 상태에 대해 **정확히** 변경된 집합만 반환함을 속성 테스트로 보장받기를, so that 재조정 스캔(FR-04)이 변경을 빠뜨리거나 과잉 보고하지 않고 놓친 변경만 복구한다.

**Acceptance Criteria:**
- 속성형: 모든 (last_committed, current) 매니페스트 쌍에 대해, `diff`가 반환하는 집합은 추가·수정·삭제된 파일의 집합과 정확히 동일하다(누락 0, 오탐 0).
- 속성형: 모든 상태에 대해, `current == last_committed`이면 `diff`는 공집합이다.
- 속성형: 모든 상태에 대해, diff가 지목한 파일에 대해서만 해시/존재가 실제로 달라진다(변경 집합의 정확성).
- [ ] shrinking·고정 시드·CI 통합 (PBT-08); 최종 속성 형태는 Functional Design 확정 (PBT-01).

**Cross-links:** E1 모니터링 & 트리거링의 FR-04 재조정 스캔 및 FR-02 매니페스트 diff 스토리.
**Traceability:** NFR-12, FR-04, FR-02, PBT-01, PBT-08
**Persona:** P1

### US-E7-06 - 직렬화 무손실 round-trip 속성 (NFR-13)
As a Vault Owner (P1), I want 매니페스트·큐 항목·히스토리 레코드의 직렬화/역직렬화가 임의 값에 대해 무손실 round-trip임을 속성 테스트로 보장받기를, so that 크래시/재시작 후 디스크에서 복구된 상태(FR-03)와 히스토리(FR-16)가 원래 값과 정확히 같다.

**Acceptance Criteria:**
- 속성형: 모든 매니페스트 값에 대해, `deserialize(serialize(m)) == m`.
- 속성형: 모든 큐 항목(FR-03)에 대해, `deserialize(serialize(e)) == e`.
- 속성형: 모든 히스토리 레코드(FR-16: 해시/타임스탬프/상태/바이트/오류상세)에 대해, `deserialize(serialize(r)) == r` (자유형 오류 문자열·유니코드 포함).
- [ ] shrinking·고정 시드·CI 통합 (PBT-08); 최종 속성 형태는 Functional Design 확정 (PBT-01).

**Cross-links:** E3 지속 큐(FR-03), E1/E2 매니페스트(FR-02), E5 히스토리(FR-16) 스토리.
**Traceability:** NFR-13, FR-02, FR-03, FR-16, PBT-01, PBT-08
**Persona:** P1

### US-E7-07 - SafetyLimits 경계 정확·단조성 속성 (NFR-14)
As a Vault Owner (P1), I want 클라이언트 SafetyLimits 사전검사가 임의 입력에 대해 경계 정확(boundary-correct)하고 단조(monotonic)임을 속성 테스트로 보장받기를, so that 사전검사 거부(FR-05)가 한도 경계에서 일관되고, 파일/바이트를 더해도 거부가 수락으로 뒤집히지 않는다.

**Acceptance Criteria:**
- 속성형: 모든 볼트 상태에 대해, 사전검사 판정은 한도(≤20 GiB 총량, ≤2 GiB/파일, ≤100k 파일) 정확히 경계에서 accept/reject를 가른다(경계 오프바이원 없음).
- 속성형(단조성): 이미 reject된 상태에 파일 또는 바이트를 추가한 모든 경우, 판정이 accept로 뒤집히지 않는다.
- 속성형: 세 한도 중 하나라도 초과하는 모든 상태에 대해, 판정은 reject이며 어떤 한도를 초과했는지 보고한다(FR-05 실행 가능한 리포트).
- [ ] shrinking·고정 시드·CI 통합 (PBT-08); 최종 속성 형태는 Functional Design 확정 (PBT-01).
- 참고: 프로젝트 수준 ≤10-sources 한도는 **서버 권위 검증**(DEP-04)이므로 이 클라이언트 속성 범위 밖 — US-E7 클라이언트 검사는 볼트 단위 3개 한도만 대상.

**Cross-links:** E2 업로드 프로토콜의 FR-05 사전검사 스토리 및 US-E2-08(런타임 용량 초과 처리).
**Traceability:** NFR-14, FR-05, PBT-01, PBT-08
**Persona:** P1

### US-E7-08 - 지속 큐 상태 기반(model-based) 명령 시퀀스 속성 (PBT-06)
As a Vault Owner (P1), I want 지속 큐(FR-03)가 임의의 명령 시퀀스(enqueue/dequeue/coalesce/persist/recover)에 대해 참조 모델과 관측적으로 동일하게 동작함을 상태 기반 속성 테스트로 보장받기를, so that 크래시/재시작을 포함한 어떤 조작 순서에서도 큐 상태가 오염되거나 변경이 유실되지 않는다.

**Acceptance Criteria:**
- 속성형: 모든 명령 시퀀스(enqueue/dequeue/coalesce/persist/recover의 임의 교차)에 대해, 실제 큐의 관측 가능한 상태는 참조 모델의 상태와 항상 일치한다.
- 속성형: 시퀀스 중 임의 지점에 persist→recover(크래시-복구 모사)를 삽입해도, 복구 후 큐 상태는 크래시가 없었을 때와 관측적으로 동일하다(NFR-03과 정합).
- 속성형: 같은 파일에 대한 임의 교차 enqueue에 대해, coalesce 후 최신 상태만 남는다(NFR-11과 정합).
- [ ] 참조 모델·명령 집합의 최종 형태는 Functional Design에서 확정 (PBT-01/PBT-06).
- [ ] shrinking·고정 시드·CI 통합 (PBT-08).

**INVEST note:** 다른 무상태 속성 스토리(US-E7-01..07)와 달리 상태·명령 시퀀스 모델링이 필요해 독립된 스토리로 분리 — 단일 컴포넌트(FR-03 큐)에 국한되어 여전히 Small/Testable.

**Cross-links:** E3 오프라인·백프레셔·재시도의 FR-03 지속 큐 스토리 및 FR-12 합치기 스토리; US-E7-04, US-E7-09와 연결.
**Traceability:** PBT-06, FR-03, PBT-01, PBT-08
**Persona:** P1

### US-E7-09 - 크래시/재시작 zero-loss + T_recon 검출 지연 속성 (NFR-03)
As a Vault Owner (P1), I want 관측/적재/커밋된 어떤 변경도 크래시·재시작을 넘어 유실되지 않고, 미관측 변경은 T_recon 이내에 검출됨을 속성/시나리오 테스트로 보장받기를, so that 데몬이 다운되었다 살아나도 내 편집이 하나도 사라지지 않는다.

**Acceptance Criteria:**
- Given 큐에 적재되었거나 커밋된 변경이 있고, When 프로세스가 임의 시점에 크래시 후 재시작되면, Then 적재/커밋된 변경은 하나도 유실되지 않는다.
- 속성형: 관측/적재/커밋된 모든 변경 집합에 대해, 크래시·재시작 후 재구성된 상태는 그 변경들을 전부 포함한다(zero loss).
- 속성형: 파일시스템 이벤트로 관측되지 못한 모든 변경에 대해, 재조정 스캔(FR-04)이 T_recon 이내에 이를 검출한다(검출 지연 ≤ T_recon).
- [ ] 인수 기준: (i) 적재/커밋 변경 zero loss, (ii) 미관측 변경 검출 지연 ≤ T_recon.
- [ ] shrinking·고정 시드·CI 통합 (PBT-08); 최종 속성/시나리오 형태는 Functional Design 확정 (PBT-01).

**Cross-links:** E3 지속 큐(FR-03), E1 재조정 스캔(FR-04) 스토리; US-E7-08(상태 기반 큐), US-E7-11(크래시 주입 테스트)과 연결.
**Traceability:** NFR-03, FR-03, FR-04, PBT-01, PBT-08
**Persona:** P1

### US-E7-10 - 속성 프레임워크/기술 스택 가정 확정 (NFR-17)
As a Vault Owner (P1), I want 결정적 코어의 기술 스택과 속성 프레임워크 가정이 NFR Requirements 단계에서 명시적으로 확정되기를, so that PBT-09 프레임워크 의무가 실제 코어 언어와 정합하고 US-E7-01..09 속성이 동일한 하니스로 실행된다.

**Acceptance Criteria:**
- [ ] **가정(assumption-to-confirm):** 결정적 코어(해싱·스냅샷·매니페스트·dedup)는 Rust로 구현되어 **proptest**를 채택하고 okc-core 프리미티브(`raw_sha256`, `vault_content_id`)를 재사용한다 — NFR Requirements 단계에서 확정.
- [ ] Rust가 아닌 결정이 나면 PBT-09를 충족하는 대체 프레임워크를 재선정하고, 그 결정을 US-E7-01..09 하니스에 반영한다.
- [ ] 확정된 프레임워크는 shrinking·고정 시드·CI 통합을 지원한다(PBT-08 전제).

**INVEST note:** 결과물이 아니라 다른 모든 트랙 스토리의 실행 전제를 고정하는 결정 스토리 — 의도적으로 얇고 NFR Requirements 단계에서 확정(Negotiable). 이 트랙에서 명시적으로 소유해 미결로 흘러가지 않게 한다.

**Cross-links:** US-E7-01..09 전체(공통 하니스 전제); NFR Requirements 단계 기술 스택 선정.
**Traceability:** NFR-17, PBT-09, PBT-08
**Persona:** P1

### US-E7-11 - 회복탄력성 테스트 전략 (크래시 주입·오프라인/재개·이벤트 드롭) (RESILIENCY-14)
As a Vault Owner (P1), I want 크래시 주입·오프라인/재개 시뮬레이션·이벤트 드롭 시뮬레이션으로 데몬의 회복탄력성을 검증하는 테스트 전략이 트랙 항목으로 소유되기를, so that 순수 속성 테스트를 넘어서는 실패 모드(중단·재연결·이벤트 유실)에서도 zero-loss 목표(NFR-03)가 실증된다.

**Acceptance Criteria:**
- [ ] 회복탄력성 테스트 범위 = 크래시 주입, 오프라인→재연결/재개 시뮬레이션, 파일시스템 이벤트 드롭 시뮬레이션.
- [ ] 크래시 주입 테스트: 업로드/큐 조작 중 임의 지점 종료 후 재시작해도 적재/커밋 변경 zero loss(US-E7-09 검증).
- [ ] 이벤트 드롭 시뮬레이션: 이벤트가 유실된 경우에도 재조정(FR-04)이 T_recon 이내에 변경을 검출.
- [ ] 재개 시뮬레이션: 전송 중단 후 재개가 바이트 일치를 보존(US-E7-03 검증).
- 참고: 상세 설계·구현 형태는 **NFR Design / Operations**로 이월(deferred) — 이 스토리는 트랙에서 범위/책임만 고정.

**INVEST note:** 상세는 이월되지만 트랙에서 명시 소유해 RESILIENCY-14가 누락되지 않게 하는 전략 스토리 — 범위가 명확해 Negotiable/Testable.

**Cross-links:** US-E7-09(zero-loss/T_recon), US-E7-03(재개 바이트 일치), US-E7-08(상태 기반 큐); E3 지속 큐(FR-03)/오프라인 재시도(FR-11), E1 재조정(FR-04), E2 재개 업로드(FR-08).
**Traceability:** RESILIENCY-14, FR-03, FR-04, FR-08
**Persona:** P1

---

## 요구사항 커버리지 매트릭스 (traceability)

크리틱 검증 결과: 인벤토리 55개 ID(FR-01..23, NFR-01..17, RISK-01/02, DEP-01..07, PBT-06 + 프로세스 노트 PBT-01/08/09)가 모두 **소유 스토리 ≥1** 또는 명시적 blocked-on-server / optional / deferred 노트로 커버됨. 미커버 0.

| 요구사항 | 소유 스토리 | 비고 |
|---|---|---|
| FR-01 감시+디바운스 | US-E1-01 | 상호참조 US-E6-03(pause/resume), US-E3-04 |
| FR-02 매니페스트+diff | US-E1-02 | 속성 US-E7-01/05/06; 정리 US-E6-04 |
| FR-03 지속 큐 | US-E3-01 | 속성 US-E7-06/08/09; 정리 US-E6-04 |
| FR-04 재조정 스캔 | US-E1-03, US-E1-04 | 시작 스캔 + 주기 백스톱; 속성 US-E7-05/09 |
| FR-05 SafetyLimit 사전검사 | US-E2-02, US-E2-08 | 속성 US-E7-07 |
| FR-06 원시 볼트 업로드 | US-E2-05 | MVP; 트리거 골격 US-E1-01, sync-now US-E6-03 |
| FR-07 have/want | US-E2-03 | MVP; 속성 US-E7-02 |
| FR-08 재개 전송 | US-E2-04, US-E2-07 | MVP; 속성 US-E7-03 |
| FR-09 커밋/바인딩 | US-E2-05 | 빈-커밋 가드 US-E1-06 |
| FR-10 멱등성 | US-E2-06 | |
| FR-11 백오프 재시도 | US-E3-03 | |
| FR-12 coalesce | US-E3-02 | 속성 US-E7-04 |
| FR-13 토큰 인증 | US-E4-01 | secure store US-E4-02, 재입력 US-E4-03, 정리 US-E6-04 |
| FR-14 상시 동의 | US-E4-05, US-E4-06 | 부여 + 조회/철회 |
| FR-15 민감 콘텐츠 미필터 | US-E4-04 | AC-level (부정형 SHALL-NOT) |
| FR-16 업로드 히스토리 | US-E5-03 | 정리 US-E6-04; 속성 US-E7-06 |
| FR-17 구조화 로그 | US-E5-01 | 정리 US-E6-04 |
| FR-18 트레이 아이콘 | US-E5-05 | **[optional — confirm-or-drop]** |
| FR-19 중대 오류 표면화 | US-E5-04 | 케이스 원천: US-E4-03(1), US-E2-08(3), US-E6-02(4) |
| FR-20 자동 업데이트+롤백 | US-E6-02 | |
| FR-21 데몬/서비스 자동시작 | US-E6-01, US-E6-03, US-E6-04 | |
| FR-22 일관 스냅샷 | US-E2-01 | 속성 US-E7-03 |
| FR-23 단일 인스턴스 잠금 | US-E1-05 | |
| NFR-01 변경 감지 지연 | US-E1-01, US-E1-02 | AC |
| NFR-02 해싱 확장성 | US-E2-01 | AC US-E1-02 |
| NFR-03 무손실 | US-E7-09 | 속성; 상호참조 US-E1-03, US-E3-01 |
| NFR-04 타임아웃/백오프/오프라인 | US-E3-03, US-E3-04 | |
| NFR-05 이식성 | US-E1-01, US-E6-01/02/04 | AC |
| NFR-06 자격증명/TLS | US-E4-01, US-E4-02 | |
| NFR-07 클라 필터 없음 | US-E4-04 | AC |
| NFR-08..14 결정적 속성 | US-E7-01..07 | |
| NFR-15 관측성 | US-E5-01/02/04/05 | |
| NFR-16 롤백 | US-E6-02 | |
| NFR-17 기술스택/proptest | US-E7-10 | assumption-to-confirm |
| PBT-06 상태기반 큐 | US-E7-08 | |
| PBT-01 / PBT-08 | US-E7-01..09 전반 | 프로세스 노트 |
| PBT-09 프레임워크 선정 | US-E7-10 | |
| RISK-01 소스 노출 | US-E4-04(고지), US-E4-06, US-E5-01/03, US-E6-04 | |
| RISK-02 재승인 thrash | US-E4-04 | 노트 **[blocked-on-server]** |
| DEP-01 토큰 발급/회전 | US-E4-01, US-E4-03 | 목 계약 |
| DEP-02 동의 persist/강제 | US-E4-05, US-E4-06 | **[blocked-on-server]** |
| DEP-03 업로드 엔드포인트 | US-E2-03/04/05 | 목 계약 |
| DEP-04 권위 SafetyLimit(≤10 sources) | US-E2-02 | **[blocked-on-server]** |
| DEP-05 멀티테넌시 | US-E4-01 | 서버 소유 |
| DEP-06 integrate/승인/compile | US-E4-06 | 서버 소유 |
| DEP-07 고승인 감내 | US-E4-04 | **[blocked-on-server]** |
| RESILIENCY-14 회복탄력성 테스트 | US-E7-11 | NFR Design/Operations로 이월(deferred) |

## 미결 / 확인 항목 (Workflow Planning 또는 다음 단계로 이월)

1. **FR-18 트레이 아이콘** — 데몬 모델에서 순수 선택 항목으로 표시(US-E5-05, `[optional — confirm-or-drop]`). 채택/제거를 Functional Design에서 확정.
2. **토큰 저장 경로** — ✅ **확정 (2026-09-08, 사용자 "config에 둬" + "config 기본 + 보안저장소 opt-in 유지")**: 1차·기본 저장 = 단일 JSON config 평문 `token` 필드(+ env 폴백, keyring-less); OS 보안 저장소(US-E4-02)는 **선택적 opt-in 강화로 유지**(헤드리스 시 config로 안전 폴백). config 토큰은 RISK-01 로컬 평문 산출물에 포함(US-E6-04 정리 대상). FR-13/NFR-06 원문("OS 보안 저장소")은 `requirements.md` §13 부록으로 정정. US-E4-01(config 1차)/US-E4-02(선택적 강화) 스토리는 이 확정과 이미 정합.
3. **MVP 집합** — 현재 MVP 플래그는 전송 골격(FR-01/06/07/08/03)만 표시. 최소 동작 경로의 전제 의존(FR-02 매니페스트, FR-22 스냅샷, FR-13 인증)의 우선순위 확정은 Workflow Planning에서.
4. **서버 대기 항목** — DEP-02/04/06/07 및 RISK-01의 실효적 철회는 미구축 서버 의존. 실서버 계약 확정 시 재검토.

## FQ-3=A 경미 수정 부록 (Application Design 정합)

**추가일**: 2026-09-08 · **근거**: FQ-1=A(okc-core 코드 의존성 0), FQ-2=A(최신 상태 대체 모델), FQ-3=A(영향 스토리를 단순 모델에 맞게 **경미 수정**).

> 아래는 위 스토리들의 **원문을 대체하지 않는 오버레이**다. 원문은 사용자 승인 기록으로 보존하고, 단순화 아키텍처와 어긋나는 문구만 정정한다. 각 스토리의 **가치(so that)와 수용 의도는 불변**이며, 달성 메커니즘 표현만 바뀐다. 요구사항 정정 원본은 `requirements.md` §12 참조.

### FQ-1=A — 표준 SHA-256 (okc-core 재현 제거)
- **US-E7-01** (해싱 결정성, NFR-08): "`okc-core raw_sha256`과 일치" → **표준 `sha256sum`과 일치**로 정정. okc-core 인용 삭제, 결정성·주소지정 프로퍼티는 불변.
- **US-E1-02**(line 54/57)·**US-E2-01**(line 149 계열)의 "okc-core와 동일 SHA-256 스킴"/"okc-core `raw_sha256`" 표현도 동일하게 **표준 SHA-256(sha256sum)** 으로 읽는다. `vault_content_id`의 권위 계산은 서버 소유(클라이언트는 `manifest_digest`만 유지).
- **US-E7-10**(기술 스택, line 832): "okc-core 프리미티브 재사용" 가정 **삭제** — okc-core 의존성 0, 표준 SHA-256 Rust 직접 구현. proptest 채택 근거·PBT-09 의무는 불변.

### FQ-2=A — 최신 상태 대체 모델 (지속 이벤트 큐 제거)
- **US-E3-01** (지속 큐): 지속 저장 대상이 **큐 → `SyncStateStore`**(마지막 커밋 매니페스트 + dirty 표시 + 재개 오프셋)로 이동. **크래시/재시작 무손실 가치는 불변** — dirty·재개 오프셋 복구 + 다음 재스냅샷이 미반영 편집을 자동 흡수. "지속 큐를 그대로 이어받는다" → "지속 상태(마지막커밋+dirty+재개오프셋)를 복구하고 다음 사이클이 재스냅샷한다"로 읽는다.
- **US-E3-02** (coalesce): 별도 coalesce 단계 **삭제**. 매 사이클 폴더 재스냅샷이 곧 최신 상태이므로 **"newer supersedes older"가 구조적으로 자동 성립**. 수용 기준의 "파일별 최신 상태만 유지"는 재스냅샷→diff로 자연 달성.
- **US-E3-03/US-E3-04** (백오프·오프라인 배출): "지속 큐를 통해 재시도"·"큐 배출" → **전체 사이클 백오프 재시도**로 읽는다. 별도 배출(drain) 루프 없이 **다음 사이클이 곧 drain**이며, 재연결 시 최신 폴더 상태를 재스냅샷·전송. "배출 전 coalesce 적용"은 불필요(재스냅샷이 이미 최신).
- **US-E7-04** (coalescing 속성, NFR-11): 프로퍼티 대상이 큐 인터리빙 → **`SyncStateStore` 상태 전이**(재스냅샷→diff→커밋)의 "커밋된 상태 = 마지막 폴더 상태" 불변식으로 이동.
- **US-E7-08** (상태 기반 큐 속성, PBT-06): 모델 대상이 **큐 → `SyncStateStore` 상태머신**: `commit_manifest / mark_dirty / set_resume_offset / persist / recover` 명령 시퀀스 vs 참조 모델. 단일 컴포넌트 국한·Small/Testable 성격 불변.
- **US-E7-09**(zero-loss)·**US-E1-03**(재조정 스캔): "지속 큐" 표현은 위 `SyncStateStore` 모델로 읽되, **무손실/≤T_recon 탐지 지연 보증은 불변**.
- **파생 스토리 표현**: US-E1-01(단일 인스턴스), US-E5-02(`watcher status`의 "지속 큐 깊이"), US-E6-01(graceful stop), US-E6-04(제거 시 로컬 평문 삭제)의 "지속 큐(FR-03)" 문구는 **`SyncStateStore` 지속 상태 파일**로 읽는다. `status`의 "큐 깊이"는 **dirty 여부 + 진행 중 재개 오프셋**으로 표면화.

### 추적성 테이블 정정 표기
`## 요구사항 → 스토리 매핑` 및 상단 MVP 문구의 **"FR-03 지속 큐" → "FR-03 지속 상태(`SyncStateStore`)"**, **"FR-12 coalesce" → "FR-12 latest-state-wins(구조적)"** 로 읽는다(테이블 원문은 승인 기록으로 보존).

### 불변(정정 없음) 확인
E1 트리거·재조정, E2 업로드 프로토콜·재검증(US-E2-*), E4 인증·동의, E5 관측(status/log/notify), E6 수명주기, US-E7-02/03/05/06/07/11 및 페르소나·실행 모델은 단순화 모델과 **정합하며 정정 없음**.
