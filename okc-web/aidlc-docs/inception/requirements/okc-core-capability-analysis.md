# okc-core Capability Analysis (Requirements Grounding)

**Purpose**: okc-web 요구사항을 okc-core(OKC = Obsidian Knowledge Compilation)의 *실제* 기능·제약에 근거해 작성하기 위한 분석 문서.
**Source**: okc-core `c:/Users/genie/workplace/okc-core` (Rust workspace v0.3.0) 의 6개 서브시스템 병렬 분석 + 종합.
**Generated**: 2026-09-08 (INCEPTION / Requirements Analysis)

> ⚠️ 이 문서는 okc-web이 무엇을 **재사용**할 수 있고 무엇을 **새로 만들어야** 하는지를 규정합니다. 요구사항·설계의 근거로 사용됩니다.

---

## 1. OKC란 무엇인가

OKC는 "폴더 병합기"가 아니라 **지식 컴파일러(knowledge compiler)** 입니다. 불변(immutable)·비신뢰(hostile) Obsidian Vault 스냅샷들을 입력받아, **결정론적(deterministic)·감사가능(auditable)** 한 새 Vault 디렉터리(`knowledge/` 정규 노트 + `legacy/` 리다이렉트 스텁 + `.okc/` 감사 봉투)로 컴파일합니다.

- 아키텍처는 안쪽으로만 의존: `okc`(CLI/TUI) · Python · Node 어댑터 → `okc-interop`(런타임 중립 facade) → `okc-app`(서비스) → `okc-core`(엔진) → FS/SQLite/AI providers.
- **ADR-0002**: "CLI, MCP, Obsidian, web, registry 통합"은 공개 오퍼레이션 위의 **얇은 어댑터**여야 하며, 정규 상태나 컴파일 정책을 소유해서는 안 됨 → **okc-web은 정당한 어댑터로 예정되어 있으나, 두 번째 정책 소유자가 되어선 안 됨**.
- 최종 compile/verify/explain 경로는 **네트워크 호출이 전혀 없음**(offline·provider-free). Provider는 오직 비신뢰 proposal만 반환하며, 로컬 검증 + 독립 critic + 명시적 curator 승인(hash-bound)이 모든 것을 게이팅.

## 2. 핵심 상태 기계 (Integration State Machine)

```
PreparedCorpus → (sensitive preflight) → recorded embeddings/candidates
  → TaxonomyProposal → approve_taxonomy → SynthesisProposal → CriticReport
  → approve_cluster / regenerate_cluster → ApprovedIntegrationPlan
  → compile → CompiledVaultManifest → verify / explain → ProvenanceRecord
```
- `ApprovedIntegrationPlan` 이 **유일한 컴파일 권한**이며 hash-bound. 상위 입력(소스/설정/taxonomy)이 하나라도 바뀌면 **모든 하위 승인이 무효화(stale)** 됨.
- `IntegrationCheckpoint`: `NeedsProvider/NeedsSources/NeedsDisclosure/NeedsTaxonomy/NeedsClusters/ReadyToCompile/Verified` — 통합은 **원클릭이 아니라 다단계 human-in-the-loop 루프**.

## 3. 경계(Boundary): 누가 무엇을 소유하는가

### ✅ okc-core가 이미 제공
- 디렉터리/ZIP/tar.zst Vault 다중 소스 인제스트(**프로젝트당 최대 10개**), content hashing, symlink 거부, 중복 검출.
- AI 파이프라인: embedding → in-memory cosine 후보 → taxonomy organizer → 클러스터별 synthesis → critic.
- Curator 승인 게이트: `approve_taxonomy(edited_clusters, rationale)`, `approve_cluster(omission_rationales, minor_waivers)`, `regenerate_cluster(feedback)` — 모두 hash-bound.
- **모순 보존(Contradiction preservation)**: 상충하는 소스 주장은 증거와 함께 전부 보존(승자 선택 없음).
- 결정론적·offline·provider-free `compile()` → 병합 Vault 디렉터리, no-clobber atomic publish. `verify()`/`explain()` 재유도.
- Remote provider disclosure + 민감정보 consent 게이트(`allow_remote_provider` AND `remote_disclosure_confirmed`).
- 구조화된 에러 taxonomy(`OkcError{code,category,retryable,details}`), bounded/cancellable Job 모델, per-project single-writer `project.lock`, append-only SQLite 감사 저널(schema 4).
- AI provider 자격증명 저장(OS keychain 또는 env-var 이름) — **서버측 AI 키 전용**.
- `okc-interop` 런타임 중립 facade + Python/Node 바인딩(전체 파이프라인 노출).

### 🔨 okc-web이 새로 만들어야 함 (greenfield)
- **인증/인가/역할(RBAC)/관리자 게이팅 전부** (req 2) — okc-core에는 auth가 전무.
- 사용자/부서/개인 **identity 모델** 및 이를 `SourceId` + `owner_display_name`(장식용 라벨일 뿐)에 매핑.
- 인증된 admin을 okc-core의 free-text `curator_id`(검증 안 됨)에 바인딩.
- **개인별 업로드 서비스**: HTTP 엔드포인트, 토큰 발급/검증, 쿼터/포맷/악성 검사, 업로드 바이트를 로컬 절대경로(디렉터리/ZIP/tar.zst)로 착지 후 `add_source` (req 4).
- `IntegrationCheckpoint` 루프 오케스트레이션(admin 워크플로우).
- **Conflict 리뷰 UI**: 불투명 JSON(critic findings, 제안된 omission, 보존된 contradiction) 파싱 → admin 선택을 `approve_cluster` 키 또는 `regenerate` 피드백으로 매핑 (req 3).
- 10-소스 상한을 넘기기 위한 **federation 전략(또는 상한 상향 ADR)** (req 1).
- 병합 Vault 디렉터리의 **read API / 서빙**(okc-mcp에 위임하지 않는 한) (req 5).
- 멀티테넌트 격리 + 평문 데이터 at-rest 암호화, cross-process 락/큐/lease + 진행상황 전달(Job.events → SSE/WebSocket).
- 테넌트별 AI provider 자격증명 프로비저닝(바인딩은 env-var 이름만 수용).

