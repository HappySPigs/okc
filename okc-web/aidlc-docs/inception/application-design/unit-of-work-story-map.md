# 유닛-스토리 맵 (Unit of Work ↔ Story Map) — okc-web

**Stage**: INCEPTION / Units Generation — Part 2 (Generation) · **Depth**: Standard
**Date**: 2026-09-08
**목적**: `stories.md`의 29개 스토리(에픽 E1–E5)를 승인된 7-유닛(U0–U6)에 **정확히 1개의 소유 유닛(owner)** 으로 배정하고, 각 스토리가 관통하는 **지원 유닛(supporting)** 을 명시한다.
**Grounding**: `stories.md`(권위 스토리 목록 29개/5 에픽) · `application-design.md` §12 화면→스토리 매트릭스·§13 에픽→유닛→모듈 맵·§9 SQLite 소유권 · `component-dependency.md` 의존성 매트릭스 · `services.md` 서비스 소유권 · `execution-plan.md`(U0–U6 분해·빌드 순서) · `unit-of-work-plan.md`(Part 1 승인값 Q1..Q7 = 전부 A) · `module-integration-guide.md` §2 내부 모듈 맵.

---

## 1. 매핑 원칙 (Mapping Principles)

1. **단일 소유(one owner) 규칙**: 모든 스토리는 정확히 하나의 소유 유닛(owner)을 가진다. 여러 유닛이 관여하면 나머지는 **지원 유닛(supporting)** 으로만 표기한다.
2. **에픽→유닛 정렬(PRIMARY capability)**: `application-design.md` §13(권위 에픽→유닛→모듈 맵)과 `unit-of-work-plan.md` Q1=A(에픽/역량 정렬)에 따라 각 스토리의 소유 유닛은 그 스토리가 속한 에픽의 역량 유닛으로 결정된다.
   - E1(인증/RBAC) → **U1**(module `auth`)
   - E2(업로드/토큰) → **U2**(module `upload`)
   - E3(통합 오케스트레이션) → **U3**(module `orchestration`)
   - E4(Conflict/Critic 리뷰) → **U4**(module `review`)
   - E5(서빙/okc-mcp 계약) → **U5**(module `serving`)
3. **U0·U6는 횡단 지원 유닛(cross-cutting)**: `application-design.md` §13은 U0(foundation)과 U6(frontend)를 에픽 없는 `(cross)` 행으로 둔다. 두 유닛은 **사용자 스토리를 소유하지 않고 전 스토리를 지원**한다.
   - **U0 Foundation** = `app/adapter`(+`adapter.queue`) · `app/shared`(authz/error/jobs/audit/state). 유일한 `okc` 바인딩 import 지점, 단일-writer 엔진 큐, RBAC 가드, `OkcError→HTTP` 매핑, `JobStore`, 큐레이터 감사, SQLite — **모든 스토리의 기반 기전을 제공**한다.
   - **U6 Frontend** = module `web`(React + Vite SPA). API 소비층(`apiClient`·`okcErrorMap`·`queryKeys`·`useJobPolling`)과 화면 배선 — **화면이 있는 모든 스토리를 지원**한다.
4. **횡단 성격이 강한 스토리도 owner는 에픽 유닛**: 예) E1-S3(변경성 op admin 게이팅)의 실행 기전인 RBAC 가드 `Depends` 의존성은 U0 `shared.authz`가 제공하지만, 스토리의 **1차 역량(인증/권한 정책)** 은 E1이므로 owner는 U1이고 U0는 게이팅 기전을 제공하는 지원 유닛으로 표기한다(§13에서 E1 = `U1 (+U0 authz)`). E3-S7(진행/오류 표시)도 동일하게 owner U3, 기전(JobStore·error 매핑·PROJECT_BUSY 큐)은 U0 지원.

> 표기 규약: 지원 유닛은 `Ux(제공 기전)` 형태. `FE`=프런트 화면 배선, `authz/error/jobs/audit/state`=U0 `shared.*` 하위 관심사, `adapter/queue`=U0 엔진 어댑터·단일-writer 큐.

