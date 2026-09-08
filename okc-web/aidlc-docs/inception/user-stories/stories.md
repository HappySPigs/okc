# User Stories — okc-web (Obsidian Vault 통합 웹 플랫폼)

**Stage**: INCEPTION / User Stories — Part 2 (Generation, Finalized)
**Role**: Product Owner
**Grounding**: `requirements.md`(FR-AUTH/UP/INT/SRV, NFR, §9 데모 시나리오, §10 추적성) · `okc-core-capability-analysis.md`(C-1..C-7 하드 제약) · `story-generation-plan.md`(Q1..Q7 승인값)
**Methodology (승인값)**: 페르소나 3종(Q1=A) · 페르소나×에픽 하이브리드(Q2=A) · 중간 세분성/INVEST(Q3=A) · Given/When/Then(Q4=A) · MVP 데모 경로 한정(Q5=A) · MoSCoW(Q6=A) · req③ 재해석 결정 표면(Q7=A)
**Finalize note**: 검토 게이트 크리틱의 recommended_fixes를 전부 반영(신규 Must 스토리 E3-S4 추가로 데모-경로 공백 해소, 비테스트성 AC 재작성, E3-S3 분할, curator_id 중복 제거, no-clobber 귀속 정정, E4-S2 스택 정합화, 기여자 커버리지 보강 E2-S6, 페르소나 로그인 정합화).

---

## 0. 공통 규약 (모든 스토리에 적용되는 okc-core 하드 제약)

이 규약은 각 스토리의 AC에서 반복되며, 위반 시 오버프로미스로 간주한다.

- **C-1 인증 부재**: okc-core에는 auth/RBAC/멀티테넌시가 없다. 모든 권한 게이팅은 okc-web 책임이며, admin 식별자는 okc-core에 **검증되지 않는 `curator_id` 자유 텍스트 라벨**로만 전달된다(감사/표시 전용, 신뢰 경계 아님).
- **C-2 승자 선택 없음**: 모순(Contradiction)은 모든 근거와 함께 **보존**되고 "승자 선택" API가 없다. 구조적(path/link) conflict는 내부에서 폐기되어 caller에 도달하지 않는다. Critic **Major/Critical 지적은 waive 불가**(오직 `regenerate_cluster`로만 해소), **Minor 지적만 사유와 함께 waive 가능**.
- **C-3 소스 ≤10**: 프로젝트당 소스 하드 상한 10개(ADR-0016). federation은 범위 밖.
- **C-4 hash-bound freeze-then-run**: 승인은 hash-bound 단일 실행. 상위 입력(소스/설정/taxonomy)이 하나라도 바뀌면 모든 하위 승인이 **무효(stale)** 가 된다.
- **C-6 compile 결정론/offline**: compile은 offline·결정론적으로 Markdown 전용 병합 Vault(`knowledge/`+`legacy/`+`.okc/`)를 생성하며 publish는 **no-clobber**.
- **C-5/C-7 서빙·스택**: HTTP/업로드/토큰/서빙/RAG는 전부 okc-web 신규. 확정 스택은 Rust axum + `okc-interop` 직접 링크(Q8=A) → critic/synthesis 결과는 **타입드 DTO(interop schema v2)** 로 전달(불투명 JSON `VersionedPayload`는 Python/Node 바인딩 폴백 경로에서만).
- **오류 규약(FR-INT-8)**: 오류는 `OkcError{code,category}` 의 **code/category로 분기**(메시지 문자열 파싱 금지). `PROJECT_BUSY` 는 재시도/큐잉으로 처리한다.

---

## Epic E1 — 인증 & 권한 (Authentication & Authorization)

**원 요구사항**: ①(권한 관리로 부서간 개인 Vault 통합) · ②(관리자만 통합 수행)
**대응 FR**: FR-AUTH-1..4
**핵심 제약(C-1)**: okc-core에 auth/RBAC/멀티테넌시가 전무 → 모든 인증·역할·게이팅은 100% okc-web 책임. okc-core로 넘어가는 `curator_id`는 비검증 라벨.
**범위**: MVP 데모 경로. OIDC/SSO, at-rest 암호화, 멀티테넌시 격리는 out-of-scope(§6).

---

### E1-S1 — 관리자 로그인 및 세션/토큰 발급
**As an** 관리자(Administrator/Curator), **I want** 관리자 자격증명(비밀번호 또는 토큰)으로 로그인해 인증 세션/토큰을 발급받고, **so that** 이후 변경성(mutating) 통합 작업을 안전하게 수행할 수 있다.

**MoSCoW**: **Must** (§9 "관리자 로그인" 진입점)

**Acceptance Criteria (Given/When/Then)**
- **Given** 유효한 자격증명을 가진 등록된 admin, **When** 로그인 API를 호출하면, **Then** 시스템은 인증 세션/토큰을 발급하고 `admin` 역할을 해당 세션에 결합한다.
- **Given** 잘못되었거나 만료된 자격증명, **When** 로그인을 시도하면, **Then** 로그인을 401 계열로 거부하고 세션/토큰을 발급하지 않는다.
- **Given** 인증되지 않은 클라이언트, **When** 변경성(mutating) 요청을 보내면, **Then** **401**을 반환하고 요청을 처리하지 않는다.
- **Given** 기여자(Contributor)는 전체 로그인 없이 업로드 토큰만으로 접근한다는 방침(FR-UP-2, Persona 2), **When** 기여자가 업로드 경로를 사용하면, **Then** 로그인 세션 발급 흐름을 거치지 않으며 시스템은 기여자용 로그인 세션을 요구하지 않는다.

**제약/설계 노트**: 기여자 계정/로그인은 선택적이며 **데모 경로 밖**이다. okc-core는 호출자를 인증하지 않으므로(C-1) 모든 인증·세션 판단은 okc-web에서 완결되고, okc-core로는 검증되지 않은 라벨만 전달된다(바인딩 의미의 단일 소스는 E1-S4).

**Traceability**: FR-AUTH-1 (연계 FR-UP-2) · 원 요구사항 ①②

---

### E1-S2 — 역할 부여 및 관리 (admin/contributor)
**As an** 관리자, **I want** 사용자에게 역할(admin 또는 contributor)을 부여·변경하고, **so that** 각 사용자의 권한 경계를 명확히 하여 "관리자만 통합 수행"을 통제할 수 있다.

**MoSCoW**: **Should** (역할 모델은 필수이나, 데모에서는 admin/contributor 계정 시드로 대체 가능하므로 관리 UI/API 자체는 Should)

**Acceptance Criteria (Given/When/Then)**
- **Given** 로그인한 admin, **When** 특정 사용자에게 역할을 부여/변경하면, **Then** 변경된 역할이 저장되고 이후 발급되는 인증 세션에 반영된다.
- **Given** 신규 계정, **When** 계정이 생성되면, **Then** 정확히 하나의 역할을 가지며 기본값은 최소 권한인 `contributor`이다.
- **Given** contributor 세션, **When** 역할 부여/변경 API를 호출하면, **Then** **403**으로 거부된다(역할 관리도 admin 전용).
- **Given** Viewer를 별도 역할로 구현하지 않고 admin/contributor로 흡수한다는 방침(Q1=A), **When** 시스템의 역할 집합을 조회하면, **Then** 역할은 `admin`과 `contributor`만 존재하고 **별도 `viewer` 역할이 존재하지 않으며**, 조회 전용 접근은 admin/contributor 권한으로 처리된다.

**Traceability**: FR-AUTH-2 · 원 요구사항 ②

---

### E1-S3 — 관리자 전용 변경성 통합 작업 게이팅
**As a** 플랫폼 운영 관리자, **I want** 모든 변경성 통합 작업이 admin 역할로만 호출되도록 서버측에서 게이팅되기를 원하고, **so that** 원 요구사항 ②(관리자만 통합 수행)가 강제되어 기여자가 통합 상태를 바꾸지 못한다.

**MoSCoW**: **Must** (원 요구사항 ②의 직접 강제)

