# Personas — okc-web (Obsidian Vault 통합 웹 플랫폼)

**Stage**: INCEPTION / User Stories — Part 2 (Generation, Finalized)
**Grounding**: `requirements.md`(§3 Actors, FR-AUTH/UP/INT/SRV, §9 데모 시나리오, §10 추적성) · `okc-core-capability-analysis.md`(C-1..C-7 하드 제약)
**Scope**: 승인된 스토리 플랜(Q1=A) 기준 — **관리자/큐레이터, 기여자, okc-mcp 소비자 3종**. MVP 데모 경로에 집중.
**Finalize note**: 검토 게이트 크리틱 반영 — (1) 에픽 색인 표 E3 행에 **FR-INT-8 추가**, (2) Contributor는 **토큰-온리**(전체 로그인 없음)로 정합화하고 커버리지 보강(E2-S6), (3) Administrator 권한에 **공급자 지정·remote disclosure/민감정보 consent**(E3-S4) 명시.

## 에픽 색인 (페르소나 ↔ 에픽 매핑용)

| ID | 에픽 | 원 요구사항 | FR |
|---|---|---|---|
| **E1** | 인증 & 권한 | ①② | FR-AUTH-1..4 |
| **E2** | 개인 Vault 업로드 & 토큰 | ④ | FR-UP-1..4 |
| **E3** | 통합 오케스트레이션 | ① | FR-INT-1,2,7,8 |
| **E4** | Conflict/Critic 리뷰 & 해소 | ③ | FR-INT-3,4,5,6 |
| **E5** | 병합 Vault 서빙 & okc-mcp 계약 | ⑤ | FR-SRV-1..3 |

> **정정 노트**: E3 에픽은 진행상황·오류 표시(E3-S7)를 통해 **FR-INT-8**을 커버하므로 색인 표에 FR-INT-8을 포함한다(실제 에픽 헤더·스토리와 일치).

> **참고 — Viewer 흡수 결정 (Q1=A)**: 원 요구사항 §3의 선택적 **Viewer**(컴파일된 Vault 단순 조회)는 별도 페르소나로 두지 않는다. 조회 니즈는 (a) **Administrator**가 서빙 공개 전 병합 Vault를 검수·`verify()`/`explain()` 확인하는 흐름(E5), (b) 비인간 **okc-mcp 소비자**의 read-only 소비(E5)로 충분히 커버된다. 별도 `viewer` 역할은 시스템에 존재하지 않으며(E1-S2), 조회 전용 접근은 admin/contributor 권한으로 처리한다. 필요 시 향후 확장.

---

## Persona 1 — Administrator / Curator (관리자 · 큐레이터)

**이름/역할 (예시)**: "정하나" — 부서 지식 통합 큐레이터. okc-web에서 프로젝트를 생성하고 소스를 고정한 뒤 통합을 실행하며, conflict/critic 리뷰를 거쳐 병합 Vault를 컴파일·공개하는 **유일한 변경성(mutating) 작업 주체**.

### 목표 · 동기
- 여러 부서·개인의 흩어진 개인 Vault를 **하나의 감사가능한 병합 Vault로 통합**해 조직 지식의 단일 출처를 만든다 (원 요구사항 ①).
- 통합 과정에서 okc-core가 제기하는 **taxonomy 제안·모순·critic 지적**을 검토하고, 자신의 판단(승인/waive/omission/regenerate)을 남겨 **품질과 provenance를 책임진다** (원 요구사항 ③).
- 완성된 병합 Vault를 **read-only로 공개**해 okc-mcp(및 후속 RAG 소비자)가 소스로 쓸 수 있게 한다 (원 요구사항 ⑤).
- 동기: 신뢰 가능하고 결정론적이며 감사 로그가 남는 통합 결과. "누가 무엇을 왜 승인했는지"가 남아야 한다.

### 권한 경계 (할 수 있는 것 / 없는 것)
**할 수 있는 것**
- 로그인 후 프로젝트 생성, `curator_id` 설정, **소스 최대 10개 고정(freeze)** (FR-INT-1, C-3).
- **모든 변경성 통합 작업 수행**: `add_source`, 통합 실행, **AI 공급자 지정 및 remote disclosure/민감정보 consent 확인**(`allow_remote_provider` AND `remote_disclosure_confirmed`)으로 `NeedsProvider`/`NeedsDisclosure` 체크포인트 해소(FR-INT-2, E3-S4), `approve_taxonomy`, `approve_cluster`, `regenerate_cluster`, `compile`, 서빙 공개 — 이 작업들은 **`admin` 역할만** 가능 (FR-AUTH-3, 원 요구사항 ②).
- 기여자/부서 단위 **업로드 토큰 발급·조회·폐기** (FR-UP-1, E2 진입점).
- 클러스터 **승인**, **Minor 지적 waive(사유 필수)**, **omission 제안(사유 필수)**, 피드백 기반 **regenerate** (FR-INT-5).
- 병합 Vault 검수 및 `verify()`/`explain()`(무결성·provenance) 확인 (FR-SRV-2). — 흡수된 Viewer 조회 니즈를 여기서 충족.

