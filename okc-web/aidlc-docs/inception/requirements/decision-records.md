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