**Acceptance Criteria (Given/When/Then)**
- **Given** contributor 세션, **When** 변경성 통합 작업(프로젝트 생성, `add_source`, 공급자 설정/consent, `approve_taxonomy`/`approve_cluster`, `regenerate_cluster`, `compile`, 서빙 공개) 중 하나를 호출하면, **Then** 시스템은 **403**을 반환하고 okc-core를 **전혀 호출하지 않는다**.
- **Given** admin 세션, **When** 동일한 변경성 작업을 호출하면, **Then** 권한 검사를 통과하여 okc-core 오케스트레이션으로 진행한다.
- **Given** 인증되지 않은 요청, **When** 변경성 작업을 호출하면, **Then** **401**을 반환한다.
- **Given** 읽기 전용 작업(통합 상태/taxonomy 조회 등), **When** contributor가 호출하면, **Then** 게이팅 대상이 아니어야 한다(변경성 작업만 차단).

**제약/설계 노트**: okc-core는 게이팅을 보장하지 않으므로(C-1) 권한 판단은 **반드시 okc-core 호출 이전 단계**에서 수행된다(코어 무수정, ADR-0002 어댑터 원칙).

**Traceability**: FR-AUTH-3 (+FR-AUTH-2) · 원 요구사항 ②

---

### E1-S4 — 인증된 admin을 okc-core `curator_id` 라벨로 바인딩 (바인딩 의미의 단일 소스)
**As an** 관리자, **I want** 인증된 내 식별자가 okc-core 프로젝트의 `curator_id` 라벨로 전달·기록되기를 원하고, **so that** 통합 승인 감사 추적에 어떤 큐레이터가 결정을 내렸는지 남는다.

**MoSCoW**: **Must** (감사·provenance 무결성; 승인이 hash-bound로 기록되는 경로의 일부)

> **정합화 노트**: `curator_id` 바인딩의 의미·규약은 **본 스토리가 단일 소스**다. E3-S1(프로젝트 생성)은 이 바인딩을 *참조*만 하며 동일 AC를 중복하지 않는다(Independent 강화).

**Acceptance Criteria (Given/When/Then)**
- **Given** admin이 프로젝트를 생성/오케스트레이션할 때, **When** okc-web이 okc-core를 호출하면, **Then** 인증된 admin의 식별자를 `curator_id`로 설정하여 전달한다.
- **Given** 승인/waive/regenerate 등 결정 작업, **When** 실행되면, **Then** 해당 결정이 admin의 `curator_id`와 함께(hash-bound 승인 레코드로) 기록된다.
- **Given** okc-core는 `curator_id`를 검증하지 않는 자유 텍스트 라벨로 취급한다는 제약(C-1), **When** 이 값을 전달하면, **Then** okc-web은 라벨을 신뢰 경계로 사용하지 않으며 실제 권한 강제는 E1-S3 게이팅으로 수행한다(라벨은 감사/표시 전용).
- **Given** 미인증 또는 역할 없는 호출, **When** `curator_id` 바인딩 시점에 도달하면, **Then** E1-S3 게이팅에서 이미 차단되어 라벨 바인딩 자체가 발생하지 않는다.

**Traceability**: FR-AUTH-4 (+FR-AUTH-3) · 원 요구사항 ①②

---

### E1-S5 — 인증 시크릿(비밀번호/토큰) 안전 저장
**As a** 관리자(보안 담당), **I want** 로그인 비밀번호와 세션/업로드 토큰이 평문이 아닌 해시/시크릿 형태로 저장되기를 원하고, **so that** 저장소가 노출되어도 자격증명이 곧바로 유출되지 않는다.

**MoSCoW**: **Should** (보안 확장 규칙은 opt-out(Q11)이나, NFR-SEC-1의 최소 안전 저장은 기능 요구로 유지)

**Acceptance Criteria (Given/When/Then)**
- **Given** 사용자 비밀번호, **When** 저장되면, **Then** 평문이 아닌 솔트 포함 단방향 해시로 저장된다.
- **Given** 세션/업로드 토큰, **When** 저장·검증되면, **Then** 원문 대신 해시 또는 안전한 시크릿 참조로 다뤄진다.
- **Given** 저장된 시크릿, **When** 조회 API/목록을 통해 노출되면, **Then** 원문이 아닌 마스킹된 식별자/해시 참조만 반환된다.

**제약/설계 노트**: at-rest 암호화·멀티테넌시 격리는 out-of-scope(§6)이며 데모 신뢰 환경 가정(A-1). 본 스토리는 NFR-SEC-1의 최소 안전 저장만 강제한다.

**Traceability**: FR-AUTH-1 (NFR-SEC-1) · 원 요구사항 ①②

---

## Epic E2 — 개인 Vault 업로드 & 토큰 (Upload & Tokens)

**원 요구사항**: ④(개인마다 local Vault를 업로드할 수 있는 URL/API + token 제공)
**대응 FR**: FR-UP-1..4
**범위**: MVP 데모 경로(§9 1단계 "업로드"). federation/멀티테넌시/at-rest 암호화는 out-of-scope.
**okc-core 제약 반영**: auth 전무(게이팅은 okc-web) · `owner_display_name`은 비검증 장식 라벨 · 소스 ≤10(C-3) · 소스 변경 시 hash-bound 승인 무효화(C-4) · symlink 거부·hostile-input·content hashing 중복 검출은 코어에도 존재(중복 방어) · 오류는 code로 분기(C-8)·`PROJECT_BUSY` 재시도.

---

### E2-S1 — 업로드 토큰 발급
**As a** 관리자(Administrator/Curator), **I want** 특정 기여자/부서 단위로 업로드 토큰을 발급하고, **so that** 각 개인·부서가 전체 계정 로그인 없이 자신의 슬롯에만 Vault를 제출할 수 있다.

**MoSCoW**: **Must** (업로드의 전제)

**Acceptance Criteria (Given/When/Then)**
- **Given** admin 역할로 인증된 세션, **When** 대상 라벨(`owner_display_name`, 예: "인사팀/홍길동")과 대상 프로젝트/슬롯을 지정해 토큰 발급을 요청하면, **Then** 시스템은 고유 업로드 토큰과 업로드 URL을 **1회만** 표시하고, 토큰 원문은 저장하지 않고 해시(NFR-SEC-1)만 저장한다.
- **Given** 발급된 토큰, **When** 발급 응답을 확인하면, **Then** 대상 슬롯·`owner_display_name`·(선택)만료 시각 메타데이터가 함께 반환된다.
- **Given** `contributor` 역할 또는 비인증 사용자, **When** 토큰 발급 API를 호출하면, **Then** okc-web이 게이팅해 **403**으로 거부한다(okc-core에 auth가 없으므로 이 통제는 전적으로 okc-web 책임).
- **Given** 이미 1회 표시된 토큰, **When** 목록/상세를 다시 조회하면, **Then** 원문 토큰은 다시 표시되지 않고 마스킹된 식별자/해시 참조만 노출된다.

**Traceability**: FR-UP-1 (게이팅 FR-AUTH-3) · 원 요구사항 ④②

---

### E2-S2 — 업로드 토큰 조회 & 폐기
**As a** 관리자(Administrator/Curator), **I want** 발급한 업로드 토큰을 조회하고 폐기하고, **so that** 유출되었거나 더 이상 쓰지 않는 토큰으로의 업로드를 즉시 차단할 수 있다.

**MoSCoW**: **Should** (발급이 Must, 조회/폐기는 데모 안전성 강화)

**Acceptance Criteria (Given/When/Then)**
- **Given** 여러 토큰이 발급된 상태, **When** 관리자가 토큰 목록을 조회하면, **Then** 각 토큰의 상태(active/expired/revoked), `owner_display_name`, 대상 슬롯, 발급/만료 시각이 표시되고 원문 토큰 값은 노출되지 않는다.
- **Given** active 상태의 토큰, **When** 관리자가 해당 토큰을 폐기하면, **Then** 상태가 revoked로 전환되고 이후 그 토큰을 사용한 업로드 시도는 거부(401/403)된다.
- **Given** 이미 폐기·만료된 토큰, **When** 관리자가 다시 폐기를 시도하면, **Then** 멱등하게 성공 처리하거나 명확한 상태 메시지를 반환한다(중복 폐기로 오류가 나지 않는다).
- **Given** `contributor`/비인증 사용자, **When** 토큰 목록/폐기 API를 호출하면, **Then** okc-web이 **403**으로 거부한다.

**Traceability**: FR-UP-1 (게이팅 FR-AUTH-3) · 원 요구사항 ④②

---

### E2-S3 — 토큰 인증 업로드 엔드포인트
**As a** 기여자(Contributor, 개인 기여자), **I want** 발급받은 토큰과 업로드 URL로 내 로컬 Vault(디렉터리 zip / tar.zst)를 업로드하고, **so that** 전체 계정 로그인 없이 내 슬롯에 지식을 제출할 수 있다.