**할 수 없는 것 / 제약 (okc-core 하드 제약 반영)**
- **모순(Contradiction)의 "승자 선택" 불가**: okc-core에는 승자 선택 API가 없다. 상충 주장은 모든 근거와 함께 **보존**될 뿐이며, 어느 한쪽을 정답으로 고를 수 없다 (C-2, FR-INT-6). UI는 이를 명확히 안내한다.
- **Critic Major/Critical 지적 waive 불가**: 차단성 지적은 오직 `regenerate_cluster`로만 해소된다. 차단 findings를 남긴 채 컴파일 시도 시 거부(`APPROVAL_REQUIRED` 계열) (C-2, FR-INT-5, E4-S6).
- **구조적 conflict(path/link)는 관리자에게 도달하지 않음**: 내부에서 계산·폐기되므로 리뷰 대상이 아니다 (C-2).
- **승인은 hash-bound·단일 실행**: 소스/설정/taxonomy를 하나라도 바꾸면 **이전 승인이 모두 무효(stale)**가 되어 재승인이 필요하다 (C-4, freeze-then-run).
- **관리자 신원은 okc-core가 검증하지 않음**: `curator_id`는 비검증 free-text 라벨이므로, "관리자만 통합"의 실제 강제는 100% okc-web의 RBAC 책임이다 (C-1, FR-AUTH-4). okc-core 자체는 게이팅을 보장하지 않는다.
- **소스 11개 이상 통합 불가**: 프로젝트당 ≤10 (C-3). federation은 범위 밖.
- **okc-core 정책 무수정**: 관리자용 신규 코어 API를 만들 수 없다 — 기존 승인/waive/omission/regenerate 표면만 사용한다 (ADR-0002, 어댑터 원칙).

### 기술 수준
- **높음 (도메인 전문)**. Obsidian Vault 구조, 지식 큐레이션, 통합 품질 판단에 능숙. 다단계 human-in-the-loop 워크플로우(체크포인트 루프)를 이해하고 사유를 명료히 기술할 수 있다. CLI 대신 웹 UI로 작업하기를 선호.

### 대표 시나리오
1. 로그인 → 프로젝트 생성(`curator_id`=본인) → 기여자들에게 업로드 토큰 발급(E2 진입).
2. 업로드된 소스 2~3개를 확인하고 **고정(freeze)** → 통합 실행 → **AI 공급자 지정·remote disclosure/consent 확인**으로 루프를 taxonomy 단계까지 진행(E3-S3, E3-S4).
3. taxonomy 제안을 편집·승인 → 클러스터별 synthesis + critic findings 검토 → Minor waive(사유)/omission(사유)/regenerate 결정, 차단 findings는 regenerate로 해소 (E4).
4. 모든 승인 완료 후 **컴파일** → 병합 Vault(`knowledge/`+`legacy/`+`.okc/`) 생성(no-clobber) → `verify()`/`explain()`로 검수 (E3, E5).
5. 병합 Vault를 read-only URL로 **공개** → okc-mcp 연동 계약 제시 (E5).

### 관련 에픽
**E1**(주체) · **E2**(토큰 발급 측) · **E3**(주체) · **E4**(주체) · **E5**(공개·검수 측). — 사실상 데모 핵심 경로 전반의 주 행위자.

---

## Persona 2 — Contributor (개인 기여자)

**이름/역할 (예시)**: "김둘" — 특정 부서의 개인 지식 노동자. 자신의 로컬 Obsidian Vault를 **발급받은 업로드 토큰으로 제출**하는 것이 유일한 임무. 통합 의사결정에는 관여하지 않는다.

### 목표 · 동기
- 자신의 로컬 Vault(디렉터리 zip / tar.zst)를 **간편하게, 전체 로그인 없이 토큰만으로 업로드**해 조직 통합에 기여한다 (원 요구사항 ④, FR-UP-2).
- 업로드가 검증·수락되어 소스로 등록됐는지, 무엇이 왜 거부됐는지(포맷/크기/경로 안전성/비Markdown) **명확한 피드백**을 조회한다 (FR-UP-3, E2-S6).
- 동기: 최소한의 마찰로 내 지식을 공유하되, 내 소스가 `owner_display_name`으로 식별되어 provenance가 남기를 원한다.

