# Application Design Plan — okc-web (Obsidian Vault 통합 웹 플랫폼)

**Stage**: INCEPTION / Application Design — Part 1 (Planning)
**Depth**: Comprehensive (execution-plan.md 결정)
**Date**: 2026-09-08
**Grounding**: `requirements.md`(FR/NFR/C-1..8/§9), `okc-core-capability-analysis.md`, `decision-records.md`(경우 B), `stories.md`(29 stories/5 epics), `personas.md`, `execution-plan.md`(7-unit U0–U6), 선착수 `ui-screens.md`/`design-system.md`

> **✅ 답변 상태**: 사용자의 기존 선호("추천값으로 채워줘", 해커톤 속도 우선)에 따라 **각 질문에 해커톤 권장값을 미리 채워** 두었습니다.
> 바꾸고 싶은 항목의 `[Answer]:` 만 수정하시면 됩니다. 그대로 좋으면 "승인"이라고 알려 주세요.

> **범위 노트**: 프런트엔드 화면/라우트/IA/디자인 토큰은 `ui-screens.md`(23개 화면 + 3 focal) · `design-system.md`(색·타이포·배지·App Shell·RBAC 내비)에서 이미 확정됨. 본 Application Design은 **백엔드 서비스/컴포넌트 계층 + 의존성 계약 + 어댑터 경계**에 comprehensive 집중하고, 프런트는 그 API에 대한 배선 계약만 정의(중복 재설계 금지).

---

## A. 설계 결정 질문 (Part 1)

## Question 1 — 백엔드 모듈 조직 방식은?
execution-plan의 7-unit(U0–U6)을 백엔드 코드 구조에 어떻게 반영할지.

A) **단일 axum 바이너리 크레이트 + 유닛 경계별 모듈**(`adapter`, `auth`, `upload`, `orchestration`, `review`, `serving`, `shared`) — okc-interop는 path dep. 해커톤 단순성 + 유닛↔모듈 1:1 추적성(C1/C6) — 권장

B) 멀티 크레이트 워크스페이스(유닛별 lib 크레이트 + 얇은 bin) — 경계 강제 강하나 해커톤엔 과함

C) 계층형(handlers / services / repositories / adapter) — 유닛 경계와 교차

X) 기타 (please describe after [Answer]: tag below)

[Answer]: A

## Question 2 — okc-interop 어댑터 인터페이스 스타일은? (ADR-0002 경계)
okc-web 나머지 코드가 okc-interop 타입에 직접 의존할지, 트레이트 뒤로 숨길지.

A) **트레이트 경계**(`trait OkcEngine`) — 하나의 concrete impl이 okc-interop을 래핑, 나머지 모듈은 트레이트+okc-web 자체 typed-DTO에만 의존. ADR-0002 어댑터 원칙 강제 + schema-version 가드 지점 단일화 + 테스트 mock 용이 — 권장

B) okc-interop 타입 직접 사용(어댑터 얇게) — 빠르나 경계 흐려짐·mock 어려움·schema drift 노출

X) 기타 (please describe after [Answer]: tag below)

[Answer]: A

## Question 3 — okc-web 자체 상태(계정/역할/업로드 토큰/큐레이터 결정·감사/Job/세션) 저장소는?
okc-core는 자체 프로젝트 데이터만 로컬 저장(C-6). okc-web의 운영 상태는 별도 필요.

A) **SQLite**(단일 파일, 임베디드, 트랜잭션) — 단일 프로세스 토폴로지에 정합, 동시성 안전, 스키마·마이그레이션 명확. `sqlx`(compile-time 검증) 또는 `rusqlite` — 권장

B) JSON/파일 기반 저장 — 초간단하나 동시성·무결성 취약(단일-writer 큐가 있어도 감사 append 경합)

C) 인메모리만(프로세스 재시작 시 소실) — 데모 중 재시작하면 계정·토큰·승인 소실 위험

X) 기타 (please describe after [Answer]: tag below)

[Answer]: A

## Question 4 — 관리자 세션 인증 방식은?
로그인 후 admin 세션 유지 방식(업로드 토큰과는 별개).

A) **서버측 세션**(HTTP-only·SameSite 쿠키에 opaque 세션 ID, 세션 레코드는 SQLite) — 즉시 폐기 가능·JWT 키관리 불필요·데모 단순, 비밀번호는 argon2/bcrypt 해시(NFR-SEC-1) — 권장

B) 무상태 JWT(서명 토큰) — 스케일 이점 있으나 폐기·키관리 부담, 단일 프로세스엔 이득 적음

X) 기타 (please describe after [Answer]: tag below)