---

## 2. 소유 유닛별 스토리 매핑 (Per-Unit Ownership Tables)

### 2.1 U1 — Auth (module `auth`, epic E1) — 소유 5

| story-id | 스토리 제목(짧게) | 소유 유닛 | 지원 유닛 | epic |
|---|---|---|---|---|
| E1-S1 | 관리자 로그인·세션/토큰 발급 | U1 | U0(authz 가드·state accounts/sessions), U6(FE E1-1/E1-5) | E1 |
| E1-S2 | 역할 부여·관리(admin/contributor) | U1 | U0(authz·state), U6(FE E1-3) | E1 |
| E1-S3 | 변경성 통합 작업 admin 전용 게이팅 | U1 | U0(RBAC 가드 `Depends` 의존성 `shared.authz` — 403-never-calls-core 기전), U6(FE E1-4 403/401) | E1 |
| E1-S4 | 인증 admin→okc-core `curator_id` 라벨 바인딩 | U1 | U0(authz `Principal.Admin.curator_label`·audit 기록), U3(create_project 시 바인딩 적용), U6(FE) | E1 |
| E1-S5 | 인증 시크릿(비밀번호/토큰) 안전 저장 | U1 | U0(state hashed@rest 저장), U2(업로드 토큰 verifier 해시 동일 패턴), U6(FE E1-1/E1-5 마스킹·1회 노출) | E1 |

### 2.2 U2 — Upload (module `upload`, epic E2) — 소유 6

| story-id | 스토리 제목(짧게) | 소유 유닛 | 지원 유닛 | epic |
|---|---|---|---|---|
| E2-S1 | 업로드 토큰 발급(1회 표시) | U2 | U1(admin 세션 전제), U0(authz admin 게이트·state upload_tokens), U6(FE E2-1/E2-2) | E2 |
| E2-S2 | 업로드 토큰 조회·폐기 | U2 | U1(admin 세션), U0(authz·state), U6(FE E2-1) | E2 |
| E2-S3 | 토큰 인증 업로드 엔드포인트 | U2 | U0(authz 토큰 resolver `Principal.UploadToken`), U6(FE E2-3/E2-4) | E2 |
| E2-S4 | 업로드 바이트 검증(포맷/크기/경로·symlink) | U2 | U0(error 매핑 거부 code), U6(FE E2-4) | E2 |
| E2-S5 | 검증분 로컬 착지 & `add_source`(≤10) | U2 | U0(adapter/queue `add_source`·jobs·audit), U3(`sources` 레지스트리·≤10 reconcile·projects), U6(FE E2-4/E2-5) | E2 |
| E2-S6 | 업로드 결과·거부 사유 조회(기여자 피드백) | U2 | U0(JobStore `/u/{token}/jobs` 스냅샷), U6(FE E2-5) | E2 |

### 2.3 U3 — Orchestration (module `orchestration`, epic E3) — 소유 7

| story-id | 스토리 제목(짧게) | 소유 유닛 | 지원 유닛 | epic |
|---|---|---|---|---|
| E3-S1 | 프로젝트 생성·`curator_id` 기록 | U3 | U1(admin 신원→curator_id), U0(adapter create_project·audit), U6(FE E3-2) | E3 |
| E3-S2 | 소스 ≤10 고정(freeze) | U3 | U0(adapter·state projects.freeze_state), U2(`add_source` ≤10 reconcile), U6(FE E3-4/E3-6) | E3 |
| E3-S3 | IntegrationCheckpoint 루프 상태 표시/오케스트레이션 셸 | U3 | U0(adapter status·단일-writer 큐·jobs·PROJECT_BUSY 직렬화), U6(FE E3-3/E3-5 스테퍼) | E3 |
| E3-S4 | AI 공급자 설정·remote disclosure/consent(NeedsProvider/NeedsDisclosure 해소) | U3 | U0(adapter/queue integrate + disclosure 플래그), U6(FE E3-5 consent 프롬프트) | E3 |
| E3-S5 | 소스/설정 변경 시 승인 무효화(stale) UX | U3 | U0(adapter `ApprovalStale`·audit fingerprint), U6(FE E3-6 경고) | E3 |
| E3-S6 | 승인 완료 시 `compile` 병합 Vault 생성(no-clobber) | U3 | U0(adapter/queue compile), U5(compiled path/manifest 소비), U6(FE E5-1) | E3 |
| E3-S7 | 진행상황·오류 표시(Job/OkcError code·category/PROJECT_BUSY 재시도) | U3 | U0(JobStore 폴링·`OkcError→HTTP`·PROJECT_BUSY 큐 — 진행/오류 기전), U6(FE 진행/오류 UI) | E3 |

