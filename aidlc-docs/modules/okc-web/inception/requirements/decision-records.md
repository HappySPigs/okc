# Decision Records — okc-web

**목적**: okc-web INCEPTION/CONSTRUCTION 과정에서 내린 확정 결정을 추적 가능하게 기록한다 (해커톤 심사 C1: 단계별 결정의 연속성·반영 증거).

**형식**: 결정마다 배경 / 확정 결정 / 기각된 대안 / 코드 근거 / 추적성.

---

## 결정 기록 — req-3 "conflict 확인·선택" 표면: 경우 B (Decision-Surface)

**날짜**: 2026-09-08
**검증 워크플로우**: wf_91476e22-fe1 (okc-core 코드 검증), wf_4b277935-b79 (산출물 정합성 감사)
**상태**: 확정(Confirmed) — 사용자 승인 2026-09-08

### 배경 (Context)
사용자의 원 요구사항 ③은 "관리자가 conflict를 확인·선택해서 통합한다"였고, 이를 문자 그대로 읽으면 관리자가 상충하는 두 주장 중 하나를 정답(승자)으로 고르는 "승자 선택(winner-select)" 흐름으로 오해될 수 있었다. 사용자는 실제로 이 승자 선택 흐름이 가능한지/필요한지 문의했다. 그러나 okc-core는 상충 주장을 증거와 함께 전부 보존하며(모순 보존), 승자를 고르는 public API가 존재하지 않는다(`okc-core-capability-analysis.md` 라인 36·70, C-2). 따라서 req-3의 UI/도메인 표면을 어떻게 정의할지 결정이 필요했다.

### 확정 결정 (경우 B / Decision-Surface)
관리자가 conflict/critic 리뷰에서 하는 "선택"은 **승자 선택이 아니라 결정(행동)** 이다. 관리자는 다음 결정 집합 중 하나를 남긴다:
- **클러스터 이대로 승인**(모순 보존) → `approve_cluster`
- **사유와 함께 regenerate** → `regenerate_cluster(feedback)`
- **omission 제안**(사유 필수)
- **minor finding waive**(사유 필수)

웹(okc-web)은 이 결정을 수집해 okc-core의 **기존 curator 표면**(`approve_cluster`(omission_rationales, minor_waivers) / `regenerate_cluster`(feedback))으로만 전달한다. 이후 처리(IntegrationCheckpoint 파생 → ReadyToCompile 파생 → `compile()`)는 okc-core가 담당한다. Contradiction은 **불변·hash-bound 데이터**로 보존되어 병합 산출물의 `## Contradictions` 섹션과 `.okc/` provenance에 **전량 렌더**되며, 어느 한쪽을 정답으로 고르는 API도 계획도 없다. 이 "정보를 조용히 버리지 않는" 보존 설계가 곧 **C3 차별성**(정보를 은밀히 삭제하는 흔한 병합 도구와의 대비)이다.

### 기각된 대안 (Rejected Alternatives)
1. **웹 레이어 선호 주석(web-layer preference annotation)**: okc-core 데이터는 그대로 두되, 웹이 관리자가 선호하는 주장에 "선호/우세" 주석·플래그를 별도 저장하는 방식. — **기각**. 그림자 승자 선택(shadow winner-select)을 재도입해 한쪽 주장을 은밀히 우대하고, 웹 상태가 hash-bound 코어 provenance와 어긋나 단일 출처·감사가능성이 깨진다. 모순 보존 원칙(C-2, ADR-0024)에 반한다.
2. **진짜 승자 선택 — 코어 포크 필요(true winner-select requiring core fork)**: okc-core에 승자 선택용 신규 public API를 추가(사실상 포크). — **기각**. okc-core에는 그런 API도 계획도 없고(ADR-0024: 모순을 투표로 지우지 말 것), 어댑터 원칙(ADR-0002)에 위배되며, 정보를 버려 C3 차별성과 정반대다.

