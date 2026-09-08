# Requirements — okc-web (Obsidian Vault 통합 웹 플랫폼)

**Phase**: INCEPTION / Requirements Analysis
**Depth**: Comprehensive (다중 컴포넌트 · cross-system · 높은 리스크)
**Generated**: 2026-09-08
**Grounding**: `okc-core-capability-analysis.md` (okc-core v0.3.0 실제 능력 분석)
**Scope profile**: **해커톤 MVP** — 답변(Q1–Q13, `requirement-verification-questions.md`) 기준

---

## 1. Intent Analysis

- **User Request (원문 요지)**: `@../okc-core` 를 이용해 Obsidian Vault 통합 웹 플랫폼을 만든다.
  1. 권한 관리를 통해 부서간 개인 Vault를 통합하는 서비스
  2. 관리자 권한자가 통합을 수행
  3. 통합 중 okc-core가 conflict를 내면 관리자가 확인·선택해서 통합
  4. 개인마다 local Vault를 업로드할 수 있는 URL/API + token 제공
  5. 합쳐진 Vault를 okc-mcp에서 RAG로 쓰도록 연결하는 URL/API 제공 (okc-mcp는 현재 미구현)
- **Request Type**: New Project (greenfield 웹 플랫폼, 기존 Rust 엔진 okc-core 위 어댑터)
- **Scope Estimate**: Cross-system (okc-web ↔ okc-core 의존성 ↔ 미래 okc-mcp)
- **Complexity Estimate**: Complex (권한·업로드·통합 오케스트레이션·서빙 4개 신규 계층)
- **Clarity**: 의도는 명확하나 다수 아키텍처 결정 미지정 → 13문항으로 확정

## 2. 확정된 범위 결정 (Q&A 기반)

| # | 결정 | 값 | 근거 |
|---|---|---|---|
| Q1 | 테넌시 | **단일 조직 / 단일 프로세스** | 격리·멀티테넌시 배제로 MVP 단순화 |
| Q2 | 인증 | **토큰/비밀번호 + 역할 플래그** (RBAC) | OIDC/SSO 미도입, 빠른 구현 |
| Q3 | 소스 상한 | **≤10 소스로 제한, federation 미구현** | okc-core 하드 제한 준수, 코어 무수정 |
| Q4 | conflict 의미 | **기존 API로 재해석** (승인/waive/omission/regenerate) | 승자 선택 API 부재 → 코어 무수정 |
| Q5 | 통합 방식 | **Freeze-then-run** | hash-bound 단일 실행 모델에 부합 |
| Q6 | 업로드/토큰 | **okc-web 토큰 스토어 + 로컬 디스크 착지 → add_source** | S3 없이 로컬로 완결 |
| Q7 | req5 서빙 | **okc-web이 read-only API 제공, okc-mcp 이연** | okc-mcp 미구현 |
| Q8 | 백엔드 스택 | **Rust(axum) + okc-interop 직접 링크** ⚠️팀 숙련도 시 Python로 변경 | FFI 고통 없음, 타입드 |
| Q9 | 토폴로지 | **단일 장수 엔진 프로세스, 실행 직렬화** | 분산 락 불필요 |
| Q10 | 코어 안정성 | **pre-stable 0.3.0 수용, Markdown 전용, commit 핀** | 데모 실현성 |
| Q11–13 | 확장 | Security/Resiliency/PBT **전부 opt-out** | 해커톤 PoC |

## 3. Actors / Personas

- **Administrator (Curator)**: 로그인 후 프로젝트 생성·소스 고정·통합 실행·conflict/critic 리뷰·승인·컴파일·서빙 공개. okc-core `curator_id` 로 매핑.
- **Contributor (개인 기여자)**: 발급받은 업로드 토큰으로 자신의 로컬 Vault(디렉터리 zip/tar.zst)를 업로드. 통합 권한 없음.
- **Viewer (선택)**: 컴파일된 Vault를 조회. (MVP에서는 admin/contributor로도 충분, 선택 구현)
- **okc-mcp (미래 소비자, 비인간)**: okc-web이 노출한 read API/디렉터리를 RAG 소스로 소비. **본 범위에서는 계약만 정의, 구현 이연**.

## 4. Functional Requirements

> 표기: 각 FR은 원 요구사항 ①–⑤ 로 추적. `[코어]`=okc-core 재사용, `[신규]`=okc-web 구현.

