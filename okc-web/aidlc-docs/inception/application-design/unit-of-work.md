# Unit of Work — okc-web (Obsidian Vault 통합 웹 플랫폼)

**Stage**: INCEPTION / Units Generation — Part 2 (Generation)
**Depth**: Standard
**System**: okc-web = okc-core 엔진 위에 얹은 Rust(axum) + Next.js 컨트롤-플레인·서빙 계층. `okc-interop`을 path dep로 직접 링크(ADR-0002), 단일 장기 프로세스·단일 SQLite 상태 파일·단일-writer 엔진 큐.
**Grounding**: `execution-plan.md`(승인된 7-유닛 U0–U6 분해·빌드 시퀀스·유닛별 depth) · `unit-of-work-plan.md`(Part 1 결정 7문항 모두 A) · `application-design.md`(모듈 맵·epic→unit 맵·screen→story 매트릭스·의존성·SQLite 상태·§2 어댑터 경계·§3 RBAC·§4 큐·§5 CuratorDecision·§6 staleness) · `component-dependency.md`(유닛 의존성 매트릭스·통신 패턴) · `services.md`(유닛별 서비스·read-path 정책·okc-core 메서드) · `stories.md`(5 epic E1–E5, 29 스토리) · `module-integration-guide.md`(§2 okc-web 내부 모듈 맵, §4 모노레포 레이아웃)

> **범위 고정**: 7-유닛(U0–U6)은 execution-plan.md + application-design.md에서 **이미 승인·확정**되었다. 본 문서는 각 유닛의 정의·핵심 책임·경계·okc-core 접점을 확정하고 Greenfield 코드 조직 전략을 명문화할 뿐, **유닛을 재명명하거나 재범위화하지 않는다**. 빌드 시퀀스는 `U0 → U1 → U2 → U3 → U4 → U5`이며 **U6는 인터리브**(각 유닛의 API가 나오는 즉시 해당 화면 배선). 모든 유닛은 U0에 의존한다. 유닛 간 의존성 그래프·Mermaid는 별도 산출물(`unit-of-work-dependency.md`)에서 다룬다.

---

## 1. 유닛 정의

### U0 — Foundation (Platform & okc-interop Adapter)