### 코드 근거 (Code Grounding)
- **IntegrationCheckpoint 파생 상태**: 유일한 상태 enum은 `{NeedsProvider,NeedsSources,NeedsDisclosure,NeedsTaxonomy,NeedsClusters,ReadyToCompile,Verified}` (integration_service.rs:27-37); 저장되지 않고 매 `checkpoint()` 호출마다 파생된다. `Conflict`/`AwaitingWinner` 같은 변형은 없다. 결정은 hash-bound curator 표면을 통해서만 반영된다.
- **Contradiction 불변·보존**: `ContradictionSet`/`ContradictionClaim` (integration.rs:275-291)은 `SynthesisProposal.contradictions` 필드로 `proposal_hash`에 바인딩된 불변 데이터(라인 304). 검증은 claim ≥2 요구, `render_canonical_note`는 전 claim을 무조건 렌더. 승자 선택 없음(C-2).
- **critic 차단성 findings**: Major/Critical은 waive 불가, 오직 `regenerate_cluster`로만 해소(integration_service.rs:374-383; core 방어선 integration.rs:1074-1082). 구조적 path/link conflict는 caller 도달 전 내부 폐기(ADR-0017) — 모순 보존과 별개의 core 동작.
- **ADR-0024**(모순 보존 — 투표로 지우지 않음) · **ADR-0002**(어댑터 원칙 — okc-core 무수정, 기존 표면만 소비).

### 추적성 (Traceability)
- **FR-INT-5** — 원 요구사항 ③(conflict 확인·선택)을 승인/waive/omission/regenerate 결정 표면으로 제공(승자 선택 API 부재).
- **FR-INT-6** — 모순 보존·승자 없음(no winner).
- **C-2** — okc-core 하드 제약: 모순 보존, 승자 선택 API 없음, 차단성 findings는 regenerate로만 해소.
- **R-1** — 리스크: 문자 그대로의 req-3이 core 모델과 불일치 → 경우 B로 재해석하여 해소.
- **Epic E4** — Conflict/Critic 리뷰(스토리 E4-S1..S6, decision-surface UI: 승인/waive/omission/regenerate만 노출, 승자 버튼 없음).
- **Persona P1** — Administrator / Curator: 유일한 변경성 작업 주체로서 승인/waive/omission/regenerate 결정을 남기고 provenance를 책임진다(승자 선택 불가 명시).

---

## ADR-0025 — 스택 전환: 백엔드 Rust(axum) → FastAPI(Python) · 프런트 Next.js → React SPA(Vite)

**날짜**: 2026-09-08
**상태**: 확정(Confirmed) — 사용자 승인(autopilot 지시 2026-09-08: "backend rust로 되어있는거 fastapi로 전체 변경 … 권장안대로 … MVP 범위/설계 구현 범위 확대 금지 … front도 fastapi에 맞는 프레임워크로 변경 … 설계안 바꾸고 construction까지도").
**세부 매핑 사양**: [`../../construction/plans/stack-migration-spec.md`](../../construction/plans/stack-migration-spec.md) (AUTHORITATIVE — 모든 문서/코드 편집의 단일 출처).

### 배경 (Context)
INCEPTION은 백엔드를 **Rust(axum) + `okc-interop` Rust 직접 링크**로, 프런트를 **Next.js(App Router)**로 확정했다(ADR-0002 어댑터 경계 위). 그러나 `requirements.md` Q8은 이미 *"⚠️팀 숙련도 시 Python로 변경"* 을, C-7은 바인딩 경로(불투명 JSON payload)를, R-5는 *"Q8 스택 선택은 … 재검토 여지"* 를 명시적으로 남겨두었다. 사용자가 이 재검토 여지를 발동해 백엔드를 FastAPI로, 프런트를 그에 맞는 프레임워크로 전면 전환하도록 지시했다. 관건은 **MVP·설계 범위를 절대 키우지 않는 충실한 1:1 포팅**으로 수행하는 것이다.

### 확정 결정 (Decision)
1. **백엔드 = FastAPI (Python 3.11+)**, okc-core를 **okc Python 바인딩**(`okc-compiler` 0.3.0, maturin/pyo3, `okc-core/bindings/python`)으로 소비한다. 이 바인딩은 `okc-interop`과 **동일 표면**(`OkcClient`·`Project`·`Job`·`OkcError`·`INTEROP_SCHEMA_VERSION==2`, 동일 메서드명)을 노출하므로 ADR-0002 어댑터 경계가 그대로 성립한다(단일 링크점 = `app/adapter/`). 대부분 메서드가 `dict[str,Any]`를 반환(C-7의 "불투명 JSON")하므로 **어댑터가 Pydantic으로 파싱**해 downstream을 계약으로부터 격리한다.
2. **프런트 = React + Vite SPA (TypeScript, React Router v6)**. FastAPI는 순수 JSON API이므로 자연스러운 짝은 순수 클라이언트 SPA다. **`design-system.md`/`ui-screens.md`의 디자인 시스템·화면·라우트·토큰·컴포넌트 인벤토리(Tailwind·shadcn/ui·Tremor·lucide·Radix Colors·TanStack Query/Table)는 전부 보존**되고, Next.js 셸(App Router·server component·`next/font`)만 SPA로 대체된다.
3. **모든 설계 결정·불변식은 무변경 보존**: ADR-0002 단일 시임, RBAC-before-core(C-1), 단일 프로세스+단일-writer 큐+`PROJECT_BUSY`, **winner-select 없는 3-변형 `CuratorDecision`(C3)**, hash-bound staleness(C-4), SQLite 상태 모델, OkcError→HTTP 매핑표, E4-3/E5-2 focal 계약, screen→story 매트릭스, epic→unit→module 맵, 숨은 런타임 spine, 29 스토리/5 에픽. 자세한 Rust↔Python·Next↔React 대응은 마이그레이션 사양 §1–§5.