**MoSCoW**: **Must** (§9 1단계)

**Acceptance Criteria (Given/When/Then)**
- **Given** 유효(active·미만료)한 업로드 토큰, **When** 기여자가 업로드 엔드포인트로 Vault 아카이브를 전송하면, **Then** 시스템은 업로드를 수락하고 접수 식별자(수신 확인)를 반환한다.
- **Given** 만료·폐기·무효·누락 토큰, **When** 업로드를 시도하면, **Then** 401/403으로 거부되고 파일 바이트는 저장하지 않는다.
- **Given** 유효 토큰이지만 요청이 토큰에 바인딩된 슬롯 밖의 대상을 지정한 경우, **When** 업로드하면, **Then** 토큰에 바인딩된 슬롯/`owner_display_name` 범위로만 처리되고 범위를 벗어난 대상 지정은 무시/거부된다.
- **Given** 업로드가 접수됨, **When** 후속 검증(E2-S4)이 진행되면, **Then** 검증 결과(수락/거부 사유)를 조회할 수 있는 상태가 남으며, 이는 **기여자 피드백 표면(E2-S6)** 으로 노출된다.

**Traceability**: FR-UP-2 (연계 E2-S6) · 원 요구사항 ④

---

### E2-S4 — 업로드 바이트 검증 (포맷·크기·Markdown·경로/symlink 안전성)
**As a** 관리자(Administrator/Curator), **I want** 업로드된 바이트가 로컬 착지 전에 포맷·크기·콘텐츠·경로 안전성 기준으로 검증되기를 원하고, **so that** 악성·대용량·비Markdown 입력이 프로젝트 코퍼스에 유입되지 않는다.

**MoSCoW**: **Must** (안전한 소스만 등록)

**Acceptance Criteria (Given/When/Then)**
- **Given** 업로드된 아카이브, **When** 포맷을 검사하면, **Then** 디렉터리 zip 또는 tar.zst만 허용하고 그 외 포맷은 거부한다(거부는 `OkcError` 계열 code로 분기, 메시지 문자열 파싱 금지).
- **Given** 아카이브 크기(압축/해제 후), **When** 크기 상한을 검사하면, **Then** 상한 초과 시 거부하고 과도 팽창(zip-bomb 등)은 해제 단계에서 차단한다.
- **Given** 아카이브 내 각 엔트리, **When** 경로/심볼릭 링크 안전성을 검사하면, **Then** 경로 이탈(`../`)·절대경로·심볼릭 링크 엔트리를 거부한다(okc-core의 symlink 거부·hostile-input 보호와 중복되는 방어선으로, okc-web이 **착지 이전에 선제 차단**).
- **Given** 콘텐츠, **When** Markdown 중심 여부를 검사하면, **Then** 데모 범위에서 콘텐츠는 `.md`로 한정되어 비Markdown 자료는 정책에 따라 무시/거부되고, 검증 실패 시 기여자가 조회 가능한 명확한 거부 사유가 남는다(E2-S6).
- **Given** 검증 실패, **When** 처리 결과를 반영하면, **Then** 어떤 바이트도 착지 경로에 남지 않고 `add_source`도 호출되지 않는다.

**Traceability**: FR-UP-3 (NFR-SEC-1 최소 기준) · 원 요구사항 ④

---

### E2-S5 — 검증분 로컬 착지 & add_source 등록 (owner_display_name, 소스 ≤10)
**As a** 관리자(Administrator/Curator), **I want** 검증 통과분을 로컬 절대경로에 착지시키고 `owner_display_name`과 함께 okc-core `add_source`로 등록하고, **so that** 개인·부서 Vault가 통합 대상 소스가 된다.

**MoSCoW**: **Must** (업로드→소스 등록의 종결)

**Acceptance Criteria (Given/When/Then)**
- **Given** 검증을 통과한 업로드, **When** 시스템이 콘텐츠를 로컬 절대경로에 착지시키면, **Then** 해당 경로를 okc-core `add_source`에 전달하고 `owner_display_name`(비검증 장식 라벨)을 소스에 부착한다.
- **Given** 이미 10개의 소스가 등록된 프로젝트, **When** 11번째 소스를 `add_source`로 등록하려 하면, **Then** 하드 상한(소스 ≤10, C-3/ADR-0016)에 따라 거부되고 해당 `OkcError` code로 분기해 사용자에게 안내한다.
- **Given** 프로젝트가 다른 실행/변경으로 사용 중, **When** `add_source`를 시도해 `PROJECT_BUSY` code를 받으면, **Then** 재시도/큐잉으로 처리한다(메시지 문자열이 아닌 code로 분기).
- **Given** 이미 승인(`ApprovedIntegrationPlan`)이 존재하는 프로젝트, **When** 새 소스가 등록/변경되면, **Then** 소스 집합 변경이 hash-bound 승인을 무효화(stale)함을 UX에 반영한다(C-4 freeze-then-run).
- **Given** 동일 경로/내용의 재업로드, **When** 착지 및 등록을 시도하면, **Then** okc-core의 **content hashing/중복 검출**로 동일 콘텐츠가 식별되어 소스로 중복 등록되지 않는다.

> **정합화 노트**: 업로드 중복 억제는 okc-core의 content hashing/중복 검출 속성이다. `no-clobber`는 compile publish 속성이므로 여기에는 귀속하지 않는다(E3-S6 참조).

**Traceability**: FR-UP-4 (연계 FR-INT-1; 제약 C-3/C-4/C-6) · 원 요구사항 ④①

---

### E2-S6 — 업로드 결과·거부 사유 조회 (기여자 피드백 표면)
**As a** 기여자(Contributor), **I want** 내가 제출한 업로드의 처리 결과(수락 / 거부 사유)를 접수 식별자로 조회하고, **so that** 무엇이 왜 거부됐는지 명확히 알고 필요하면 재업로드할 수 있다.

**MoSCoW**: **Should** (Persona 2 가치 강화; 데모 서사의 "명확한 피드백" 표면)

**Acceptance Criteria (Given/When/Then)**
- **Given** 업로드가 접수되어 접수 식별자가 발급된 상태, **When** 기여자가 유효한 업로드 토큰과 접수 식별자로 결과 조회를 요청하면, **Then** 시스템은 해당 업로드의 상태(수락됨 / 검증 실패 / 처리 중)를 반환한다.
- **Given** 검증에서 거부된 업로드, **When** 기여자가 결과를 조회하면, **Then** 거부 사유가 안정적인 code/category(포맷·크기·경로/심볼릭 링크·비Markdown 등)로 반환되어 기여자가 원인을 식별할 수 있다(메시지 문자열 파싱 불필요).
- **Given** 수락되어 소스로 등록된 업로드, **When** 기여자가 결과를 조회하면, **Then** 등록된 `owner_display_name`(개인/부서 라벨)이 함께 표시된다.
- **Given** 기여자는 통합 의사결정 표면(taxonomy/critic/승인)에 접근할 수 없다, **When** 기여자가 결과 조회 범위를 벗어난 통합 상태를 조회하려 하면, **Then** 응답은 자신의 업로드 결과 범위로만 한정되고 통합 리뷰 데이터는 노출되지 않는다.

**Traceability**: FR-UP-3 (연계 FR-UP-2, FR-UP-4) · 원 요구사항 ④

---

## Epic E3 — 통합 오케스트레이션 (Integration Orchestration)

**원 요구사항**: ①(권한 관리로 부서간 개인 Vault 통합) · 연계 ②③
**대응 FR**: FR-INT-1, FR-INT-2, FR-INT-7, FR-INT-8
**페르소나**: Administrator/Curator
**에픽 초점**: 프로젝트 생성(curator_id)·소스 ≤10 고정 → freeze-then-run으로 IntegrationCheckpoint 루프 오케스트레이션 → **NeedsProvider/NeedsDisclosure 체크포인트 해소(공급자·disclosure/consent)** → 소스/설정 변경 시 승인 무효화 UX → 승인 완료 시 compile(no-clobber) → 진행상황/오류 표시.