### 권한 경계 (할 수 있는 것 / 없는 것)
**할 수 있는 것**
- **유효한 업로드 토큰**으로 지정된 슬롯에 Vault 업로드 (FR-UP-2). **전체 로그인 계정 없이 토큰만으로** 가능하다.
- 자신의 업로드 결과(수락/거부 사유: 포맷·크기·경로/심볼릭 링크 안전성·비Markdown) 조회 (FR-UP-3, E2-S6).
- 자신의 소스가 `owner_display_name`(개인/부서 라벨)으로 등록됨을 기대 (FR-UP-4).

**할 수 없는 것 / 제약**
- **통합 관련 어떤 변경성 작업도 불가**: 프로젝트 생성, `add_source`(직접 호출), 공급자 설정/consent, `approve_*`, `regenerate`, `compile`, 서빙 공개는 전부 금지. 시도 시 **403** (FR-AUTH-3, 원 요구사항 ②).
- **로그인 세션 없음(토큰-온리)**: 기여자는 로그인 세션을 발급받지 않는다. 기여자 계정/로그인은 **선택적이며 데모 경로 밖**이다 (E1-S1 노트, FR-UP-2).
- **토큰 발급/폐기 불가**: 토큰 라이프사이클은 관리자 소관 (FR-UP-1). 만료·폐기·무효 토큰으로는 업로드가 거부된다 (FR-UP-2).
- **conflict/critic 리뷰 열람·개입 불가**: 통합 의사결정 표면(E4)에 접근하지 않으며, 결과 조회는 자신의 업로드 범위로만 한정된다 (E2-S6).
- **소스 상한 영향**: 프로젝트당 ≤10 슬롯이므로, 상한 초과 시 관리자 정책에 따라 업로드가 제한될 수 있다 (C-3).
- **업로드 바이트는 비신뢰(hostile)로 취급**: 악성/비Markdown/경로 이탈 콘텐츠는 검증에서 거부될 수 있다 (FR-UP-3, okc-core hostile-input 보호와 중복 방어).

### 기술 수준
- **중간**. Obsidian으로 개인 노트를 관리할 줄 알고 디렉터리를 zip/압축해 업로드하는 정도는 가능. 통합 파이프라인 내부(taxonomy/critic/hash-bound 승인)는 알 필요가 없으며, 알기를 기대하지도 않는다. 거부 사유는 안정적 code/category로 제공되어 원인을 식별할 수 있다.

### 대표 시나리오
1. 관리자로부터 업로드 URL + 토큰을 전달받는다 (E2-S1 산출물).
2. 로컬 Vault를 zip/tar.zst로 묶어 토큰 인증 엔드포인트에 업로드한다 (E2-S3).
3. 접수 식별자로 결과를 조회 — 검증 통과 시 소스로 등록(`owner_display_name`) "수락됨" 피드백 수신; 실패 시 사유(포맷/크기/경로/비Markdown) 확인 후 재시도 (E2-S6).

### 관련 에픽
**E2**(주체: E2-S3 업로드, E2-S6 결과 조회). — E1은 토큰 인증/403 게이팅 경계로만 간접 접촉하며, E3/E4/E5에는 접근 권한이 없다.

---

## Persona 3 — okc-mcp Consumer (비인간, 미래 소비자)

**이름/역할**: **okc-mcp** — 사람이 아닌 **기계 소비자(programmatic consumer)**. okc-web이 공개한 병합 Vault를 RAG 소스로 소비한다. **본 범위에서는 연동 "계약(contract)"만 정의하고 okc-mcp 자체 구현은 이연**한다 (원 요구사항 ⑤, Q7 out-of-scope 표기). 데모에서는 HTTP 클라이언트로 계약을 검증한다.

### 목표 · 동기
- okc-web이 노출한 **병합 Vault의 위치(로컬 디렉터리 경로 또는 read-only API 엔드포인트)와 형식**을 안정적 계약으로 받아, 자신의 RAG 파이프라인 입력으로 삼는다 (FR-SRV-1, FR-SRV-3).
- 파일 목록·본문을 **읽기 전용으로 조회**하고, 필요 시 `verify()`/`explain()` 결과(무결성·provenance)를 함께 소비한다 (FR-SRV-2).
- 동기: 결정론적·Markdown 전용 산출물을 예측 가능한 계약으로 소비해 재임베딩/인덱싱을 수행. (소비자는 "완성되고 승인된" Vault만 본다.)