[Answer]: A

## Question 5 — 업로드 토큰(FR-UP-1,2) 표현·저장은?
contributor가 로그인 없이 `/u/[token]`으로 업로드하는 자격증명.

A) **고엔트로피 랜덤 토큰, 저장은 해시(salted)**, 발급 시 1회만 평문 노출(ui-screens E2-2와 정합), 프로젝트-슬롯·만료·폐기 상태를 SQLite에 기록 — 권장

B) 토큰을 평문 저장(조회 편의) — NFR-SEC-1 위반, 비권장

X) 기타 (please describe after [Answer]: tag below)

[Answer]: A

## Question 6 — 통합 실행 직렬화(NFR-CONC-1) 구현 패턴은?
scheduler/PROJECT_RESERVATIONS가 프로세스-글로벌 statics → okc-web이 스스로 직렬화해야 함.

A) **단일 소유 in-process 워커**(엔진 핸들을 소유한 1개 tokio task + mpsc 작업 큐; 모든 변경성 엔진 op이 이 큐를 통과) — 동시 요청은 큐잉, 코어 경합 시 `PROJECT_BUSY`→재시도 안내. 단일-writer 정확성 보장 — 권장

B) 전역 async mutex로 엔진 호출 감싸기 — 간단하나 공정성/취소/진행표시 약함

X) 기타 (please describe after [Answer]: tag below)

[Answer]: A

## Question 7 — Job 진행 상황(FR-INT-8) 전송 방식은?
수 분 소요 human-in-the-loop 통합의 진행/오류를 UI로 전달.

A) **클라이언트 폴링**(Job 상태 엔드포인트를 주기 폴링; TanStack Query) — 견고·단순, FR-INT-8 "Job 이벤트 폴링"과 정합, 프록시/재접속에 강함 — 권장

B) SSE 스트림 — 실시간감 좋으나 재접속/프록시 복잡, 데모 리스크

X) 기타 (please describe after [Answer]: tag below)

[Answer]: A

## Question 8 — 경우-B 큐레이터 결정(FR-INT-5)의 내부 표현은?
관리자 결정을 코어 전달 전 어떻게 모델링할지(승자 선택 변형 없음).

A) **명시적 `CuratorDecision` enum** {`ApproveCluster{omission_rationales, minor_waivers}`, `RegenerateCluster{feedback}`, `ApproveTaxonomy{edited_clusters, rationale}`} — append-only 감사 저장소에 기록 후 okc-interop `approve_cluster`/`regenerate_cluster`/`approve_taxonomy`로 매핑. winner-select 변형 부재를 타입으로 강제(C3 차별자·ADR-0024) — 권장

B) 자유형 필드로 결정 저장 후 핸들러에서 분기 — 타입 안전성·감사 명확성 약함

X) 기타 (please describe after [Answer]: tag below)

[Answer]: A

## Question 9 — API 표면 형태는?
프런트엔드(Next.js)와 okc-mcp가 소비할 HTTP 계약.

A) **REST 리소스 지향, JSON 본문**: admin 라우트는 쿠키 세션 인증, `/u/[token]/*` 업로드는 bearer 토큰 인증, `/api/serving/*`는 read-only(ui-screens 라우트 맵과 정합). 오류는 `OkcError{code,category}`→HTTP(401/403/404/405/409 PROJECT_BUSY/422 APPROVAL_REQUIRED) 단일 매핑 — 권장

B) GraphQL / RPC — 데모 대비 과함, 스크린샷·계약 가독성 낮음

X) 기타 (please describe after [Answer]: tag below)

[Answer]: A

## Question 10 — 프런트엔드 아키텍처 확정(UI는 선착수됨)
`ui-screens.md`/`design-system.md`가 화면·IA·토큰을 고정. 아키텍처만 확정.

A) **Next.js App Router**(서버 컴포넌트=셸/정적, 클라이언트 컴포넌트=리뷰/업로드/폴링 인터랙션), **TanStack Query**로 데이터·Job 폴링, shadcn/ui+Tremor+Lucide, axum API 소비 — 권장

B) Next.js Pages Router / SPA(CSR only) — 선착수 IA(3-tier 라우트)와 덜 정합

X) 기타 (please describe after [Answer]: tag below)

[Answer]: A

## Question 11 — okc-core 의존 형태·핀은?
NFR-PORT-1: 특정 commit 핀 + CI 빌드.

A) **개발 중 local path dep + CI에서 정확한 commit 핀(빌드 검증)** — 실제 핀 해시는 U0/Build&Test에서 확정(Workflow Planning open-question #2). 지금은 형태만 확정 — 권장

B) 지금 즉시 commit 핀 고정 — 아직 U0 착수 전이라 이르나, 사용자가 해시를 알면 지정 가능