> **okc-core 제약 반영**: 권한 게이팅은 okc-web 책임(C-1), 승인은 hash-bound 단일 실행(C-4), compile은 결정론적·offline·Markdown 전용·no-clobber(C-6), 오류는 code/category 분기·`PROJECT_BUSY` 재시도(FR-INT-8).
> **분할 노트(E3-S3 SMALL 정합화)**: 루프 **상태-표시/오케스트레이션 셸**(E3-S3)과 **체크포인트 해소 행위**(E3-S4)를 분리했다. 각 체크포인트의 해소 주체 — NeedsProvider/NeedsDisclosure=**E3-S4**, NeedsTaxonomy=**E4-S1**, NeedsClusters=**E4-S2..S4**, ReadyToCompile 게이트=**E4-S6**, compile=**E3-S6**.

---

### E3-S1 — 프로젝트 생성 및 curator_id 기록
**As a** Administrator/Curator, **I want** 새 통합 프로젝트를 생성하고, **so that** 이후 모든 통합 산출물의 큐레이션 주체를 추적할 수 있다.

**MoSCoW**: **Must**

> **정합화 노트**: `curator_id` 바인딩 *의미*는 **E1-S4가 단일 소스**다. 본 스토리는 프로젝트 생성 시 그 바인딩을 *적용/참조*하며 동일 AC를 중복하지 않는다.

**Acceptance Criteria (Given/When/Then)**
- **Given** 로그인한 admin이 프로젝트 생성 화면에 있고, **When** 프로젝트 이름과 로컬 저장 경로를 입력해 생성을 요청하면, **Then** okc-web은 okc-core에 새 프로젝트를 만들고 **E1-S4에서 정의한 `curator_id` 바인딩을 적용**한다.
- **Given** 프로젝트가 생성되면, **When** 생성 결과를 표시하면, **Then** UI는 `curator_id`가 "발행자 진위 보증이 아니라 큐레이션 라벨"이며 실제 admin 게이팅은 okc-web(RBAC)이 책임진다는 점을 안내한다.
- **Given** contributor 역할 사용자, **When** 프로젝트 생성 API를 호출하면, **Then** **403**으로 거부된다(E1-S3 게이팅).

**Traceability**: FR-INT-1 (연계 FR-AUTH-4는 E1-S4) · 원 요구사항 ①(②)

---

### E3-S2 — 소스 ≤10 고정(freeze)
**As a** Administrator/Curator, **I want** 프로젝트에 등록된 소스를 최대 10개까지 확정하고 고정(freeze)하고, **so that** 확정된 입력 집합으로 재현 가능한 단일 통합 실행을 시작할 수 있다.

**MoSCoW**: **Must**

**Acceptance Criteria (Given/When/Then)**
- **Given** 프로젝트에 등록된 소스가 10개 이하이고, **When** admin이 소스 집합을 고정(freeze)하면, **Then** okc-web은 소스 목록을 잠그고 통합 실행 준비 상태로 전이한다.
- **Given** 이미 소스가 10개 등록된 프로젝트에서, **When** admin이 11개째 소스를 `add_source`하려 하면, **Then** okc-web은 하드 캡(C-3)을 검사해 요청을 거부하고 "프로젝트당 최대 10 소스(federation 미구현)" 제약을 안내한다.
- **Given** 소스가 고정된 상태에서, **When** admin이 통합을 시작하면, **Then** freeze 시점의 소스 집합 해시가 이후 승인·컴파일의 기준(hash-bound, C-4)이 된다.

**Traceability**: FR-INT-1, FR-INT-2 · 원 요구사항 ①

---

### E3-S3 — freeze-then-run IntegrationCheckpoint 루프 상태 표시/오케스트레이션 셸
**As a** Administrator/Curator, **I want** 고정된 소스로 통합을 실행하면 okc-web이 IntegrationCheckpoint 루프의 현재 단계를 표시하고 실행을 직렬화해 주길 원하고, **so that** 다단계 human-in-the-loop 통합을 한눈에 따라가며 지금 필요한 작업을 알 수 있다.

**MoSCoW**: **Must**

**Acceptance Criteria (Given/When/Then)**
- **Given** 소스가 고정된 프로젝트에서, **When** admin이 통합 실행을 시작하면, **Then** okc-web은 okc-core의 IntegrationCheckpoint 상태(`NeedsProvider→NeedsSources→NeedsDisclosure→NeedsTaxonomy→NeedsClusters→ReadyToCompile→Verified`)를 순차적으로 오케스트레이션하고 **현재 체크포인트를 UI에 표시**한다.
- **Given** 현재 체크포인트가 admin 조치를 요구하면, **When** 해당 단계 화면이 렌더링되면, **Then** UI는 지금 필요한 결정만 노출하고(원클릭이 아닌 다단계 루프) 이후 단계는 잠근 채로 유지한다. (각 체크포인트의 해소 *행위*는 별도 스토리: E3-S4/E4-S1/E4-S2..S4/E4-S6/E3-S6.)
- **Given** 통합 실행이 이미 진행 중인 프로젝트에서, **When** 두 번째 실행을 동시에 시작하려 하면, **Then** okc-web은 단일 엔진 프로세스에서 실행을 직렬화하여 `PROJECT_BUSY` code로 응답하고 재시도/큐잉을 안내한다.

**Traceability**: FR-INT-2 · 원 요구사항 ①

---

### E3-S4 — AI 공급자 설정 및 remote disclosure/민감정보 consent 확인 (NeedsProvider/NeedsDisclosure 해소)
**As a** Administrator/Curator, **I want** 통합 루프의 `NeedsProvider`/`NeedsDisclosure` 체크포인트에서 서버 프로비저닝된 AI 공급자를 지정하고 remote disclosure + 민감정보 consent를 명시적으로 확인하고, **so that** 통합 루프가 taxonomy 생성 단계(NeedsTaxonomy)로 진행될 수 있다.

**MoSCoW**: **Must** (§9 2→3단계 전이의 임계 human-in-the-loop 게이트 — capability-analysis §2·§3)

**Acceptance Criteria (Given/When/Then)**
- **Given** 통합 루프가 `NeedsProvider` 체크포인트에 있고, **When** admin이 서버측 env-var 이름으로 프로비저닝된 AI 공급자(A-2)를 선택하면, **Then** okc-web은 선택된 공급자를 프로젝트 설정에 바인딩하고 체크포인트를 `NeedsDisclosure`(또는 다음 요구 단계)로 전이한다.
- **Given** 루프가 `NeedsDisclosure`에 있고 선택된 공급자가 remote provider인 경우, **When** admin이 remote disclosure와 민감정보 consent(`allow_remote_provider` AND `remote_disclosure_confirmed`)를 명시적으로 확인하면, **Then** okc-web은 체크포인트를 `NeedsTaxonomy`로 전이한다(이후 taxonomy 승인은 E4-S1).
- **Given** 루프가 `NeedsDisclosure`에 있고, **When** admin이 동의(consent)를 제공하지 않으면, **Then** 루프는 `NeedsDisclosure`에서 **차단**되고 **remote provider 호출이 발생하지 않는다**.
- **Given** 공급자 자격증명은 서버측에 env-var **이름**으로만 프로비저닝된다(A-2), **When** admin이 공급자를 지정하면, **Then** okc-web은 비밀 값 원문을 입력받거나 저장하지 않고 env-var 이름 참조만 사용한다.
- **Given** contributor 세션, **When** 공급자 설정/consent API를 호출하면, **Then** **403**으로 거부된다(E1-S3 게이팅).

**Traceability**: FR-INT-2 · 원 요구사항 ①

---

### E3-S5 — 소스/설정 변경 시 이전 승인 무효화 UX
**As a** Administrator/Curator, **I want** 소스나 상위 설정을 바꾸면 이전 승인이 무효화된다는 사실을 명확히 안내받고, **so that** 낡은(stale) 승인으로 잘못된 컴파일을 하지 않는다.

**MoSCoW**: **Should**

**Acceptance Criteria (Given/When/Then)**
- **Given** taxonomy/cluster 승인이 이미 완료된 프로젝트에서, **When** admin이 소스를 추가·교체하거나 상위 설정(공급자/taxonomy 등)을 변경하면, **Then** okc-web은 hash-bound 규칙(C-4)에 따라 하위 승인이 모두 무효(stale)가 됨을 감지해 UI에 경고를 표시한다.
- **Given** 하위 승인이 stale로 표시된 상태에서, **When** admin이 컴파일을 시도하면, **Then** okc-web은 컴파일을 거부하고 재승인이 필요함을 안내한다(거부 게이트 세부는 E4-S6).
- **Given** 무효화가 발생한 뒤, **When** admin이 통합을 재개하면, **Then** 영향받은 체크포인트 단계부터 승인 루프를 다시 밟도록 유도한다.