### 2.4 U4 — Review (module `review`, epic E4) — 소유 6

| story-id | 스토리 제목(짧게) | 소유 유닛 | 지원 유닛 | epic |
|---|---|---|---|---|
| E4-S1 | Taxonomy proposal 검토·편집·승인(`approve_taxonomy`) | U4 | U0(adapter/queue approve_taxonomy·audit), U3(NeedsTaxonomy 체크포인트 from PipelineService), U6(FE E4-2) | E4 |
| E4-S2 | 클러스터 synthesis 결과·critic findings 심각도 검토 | U4 | U0(adapter read-path clusters·schema-v2 가드·error 매핑), U6(FE E4-1/E4-3) | E4 |
| E4-S3 | 클러스터 승인 + Minor waive/Omission(사유 필수, `approve_cluster`) | U4 | U0(adapter/queue approve_cluster·audit record-then-act; DecisionGate는 U4 자체 소유), U6(FE E4-3) | E4 |
| E4-S4 | Regenerate로 차단(Major/Critical) findings 해소(`regenerate_cluster`) | U4 | U0(adapter/queue regenerate_cluster·jobs·PROJECT_BUSY), U6(FE E4-4 diff) | E4 |
| E4-S5 | 보존된 모순(Contradictions) 정보성 표시(승자 없음) | U4 | U0(adapter read-path clusters/`.okc/` 모순 인덱스), U5(E5-2 provenance 화면도 모순 표시), U6(FE E4-3) | E4 |
| E4-S6 | 차단 findings/미승인 잔존 시 compile 거부 게이트(APPROVAL_REQUIRED) | U4 | U0(error 매핑 422 APPROVAL_REQUIRED), U3(CompileService compile 트리거·ReviewGate 자격), U6(FE E4-1) | E4 |

### 2.5 U5 — Serving (module `serving`, epic E5) — 소유 5

| story-id | 스토리 제목(짧게) | 소유 유닛 | 지원 유닛 | epic |
|---|---|---|---|---|
| E5-S1 | 병합 Vault를 read-only 서빙 엔드포인트로 공개(publish) | U5 | U0(authz admin 게이트·state serving_publications), U3(`compiled_vault_path`/manifest·frozen-input 해시), U6(FE E5-3) | E5 |
| E5-S2 | read-only API로 파일 목록·본문 조회 | U5 | U0(error 매핑 404/405 read-only), U6(FE E5-1 트리) | E5 |
| E5-S3 | `verify()`/`explain()`로 무결성·provenance 조회 | U5 | U0(adapter read-path verify/explain 큐 우회·error 매핑), U3(`SourceRegistry` owner 라벨), U6(FE E5-2 wow) | E5 |
| E5-S4 | okc-mcp RAG 소비 계약(위치·형식) 정의·게시 | U5 | U6(FE E5-4 계약 화면) | E5 |
| E5-S5 | 서빙을 hash-bound 매니페스트에 고정, 변경 시 stale 표시 | U5 | U0(state serving_publications), U3(frozen-input 해시 비교), U6(FE E5-2/E5-3) | E5 |

### 2.6 U0 — Foundation (module `adapter`+`shared`, cross) — 소유 0 (횡단 지원 전용)