### 4.1 인증 · 권한 (원 요구사항 ①②)
- **FR-AUTH-1** `[신규]` 시스템은 사용자 계정과 토큰/비밀번호 기반 로그인을 제공한다.
  - AC: 유효 자격증명 시 세션/토큰 발급, 무효 시 거부. 인증 없는 변경성 요청은 401.
- **FR-AUTH-2** `[신규]` 역할 모델(`admin`, `contributor`, 선택적 `viewer`)을 두고 사용자에게 부여한다.
- **FR-AUTH-3** `[신규]` 모든 변경성 통합 작업(프로젝트 생성, add_source, approve_*, regenerate, compile, 서빙 공개)은 **`admin` 역할만** 수행 가능하다. (RBAC 게이팅) — 원 요구사항 ② 강제.
  - AC: `contributor` 가 통합 API 호출 시 403.
- **FR-AUTH-4** `[신규→코어]` 인증된 admin의 식별자를 okc-core 프로젝트의 `curator_id`(비검증 라벨)로 전달·기록한다.
  - 근거: okc-core는 호출자를 인증하지 않음 → 게이팅은 okc-web 책임.

### 4.2 개인 Vault 업로드 (원 요구사항 ④)
- **FR-UP-1** `[신규]` admin은 contributor(또는 부서/개인 단위)에 대해 **업로드 토큰을 발급/조회/폐기**할 수 있다.
- **FR-UP-2** `[신규]` 시스템은 **토큰 인증 업로드 엔드포인트(URL/API)** 를 제공한다. 토큰만으로 해당 슬롯에 업로드 가능하고, 전체 로그인은 불필요할 수 있다.
  - AC: 유효 토큰 → 업로드 수락; 만료/폐기/무효 토큰 → 거부.
- **FR-UP-3** `[신규]` 업로드 바이트를 검증한다: 허용 포맷(디렉터리 zip / tar.zst), 크기 상한, Markdown 중심 콘텐츠, 경로/심볼릭 링크 안전성(코어의 hostile-input 보호와 중복).
- **FR-UP-4** `[신규→코어]` 검증 통과분을 **로컬 절대경로에 착지**시키고, `owner_display_name`(개인/부서 라벨)과 함께 okc-core `add_source` 로 등록한다.
  - 제약: 한 프로젝트당 **소스 ≤10**(FR-INT-1 참조).

### 4.3 통합 · 충돌 리뷰 (원 요구사항 ①③)
- **FR-INT-1** `[신규→코어]` admin은 프로젝트를 생성하고(`curator_id` 설정), **소스를 최대 10개까지 고정(freeze)** 한다.
- **FR-INT-2** `[신규→코어]` **Freeze-then-run**: 소스 고정 후 okc-core `IntegrationCheckpoint` 루프(NeedsProvider→…→ReadyToCompile→Verified)를 오케스트레이션한다. 소스/설정 변경 시 이전 승인이 무효화됨을 UX에 반영한다.
- **FR-INT-3** `[신규→코어]` 시스템은 okc-core가 생성한 **taxonomy proposal** 을 표시하고, admin이 편집/승인(`approve_taxonomy(edited_clusters, rationale)`)하게 한다.
- **FR-INT-4** `[신규→코어]` 클러스터별 **synthesis 결과 + critic findings** 를 표시한다. (Major/Critical=차단, Minor=waive 대상)
- **FR-INT-5** `[신규→코어]` **원 요구사항 ③(conflict 확인·선택)** 을 다음 결정 표면으로 제공한다:
  - (a) 클러스터 **승인**(`approve_cluster`), (b) Minor 지적 **waive**(사유 필수), (c) **omission 제안**(사유 필수), (d) 피드백으로 **regenerate**(`regenerate_cluster`).
  - **명시적 제약**: okc-core에는 "승자 선택" API가 없다. Major/Critical은 waive 불가이며 오직 regenerate로만 해소된다. 이 사실을 admin UI가 명확히 안내한다.
  - AC: admin이 차단 findings를 남긴 채 컴파일을 시도하면 거부(`APPROVAL_REQUIRED` 계열)한다.