**Traceability**: FR-INT-2 · 원 요구사항 ①

---

### E3-S6 — 승인 완료 시 compile로 병합 Vault 생성 (정상 경로, no-clobber)
**As a** Administrator/Curator, **I want** 모든 승인이 완료되면 컴파일을 실행해 병합 Vault 디렉터리를 생성하고, **so that** 결정론적이고 감사 가능한 통합 산출물을 얻는다.

**MoSCoW**: **Must**

> **정합화 노트**: 미승인/차단 findings 잔존 시의 **컴파일 거부 게이트는 E4-S6(FR-INT-5)에서 단일 관리**한다. 본 스토리는 승인 완료 후 **정상 컴파일 경로**로 한정한다. `no-clobber`는 compile publish 속성으로, 본 스토리에만 귀속한다.

**Acceptance Criteria (Given/When/Then)**
- **Given** `ReadyToCompile` 상태(`ApprovedIntegrationPlan` 확보)이고 차단 findings가 없을 때, **When** admin이 컴파일을 실행하면, **Then** okc-web은 okc-core의 offline·결정론적 compile을 호출해 `knowledge/` + `legacy/` + `.okc/` 를 가진 **Markdown 전용** 병합 Vault를 생성한다.
- **Given** 대상 출력 경로에 기존 산출물이 존재하면, **When** 컴파일이 publish 단계에 도달하면, **Then** **no-clobber** 정책으로 기존 산출물을 덮어쓰지 않고 atomic publish 하거나 안전하게 신규 경로에 발행한다.
- **Given** 컴파일이 완료되면, **When** `CompiledVaultManifest`가 발행되면, **Then** 산출물은 freeze 시점 소스 집합 해시에 바인딩되며(hash-bound 단일 실행, C-4) 결정론적으로 재현 가능하다.

**Traceability**: FR-INT-7 (거부 게이트는 E4-S6/FR-INT-5) · 원 요구사항 ①

---

### E3-S7 — 진행상황·오류 표시 (Job 이벤트 / OkcError code·category / PROJECT_BUSY 재시도)
**As a** Administrator/Curator, **I want** 통합·컴파일의 진행상황과 오류를 실시간에 가깝게 확인하고, **so that** 수 분 걸리는 장기 작업 중에도 상태를 파악하고 오류에 올바르게 대응한다.

**MoSCoW**: **Must**

**Acceptance Criteria (Given/When/Then)**
- **Given** 통합 또는 컴파일 Job이 실행 중이고, **When** okc-web이 Job 이벤트를 폴링하면, **Then** 진행 단계·상태를 UI에 갱신 표시한다.
- **Given** okc-core가 `OkcError{code, category}`를 반환하면, **When** okc-web이 오류를 처리하면, **Then** 메시지 문자열을 파싱하지 않고 안정적인 **code/category로 분기**해 사용자 안내를 렌더링한다.
- **Given** 오류 code가 `PROJECT_BUSY`이면, **When** 해당 오류를 표시하면, **Then** 재시도 가능함을 알리고 재시도/큐잉 액션을 제공한다.

**Traceability**: FR-INT-8 · 원 요구사항 ①

---

**Out of scope (참고, 스토리화 안 함)**: 증분(incremental) 재통합, 10 소스 초과를 위한 federation, 다중 프로세스·외부 분산 락/큐, non-Markdown materializer(첨부/Canvas/Base/완전 link-rewrite). — §6 준수.

---

## Epic E4 — Conflict/Critic 리뷰 & 해소 (Review & Resolution)

**원 요구사항 ③의 재해석(Q7=A)**: okc-core에는 "승자 선택" API가 없다 — 모순은 보존되고(C-2), 구조적 conflict는 내부 폐기되며, critic Major/Critical은 waive 불가(regenerate만), Minor만 사유와 함께 waive 가능하다. 따라서 ③은 **taxonomy 승인 / 클러스터 승인 / minor waive / omission 제안 / regenerate** 결정 표면으로 제공되고, 승인은 hash-bound 단일 실행(C-4).
**대응 FR**: FR-INT-3,4,5,6

---

### E4-S1 — Taxonomy proposal 검토·편집·승인 (approve_taxonomy)
**As a** Administrator(Curator), **I want** okc-core가 생성한 taxonomy proposal을 화면에서 확인하고 편집한 뒤 승인하고, **so that** 이후 클러스터별 synthesis/critic 검토로 진행할 근거(승인된 분류 체계)를 확정할 수 있다.

**MoSCoW**: **Must**

**Acceptance Criteria (Given/When/Then)**
- **Given** 통합 루프가 `NeedsTaxonomy` 체크포인트에 도달하고 admin으로 로그인한 상태에서, **When** taxonomy proposal 화면을 열면, **Then** okc-core가 제안한 클러스터 목록(각 클러스터의 소속 노트/소스)이 표시된다.
- **Given** taxonomy proposal이 표시된 상태에서, **When** admin이 클러스터를 편집(병합/분리/재배치)하고 rationale(사유)를 입력해 승인하면, **Then** okc-web은 `approve_taxonomy(edited_clusters, rationale)`를 호출하고 승인 사실을 `curator_id`(비검증 라벨)와 함께 기록한다.
- **Given** taxonomy를 승인한 이후, **When** 상위 입력(소스/설정)이 변경되면, **Then** 이전 taxonomy 승인은 무효(stale)로 표시되고 재승인이 필요함을 안내한다(hash-bound 단일 실행, C-4).
- **Given** 편집 결과가 비어 있거나 유효하지 않은 상태에서, **When** 승인을 시도하면, **Then** okc-core의 `OkcError` code/category에 따라 거부 사유를 표시한다(메시지 문자열 파싱 금지).

**Traceability**: FR-INT-3 · 원 요구사항 ③(통합 진입점으로 ① 연계)

---

### E4-S2 — 클러스터별 synthesis 결과 & critic findings 심각도 검토
**As a** Administrator(Curator), **I want** 각 클러스터의 synthesis 결과와 critic findings를 심각도별로 구분해 확인하고, **so that** 어떤 지적이 컴파일을 차단하는지(Major/Critical)와 어떤 지적이 waive 대상인지(Minor)를 판단할 수 있다.

**MoSCoW**: **Must**

**Acceptance Criteria (Given/When/Then)**
- **Given** taxonomy가 승인되어 클러스터별 synthesis + CriticReport가 생성된 상태에서, **When** admin이 특정 클러스터를 열면, **Then** 합성된 노트 초안과 critic findings 목록이 심각도(Critical/Major/Minor) 배지와 함께 표시된다.
- **Given** findings가 표시된 상태에서, **When** Major 또는 Critical finding이 있으면, **Then** 해당 finding은 "차단(blocking)·waive 불가·regenerate로만 해소"로 명확히 표기된다.
- **Given** findings가 표시된 상태에서, **When** Minor finding이 있으면, **Then** 해당 finding은 "waive 가능(사유 필수)"로 표기된다.
- **Given** 확정 스택은 Rust axum + `okc-interop` 직접 링크(Q8=A, C-7)로 critic/synthesis 결과가 **타입드 DTO(interop schema v2)** 로 전달되고(불투명 JSON `VersionedPayload`는 Python/Node 바인딩 폴백 경로에서만 해당), **When** okc-web이 payload를 해석하면, **Then** **스키마 버전을 검증하고 미지원 버전은 `OkcError`의 code/category로 거부**한다(메시지 문자열 파싱 금지).

**Traceability**: FR-INT-4 (+FR-INT-8 오류 분기) · 원 요구사항 ③

---

### E4-S3 — 클러스터 승인 + Minor waive / Omission 제안 (사유 필수, approve_cluster)
**As a** Administrator(Curator), **I want** 차단 findings가 없는 클러스터를 승인하면서 필요 시 Minor finding을 waive하거나 특정 항목의 omission을 제안하되 각각 사유를 남기고, **so that** 검토 결정과 근거를 감사 가능하게 남기며 클러스터를 컴파일 대기 상태로 확정할 수 있다.

**MoSCoW**: **Must**