U0는 사용자 스토리를 소유하지 않는다(`application-design.md` §13의 `(cross) foundation` 행). 대신 **모든 엔진-접촉·게이팅·상태·오류·감사 스토리의 기반 기전**을 제공한다. 아래는 U0 기전이 **스토리 성립에 결정적(load-bearing)** 인 대표 목록이다.

| story-id | U0가 제공하는 지원(핵심 기전) | 소유 유닛(owner) |
|---|---|---|
| E1-S3 | `shared.authz` RBAC 가드 `Depends` 의존성(403이면 okc-core 미호출 불변식) | U1 |
| E1-S4 | `Principal.Admin.curator_label` 확립 + `shared.audit` 기록(HashBindings) | U1 |
| E2-S3 | `AuthProvider.resolve_token` → `Principal.UploadToken` 토큰 인증 | U2 |
| E2-S5 | `adapter.queue` 단일-writer `add_source` + JobStore + audit | U2 |
| E3-S3 | 단일-writer 엔진 큐(직렬화)·status·PROJECT_BUSY | U3 |
| E3-S6 | `adapter.queue` compile(no-clobber) 실행 | U3 |
| E3-S7 | `JobStore` 폴링 + `OkcError→HTTP` code/category 분기 + PROJECT_BUSY 재시도 | U3 |
| E4-S3 | `adapter.queue` approve_cluster + `shared.audit` record-then-act | U4 |
| E4-S6 | `shared.error` 422 APPROVAL_REQUIRED 매핑 | U4 |
| E5-S2 | `shared.error` 404/405 read-only 매핑 | U5 |
| E5-S3 | `adapter` read-path(큐 우회) verify/explain + schema-v2 가드 | U5 |

> 위 목록은 예시가 아니라 U0 기전이 1차 결정적인 스토리다. 그 외 전 스토리(계정·토큰·프로젝트·서빙 상태)도 U0 `shared.state`(단일 SQLite WAL)와 `shared.error` 매핑을 공유한다.

### 2.7 U6 — Frontend (module `web` React+Vite, cross) — 소유 0 (횡단 지원 전용)

U6도 사용자 스토리를 소유하지 않는다(`(cross) frontend` 행). **화면이 존재하는 모든 스토리를 배선**한다. UI는 `ui-screens.md`/`design-system.md`로 동결(frozen)되어 U6는 API-소비 배선(`apiClient`·`okcErrorMap`·`queryKeys`·`useJobPolling`) + 화면 상태 매트릭스만 담당한다. `application-design.md` §12의 모든 화면(E1-1..E5-4)이 U6 배선 대상이며, 기계용 read API(`/api/serving/*`, E5-S2/S4의 소비자 관점)만 UI 없이 U5 백엔드가 직접 노출한다.

---

## 3. 전체 29 스토리 마스터 테이블 (한눈 보기)