- **FR-INT-6** `[신규→코어]` 보존된 **모순(Contradictions)** 을 정보성으로 표시한다(모든 근거 포함, 승자 없음).
- **FR-INT-7** `[신규→코어]` 모든 승인 완료(`ApprovedIntegrationPlan`) 시 **컴파일**을 실행해 병합 Vault 디렉터리(`knowledge/` + `legacy/` + `.okc/`)를 생성한다. 기존 산출 대상은 덮어쓰지 않는다(no-clobber).
- **FR-INT-8** `[신규→코어]` 통합 진행상황과 오류를 표시한다. Job 이벤트를 폴링해 UI에 전달하고, 오류는 `OkcError`의 **code/category 로 분기**(메시지 문자열 파싱 금지)한다. `PROJECT_BUSY` 는 재시도 안내.

### 4.4 병합 Vault 서빙 (원 요구사항 ⑤)
- **FR-SRV-1** `[신규]` 컴파일된 Vault를 **read-only HTTP API/URL** 로 노출한다(파일 목록·본문 조회). — okc-mcp가 RAG 소스로 소비할 대상.
- **FR-SRV-2** `[신규→코어]` `verify()` / `explain()` 결과(무결성·provenance)를 조회 가능하게 노출한다.
- **FR-SRV-3** `[신규]` **okc-mcp 연동 계약**을 정의한다: okc-mcp가 소비할 Vault의 위치(로컬 디렉터리 경로 또는 read API 엔드포인트)와 형식. **okc-mcp 자체(청킹·임베딩·vector index·쿼리)는 본 범위에서 구현하지 않고 이연**한다.
  - 명시: RAG 리트리벌은 okc-mcp(또는 외부 엔진)의 몫이며, 임베딩은 컴파일 후 산출물을 **재임베딩**해야 한다(코어 임베딩은 일시적).

## 5. Non-Functional Requirements

- **NFR-SEC-1** 인증 토큰/비밀번호는 안전 저장(해시/시크릿)한다. 업로드 입력은 비신뢰로 취급(크기·포맷·경로 검증). *보안 확장 규칙은 미적용(Q11=B)이나 위 최소 기준은 기능 요구로 유지.*
- **NFR-CONC-1** 단일 엔진 프로세스에서 통합 실행을 **직렬화**한다. 동시 변경 시 `PROJECT_BUSY` 를 재시도/큐잉으로 처리한다. cross-process 락은 `project.lock` 에 의존(다중 프로세스 미도입).
- **NFR-AVAIL-1** 통합은 수 분 소요의 human-in-the-loop 장기 작업이다. 작업 상태를 저장하고, 컴파일 publish barrier 이전에만 취소를 허용한다.
- **NFR-STORE-1** okc-core 프로젝트 데이터는 평문·single-writer 로컬 저장이다. **데모 범위에서는 로컬/신뢰 환경 가정**(암호화·격리는 out of scope, §6).
- **NFR-OBS-1** 진행상황·오류를 사용자에게 표시한다. 로깅은 안정적 error code/category 기준.
- **NFR-PORT-1** okc-core는 **특정 commit에 핀**하고 CI에서 빌드한다(바인딩 미배포). 출력은 **Markdown 전용**.
- **NFR-DET-1** 산출물의 결정성·provenance(파일별 `ProvenanceRecord`)를 보존·노출한다. (검증은 내부 일관성 증명이지 발행자 진위 보증이 아님을 UI에 명시.)
- **NFR-PERF-1** 데모 규모: 프로젝트당 소스 ≤10, in-memory cosine 후보 생성의 O(n²) 특성을 고려한 코퍼스 크기 상한.

## 6. Out of Scope / Deferred (명시적 제외)

- 멀티테넌시·조직 간 격리, 저장 데이터 at-rest 암호화 (Q1=A)
- OIDC/SSO 연동 (Q2=B)
- 10 소스 초과를 위한 **federation** 구현 (Q3=X — 설계 방향만 문서화)
- **okc-mcp 구현**(RAG 청킹/임베딩/vector index/쿼리, MCP 툴 표면) (Q7=A)
- 비-Markdown 자료(첨부/Canvas/Base), 완전 link-rewrite, OKCPack (Q10=A, 코어 release blocker)
- 다중 프로세스·외부 분산 락/큐 (Q9=A)
- 증분(incremental) 재통합 (Q5=A)
- okc-core에 대한 신규 public API/코어 수정 (Q3=C, Q4=B/C 배제)
- Security / Resiliency / Property-Based Testing 확장 규칙 강제 (Q11–13 opt-out)

## 7. Constraints & Assumptions (okc-core 현실)