**Acceptance Criteria (Given/When/Then)**
- **Given** 클러스터에 Major/Critical finding이 없는 상태에서, **When** admin이 Minor waive(항목별 사유 필수)와 omission 제안(항목별 사유 필수)을 입력해 승인하면, **Then** okc-web은 `approve_cluster(omission_rationales, minor_waivers)`를 호출하고 결정을 `curator_id`와 함께 기록한다.
- **Given** admin이 Minor finding 또는 omission을 사유 없이 처리하려 하면, **When** 승인을 시도하면, **Then** okc-web은 사유 누락을 검증해 승인을 막고 사유 입력을 요구한다(사유 필수).
- **Given** 클러스터에 Major 또는 Critical finding이 남아 있는 상태에서, **When** admin이 해당 finding을 waive하려 하면, **Then** okc-web은 waive를 허용하지 않고 "Major/Critical은 waive 불가, regenerate로만 해소"임을 안내한다(C-2).
- **Given** 이 검토가 어떤 소스를 "승자"로 고르는 행위가 아님을, **When** 승인 UI를 사용하면, **Then** UI는 승자 선택 개념이 없음을 명시하고 승인 / waive / omission 제안 / regenerate 결정만 제공한다.
- **Given** 클러스터 승인 후, **When** 상위 입력(소스/설정/taxonomy)이 변경되면, **Then** 이 승인은 무효(stale)로 표시된다(hash-bound 단일 실행, C-4).

**Traceability**: FR-INT-5 · 원 요구사항 ③

---

### E4-S4 — Regenerate로 차단(Major/Critical) findings 해소 (regenerate_cluster)
**As a** Administrator(Curator), **I want** 차단 findings가 있는 클러스터에 대해 피드백을 담아 재생성을 요청하고, **so that** waive가 불가능한 차단 지적을 유일하게 허용된 방식으로 해소하고 다시 검토할 수 있다.

**MoSCoW**: **Must**

**Acceptance Criteria (Given/When/Then)**
- **Given** 클러스터에 Major/Critical finding이 있는 상태에서, **When** admin이 피드백을 입력해 regenerate를 요청하면, **Then** okc-web은 `regenerate_cluster(feedback)`를 호출하고 새 synthesis + CriticReport를 생성해 다시 표시한다.
- **Given** regenerate가 완료된 후, **When** 새 CriticReport에 더 이상 Major/Critical이 없으면, **Then** 해당 클러스터는 `approve_cluster`로 승인 가능한 상태가 된다.
- **Given** regenerate 외의 수단으로 Major/Critical을 제거하려는 시도(예: waive)가 발생하면, **When** admin이 시도하면, **Then** 거부되고 "차단 findings는 regenerate로만 해소" 안내가 표시된다(승자 선택 없음, C-2).
- **Given** 엔진이 다른 실행으로 바쁜 상태에서, **When** regenerate를 요청하면, **Then** okc-web은 `PROJECT_BUSY` code를 감지해 재시도를 안내한다(메시지 문자열이 아닌 code로 분기).

**Traceability**: FR-INT-5 (+FR-INT-8) · 원 요구사항 ③

---

### E4-S5 — 보존된 모순(Contradictions) 정보성 표시 (승자 없음)
**As a** Administrator(Curator), **I want** 소스 간 상충하는 주장(모순)을 모든 근거와 함께 정보성으로 확인하고, **so that** okc-core가 승자를 고르지 않고 모순을 보존한다는 사실을 이해하고 검토 맥락으로 활용할 수 있다.

**MoSCoW**: **Should** (§9 데모 임계 경로에 모순 시연이 명시되지 않아 Should 유지; 데모 서사에 "모순 보존" 시연을 포함하기로 하면 Must 승격 가능)

**Acceptance Criteria (Given/When/Then)**
- **Given** 클러스터에 보존된 모순(Contradictions)이 있는 상태에서, **When** admin이 클러스터 상세를 열면, **Then** 상충하는 각 주장이 출처(소스 / `owner_display_name` / provenance)와 함께 **모두** 표시된다.
- **Given** 모순이 표시된 상태에서, **When** admin이 이를 검토하면, **Then** UI는 이것이 정보성 표시이며 "승자 선택 없음(모든 근거 보존)"임을 명시하고, 모순을 해소하거나 승자를 고르는 액션 버튼을 제공하지 않는다.
- **Given** 구조적(path/link) conflict는 okc-core 내부에서 폐기되어 caller에 도달하지 않는다는 제약(C-2) 하에, **When** 리뷰 화면을 사용하면, **Then** 표시 대상은 보존된 의미적 모순으로 한정되며 구조적 conflict 선택 UI는 존재하지 않는다.

**Traceability**: FR-INT-6 · 원 요구사항 ③

---

### E4-S6 — 차단 findings/미승인 잔존 시 컴파일 거부 게이트 (APPROVAL_REQUIRED)
**As a** Administrator(Curator), **I want** 차단 findings(Major/Critical)나 미승인 클러스터가 남아 있는 상태에서 컴파일을 시도하면 명확한 거부 안내를 받고, **so that** 무엇을 해소해야 컴파일이 가능한지 알고 검토 게이트를 우회하지 않도록 보장할 수 있다.

**MoSCoW**: **Must** (§9 "차단 findings 시 컴파일 거부"의 단일 게이트)

> **정합화 노트**: 컴파일 거부 게이트는 **본 스토리에 단일화**한다. E3-S6은 승인 완료 후 정상 컴파일 경로만 다룬다.

**Acceptance Criteria (Given/When/Then)**
- **Given** 하나 이상의 클러스터에 Major/Critical finding이 남아 있거나 미승인 클러스터가 있는 상태에서, **When** admin이 컴파일을 시도하면, **Then** okc-web은 컴파일을 진행하지 않고 `APPROVAL_REQUIRED` 계열 code를 근거로 거부 사유와 미해소 항목 목록을 표시한다.
- **Given** 거부 안내가 표시된 상태에서, **When** admin이 항목을 확인하면, **Then** 각 항목별 필요한 조치(Major/Critical → regenerate, Minor → waive+사유, 미승인 → approve_cluster)를 안내한다.
- **Given** 모든 클러스터가 승인되어 `ApprovedIntegrationPlan`이 성립한 상태에서, **When** admin이 컴파일을 시도하면, **Then** 컴파일이 허용된다(정상 경로는 E3-S6).
- **Given** 거부/오류 판단이 필요할 때, **When** okc-web이 처리하면, **Then** `OkcError`의 code/category로 분기하고 메시지 문자열을 파싱하지 않는다.

**Traceability**: FR-INT-5 (+FR-INT-8 오류 분기) · 원 요구사항 ③

---

## Epic E5 — 병합 Vault 서빙 & okc-mcp 계약 (Serving & okc-mcp Contract)

**원 요구사항**: ⑤(합쳐진 Vault를 okc-mcp RAG 소스로 연결)
**대응 FR**: FR-SRV-1..3
**에픽 초점**: 컴파일된 병합 Vault를 read-only HTTP API/URL로 노출(파일 목록·본문), `verify()`/`explain()`(무결성·provenance) 조회, okc-mcp가 RAG 소스로 소비할 위치·형식 계약 정의.
**경계 고정(Out-of-scope)**: okc-mcp 자체의 RAG 리트리벌(청킹·임베딩·vector index·쿼리)과 MCP 툴 표면은 구현하지 않는다. okc-web은 read-only 소스 제공까지만 책임지고, 코어 임베딩은 일시적이므로 소비 측이 산출물을 **재임베딩**해야 함을 계약으로만 명시한다.

---

### E5-S1 — 컴파일된 병합 Vault를 read-only 서빙 엔드포인트로 공개
**As an** 관리자(Administrator/Curator), **I want** 컴파일이 완료된 병합 Vault를 read-only HTTP 엔드포인트/URL로 공개하고, **so that** okc-mcp나 조회자가 확정된 산출물을 안정적인 주소로 소비할 수 있다.

**MoSCoW**: **Must**