| story-id | 스토리 제목(짧게) | 소유 유닛 | 지원 유닛 | epic |
|---|---|---|---|---|
| E1-S1 | 관리자 로그인·세션/토큰 발급 | U1 | U0, U6 | E1 |
| E1-S2 | 역할 부여·관리(admin/contributor) | U1 | U0, U6 | E1 |
| E1-S3 | 변경성 통합 작업 admin 전용 게이팅 | U1 | U0, U6 | E1 |
| E1-S4 | admin→`curator_id` 라벨 바인딩 | U1 | U0, U3, U6 | E1 |
| E1-S5 | 인증 시크릿(비밀번호/토큰) 안전 저장 | U1 | U0, U2, U6 | E1 |
| E2-S1 | 업로드 토큰 발급(1회 표시) | U2 | U1, U0, U6 | E2 |
| E2-S2 | 업로드 토큰 조회·폐기 | U2 | U1, U0, U6 | E2 |
| E2-S3 | 토큰 인증 업로드 엔드포인트 | U2 | U0, U6 | E2 |
| E2-S4 | 업로드 바이트 검증(포맷/크기/경로·symlink) | U2 | U0, U6 | E2 |
| E2-S5 | 검증분 로컬 착지 & `add_source`(≤10) | U2 | U0, U3, U6 | E2 |
| E2-S6 | 업로드 결과·거부 사유 조회(기여자 피드백) | U2 | U0, U6 | E2 |
| E3-S1 | 프로젝트 생성·`curator_id` 기록 | U3 | U1, U0, U6 | E3 |
| E3-S2 | 소스 ≤10 고정(freeze) | U3 | U0, U2, U6 | E3 |
| E3-S3 | 체크포인트 루프 상태 표시/셸 | U3 | U0, U6 | E3 |
| E3-S4 | 공급자 설정·disclosure/consent | U3 | U0, U6 | E3 |
| E3-S5 | 소스/설정 변경 시 승인 무효화 UX | U3 | U0, U6 | E3 |
| E3-S6 | `compile` 병합 Vault 생성(no-clobber) | U3 | U0, U5, U6 | E3 |
| E3-S7 | 진행상황·오류 표시(Job/OkcError/PROJECT_BUSY) | U3 | U0, U6 | E3 |
| E4-S1 | Taxonomy 검토·편집·승인(`approve_taxonomy`) | U4 | U0, U3, U6 | E4 |
| E4-S2 | synthesis/critic 심각도 검토 | U4 | U0, U6 | E4 |
| E4-S3 | 클러스터 승인 + Minor waive/omission(`approve_cluster`) | U4 | U0, U6 | E4 |
| E4-S4 | Regenerate로 차단 findings 해소(`regenerate_cluster`) | U4 | U0, U6 | E4 |
| E4-S5 | 보존된 모순 정보성 표시(승자 없음) | U4 | U0, U5, U6 | E4 |
| E4-S6 | 차단 findings 시 compile 거부 게이트(APPROVAL_REQUIRED) | U4 | U0, U3, U6 | E4 |
| E5-S1 | 병합 Vault read-only 공개(publish) | U5 | U0, U3, U6 | E5 |
| E5-S2 | read-only API 목록·본문 조회 | U5 | U0, U6 | E5 |
| E5-S3 | `verify`/`explain` provenance 조회 | U5 | U0, U3, U6 | E5 |
| E5-S4 | okc-mcp RAG 소비 계약 정의·게시 | U5 | U6 | E5 |
| E5-S5 | 서빙 매니페스트 해시 바인딩·stale 표시 | U5 | U0, U3, U6 | E5 |

---

## 4. 커버리지 요약 & 무결성 단언 (Coverage Summary & Integrity Assertion)

### 4.1 소유 유닛별 개수

| 유닛 | 모듈 | 성격 | 소유 스토리 수 | 소유 story-id |
|---|---|---|---|---|
| **U0** Foundation | `adapter`(+`queue`)·`shared` | 횡단(cross) | **0** | — (전 스토리 지원) |
| **U1** Auth | `auth` | epic E1 | **5** | E1-S1, E1-S2, E1-S3, E1-S4, E1-S5 |
| **U2** Upload | `upload` | epic E2 | **6** | E2-S1, E2-S2, E2-S3, E2-S4, E2-S5, E2-S6 |
| **U3** Orchestration | `orchestration` | epic E3 | **7** | E3-S1, E3-S2, E3-S3, E3-S4, E3-S5, E3-S6, E3-S7 |
| **U4** Review | `review` | epic E4 | **6** | E4-S1, E4-S2, E4-S3, E4-S4, E4-S5, E4-S6 |
| **U5** Serving | `serving` | epic E5 | **5** | E5-S1, E5-S2, E5-S3, E5-S4, E5-S5 |
| **U6** Frontend | `web`(React+Vite) | 횡단(cross) | **0** | — (화면 있는 전 스토리 배선) |
| **합계** | — | — | **29** | — |

계산: 0 + 5 + 6 + 7 + 6 + 5 + 0 = **29**.

### 4.2 무결성 단언 (명시)