### 권한 경계 (할 수 있는 것 / 없는 것)
**할 수 있는 것 (계약 수준)**
- 공개된 병합 Vault를 **read-only**로 조회: 파일 목록 및 본문 (FR-SRV-1, E5-S2).
- `verify()`/`explain()` provenance 정보 조회 (FR-SRV-2, E5-S3).
- 계약(discovery) 엔드포인트에서 소비 위치·형식 확인 (FR-SRV-3, E5-S4).

**할 수 없는 것 / 제약**
- **모든 변경성 작업 불가**: 업로드, 소스 추가, 통합 실행, 승인, 컴파일, 공개 — 어느 것도 트리거할 수 없다. 소비자는 오직 **읽기**만 한다. 쓰기성 메서드는 405/거부 (FR-SRV-1 read-only 계약, E5-S2).
- **RAG 리트리벌은 okc-web 밖**: 청킹·임베딩·vector index·쿼리 인터페이스는 okc-mcp(또는 외부 엔진)의 몫이며 **본 범위에서 구현하지 않는다**. 계약(discovery)에는 read-only 조회 엔드포인트만 노출되고 MCP 툴 표면은 포함되지 않는다 (FR-SRV-3, Q5/Q7 out-of-scope, E5-S4).
- **재임베딩 필요**: okc-core 임베딩은 컴파일 전 코퍼스 대상이며 일시적(폐기)이므로, 소비자는 **컴파일된 산출물을 재임베딩**해야 한다 (capability-analysis §5-10).
- **Markdown 전용 소비**: 병합 산출물은 Markdown 전용(`knowledge/`+`legacy/`+`.okc/`). 비Markdown 자료(첨부/Canvas/Base)는 계약에 포함되지 않는다 (NFR-PORT-1).
- **인증/신뢰 경계**: 읽기 접근 자체의 게이팅(공개 범위·토큰)은 okc-web 책임이며, okc-core는 관여하지 않는다 (C-1, C-5).
- **stale 인지**: 소스/설정 변경 시 서빙 산출물이 stale로 표시될 수 있으며(C-4), 소비자는 매니페스트 식별 정보로 신선도를 판단한다 (E5-S5).

### 기술 수준
- **해당 없음 (비인간/기계)**. 안정적 파일 목록·본문 조회 API와 명세된 디렉터리 레이아웃을 소비할 수 있으면 충분. `OkcError{code,category}` 기반 오류 분기, `PROJECT_BUSY` 재시도 등 계약 규약을 따른다(메시지 문자열 파싱 금지) (FR-INT-8 규약 준용).

### 대표 시나리오
1. 관리자가 병합 Vault를 read-only URL/경로로 공개하고 **okc-mcp 연동 계약**을 제시한다 (E5-S1, E5-S4).
2. okc-mcp(미래 구현)가 계약된 엔드포인트에서 파일 목록·본문을 읽어(E5-S2) 자신의 RAG 인덱스로 재임베딩한다. — **이 단계의 RAG 내부 구현은 스토리화하지 않고 이연**.

### 관련 에픽
**E5**(소비 대상, 계약 정의 한정; 주체 스토리 E5-S2/E5-S3/E5-S4). — okc-web 범위에서는 **read-only 서빙 계약(FR-SRV-1..3)**까지만 스토리화하며, okc-mcp 내부(RAG)는 out-of-scope로 명시한다.

---

## 페르소나 요약 매핑

| 페르소나 | 인간? | 주체 스토리 | 변경성 권한 | 핵심 okc-core 제약 |
|---|---|---|---|---|
| Administrator/Curator | 예 | E1 전부, E2-S1/S2/S4/S5, E3 전부, E4 전부, E5-S1/S3/S5 | **전부** (유일) | 승자 선택 불가·Major/Critical waive 불가·hash-bound 승인·≤10 소스·curator_id 비검증·공급자 disclosure/consent 게이트 |
| Contributor | 예 | E2-S3, E2-S6 | 없음(토큰 업로드/결과 조회만) | 토큰-온리(로그인 없음)·토큰 만료/폐기·비신뢰 입력 검증·403 게이팅 |
| okc-mcp Consumer | 아니오 | E5-S2, E5-S3, E5-S4 | 없음(read-only) | RAG 이연·재임베딩 필요·Markdown 전용·stale 인지 |

_다음_: `stories.md` — 위 페르소나 × 에픽(E1..E5)의 INVEST 스토리 + Given/When/Then 수용 기준 + MoSCoW + 추적성.