### 기각된 대안 (Rejected Alternatives)
1. **백엔드는 FastAPI로 바꾸되 프런트는 Next.js 유지** — Next.js는 그 자체가 풀스택 프레임워크(자체 서버·server component)라 순수 JSON 백엔드와 짝이 어색하고, 사용자가 "front도 변경"을 명시했다. 기각.
2. **Python 풀스택 서버렌더(Jinja2/HTMX·Streamlit·NiceGUI 등)로 프런트 대체** — 이미 확정된 43KB `ui-screens.md`+22KB `design-system.md`의 리치 인터랙션 화면(focal E4-3/E5-2)을 폐기해야 하므로 **설계 범위를 크게 확대**하고 C1(단계 연속성) 추적성을 훼손한다. 사용자의 "설계 구현 범위가 커지는 방향으로 절대 가지마" 제약에 정면 위배. 기각.
3. **okc-core를 HTTP/서브프로세스로 감싸 언어 무관 소비** — 새 IPC 계층 = 명백한 범위 확대이자 ADR-0002(단일 시임) 위배. 기각. 바인딩 직접 임포트가 최소 범위.

### 코드 근거 (Code Grounding)
- **동일 표면 확인**: `okc-core/bindings/python/okc/__init__.pyi` — `INTEROP_SCHEMA_VERSION:int`, `class OkcClient`(`create_project`/`open_project`/`verify_artifact`/`explain_artifact`/`api_info`), `class Project`(`status`/`add_source`/`replace_sources`/`preflight`/`integrate`/`taxonomy`/`approve_taxonomy`/`clusters`/`approve_cluster`/`regenerate_cluster`/`compile`/`manifest`), `class Job[_T]`(`.state`/`.events()`/`.result()`/`.cancel()`), `class OkcError(RuntimeError){code,category,message,retryable,details}`. `okc-interop`의 예약/비예약 op 분할과 `Job.result()` 블로킹 특성 동일 → 단일-writer 액터 패턴이 Python 단일 워커 스레드로 충실히 이식.
- **바인딩 패키징**: `okc-core/bindings/python/pyproject.toml` — `name="okc-compiler"`, `requires-python>=3.11`, maturin/pyo3(`abi3-py311`), `module-name="okc._native"`. 빌드타임 Rust 툴체인 필요(이미 설치된 cargo 1.98.1).
- **설계상 예고**: `requirements.md` Q8(팀 숙련도 시 Python)·C-7(바인딩 payload 불투명 JSON)·R-5(스택 재검토 여지) → 본 결정이 그 재검토를 확정.

### 추적성 (Traceability)
- **Q8 / C-7 / R-5** — 예고된 Python 대안을 확정; 불투명-JSON 트레이드오프를 어댑터 Pydantic 파싱으로 해소.
- **ADR-0002** — 어댑터 경계 무변경(단일 링크점이 Rust `okc-interop` → Python `okc`로만 바뀜). okc-core 무수정.
- **NFR-CONC-1** — 단일-writer 직렬화: Rust 블로킹 액터 스레드 → Python `ThreadPoolExecutor(max_workers=1)` + read-path 별도 클라이언트(큐 우회). uvicorn `--workers 1`로 단일 프로세스 소유 보장.
- **C1(협업 진정성)** — 앞 단계 결정(Q8/C-7/R-5)이 뒤에 반영되는 연속성 증거. 화면/스토리/결정 무변경 보존으로 추적성 강화.
- **C3(차별성)** — `CuratorDecision`을 Pydantic 판별 유니온의 정확히 3-변형(winner-select 구조적 표현 불가)으로 유지 → 코드 검증 가능한 차별성 보존.
- **전 유닛(U0–U6)** — 유닛 경계·모듈 1:1 맵·웨이브 스케줄(W0–W5) 무변경; 오직 언어/프레임워크만 치환.