- **총 스토리 수 = 29** (`stories.md` 실측: E1×5 + E2×6 + E3×7 + E4×6 + E5×5 = 29). 브리프 기대치(29)와 **일치**.
- **미배정(unassigned) = 0**: 29개 story-id 전부가 정확히 1개 owner를 가진다.
- **중복 소유(duplicated owner) = 0**: 어떤 story-id도 2개 이상의 owner를 갖지 않는다(§3 마스터 테이블의 각 행 owner 컬럼은 단일 값).
- **소유 합 = 29**: 유닛별 소유 개수 합(0+5+6+7+6+5+0)이 전체 29와 정확히 일치 → 누락·중복 없음.
- **U0·U6 owner=0은 의도적 설계**: `application-design.md` §13이 U0(foundation)·U6(frontend)를 에픽 없는 `(cross)` 행으로 두므로 두 유닛은 스토리를 소유하지 않고 전 스토리를 지원한다(누락이 아님). 각 스토리의 지원 유닛 목록(§2·§3)에서 U0·U6의 관여를 명시했다.

---

## 5. 검증 노트 (근거 추적성, C1)

- **에픽→유닛 owner 배정**: `application-design.md` §13 표(E1 auth/RBAC→U1, E2→U2, E3→U3, E4→U4, E5→U5; `(cross)` frontend=U6, `(cross)` foundation=U0)와 1:1 일치. `unit-of-work-plan.md` Q1=A(에픽/역량 정렬) 승인값 준수.
- **지원 유닛 도출**: `application-design.md` §12 화면→스토리 매트릭스의 `Unit(s)` 컬럼(예: E2-5=`U2 (+U0 jobs)`, E5-1=`U3 (compile) + U5`, E1-4=`U0 authz + U6`), `component-dependency.md` 의존성 매트릭스(U2→U0 queue·U3 state, U3→U5 compiled path, U5→U3 SourceRegistry 등), `services.md` 서비스 소유권(DecisionGate=U4, 단일-writer 큐/error/audit=U0, StalenessProjection=U3)을 근거로 배정.
- **횡단 owner 판단 근거**: E1-S3(RBAC 가드)·E3-S7(Job/OkcError/PROJECT_BUSY)은 기전이 U0에 있으나, 1차 역량이 각각 인증-정책(E1)·통합 진행/오류(E3)이므로 owner를 U1·U3로 두고 U0를 지원으로 표기(§1-4 원칙). 이는 §13의 `U1 (+U0 authz)`·`U3 (+U0 adapter/queue)` 표기와 정합.
- **하드 제약 반영**: 승자 선택 없음(C-2)은 E4-S5 owner U4 + `.okc/` read-path(U0) 지원으로 표현(별도 winner-select 스토리·유닛 없음). 소스 ≤10(C-3)은 E2-S5/E3-S2가 U2·U3 공동 관여(owner는 각 에픽). verify/explain 큐 우회(비변경)는 E5-S3의 U0 adapter read-path 지원으로 표기.
- **적대적 검증 수정(wf_9cd09efe-6b8)**: verifier 판정 coverage_ok·acyclic_ok·consistency_ok·codeorg_ok 전부 true(29/29, 순환 없음). 지적 1건 반영 — E1-S5(시크릿 저장)는 화면 있는 스토리(E1-1/E1-5)이므로 §2.1·§3 지원 유닛에 **U6 추가**(§2.7 "화면 있는 스토리는 U6 지원" 원칙과 정합; owner·커버리지 합계 불변). 참고: `component-dependency.md` 매트릭스의 잠재 U2↔U3 상호간선은 U2의 project/sources 접근을 U0 `shared.state`로 귀속시켜 DAG·빌드순서를 보존(본 산출물 및 dependency 산출물에서 명시적 해소).

_다음_: 유닛 경계·의존성 검증(순환 없음, U0 단방향 기반) → CONSTRUCTION PHASE(U0→U1→U2→U3→U4→U5, U6 인터리브).