**Acceptance Criteria (Given/When/Then)**
- **Given** 프로젝트가 `Verified` 상태이고 병합 Vault(`knowledge/`+`legacy/`+`.okc/`)가 컴파일되어 존재할 때, **When** 관리자가 서빙 공개(publish)를 실행하면, **Then** 시스템은 해당 `CompiledVaultManifest`에 바인딩된 read-only URL/엔드포인트를 발급하고 공개 상태로 표시한다.
- **Given** 컴파일이 아직 완료되지 않았거나 차단 findings로 인해 승인(`ApprovedIntegrationPlan`)이 없는 경우, **When** 관리자가 서빙 공개를 시도하면, **Then** 공개를 거부하고 컴파일 선행이 필요함을 안내한다.
- **Given** `contributor` 역할 사용자, **When** 서빙 공개 API를 호출하면, **Then** **403**으로 거부한다(공개는 admin 전용, 게이팅은 okc-web 책임 — okc-core는 호출자를 인증하지 않고 admin 식별자는 비검증 `curator_id` 라벨로만 전달, C-1).
- **Given** 이미 공개된 컴파일 산출(`CompiledVaultManifest`)이 존재할 때, **When** 관리자가 (재)공개를 실행하면, **Then** 기존 `CompiledVaultManifest` 산출물은 **불변**으로 유지되고 서빙 URL은 대상 매니페스트에 **재바인딩**될 뿐 기존 컴파일 산출을 변경하지 않는다.

> **정합화 노트**: 서빙 (재)공개는 "매니페스트 재바인딩 + 기존 컴파일 산출 불변"이다. `no-clobber`(compile publish 속성)는 E3-S6에만 귀속한다.

**Traceability**: FR-SRV-1, FR-AUTH-3, FR-INT-7 · 원 요구사항 ⑤(권한 게이팅은 ②)

---

### E5-S2 — read-only API로 병합 Vault 파일 목록·본문 조회
**As an** okc-mcp 소비자(비인간, 데모에서는 HTTP 클라이언트로 검증), **I want** 공개된 병합 Vault의 파일 목록과 각 파일 본문을 read-only API로 가져오고, **so that** RAG 소스로 재임베딩할 원본 Markdown을 확보할 수 있다.

**MoSCoW**: **Must**

**Acceptance Criteria (Given/When/Then)**
- **Given** 공개된 병합 Vault URL, **When** 소비자가 목록 엔드포인트를 GET 하면, **Then** `knowledge/` · `legacy/` · `.okc/` 하위의 상대경로 파일 목록(Markdown 전용 산출물)을 반환한다.
- **Given** 목록에 존재하는 특정 파일 상대경로, **When** 소비자가 본문 엔드포인트를 GET 하면, **Then** 해당 파일의 원본 Markdown 본문을 반환한다.
- **Given** read-only 계약, **When** 소비자가 쓰기/수정성 메서드(POST/PUT/PATCH/DELETE 등)를 호출하면, **Then** **405**(또는 거부)로 응답하고 어떤 상태도 변경하지 않는다.
- **Given** 존재하지 않거나 서빙 루트를 벗어나는(경로 이탈) 파일 경로, **When** 조회하면, **Then** **404**로 거부하고 서빙 루트 밖 경로 접근을 차단한다.

**Traceability**: FR-SRV-1 · 원 요구사항 ⑤

---

### E5-S3 — verify()/explain()로 무결성·provenance 조회
**As an** okc-mcp 소비자 및 관리자, **I want** 공개된 Vault의 무결성 검증 결과와 파일별 provenance를 조회하고, **so that** 소비하기 전에 산출물이 승인된 계획과 내부적으로 일관되는지 확인할 수 있다.

**MoSCoW**: **Must**

**Acceptance Criteria (Given/When/Then)**
- **Given** 공개된 병합 Vault, **When** verify 조회를 요청하면, **Then** okc-core `verify()` 결과(무결성·일관성 통과 여부)를 구조화된 형태로 반환한다.
- **Given** 특정 파일, **When** explain 조회를 요청하면, **Then** 해당 파일의 `ProvenanceRecord`(어떤 소스/클러스터에서 유래했는지)를 반환한다.
- **Given** verify 결과 표시, **When** UI/응답을 렌더링하면, **Then** "검증은 산출물의 내부 일관성 증명이며 발행자(소스) 진위 보증이 아님"을 명시한다.
- **Given** okc-core가 `OkcError`를 반환하는 경우(예: `PROJECT_BUSY`), **When** 조회가 실패하면, **Then** 메시지 문자열 파싱이 아니라 code/category로 분기하고 `PROJECT_BUSY`는 재시도를 안내한다.

**Traceability**: FR-SRV-2, NFR-DET-1, FR-INT-8 · 원 요구사항 ⑤

---

### E5-S4 — okc-mcp RAG 소스 소비 계약(위치·형식) 정의·게시
**As an** okc-mcp 소비자(계약 관점) 및 관리자, **I want** 병합 Vault를 RAG 소스로 소비하기 위한 위치·형식 계약을 명확히 제공받고, **so that** okc-web과 okc-mcp의 경계를 고정하고 향후 통합을 안전하게 붙일 수 있다.

**MoSCoW**: **Must** (경계 고정)

**Acceptance Criteria (Given/When/Then)**
- **Given** 공개된 병합 Vault, **When** 소비자가 계약(discovery) 문서/엔드포인트를 조회하면, **Then** 소비 위치(로컬 디렉터리 절대경로 또는 read API 엔드포인트)와 형식(Markdown 전용, `knowledge/` · `legacy/` · `.okc/` 레이아웃)을 반환한다.
- **Given** 계약 명세, **When** okc-mcp가 이를 참조하면, **Then** RAG 리트리벌(청킹·임베딩·vector index·쿼리)은 okc-mcp(또는 외부 엔진)의 몫으로 out-of-scope임을 명시하고, 코어 임베딩은 일시적이므로 소비 측이 산출물을 **재임베딩**해야 함을 안내한다.
- **Given** 본 범위는 read-only 소스 제공까지로 한정, **When** 소비자가 계약(discovery) 엔드포인트를 조회하면, **Then** 계약에는 read-only 조회 엔드포인트(목록·본문·verify/explain·discovery)만 노출되고 **okc-mcp 내부(MCP 툴 표면, RAG 리트리벌 트리거 등)를 호출하는 항목은 존재하지 않는다**.

**Traceability**: FR-SRV-3 · 원 요구사항 ⑤(okc-mcp 구현은 이연)

---

### E5-S5 — 서빙을 hash-bound 컴파일 매니페스트에 고정, 변경 시 stale 표시
**As an** 관리자(Administrator/Curator), **I want** 공개된 서빙 URL이 특정 승인·컴파일된 매니페스트에 바인딩되고 소스/설정 변경 시 stale로 표시되길 원하고, **so that** 소비자가 무효화된 산출물을 신선한 것으로 오인하지 않는다.

**MoSCoW**: **Should**

**Acceptance Criteria (Given/When/Then)**
- **Given** 서빙이 특정 `CompiledVaultManifest`(hash-bound 단일 실행)에 바인딩되어 공개된 상태, **When** 소비자가 조회하면, **Then** 어떤 매니페스트/버전을 서빙 중인지 식별 정보를 함께 제공한다.
- **Given** 공개 후 소스/설정/taxonomy가 변경되어 이전 승인이 stale가 됨(C-4 freeze-then-run), **When** 관리자가 서빙 상태를 확인하면, **Then** 현재 서빙 산출물이 stale임을 표시하고 재컴파일·재공개가 필요함을 안내한다.
- **Given** stale 상태에서 아직 재컴파일이 완료되지 않은 경우, **When** 소비자가 조회하면, **Then** 기존 컴파일 산출물(`CompiledVaultManifest`, **불변**)을 계속 read-only로 제공하되 stale 라벨을 유지한다.

**Traceability**: FR-SRV-1 (제약 C-4 freeze-then-run) · 원 요구사항 ⑤

---

## 페르소나 ↔ 스토리 매핑 테이블

| 페르소나 | 주체(actor)로 등장하는 스토리 | 간접/경계 접촉 |
|---|---|---|
| **Administrator / Curator** | E1-S1, E1-S2, E1-S3, E1-S4, E1-S5, E2-S1, E2-S2, E2-S4, E2-S5, E3-S1, E3-S2, E3-S3, E3-S4, E3-S5, E3-S6, E3-S7, E4-S1, E4-S2, E4-S3, E4-S4, E4-S5, E4-S6, E5-S1, E5-S3(공동), E5-S5 | — (전 경로 주 행위자) |
| **Contributor (개인 기여자)** | E2-S3, E2-S6 | E1-S1(토큰-온리 경계), E2-S1/S4(대상), E1-S3(403 게이팅 대상) |
| **okc-mcp Consumer (비인간)** | E5-S2, E5-S3(공동), E5-S4 | E5-S1/S5(소비 대상) |