- **모듈**: `adapter`(+`adapter::queue`) · `shared`(`authz`/`error`/`jobs`/`audit`/`state`)
- **Epic**: cross-cutting — 특정 epic 소유 아님. E1–E5 전 유닛의 공유 기반이며, 특히 E3 오케스트레이션·E4 리뷰의 토대.
- **핵심 책임**:
  - 유일한 `okc-interop` 링크점(ADR-0002): `adapter::OkcEngine` 트레이트 + 단 하나의 `OkcEngineImpl`. okc-web 소유 typed-DTO(`*View`/`*Cmd`/`*Spec`)와 `From<okc_interop::…>` 변환으로 downstream을 interop churn에서 격리.
  - `SchemaGuard`: `INTEROP_SCHEMA_VERSION == 2`를 client init 및 매 DTO decode에서 단일 지점 검증; 불일치 시 `EngineError{code: SchemaUnsupported}`(silent mis-decode 금지).
  - 오류 경계: 모든 `okc_interop::OkcError`를 `adapter` 내부에서 okc-web 소유 `EngineError{code, category, retryable, retry_after_ms, message}`로 변환 — interop 타입이 U1–U6로 넘어가지 않음.
  - `adapter::queue` `EngineActor`: 단일-소유 **blocking** 워커 + bounded mpsc + oneshot. reserving/mutating op을 정확히 하나씩 직렬화(NFR-CONC-1), okc-interop 프로세스-글로벌 `PROJECT_RESERVATIONS` 위의 단일 writer. 잔여 `ProjectBusy`는 **code 기준**으로 409(+`Retry-After`).
  - read-path `OkcClient` clone: 비-reserving read(taxonomy/clusters/manifest/verify/explain)는 큐 우회(verifier #8, HOL-blocking 회피).
  - `shared::authz`: RBAC-before-core 가드·extractor·canonical `AuthContext{principal, role}`; `Role::{Admin, Contributor}`; `Principal::{Admin, UploadToken}`. **불변식(테스트 가능)**: 403/401은 `OkcEngine` 메서드가 0회 실행됐음을 보장(C-1).
  - `shared::error`: 단일 table-driven `EngineError` code/category → HTTP 매핑(`ProjectInvalid`/`ArtifactSchemaUnsupported`/`PathUnsupported`/`OutputDurabilityUncertain` + 안전 기본 500 포함); **message 문자열 파싱 금지**.
  - `shared::jobs`: `JobStore` + `job_events`(Q7 폴링 스냅샷의 단일 소스).
  - `shared::audit`: append-only `curator_decisions`(record-then-act, `HashBindings{proposal_hash, critic_hash, taxonomy_hash}`); enum 제약으로 **winner-select row 자체가 표현 불가**(C3).
  - `shared::state`: 단일 SQLite WAL 파일(`StateDb` 풀·마이그레이션·레지스트리).
- **경계 (소유하지 않음 — 명시적 non-responsibilities)**:
  - 도메인 정책/비즈니스 규칙 없음 — okc-core가 소유(ADR-0002). U0는 mechanism만 제공.
  - `DecisionGate`의 Major/Critical 판정 로직(U4 소유), `StalenessProjection` 파생 로직(U3 소유)은 U0 소유 아님. U0는 audit 저장소·큐·에러 매핑 등 mechanism만.
  - 인증(authentication) 자체는 U1/U2가 resolver로 공급 — U0는 authorize만 소유.
  - 테이블 중 U0가 소유하는 것은 `jobs`/`job_events`/`curator_decisions`뿐. `accounts`/`sessions`(U1)·`upload_tokens`(U2)·`projects`/`sources`(U3)·`serving_publications`(U5) 쓰기는 각 소유 유닛.
- **okc-core 접점**: 전체 okc-interop 표면의 **유일 링크점** — `OkcClient`/`Project`/`Job`. commit-pin path dep, CI 소스 빌드(NFR-PORT-1).

### U1 — Auth & RBAC

- **모듈**: `auth`
- **Epic**: E1(인증 & 권한) — 스토리 E1-S1, E1-S2, E1-S3, E1-S4, E1-S5.
- **핵심 책임**:
  - 계정·로그인·세션·비밀번호 해시 lifecycle. **패턴: 순수 pre-core, DB-only** — 엔진 큐를 절대 접촉하지 않음.
  - login: `AccountStore.find_by_email` → `PasswordHasher.verify`(constant-time) → disabled/bad-creds는 code 기준 401 → `SessionStore.create`(평문 id 1회, SHA-256 저장) → `SessionCookieCodec`. `Role::Admin`만 app shell 진입(E1-1); `Contributor`는 app-shell 세션 없음(업로드는 토큰 capability).
  - session resolution: `AuthContext` extractor → `SessionStore.resolve`(TTL/idle) → `touch`(sliding). 산출 `Principal::Admin{account_id, session_id_hash, curator_label}`을 U0 RBAC 가드에 공급.
  - account admin(E1-3/E1-5): create/set_role/set_status/change_password. 불변식: email 유일성, **last-admin 가드**(마지막 admin 강등/비활성 차단), disable/pw 변경 시 세션 revoke-all, 임시 시크릿 1회 노출(NFR-SEC-1).
  - `curator_id` 라벨 바인딩(비검증): 인증된 admin 식별자를 U3에 `curator_id`로 공급(E1-S4 = 바인딩 의미의 단일 소스). argon2id PHC 비밀번호 해시.
- **경계 (소유하지 않음)**:
  - RBAC 가드·ordering guarantee·canonical `AuthContext`는 U0 `authz` 소유 — U1은 authenticate(resolver 공급)만, authorize는 U0.
  - 엔진 큐·okc-core 직접 호출 없음(전적으로 pre-core, DB-only).
  - 업로드 토큰 발급/검증은 U2 소유; 업로드는 role이 아니라 `Principal::UploadToken` capability.
  - `viewer` 역할 deferred(FR-AUTH-2) — 조회 전용 접근은 admin/contributor 권한으로 처리.
- **okc-core 접점**: 없음(직접 호출 무). 인증된 admin은 비검증 `curator_id` 라벨로만 U3 경유 전달.
- **Functional Design depth**(construction 예정): standard.

### U2 — Upload & Token

- **모듈**: `upload`
- **Epic**: E2(업로드 & 토큰) — 스토리 E2-S1..E2-S6.
- **핵심 책임**:
  - `UploadTokenService`(admin-facing, DB-only, pre-core): issue(`active_slot_count` 10/10 차단 → 1회성 평문 + `verifier_hash`/`selector` 저장, owner_display_name/kind = 비검증 장식), revoke/rotate(`revoked_at` flip → 즉시 `/u/{token}` 차단).
  - `UploadIngestService`(capability-authenticated, U2 유일 mutating flow) — 순서 보장 fail-fast 파이프라인(add_source 이전 모든 단계는 거부 시 core-untouched 보장, C-1):
    1. `UploadContext` extractor(TOKEN_INVALID/EXPIRED/REVOKED → 401/403 before core).
    2. `SlotAccountant.reserve` → `SOURCE_CAP_EXCEEDED`(okc-web이 ≤10 cap을 core 앞에서 소유).
    3. `UploadReceiver.receive` → hard byte cap 스트리밍(`UPLOAD_TOO_LARGE`).
    4. `ArchiveValidator.inspect` → `ValidationReport`(Fail=FORMAT/PATH_UNSAFE/SYMLINK; Warn=CONTENT_NOT_MARKDOWN/DUPLICATE) — zip-bomb/traversal/symlink **착지 이전** 선제 차단(FR-UP-3).
    5. `SourceLander.land` → 프로젝트 sources root 하위 **절대경로** materialize(disk THEN register) + content hash.
    6. `OkcEngine::add_source`(U0 single-writer 워커, `AddSourceRequest{project_id, source_id, absolute_path, owner_display_name}`) → `JobId`.
    7. 성공: `SlotAccountant.commit` + `SourceRegistry.record`(U5용 owner-label row) + `mark_used` + audit; 실패: `release`.
  - 기여자 피드백 표면(E2-S6): `GET /u/{token}/jobs/{jobId}`로 같은 `JobStore` 스냅샷 조회. 중복은 hash 기반 no-op(`DUPLICATE_SOURCE`)으로 재시도 안전.
- **경계 (소유하지 않음)**:
  - admin 세션·계정 lifecycle은 U1; U2 admin 엔드포인트는 U1 세션 + U0 RBAC(Admin)에 의존.
  - ≤10 cap의 authoritative 강제는 okc-core(`ResourceLimit` backstop) — U2는 사전 차단.
  - `sources` 테이블 스키마 소유는 U3(`SourceRegistry`) — U2는 commit 시 row write만.
  - 중복 억제의 authoritative 판정은 okc-core content hashing — U2는 warn만.
  - taxonomy/cluster/통합 리뷰 데이터 노출 없음(기여자는 자기 업로드 결과 범위로 한정).
- **okc-core 접점**: `add_source`(via U0 single-writer 큐). `owner_display_name`(비검증 장식 라벨), ≤10 cap.
- **Functional Design depth**: standard.

### U3 — Integration Orchestration

- **모듈**: `orchestration`
- **Epic**: E3(통합 오케스트레이션) — 스토리 E3-S1..E3-S7.
- **핵심 책임**:
  - `ProjectLifecycleService`: `projects` 레지스트리 + source-freeze 전이; U1 admin 식별자를 비검증 `curator_id`로 `create_project`에 전달.
  - `PipelineService`: **파생(derived)** checkpoint 루프 — 단일 reserving `status()`에서 `StatusView{checkpoint, integration}`를 읽어 `NeedsProvider→NeedsSources→NeedsDisclosure→NeedsTaxonomy→NeedsClusters→ReadyToCompile→Verified`를 오케스트레이션, E3-3 Next-Action/E3-5 게이트 계산, `preflight`/`integrate` enqueue.
    - Disclosure surface(E3-5): `DisclosureDecision{allow_remote_provider, remote_disclosure_confirmed}` → `integrate`. 미동의 → `RemoteConsentRequired` → 422 consent prompt(remote 호출 없음). provider는 서버측 env-var **이름** 참조만(A-2).
    - Live progress: in-flight `Job.events()`/`state()`(JobStore, 비-reserving) — 새 `status()` 아님.
  - `CompileService`: `checkpoint == ReadyToCompile` 재확인 후 `compile` enqueue(no-clobber). 미승인 plan → core `ProjectInvalid` → 422.
  - `StalenessProjection`: 파생·비-authoritative staleness 라벨(recorded vs current source fingerprint 비교) — 사전 경고 전용.
  - `sources` 레지스트리(`SourceRegistry`) 테이블 소유(U2가 commit 시 write, U5가 owner-label read). ≤10 freeze(E3-S2), `source_set_fingerprint`.
- **경계 (소유하지 않음)**:
  - authoritative staleness는 okc-core 소유(`ApprovalStale`/checkpoint regression) — U3 projection은 사전 경고만이며 엔진 신호에 defer.
  - taxonomy/cluster 결정 표면·`DecisionGate`는 U4 소유; U4는 checkpoint를 `PipelineService`에서 read(재-derive 안 함).
  - compile 거부 게이트(APPROVAL_REQUIRED) 단일 관리는 U4(E4-S6); U3 `CompileService`는 정상 경로 + no-clobber만.
  - compile 산출물의 서빙/publish는 U5; U3는 `compiled_vault_path`/manifest/frozen-input hashes를 U5에 handoff, U5는 compile을 호출하지 않음.
  - 엔진 reservation 직렬화 mechanism은 U0 큐(U3는 소비자).
- **okc-core 접점**: `create_project`/`open_project`, `status`(reserving, `StatusView` 반환), `preflight`, `integrate`, `compile`(내부적으로 `IntegrationService::compile_latest`) — 모두 U0 큐 경유.
- **Functional Design depth**: comprehensive(IntegrationCheckpoint 상태머신 + hash-bound staleness 캐스케이드 — 정합성 코어).

### U4 — Conflict/Critic Review

- **모듈**: `review`
- **Epic**: E4(Conflict/Critic 리뷰 — focal wow) — 스토리 E4-S1..E4-S6.
- **핵심 책임**:
  - `CuratorDecision`(정확히 3-variant enum, 리뷰 상태 변경의 유일 경로): `ApproveTaxonomy` / `ApproveCluster` / `RegenerateCluster` — **`SelectWinner`/`ResolveContradiction` 변형 없음**(code-verifiable C3 차별자).
  - `TaxonomyReviewService`: `approve_taxonomy(edited_clusters, rationale)`(편집 시 rationale 필수).
  - `ClusterReviewService`: `approve_cluster(cluster_id, omission_rationales, minor_waivers)`; Minor waive·omission 각각 사유 필수.
  - `DecisionGate.assert_approvable`: Major/Critical 도메인 검사를 okc-web에서 수행 → `approve_cluster` 엔진 호출 이전에 `422 APPROVAL_REQUIRED`. **code-verified 필요성**: core의 blocking-approval 거부는 interop `map_project_message`를 거쳐 generic `ProjectInvalid`(not `ApprovalRequired`)로 매핑되므로, 결정론적 422는 okc-web 게이트에서 발생해야 함.
  - `RegenerationService`: `regenerate_cluster(cluster_id, feedback)` — Major/Critical의 유일 해소 경로(long AI job).
  - `ReviewGateService`: E4-1 read-only compile-eligibility 스코어보드(`Blocked|PendingApprovals|Ready`) — compile을 트리거하지 않음.
  - 모순 보존 표시(E4-3 Contradictions 탭 read-only, 양측 보존, ADR-0024) — winner-select 금지; 모순을 바꾸는 유일 경로는 full cluster Regenerate.
  - E4-3 focal `ClusterReviewView` DTO(okc-core `ClusterTaskOutput{proposal, critic}` 1:1 + gate/파생 필드) — 실 엔진 산출만(mock 금지). record-then-act: 결정을 U0 audit에 `HashBindings`와 함께 append **후** op.
- **경계 (소유하지 않음)**:
  - checkpoint 상태 재-derivation·`StalenessProjection`은 U3 소유(U4는 read).
  - audit 저장소 mechanism은 U0 `shared::audit`(U4는 producer).
  - 엔진 큐 mechanism은 U0.
  - winner-select/contradiction resolution — 개념 자체가 부재(C-2/C3).
  - compile 정상 경로는 U3 `CompileService`; U4는 거부 게이트(E4-S6)만.
  - taxonomy/clusters read는 U0 read-path clone 경유(큐 우회) — U4는 소비.
- **okc-core 접점**: `approve_taxonomy`, `approve_cluster`, `regenerate_cluster`(U0 큐 경유); `taxonomy`/`clusters` read(U0 read-path clone, 큐 우회).
- **Functional Design depth**: comprehensive(경우-B 결정 표면 + severity 게이팅 로직 — C3 차별자).

### U5 — Serving & okc-mcp Contract

- **모듈**: `serving`
- **Epic**: E5(병합 Vault 서빙 & okc-mcp 계약 — focal wow) — 스토리 E5-S1..E5-S5.
- **핵심 책임**:
  - `ServingService`(facade): `CompiledVaultStore`(filesystem 3-root read: `knowledge/`+`legacy/`+`.okc/`), `ProvenanceComposer`, `ServingStateStore`, `McpContractBuilder`로 fan-out.
  - read-only `/api/serving/*`: GET/HEAD only(mutating verb → 405; 서빙 루트 이탈 → 404); 파일 목록·본문(E5-S2).
  - verify/explain provenance(E5-S3): 비-mutating·**동기**·큐 우회. verify = 내부 일관성 증명이며 발행자 진위 보증 아님(NFR-DET-1, 영구 callout).
  - `ProvenanceComposer`(E5-2 focal): `NoteProvenanceView` = `explain`(`ProvenanceView`) + `verify`(`VerificationView`) + `SourceRegistry` owner_labels + `.okc/` 모순 인덱스; lineage graph. 전 필드 실 엔진 DTO/okc-web 상태 row(mock 없음).
  - `ServingStateStore`: publish/unpublish(okc-web-only, admin-gated SQLite flip — core op·큐 없음); status ∈ {Offline, Live, Stale}는 bound manifest 해시 vs U3 current frozen-input 해시 비교로 파생; 불변 manifest는 Stale 라벨과 함께 계속 서빙(E5-S5). `serving_publications` 테이블 소유.
  - `McpContractBuilder`(E5-S4): discovery/contract 엔드포인트 — read-only 소비 위치·형식 계약. RAG 리트리벌 out-of-scope + 재임베딩 명시; okc-mcp 내부(MCP 툴 표면 등) 호출 항목 없음.
- **경계 (소유하지 않음)**:
  - compile 실행은 U3 `CompileService`; U5는 `compiled_vault_path`/manifest read만.
  - authoritative staleness는 U3/core; U5 `serving_status`는 파생.
  - okc-mcp RAG 리트리벌(청킹·임베딩·vector index·쿼리, MCP 툴 표면)은 okc-mcp 소관 — 계약만 노출.
  - 요청 시점 live U4 리뷰 의존 없음(`.okc/` self-contained, post-compile).
  - 엔진 mutating op 없음(read-only + 비-mutating verify/explain, 큐 우회).
  - `sources`(SourceRegistry) 소유는 U3 — U5는 owner-label read.
- **okc-core 접점**: `verify_artifact`, `explain_artifact`(비-mutating, U0 read-path clone, 큐 우회); `manifest` read.
- **Functional Design depth**: skip(파일 read; UI 선착수).

### U6 — Frontend

- **모듈**: `web`(Next.js App Router)
- **Epic**: cross-cutting — 전 화면(E1-1..E5-4 + machine API), 전 story-ID.
- **핵심 책임**:
  - API-consumer 층: `apiClient`, `okcErrorMap`(U0 error map mirror, **code/category로 분기** — message 파싱 금지), `queryKeys`, `useJobPolling`(TanStack Query, terminal까지 refetch, `events[]` drain).
  - 화면 배선: E1-1..E1-5, E2-1..E2-5, E3-1..E3-6, E4-1..E4-4, E5-1..E5-4(+ machine API). 3 auth context(admin cookie / upload bearer `/u/{token}` / read-only serving).
  - server components(shell/static) + client components(review/upload/polling).
  - E4/E5 focal wow 화면; empty/loading/success/error 상태 매트릭스; in-screen helper + 문제정의 카피.
  - UI 선착수(`ui-screens.md`/`design-system.md`) 위 배선 — **UI 재설계 없음**; 마지막 폴리시.
- **경계 (소유하지 않음)**:
  - okc-core 직접 접촉 없음(HTTP only).
  - 인증/RBAC/도메인 로직 없음 — 백엔드(U0–U5) 소유. U6는 프레젠테이션/에러 표시만.
  - error → HTTP 매핑의 source of truth는 U0 `shared::error`(U6 `okcErrorMap`은 mirror).
  - job 상태 authoritative는 U0 `JobStore`(U6는 poll consumer).
- **okc-core 접점**: 없음(axum HTTP REST + JSON only).
- **Functional Design depth**: skip(프레젠테이션, UI 선착수).

---

## 2. Greenfield 코드 조직 전략

**결정(unit-of-work-plan Q6=A)**: 모노레포 준비형 단일 크레이트 백엔드 + Next.js 프론트엔드. **유닛 ↔ 백엔드 모듈 1:1**, 계층형(handler/service/repo) 대신 유닛-경계 정렬로 C1 추적성·C6 모듈화를 확보.

### 2.1 디렉터리 구조(현재 okc-web 루트)

```text
okc-web/
├── backend/                          # 단일 axum 크레이트
│   ├── Cargo.toml                    # okc-interop path dep (commit-pin) · lockfile 커밋
│   ├── Cargo.lock                    # 재현 빌드 (NFR-PORT-1)
│   └── src/
│       ├── main.rs                   # axum 엔트리포인트: 라우터 조립 + EngineActor spawn + StateDb init
│       ├── adapter/                  # U0 — 유일한 okc-interop 링크점
│       │   ├── mod.rs                #   OkcEngine 트레이트 + OkcEngineImpl (유일 okc_interop import)
│       │   ├── dto.rs                #   *View / *Cmd / *Spec + From<okc_interop::…>
│       │   ├── schema_guard.rs       #   SchemaGuard (INTEROP_SCHEMA_VERSION == 2)
│       │   └── queue.rs              #   adapter::queue — EngineActor + EngineHandle (mpsc + oneshot)
│       ├── shared/                   # U0 — 횡단 관심사
│       │   ├── authz.rs              #   RBAC-before-core 가드 · AuthContext · Role · Principal
│       │   ├── error.rs              #   EngineError code/category → HTTP (단일 table)
│       │   ├── jobs.rs               #   JobStore + job_events (폴링 스냅샷)
│       │   ├── audit.rs              #   append-only curator_decisions (record-then-act)
│       │   └── state.rs              #   SQLite StateDb 풀 · 마이그레이션 · 레지스트리
│       ├── auth/                     # U1 (E1) — 계정·세션·비밀번호 해시
│       ├── upload/                   # U2 (E2) — 토큰 발급/폐기·검증·landing·add_source
│       ├── orchestration/            # U3 (E3) — lifecycle·freeze·checkpoint·compile·staleness·sources
│       ├── review/                   # U4 (E4) — taxonomy/cluster 결정·DecisionGate·regenerate
│       └── serving/                  # U5 (E5) — read-only vault API·verify/explain·publish·mcp 계약
├── frontend/                         # U6 — Next.js App Router (shadcn/Tremor, UI 선착수)
│   ├── app/                          #   (app) 레이아웃 + E1–E5 라우트 (server/client components)
│   ├── lib/                          #   apiClient · okcErrorMap · queryKeys · useJobPolling
│   └── components/                   #   design-system.md / ui-screens.md 산출 UI
└── aidlc-docs/                       # AI-DLC 산출물 (문서 전용 — 애플리케이션 코드 아님)
```

### 2.2 유닛 ↔ 모듈 1:1 매핑

| 유닛 | 백엔드 모듈 | Epic | 관심사 | okc-core 접점 | Functional Design depth |
|---|---|---|---|---|---|
| **U0** | `adapter`(+`adapter::queue`) · `shared`(`authz`/`error`/`jobs`/`audit`/`state`) | cross | 유일 interop 링크점 · 단일-writer 큐 · RBAC 가드 · OkcError→HTTP · JobStore · 감사 · SQLite | OkcClient/Project/Job (유일 링크점) | — |
| **U1** | `auth` | E1 | 계정·역할·세션·비밀번호 해시 | 없음 (curator_id 라벨만 U3 경유) | standard |
| **U2** | `upload` | E2 | 토큰 발급/폐기 · 토큰 인증 업로드 · 검증 · landing · ≤10 cap | `add_source` (큐 경유) | standard |
| **U3** | `orchestration` | E3 | lifecycle·freeze·checkpoint 루프·compile·staleness·`sources` 레지스트리 | `create_project`/`open_project`·`status`·`preflight`·`integrate`·`compile` (큐 경유) | comprehensive |
| **U4** | `review` | E4 | taxonomy/cluster 결정 표면 · `DecisionGate`(Major/Critical→422) · regenerate | `approve_taxonomy`·`approve_cluster`·`regenerate_cluster` (큐) · `taxonomy`/`clusters` (read-path) | comprehensive |
| **U5** | `serving` | E5 | read-only compiled-vault API · verify/explain provenance · publish · mcp 계약 | `verify_artifact`·`explain_artifact`·`manifest` (비-mutating, 큐 우회) | skip |
| **U6** | `web`(Next.js) | cross | API 소비층(`apiClient`·`okcErrorMap`·`queryKeys`·`useJobPolling`) · 화면 배선 | 없음 (HTTP only) | skip |

### 2.3 모노레포 준비 레이아웃 (module-integration-guide §4 정합)

현재 `okc-web/`는 이미 `backend/` + `frontend/` + `aidlc-docs/`를 프로젝트 루트 하위에 중첩한다. 따라서 향후 모노레포 편입은 **`okc-web/` 폴더를 모노레포 루트 하위로 그대로 이동**하고 okc-interop path dep 경로만 재배선하면 되며, 내부 구조 재편은 불필요하다.

```text
<monorepo-root>/
├── okc-core/                 # Rust 엔진 — 핀된 의존성 (git submodule 또는 vendored, 상류 churn 격리)
├── okc-web/                  # 이 프로젝트가 그대로 이동
│   ├── backend/              # axum 크레이트 (okc-interop path dep → ../../okc-core/...)
│   │   └── src/{adapter,shared,auth,upload,orchestration,review,serving}/
│   ├── frontend/             # Next.js App Router
│   └── aidlc-docs/           # AI-DLC 산출물
├── okc-mcp/                  # RAG MCP 서버 — okc-web 서빙 계약 소비 (미구현, 계약만)
├── obsidian-hook/            # Obsidian 플러그인 — okc-web 업로드 계약 생산 (미구현, 계약만)
├── .github/workflows/        # 공유 CI: okc-core 핀 빌드 → okc-web(백/프론트) → (mcp/hook)
├── .env.example              # 이름만(값 없음) — provider env-var · 경로 · 포트 (C6 시크릿 위생)
└── README.md                 # 통합 개요 + 모듈 링크 + Problem Statement
```

**핵심 원칙**: (1) okc-core는 핀된 외부 의존성으로 유지(okc-web은 소비자, ADR-0002 무수정). (2) `okc-interop` 링크는 `adapter` 단일 지점 — 나머지 모듈은 okc-web typed-DTO(interop schema v2)에만 의존. (3) provider 자격증명은 env-var **이름**으로만 참조(값 커밋 금지). (4) 로컬 절대경로 기반 landing/compile은 모노레포 이동 시 config로 외부화.

---

## 3. 6-기준 self-check (C1 추적성 · C6 모듈화)

- **C1 (협업 진정성 / 추적성)** — 7 유닛(U0–U6)은 `execution-plan.md`의 승인된 분해 및 `application-design.md` §13 epic→unit→module 맵과 **1:1 일치**하며, 유닛 이름·범위는 무변경(재명명/재범위 금지 준수). 각 유닛은 `stories.md`의 epic/story-ID로 키잉된다: E1→U1, E2→U2, E3→U3, E4→U4, E5→U5, cross-cutting→U0(기반)·U6(프레젠테이션). 29 스토리의 유닛 배정 완전성(누락 0)은 후속 산출물 `unit-of-work-story-map.md`에서 screen→story 매트릭스를 근거로 검증한다. 유닛별 Functional Design depth(U3/U4 comprehensive · U1/U2 standard · U5/U6 skip)도 execution-plan을 그대로 반영.
- **C6 (유지보수성 / 모듈화)** — 5-epic 1:1 모듈 분할 + U0 공유 기반 + U6 프레젠테이션으로 관심사 분리. 유닛 간 결합은 오직 (a) `OkcEngine` 트레이트/타입 경계, (b) U0 `adapter::queue` single-writer 큐, (c) `shared::state` SQLite 레지스트리로만 이뤄지며 유닛 간 네트워크 호출이 없다(단일 프로세스, Q2/Q4=A). `okc-interop` 링크는 `adapter` 단일 지점으로 격리(ADR-0002 컴파일-타임 강제). 의존은 U0 기반 단방향으로 순환이 없으며(상세·Mermaid는 `unit-of-work-dependency.md`), `backend/src/{module}/` 1:1 디렉터리 + 모노레포 준비 레이아웃이 이 경계를 물리적으로 반영한다.
- **기타 기준(C2/C3/C4/C5)**: 본 Units Generation 단계에서는 유닛 경계 정의가 산출물이므로 C3(어댑터 격리 seam·winner-select 부재 유닛 매핑)만 부분 관여하며 확정, C2/C4/C5는 후속 Functional/Code Generation 단계에서 착지 → 본 단계에서는 N/A.