### ⏭️ okc-mcp로 이연 (현재 미구현)
- **RAG 리트리벌**: chunking, 영속 embedding/vector(또는 하이브리드 lexical) 인덱스, 쿼리 인터페이스.
- MCP 툴 표면(`open_project, integration_status, list_taxonomy/clusters, record_review, compile_approved_plan, verify_compiled_vault, explain_provenance`) — `docs/specs/mcp-adapter.md` 는 normative-**future**.
- 리트리벌을 1st-party(fixed-seed HNSW)로 할지 외부 엔진(예: obsidian-mcp)에 위임할지 결정.
- **주의**: okc-mcp는 오늘 존재하지 않음. req 5가 okc-mcp보다 먼저 출시돼야 하면 RAG 서빙은 okc-web으로 떨어짐.

## 4. 권장 통합 메커니즘

**Rust 백엔드(예: axum)에서 `okc-interop` 크레이트를 path dependency로 직접 링크** (필요 시 `okc-app` 로 타입드 접근).
- 이유: `okc-interop` 은 `publish=false`·Rust 전용 → Rust 백엔드가 JSON 왕복 없이 네이티브 링크, 타입드 DTO(interop schema v2), `OkcClient→Project→Job<T>` async/cancellable 모델, 구조화된 `OkcError`, bounded scheduler + per-project reservation을 그대로 획득.
- Node/Python 바인딩은 대안이지만 약함: 미배포 0.3.0 소스빌드, provider 시크릿을 env-var **이름**으로만 수용, 승인/클러스터 payload가 **불투명 JSON(VersionedPayload)**, Python `result()` 는 블로킹.
- **결정적 주의**: scheduler와 `PROJECT_RESERVATIONS` 는 **프로세스 전역 static** → cross-process 상호배제는 오직 on-disk `project.lock` 에 의존. okc-web은 **단일 장수 엔진 프로세스**(또는 프로젝트 셋당 소유 프로세스 1개)로 운영하고 통합 실행을 스스로 직렬화/큐잉해야 함. 모든 I/O는 절대 로컬 경로. 서비스에서 CLI shell-out은 지양.

## 5. 주요 리스크 / 갭

1. **req 3 (관리자가 conflict 해결책 선택)** 은 okc-core 모델과 근본적으로 불일치: 모순은 보존(승자 선택 API 없음), 구조적 path/link conflict는 내부 계산 후 **caller 도달 전 폐기**(ADR-0017 로 public overlay 폐지), critic Major/Critical은 **waive 불가**(regeneration만이 해소). → **확정 결정(경우 B, 2026-09-08)으로 해소됨**: 관리자의 리뷰 '선택'은 승자 선택이 아니라 **결정(행동)** — {클러스터 이대로 승인(모순 보존) / 사유와 함께 regenerate / omission 제안(사유) / minor finding waive(사유)} 중 하나 — 이며, 웹은 이를 기존 `approve_cluster`/`regenerate_cluster` 표면으로만 전달하고 okc-core는 **수정하지 않음**(신규 public API·승자 선택 API 없음; ADR-0002 어댑터·ADR-0024 준수). 상세는 `decision-records.md` 참조.
2. **req 4/5** 는 100% greenfield: HTTP 서버·업로드·토큰·서빙·RAG/vector store 전부 부재.
3. **okc-mcp 부재**: 서빙이 지금 필요하면 RAG 티어가 okc-web으로 이동. mcp-adapter 스펙은 MCP를 "컴파일러 control-plane"으로 규정(자체 RAG 서버 아님) → "RAG 데이터 소스" 기대와 어긋날 수 있음.
4. **auth 전무**: reqs 1·2 권한 모델 전부 okc-web 몫. `curator_id` 는 미검증 라벨이라 "관리자만 통합"을 자체적으로 강제 불가.
5. **10-소스 상한(ADR-0016)**: 부서/개인 스케일과 충돌 → federation vs 상향 결정 필요.
6. **hash-bound 단일 실행 승인**: 소스/설정 변경 시 전 승인 무효 → 점진적/trickle-in 업로드는 비쌈 → freeze-then-integrate 워크플로우 유력.
7. **Markdown 전용 materializer**: attachment/Canvas/Base/완전 link-rewrite/OKCPack 은 미해결 release blocker → 비Markdown/대형 Vault는 병합 출력이 불완전.
8. **0.3.0 개발 트리**(stable release 금지, 바인딩 미배포): 프로덕션 구축 시 안정성·API churn 리스크. 특정 commit 핀 + CI 빌드 필요.
9. **평문·single-writer·local-first 저장**: 공유/호스티드 서비스에 부적합 → okc-web이 암호화·격리·동시성 제어 추가 필요.
10. embedding은 컴파일 전 코퍼스 대상이며 **일시적(폐기)** → RAG는 **컴파일된 출력을 재임베딩**해야 함(export/persist 경로 없음).

## 6. 확정된 결정 (Determined without asking)

- **Project Type**: Greenfield (okc-web). okc-core는 별도 의존성 저장소 → okc-web에 대한 Reverse Engineering 불요.
- **Requirements Depth**: **Comprehensive** — 다중 컴포넌트·cross-system(okc-web + okc-core + 미래 okc-mcp)·높은 리스크.