> **주**: E5-S3(verify/explain 조회)은 admin(공개 전 검수)과 okc-mcp 소비자(소비 전 검증) 양측이 주체다.

---

## §9 데모 시나리오 커버리지 맵 (Must 스토리로 완전 커버)

데모 경로: **업로드(토큰) → 관리자 로그인/프로젝트 생성/소스 고정/통합 실행 → 리뷰(승인/waive/regenerate, 차단 findings 시 컴파일 거부) → 컴파일(병합 Vault) → 서빙(URL 조회 + provenance, okc-mcp 계약)**

| §9 단계 | 세부 동작 | 커버 스토리 (전부 Must) | FR |
|---|---|---|---|
| **1. 업로드(토큰)** | 토큰 발급 → 토큰 인증 업로드 → 바이트 검증 → 로컬 착지 & add_source | E2-S1, E2-S3, E2-S4, E2-S5 | FR-UP-1..4 |
| **2. 통합 시작** | admin 로그인 → 프로젝트 생성(curator_id) → admin 게이팅 → 소스 고정 → 통합 실행 셸 → **공급자 지정·remote disclosure/consent 확인** | E1-S1, E1-S3, E1-S4, E3-S1, E3-S2, E3-S3, **E3-S4** | FR-AUTH-1,3,4, FR-INT-1,2 |
| **3. 리뷰·선택** | taxonomy 승인 → critic findings 심각도 검토 → 승인/Minor waive/omission → regenerate → **차단 findings 시 컴파일 거부** | E4-S1, E4-S2, E4-S3, E4-S4, **E4-S6** | FR-INT-3,4,5(,6) |
| **4. 컴파일** | 승인 완료 → 병합 Vault 생성(no-clobber) | E3-S6 | FR-INT-7 |
| **5. 서빙** | read-only URL 공개 → 파일 목록·본문 조회 → verify/explain provenance → okc-mcp 계약 제시 | E5-S1, E5-S2, E5-S3, E5-S4 | FR-SRV-1..3 |
| (횡단) | 진행상황·오류·PROJECT_BUSY 재시도 | E3-S7 | FR-INT-8 |

**커버리지 결론**: 크리틱이 지적한 유일한 데모-경로 공백(2→3단계의 NeedsProvider/NeedsDisclosure human-in-the-loop 게이트)을 **신규 Must 스토리 E3-S4**로 해소하여 §9 5단계 전부가 Must 스토리로 연속 커버됨(coverage_ok = true).

---

## MoSCoW 요약

| 우선순위 | 개수 | 스토리 ID |
|---|---|---|
| **Must** | 22 | E1-S1, E1-S3, E1-S4, E2-S1, E2-S3, E2-S4, E2-S5, E3-S1, E3-S2, E3-S3, **E3-S4**, E3-S6, E3-S7, E4-S1, E4-S2, E4-S3, E4-S4, E4-S6, E5-S1, E5-S2, E5-S3, E5-S4 |
| **Should** | 7 | E1-S2, E1-S5, E2-S2, **E2-S6**, E3-S5, E4-S5, E5-S5 |
| **Could** | 0 | — |
| **Won't (this release)** | 0 (스토리 없음; 아래 Out-of-Scope 참조) | — |
| **합계** | **29** | E1×5, E2×6, E3×7, E4×6, E5×5 |

> Must = §9 데모 크리티컬 패스. Should = 데모 안전성·페르소나 가치·무효화 UX 강화(데모에 필수는 아님). Won't 항목은 스토리화하지 않고 Out-of-Scope로 명시.

---

## Out-of-Scope (스토리화하지 않음, §6 준수)

- **멀티테넌시·조직 간 격리, at-rest 암호화** (Q1=A) — 데모 신뢰 환경 가정(A-1).
- **OIDC/SSO** 연동 (Q2=B).
- **10 소스 초과를 위한 federation** 구현 (Q3=X) — 설계 방향만 문서화.
- **okc-mcp 구현**: RAG 청킹/임베딩/vector index/쿼리, MCP 툴 표면 (Q7=A) — E5는 **계약(위치·형식)까지만** 스토리화.
- **비-Markdown 자료**(첨부/Canvas/Base), 완전 link-rewrite, OKCPack (Q10=A, 코어 release blocker).
- **다중 프로세스·외부 분산 락/큐/lease** (Q9=A) — 단일 장수 엔진 프로세스로 직렬화.
- **증분(incremental) 재통합** (Q5=A) — freeze-then-run만.
- **okc-core 신규 public API/코어 수정** (Q3=C, Q4=B/C 배제) — 어댑터 원칙(ADR-0002), req③ "승자 선택 API" 미도입.
- **Security / Resiliency / Property-Based Testing 확장 규칙 강제** (Q11–13 opt-out) — NFR-SEC-1 최소 안전 저장(E1-S5)만 기능 요구로 유지.
- **기여자 계정/전체 로그인 흐름** — 기여자는 토큰-온리(FR-UP-2); 로그인은 선택적·데모 경로 밖(E1-S1 노트).

---

## Traceability Matrix — 원 요구사항 ①–⑤ → 스토리 ID

| 원 요구사항 | 대응 FR | 커버 스토리 (Must **굵게**) |
|---|---|---|
| **① 권한 관리로 부서간 개인 Vault 통합** | FR-AUTH-1..4, FR-INT-1,2,7 | **E1-S1**, E1-S2, **E1-S3**, **E1-S4**, E1-S5, **E2-S5**(add_source), **E3-S1**, **E3-S2**, **E3-S3**, **E3-S4**, E3-S5, **E3-S6**, **E3-S7**, **E4-S1**(연계) |
| **② 관리자만 통합 수행** | FR-AUTH-2,3,4 | **E1-S1**, E1-S2, **E1-S3**, **E1-S4**, E1-S5, **E2-S1**, E2-S2, **E3-S1**(연계), **E5-S1**(공개 게이팅) |
| **③ conflict 확인·선택 통합** (재해석: 승인/waive/omission/regenerate, 승자 선택 없음) | FR-INT-4,5,6 (+FR-INT-3 진입점) | **E4-S1**, **E4-S2**, **E4-S3**, **E4-S4**, E4-S5, **E4-S6** |
| **④ 개인 업로드 URL/API + token** | FR-UP-1..4 | **E2-S1**, E2-S2, **E2-S3**, **E2-S4**, **E2-S5**, E2-S6 |
| **⑤ 병합 Vault를 okc-mcp RAG로 연결** (okc-mcp 구현 이연) | FR-SRV-1..3 | **E5-S1**, **E5-S2**, **E5-S3**, **E5-S4**, E5-S5 |

### 보조: FR → 스토리 역매핑 (완전성 확인)

| FR | 스토리 |
|---|---|
| FR-AUTH-1 | E1-S1, E1-S5 |
| FR-AUTH-2 | E1-S2, E1-S3 |
| FR-AUTH-3 | E1-S3, E2-S1, E2-S2, E3-S1, E5-S1 |
| FR-AUTH-4 | E1-S4 (E3-S1 참조) |
| FR-UP-1 | E2-S1, E2-S2 |
| FR-UP-2 | E2-S3 (E1-S1 노트) |
| FR-UP-3 | E2-S4, E2-S6 |
| FR-UP-4 | E2-S5 (E2-S6 연계) |
| FR-INT-1 | E3-S1, E3-S2, E2-S5(연계) |
| FR-INT-2 | E3-S2, E3-S3, E3-S4, E3-S5 |
| FR-INT-3 | E4-S1 |
| FR-INT-4 | E4-S2 |
| FR-INT-5 | E4-S3, E4-S4, E4-S6 |
| FR-INT-6 | E4-S5 |
| FR-INT-7 | E3-S6, E5-S1(연계) |
| FR-INT-8 | E3-S7, E4-S2, E4-S6, E5-S3 |
| FR-SRV-1 | E5-S1, E5-S2, E5-S5 |
| FR-SRV-2 | E5-S3 |
| FR-SRV-3 | E5-S4 |

_모든 FR(FR-AUTH-1..4, FR-UP-1..4, FR-INT-1..8, FR-SRV-1..3)이 ≥1개 스토리로 매핑됨._

---

_다음_: Workflow Planning → Application Design → Units Generation.