- **C-1** okc-core는 인증/RBAC/멀티테넌시가 없다 → 권한은 100% okc-web 책임.
- **C-2** okc-core는 conflict "승자 선택" API가 없다(모순 보존, 구조적 conflict 폐기, Major/Critical waive 불가) → 원 요구사항 ③은 승인/waive/omission/regenerate 로 재해석(§4.3).
- **C-3** 프로젝트당 소스 ≤10 (ADR-0016).
- **C-4** 승인은 hash-bound 단일 실행 → freeze-then-run.
- **C-5** okc-core는 HTTP/업로드/토큰/서빙/RAG 레이어가 없다 → §4.2/§4.4 는 전부 신규.
- **C-6** 통합/컴파일은 절대 로컬 경로 기반 → 업로드는 디스크 착지 후 등록.
- **C-7** 권장 통합: Rust 백엔드가 `okc-interop` 직접 링크(타입드·async Job·구조화 에러). 바인딩 경로는 payload 불투명 JSON·소스 빌드 비용.
- **A-1** 데모는 로컬/신뢰 환경에서 단일 조직·소수 사용자·소수 Markdown Vault를 가정한다.
- **A-2** AI provider(임베딩/합성) 자격증명은 서버측에 env-var 이름으로 프로비저닝된다.

## 8. Boundary Summary (요약)

- **okc-core 재사용**: 다중 소스 인제스트(≤10), AI 파이프라인(embed→taxonomy→synthesis→critic), curator 승인 게이트, 모순 보존, 결정론적 compile/verify/explain, 구조화 에러·Job 모델, provider 자격증명 저장, provenance.
- **okc-web 신규 구축**: 인증/RBAC, 업로드+토큰 서비스, IntegrationCheckpoint 오케스트레이션, conflict/critic 리뷰 UI, 병합 Vault read API, okc-mcp 연동 계약, 진행상황 표시.
- **okc-mcp 이연**: RAG 리트리벌 전체, MCP 툴 표면.

## 9. Demo Acceptance Scenario (End-to-End)

1. **업로드**: contributor가 발급 토큰으로 `.md` Vault(zip)를 업로드 → 검증 후 소스 등록(owner_display_name). *(FR-UP-1..4)*
2. **통합 시작**: admin 로그인 → 프로젝트 생성 → 소스 2~3개 고정 → 통합 실행. *(FR-AUTH-3, FR-INT-1,2)*
3. **리뷰·선택**: taxonomy/critic findings 표시 → admin이 승인/waive/regenerate 선택; 차단 findings 존재 시 컴파일 거부 안내. *(FR-INT-3..6)*
4. **컴파일**: 승인 완료 → 병합 Vault 생성. *(FR-INT-7)*
5. **서빙**: 병합 Vault를 URL로 조회 + provenance 확인; okc-mcp가 이 URL/경로를 RAG 소스로 소비할 수 있음을 계약으로 제시. *(FR-SRV-1..3)*

## 10. Traceability Matrix

| 원 요구사항 | 대응 FR |
|---|---|
| ① 권한 관리로 부서간 개인 Vault 통합 | FR-AUTH-1..4, FR-INT-1..2,7 |
| ② 관리자만 통합 수행 | FR-AUTH-2,3,4 |
| ③ conflict 확인·선택 통합 | FR-INT-4,5,6 (재해석: 승인/waive/omission/regenerate) |
| ④ 개인 업로드 URL/API + token | FR-UP-1..4 |
| ⑤ 병합 Vault를 okc-mcp RAG로 연결 | FR-SRV-1..3 (okc-mcp 구현은 이연) |

## 11. Key Risks (요약; 상세는 capability-analysis §5)

- **R-1** 원 요구사항 ③의 "선택" 기대와 okc-core 모델(보존/재생성)의 간극 → §4.3 재해석 및 UI 안내로 관리. 사용자 재확인 권장.
- **R-2** req ⑤의 "RAG" 기대 대비 okc-mcp 부재 → okc-web은 소스 제공까지만, RAG는 이연(사용자 합의됨, Q7=A).
- **R-3** pre-stable 0.3.0 + Markdown 전용 → 데모 콘텐츠를 .md로 한정.
- **R-4** 소스 10 상한 → 데모 규모 제한(federation 미구현).
- **R-5** Q8 스택 선택은 팀 Rust 숙련도에 민감(재검토 여지).

---

_다음 단계 후보_: **User Stories**(다중 페르소나: admin/contributor/viewer/okc-mcp → 실행 권장) → Workflow Planning → Application Design → Units Generation.