X) 기타 (please describe after [Answer]: tag below)

[Answer]: A

---

## B. 설계 산출물 생성 체크리스트 (Part 2에서 실행)

> 승인 후 Part 2(Generation)에서 실행. 각 항목은 생성 시 [x]로 표시.

### 필수 산출물 (application-design.md 규칙 Step 10)
- [ ] `components.md` — 컴포넌트 정의·책임·인터페이스 (U0–U6 백엔드 컴포넌트 + 프런트 컴포넌트 그룹 참조)
- [ ] `component-methods.md` — 메서드 시그니처·입출력 타입·고수준 목적 (상세 비즈니스 규칙은 Functional Design)
- [ ] `services.md` — 서비스 정의·책임·오케스트레이션 패턴 (IntegrationEngine 서비스·직렬화 큐·세션·토큰·업로드·서빙)
- [ ] `component-dependency.md` — 의존성 매트릭스·통신 패턴·데이터 흐름 다이어그램(Mermaid + 텍스트 대안)
- [ ] `application-design.md` — 위 문서 통합본

### Comprehensive 심화 포인트 (execution-plan 방침)
- [ ] ADR-0002 어댑터 경계: `OkcEngine` 트레이트 + typed-DTO interop schema v2 가드 지점 명시
- [ ] RBAC-before-core 게이트 배치: 모든 변경성 op이 okc-interop 호출 **이전** 인가, 403=코어 미접근 보장
- [ ] 단일 장기 엔진 프로세스 + in-process 단일-writer 직렬화 큐(PROJECT_BUSY 정책) 토폴로지
- [ ] 경우-B 결정 표면 매핑(`CuratorDecision`→approve_cluster/regenerate_cluster/approve_taxonomy, winner-select 부재)
- [ ] 해시 바인딩 staleness 캐스케이드(freeze-then-run) 상태 소유 지점
- [ ] OkcError code/category→HTTP 단일 매핑 모듈(메시지 파싱 금지)
- [ ] 화면→story-ID 매트릭스(C1) + epic-정렬 모듈 맵(C6)
- [ ] E4/E5 focal 화면의 백엔드 데이터 계약(실 DTO 렌더, mock 금지)
- [ ] okc-web 상태 데이터 모델(계정/역할/토큰/CuratorDecision 감사/Job/세션) 개요

### 검증 & 정합
- [ ] okc-core 코드 근거로 어댑터 메서드 매핑 검증(존재하는 API만 소비 — ADR-0002)
- [ ] 산출물 상호 정합성(컴포넌트↔메서드↔서비스↔의존성) 및 요구사항/스토리 추적성
- [ ] 6-criteria 채점 게이트 self-check + state↔disk 정합

---

## C. 참고 — 잠정 백엔드 컴포넌트 지도 (Part 2에서 확정·상세화)

| 컴포넌트(모듈) | 유닛 | 주 책임 | okc-core 접점 |
|---|---|---|---|
| `adapter` (OkcEngine 트레이트+impl) | U0 | okc-interop 래핑, schema-version 가드, DTO 변환 | OkcClient/Project/Job |
| `engine` (직렬화 큐·워커) | U0 | 단일-writer 실행 큐, PROJECT_BUSY, Job 상태 | 모든 변경성 op 통과 |
| `authz` (가드 미들웨어) | U0/U1 | RBAC-before-core, 세션 검증 | — (코어 이전) |
| `errors` (OkcError→HTTP) | U0 | 단일 에러 매핑, code/category 분기 | OkcError |
| `audit` (append-only 결정 저장소) | U0 | CuratorDecision·curator_id·rationale·timestamp | — |
| `auth` | U1 | 계정·역할·로그인·세션·토큰 해시 | curator_id 라벨 |
| `upload` | U2 | 토큰 발급/폐기, 업로드 검증, landing, add_source | add_source |
| `orchestration` | U3 | 프로젝트·freeze·Checkpoint 루프·compile·disclosure | checkpoint/compile |
| `review` | U4 | taxonomy/critic 리뷰, 결정 표면, 게이트 | approve_*/regenerate |
| `serving` | U5 | read-only vault API, verify/explain, mcp 계약 | verify/explain |
| `web` (Next.js) | U6 | 화면 배선, 폴링, 상태 매트릭스 | (API 소비) |

---

_승인 시 다음_: Part 2(Generation) → 5개 산출물 생성(comprehensive) → 완료·승인 게이트.
