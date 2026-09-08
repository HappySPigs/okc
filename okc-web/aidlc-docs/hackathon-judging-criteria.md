# okc-web 해커톤 심사 기준 — 정본(canonical) 게이트 레퍼런스

> **문서 목적**: 6개 AI 심사 기준(C1–C6)을 단일 정본으로 통합하여, **모든 AI-DLC 스테이지 완료 게이트에서 반드시 이 문서를 참조**해 해당 스테이지에 걸리는 기준 항목을 점검한다. 순수 AI-DLC 실행이 놓치는 **해커톤 고유 항목**(screenshots/·result/ 시연, 실행 가능 빌드(deps/entry/lockfile/CI), 화면 내 사용성, 시크릿 관리, 로깅·관측, 단계-간 의사결정 추적성)을 특히 강조한다.
>
> **작성 시점 리포지토리 실측(2026-09-08)**: 코드 0, `README` 없음, `screenshots/`·`result/` 없음, `aidlc-docs/inception/application-design/` **없음**(그러나 `aidlc-state.md` line 8은 "실행 중"으로 주장), 본 `hackathon-judging-criteria.md`도 line 12에서 참조되나 실제로는 **미존재**. 현재 실제 스테이지: Requirements + User Stories 완료(승인 게이트 대기), **다음 = Workflow Planning**.

---

## 0. 채점 구조와 6기준 요약

### 채점 구조
- **AI 심사 40%** (인간 심사위원 gating 보정 가능) + **인간 투표 60%** (전시 페이지 투표).
- 두 축 모두 **"판정자가 리포지토리/전시 페이지에서 실제로 읽고 보는 것"** 으로 채점된다. 강력한 근거가 `aidlc-docs/` 내부에만 있으면 점수로 이어지지 않는다 → **README·전시 페이지·화면·screenshots로 표면화**가 40%·60% 양쪽의 공통 관문.
- AI 심사 = 6항목 / 100점.

### 6기준 요약표

| # | 기준 | 핵심(무엇을 채점하나) | 현재 상태 |
|---|------|----------------------|-----------|
| C1 | AI 협업 진정성 / 워크플로우 증거 | 단계별 산출물이 서로 이어지고 앞 단계 결정이 뒤에 반영되는가(문서 양·도구 무관). AI-DLC 추적성이 직접 득점 | **강함** (dangling-reference 리스크) |
| C2 | 문제 정의 | 누구의/어떤 문제를/어떻게 푸는지를 독자에게 그대로 전달; **WHEN·HOW OFTEN·대상 사용자** 구체성이 높을수록 가점 | **부분** |
| C3 | 차별성 | 기존 도구 대비 **구조적** 차이를 코드/설계 문서로 확인; "새롭다" 주장만은 0점 | **부분** |
| C4 | 실제 동작·구현 완성도 | 코드로 실제 동작 + **screenshots/ 또는 result/ 시연 스크린샷 필수**; 빌드·진입점·락파일·CI, 에러/전역 핸들러, 진입점→실제 구현 완결, 스크린샷↔README 정합, 핵심 경로 스텁/TODO 없음 | **미충족(전부 미래)** |
| C5 | 온보딩·사용성 | 처음 보는 사람이 막힘없이 시작하고 화면 앞에서 다음 행동을 안다; 화면 자체가 사용법 설명, 로딩/성공/오류/빈 화면, 핵심 시나리오 end-to-end | **미충족(groundwork 부분)** |
| C6 | 유지보수성 | 코드 구조·모듈화·설정 분리, 시크릿 비하드코딩, 인증·인가·입력 검증, 로깅·관측 (프로덕션 수준 불요) | **부분** |

---

## 1. 기준별 상세 (AI-DLC 스테이지로 그룹화)

각 표: **점검 / 단계 / 현재상태 / 근거·조치 / 미준수 리스크**. 상태 범례: ✅met · 🟡partial · 🔴gap · ⏳future.

### C1 — AI 협업 진정성 / 워크플로우 증거

**요지**: okc-web의 최대 강점 축. `okc-core-capability-analysis.md → requirement-verification-questions.md(Q1–Q13) → requirements.md(§2 결정표·§4 FR 태그·§10 추적) → story-generation-plan.md(Q1–Q7) → stories.md/personas.md(스토리별 FR+원 요구+C-1..C-7 인용, 양방향 추적)` 체인이 명시적·검증가능하고, audit.md는 진짜 append-only 기록 + 실제 critic 루프(coverage_ok false→true, 10개 수정)를 담는다. 최대 위험은 **약한 추적성이 아니라 끊어진 참조**: state/audit가 디스크에 없는 산출물을 주장.

| 점검 | 단계 | 상태 | 근거·조치 | 미준수 리스크 |
|------|------|------|-----------|---------------|
| 요구가 사전 okc-core 역량분석에 근거하고 C-1..C-7이 그 분석으로 추적됨 | Requirements | ✅ | requirements.md 헤더 'Grounding' + §7; okc-core-capability-analysis.md 존재 | 근거 원천 없으면 요구가 날조로 읽힘 |
| 모든 결정(Q1–Q13)에 선택값+근거 기록, requirements 결정표로 그대로 이월 | Requirements | ✅ | requirement-verification-questions.md + requirements.md §2 근거 열 | Q&A가 장식으로 판정 |
| FR에 [코어]/[신규] 태그 + ①–⑤→FR 추적행렬 | Requirements | ✅ | requirements.md §4 태그, §10 행렬 | 5개 요구↔설계 연결 붕괴 |
| 스토리 계획이 requirements 근거 + 방법론 Q1–Q7 사전 기록 | User Stories | ✅ | story-generation-plan.md + user-stories-assessment.md | 스토리가 요구와 무관 생성 의심 |
| 스토리별 추적(원요구+FR) + AC에 okc-core 제약 내장 + 정/역 추적행렬 | User Stories | ✅ | stories.md Traceability 라인 + 역 FR→story 맵 | orphaned 결정 → 연결 미완 |
| Requirements 아키텍처 결정(Q8=A Rust+interop, Q7=A conflict)이 스토리/페르소나에 실제 반영 | User Stories | ✅ | E4-S2 typed DTO, Epic E4 헤더 + personas P1 | 전파 주장 미증명 |
| 출력을 실제로 바꾼 AI critic/refinement 루프 증거(one-shot 아님) | User Stories | ✅ | stories.md Finalize + audit.md 10개 수정 → coverage_ok=true | C1의 최강 authenticity 신호 붕괴 |
| append-only 시간순 audit(원문 입력·응답·게이트·run ID) | Cross-cutting | ✅ | audit.md 한국어 원문·승인·run ID(wf_...) | 반복/협업 증명 불가 |
| **Application Design 산출물이 디스크에 실재 + 화면→스토리ID(E1–E5) 매핑** | Application Design | 🔴 | application-design/** 가 비어있는데 state line 8/audit는 ui-screens.md 생성 주장. **조치: application-design/ui-screens.md에 화면→스토리ID 행렬(특히 E4 review·E5 provenance) 랜딩 후 stories.md 유예 매핑 정리** | state/audit가 있다는 스테이지를 repo가 못 보여줌 → 날조된 진행으로 읽혀 C1 직격 |
| **본 per-stage judging-criteria 게이트 분해 파일이 실재하고 각 게이트에서 참조됨** | Cross-cutting | 🔴 | state line 12/audit가 참조하나 미존재. **조치: 본 문서를 실체화(현재 작업)** | 기준-주도 게이트 메커니즘 자체가 dangling → 하위 게이트 신뢰 훼손 |
| **req-③ conflict 재검증 해소 후 requirements §4.3 / stories E4 / personas P1 / 양 추적행렬을 lockstep 갱신 + audit 기록** | Requirements(revisit)→User Stories | 🟡 | audit(02:05:10Z) 재검증 wf 기록·'artifact revision deferred'인데 문서는 '승자 선택 없음' 확정으로 서술. **조치: 결과 도착 시 4곳 동시 갱신 + audit 항목** | requirements=확정 vs state=재검증 → 교차-산출물 모순 |
| 각 완료 게이트 메시지·audit에 6기준 self-check(C1 추적성 자가평가 포함) | Application Design 이후 전 게이트 | 🟡 | state line 19가 규칙화했으나 User Stories 종료 시에만 설정, 이전 게이트엔 요약 없음. **조치: Application Design부터 완료 메시지·audit에 6기준 요약 첨부** | 정책만 있고 산출에 안 보이면 미증명 |
| 하위 construction 산출물이 구현하는 story ID/FR 인용(요구→스토리→코드 연속) | Units/Functional/Code Gen | ⏳ | 코드 없음. **조치: 유닛/모듈을 E-스토리ID에 태깅(E4-S3→cluster-approval handler), commit/PR 본문에 story/FR 인용** | inception↔construction 경계에서 체인 단절(가장 흔한 지점) |
| 리포지토리에 AI-DLC 서사(capability→Q&A→req→stories→design→code)가 판정자 가독 형태로 존재(내부 audit.md뿐 아님) | Build & Test | ⏳ | 현재 서사는 audit.md/state에만. **조치: repo 루트 README/PROCESS 섹션에 산출물 체인 링크** | 최강 증거(critic 루프·결정표·추적)가 내부에 묻혀 점수화 안 됨 |
| UI 방향 결정(clean/minimal, E4·E5 focal 'wow')이 state/audit→설계 산출물→코드로 전파 | Application Design→Code Gen | 🟡 | state line 11/audit에 결정만 있고 산출물 없음. **조치: ui-screens.md에 E4/E5 focal 우선순위 인코딩 후 화면으로 이행** | prose에만 남으면 미전파 결정 = C1 감점 |
| aidlc-state.md 스테이지-진행·확장 결정이 디스크 실체와 일치(초과 주장 금지), opt-out에 'Decided At' 표기 | Cross-cutting | 🟡 | 확장표(Security/Resiliency/PBT=No)는 건전하나 line 8/12가 과대주장. **조치: 매 게이트마다 state↔disk 정합('launched, not yet landed' 표기 등)** | 권위 문서가 과대주장하면 모든 추적 주장 의심 |

### C2 — 문제 정의

**요지**: WHO(페르소나)와 how-solved 추적성은 강함. 그러나 rubric이 명시적으로 가점하는 부분이 약함 — **독립 Problem Statement 부재**, **WHEN/HOW-OFTEN 전무**, **현상유지 고통 baseline 전무**, 모든 서술이 한국어 aidlc-docs 내부에만.

| 점검 | 단계 | 상태 | 근거·조치 | 미준수 리스크 |
|------|------|------|-----------|---------------|
| requirements.md에 독립 'Problem Statement'(WHOSE/WHAT/HOW를 산문 3–5문장) — §1 Intent(요구 재진술)·페르소나 goal과 분리 | Requirements | 🟡 | 문제는 §1 Intent(기능 나열)와 personas goal에만 암시. **조치: §0/§1 문제 서술 추가(기능 열거 아님)** | 판정자가 spec으로 읽고 문제정의 아님으로 채점 |
| 대상 사용자(WHO) 역할·맥락·조직 설정 구체화 | User Stories | ✅ | personas.md 3인(정하나 Admin/Curator, 김둘 Contributor, okc-mcp Consumer). **유지 + README로 표면화** | 낮음(단, 내부에만 묻힐 위험) |
| **WHEN — 고통이 발생하는 구체 트리거 시점**(신규 입사자 온보딩, 분기 지식 통합, 분산 개인 vault의 단일화 필요 등) | Requirements | 🔴 | 어디에도 트리거 없음. §9는 happy-path. **조치: 문제 서술에 통합을 강제하는 구체 상황 명시** | rubric이 'WHEN 구체성 가점' 명시 → 직접 감점 |
| **HOW OFTEN — 고통 재발 빈도/주기** | Requirements | 🔴 | 빈도 서술 전무. freeze-then-run(episodic)이 빈도로 프레이밍 안 됨. **조치: 주기(통합 사이클/기여자 배치마다) 명시** | rubric이 'HOW OFTEN 가점' 명시 → 직접 감점 |
| 현상유지 baseline: okc-web 없이 오늘 큐레이터가 하는 것(수작업 병합, provenance 손실, 미관리 모순, 승인 감사 없음, RBAC 없음)과 왜 고통인지 | Requirements | 🔴 | §7(C-1..C-7)은 엔진 사실이지 사용자 고통 아님. **조치: 'today, without okc-web' 대비 2–3줄 추가** | 대비 없으면 '왜 필요한가' 불명 |
| how-solved를 pain→solution 독자 서사로(요구→FR→스토리ID 추적만이 아니라) | Requirements | 🟡 | 추적은 우수하나 요구-인덱스. **조치: 문제 서술 뒤 pain별 해결 문단(분산 vault→권한 통합; 미해결 conflict→E4 human-in-loop; 감사 없음→curator_id+provenance)** | 해결 메커니즘이 문제와 안 묶임 |
| 처음 보는 독자에게 최상위(README·전시 페이지)에서 문제가 가독(내부 한국어 requirements뿐 아님) | Build & Test | ⏳ | README/전시 서사 전무. **조치: Build&Test에서 README 문제 섹션 + 전시 페이지, Requirements 문제 서술 verbatim 재사용** | 40% AI + 60% 인간투표 양쪽이 문제를 못 봄 → C2 붕괴 |
| 사용자가 도착하는 화면(대시보드 헤더/빈 화면/프로젝트 생성 인트로)에서 문제 재진술('분산 부서/개인 vault를 하나의 감사가능 vault로') | Application Design | ⏳ | application-design/ 비어있음. **조치: 화면 인벤토리 생성 시 랜딩/대시보드/빈 화면에 문제 프레이밍 copy 요구** | C5와 중첩; 스크린샷만 보는 판정자에 '전달' 상실 |
| 문제 비자명성: 순진한 접근(Obsidian Sync, git merge, 공유 위키)이 왜 못 푸는가(contradiction preservation, provenance, human-in-loop, RBAC over auth-less core) | Requirements | 🟡 | 구별 제약이 엔진 사실로만 프레이밍(C-2, FR-INT-5, NFR-DET-1). **조치: 'why this is hard'로 문제 서술에 접기; E4에 앵커** | 일반적('vault 병합')으로 읽혀 C2·C3 동시 약화 |
| 문제 경계를 사용자 맥락으로 진술(단일 조직, 부서별 개인 vault, ≤10 소스/통합) | Requirements | 🟡 | 사실은 scope/아키텍처 결정으로만(§2 Q1, C-3). **조치: WHO-context로 1회 재표현** | 숫자가 제약으로만 남아 사용자 규모 감 없음 |
| 문제 프레이밍이 하위로 일관 유지(스토리 'so that'·화면이 동일 문제로 back-ref) | User Stories | 🟡 | 'so that'가 원요구 ①–⑤로만 추적(Problem Statement 미존재). **조치: 문제 서술 추가 후 페르소나/스토리 헤더에 1줄 back-ref** | 단일 앵커 없으면 문제 drift |

### C3 — 차별성 (구조적, 코드/설계 검증)

**요지**: 진짜 구조적 차별점을 이미 **보유** — deterministic·offline·provider-free 지식 COMPILER(okc-core) 위의 RBAC 컨트롤플레인 + serving 어댑터, contradiction-preservation(승자 선택 없음), un-waivable Major/Critical critic, hash-bound freeze-then-run, provenance/verify. 그러나 **차별성으로 프레이밍된 문서가 없어** 판정자는 암시적 제약만 읽는다(="새롭다"만 = 0점). 최우선: named 경쟁제품 대비 구조 대조 섹션 + 각 차별점→산출물/코드 경로 매핑, 그리고 code/screenshots로 실제 시연.

| 점검 | 단계 | 상태 | 근거·조치 | 미준수 리스크 |
|------|------|------|-----------|---------------|
| **named 대안(Obsidian Publish/Sync, Notion/Confluence, LlamaIndex/Mem0 RAG ingest, git-merge conflict resolver) 대비 구조적 차별 진술 섹션**(bare 'new' 아님) | Application Design(Req 시딩→README/전시) | 🔴 | 어떤 대안과도 대조 없음. 원재료(capability §1, requirements §8, C-1..C-7)만 존재. **조치: application-design 차별성 섹션 + README 미러** | rubric이 'new' 주장 = 0점 명시; named 대조 없으면 실재해도 미인정 |
| 'deterministic·offline·provider-free COMPILER(folder-merger/live-sync/RAG-ingest 아님)' 포지셔닝 문서화 + 실제 동작 추적(compile가 knowledge/+legacy/+.okc/ 결정론·무네트워크 생성) | Requirements(문서)→Code Gen(검증) | 🟡 | 문서: capability §1, FR-INT-7, E3-S6. 코드 미검증. **조치: Code Gen에서 offline deterministic compile 호출 + 재현가능 출력 시연** | appendix에만 있고 코드/데모에 없으면 core 주장 미확인 |
| contradiction-preservation(승자 선택 NO)를 1급 code-verified 차별점으로: E4 화면이 모든 상충 주장+evidence+provenance 렌더, winner-select/delete 컨트롤 없음, diff/merge resolver와 명시 대조 | User Stories→App Design→Code Gen | 🟡 | E4-S5+C-2+personas P1 강력 명세. 화면/코드 없음. **조치: E4 화면에 winner 액션 없이 evidence 보존, 그리고 req-③ 의미 먼저 LOCK** | req-③가 pick-a-winner로 뒤집히거나 화면 미구현 시 C3·C4 최강 'wow' 상실 |
| un-waivable Major/Critical + Minor-waivable(+rationale) + regenerate-only가 gated 결정 표면(approve_taxonomy / approve_cluster(omission_rationales, minor_waivers) / regenerate_cluster)으로 구현, blocking 잔존 시 APPROVAL_REQUIRED로 compile 거부 | User Stories→Code Gen | 🟡 | E4-S2/S3/S4/S6+C-2. **조치: 실제 okc-core 승인 호출 + APPROVAL_REQUIRED compile-거부 게이트 wiring** | 없으면 일반 approve/reject로 보여 구조 차별 미검증 |
| hash-bound freeze-then-run + stale-invalidation을 continuous-sync 대비 구조 대조로 + code-verified(소스/설정 변경 시 하위 승인 stale, 재승인까지 compile 차단) | User Stories→Code Gen | 🟡 | E3-S5, E2-S5, E4 stale AC + C-4. **조치: stale-on-change 구현·시연, E3 orchestration UI에 노출** | prose만이면 평범한 원클릭 통합으로 보임 |
| auth-less 엔진 위 RBAC trust boundary를 구조 차별로(okc-web가 게이팅 100% 소유, curator_id=미검증 라벨) + code-verified(mutating 호출은 okc-core 호출 前 403) | User Stories→Code Gen | 🟡 | E1-S3('okc-core NOT called at all'), E1-S4 + C-1. **조치: pre-core RBAC 게이트 구현 + 데모 403; okc-core엔 auth 없음을 차별성 문서에 명시** | 'permission-managed'가 일반 로그인으로 붕괴(요구 (1)+(2)) |
| adapter-not-policy-owner(ADR-0002) + typed-DTO 직접 링크(interop schema v2, CLI shell-out/FFI round-trip 아님)를 의도적 구조 선택으로 문서화 + 스키마 버전 검증 | Requirements/NFR Design→Code Gen | 🟡 | Q8/C-7 + capability §4 + E4-S2 AC. **조치: 직접-링크 백엔드 구현 + schema-version-reject 유지, 차별성 문서에 명시** | shell-out wrapper면 일반 프런트로 보여 미인정 |
| provenance + verify()/explain()를 serving 차별로: per-file ProvenanceRecord + verify=내부일관성 증명(발행자 진위 아님)임을 화면에 명시, provenance 버리는 RAG 대비 | User Stories→App Design(E5 'wow')→Code Gen | 🟡 | E5-S3+NFR-DET-1. state에 focal 지정. **조치: provenance/verify 뷰 구축, .okc/ 감사 envelope 노출, 내부일관성-not-진위 명시** | prose만이면 auditable 차별 불가시, C3·C5 고가치 화면 상실 |
| okc-mcp serving CONTRACT + re-embedding 경계(okc-core 임베딩 ephemeral → consumer가 re-embed)를 end-to-end RAG 번들 도구와 구별되는 구조 경계로 문서화 | User Stories→App Design/Code Gen | 🟡 | E5-S4 + capability §5-10 + FR-SRV-3. **조치: read-only discovery/contract 엔드포인트 구현 + re-embed 요구 문서화; 'source만 serve, RAG는 별 tier'를 의도 구조로** | 프레이밍 없으면 미완성 제품으로 읽힘 |
| 구조적 차별점들이 **screenshots/ 또는 result/** 캡처에 가시(주장 아닌 SEE) | Build & Test | ⏳ | screenshots/·result/·코드 없음. **조치: E4 review·E5 provenance·403 게이트 캡처, README 차별성 주장과 매칭** | code/design 확인 요구 → 데모 없으면 일반 upload+merge 웹앱 취급 |
| 차별성 추적 스레드: requirements→stories→design→code로 각 구조 주장이 산출물/코드로 확정(C-1..C-7 → 특정 story/screen/module) | Application Design(통합)→cross-stage | 🔴 | C-1..C-7이 개별 threading되나 구조 차별점↔하위 산출물 단일 뷰 없음. **조치: differentiator→artifact→(future)module 행렬 추가** | 흩어진 제약은 '설계됨' 미인식 → 'claimed'로 강등 |
| 차별성을 실제 BUILT 위에 앵커(미구현 RAG/okc-mcp 과대주장 금지) | Requirements→App Design/README | ✅ | R-2, §6, E5 out-of-scope, personas P3가 okc-mcp/RAG 유예 + re-embed 요구. **유지: README/전시 차별성을 built compiler+approval+provenance+RBAC serving에 한정** | RAG에 기대면 미검증 → C3 0점 + C4 손상 |

### C4 — 실제 동작 / 구현 완성도

**요지**: 전 구간 INCEPTION(코드/screenshots/result/CI/lockfile 전무). C4는 근본적으로 **실행되는 코드 + 필수 시연 스크린샷**이라 대부분 정당히 'future'이나, §9 end-to-end가 22 Must 스토리로 커버되고 오류/게이트/serving이 테스트가능 AC로 명세된 groundwork는 이례적으로 강함.

| 점검 | 단계 | 상태 | 근거·조치 | 미준수 리스크 |
|------|------|------|-----------|---------------|
| **repo 루트에 screenshots/ 또는 result/ 존재 + 실제 실행 화면 캡처(목업/Figma 아님)** | Build & Test | 🔴 | 둘 다 없고 캡처 스케줄 없음. **조치: Code Gen 후 앱 실행·실제 캡처 커밋, 'demo screenshots 커밋'을 Build&Test exit 산출물로 + Workflow Planning에 명시** | 시연 스크린샷은 하드 MUST → 코드 품질 무관 C4 즉시 실패 |
| 스크린샷이 두 focal flow 커버: E4 conflict/critic review(severity 배지·Minor waive vs un-waivable regenerate·APPROVAL_REQUIRED 차단·보존된 contradiction) + E2 토큰 업로드(토큰 발급→업로드→accept/reject 피드백) | App Design + Build & Test | 🔴 | §9 map이 demo-critical로 표시하나 화면 없음. **조치: App Design이 정확히 이 화면 생성, Build&Test가 캡처** | E4는 데모 중심 + 구조 차별점; 미시연 시 C4·C3 최강 증거 상실 |
| 백엔드 진입점 존재·부팅: main()+router+bind 주소 있는 axum 바이너리 크레이트 | Code Generation | ⏳ | .rs/Cargo.toml 전무. 스택=Rust+axum+okc-interop(Q8). **조치: Code Gen에서 서비스 바이너리 생성** | 부팅 경로 없으면 판정자 실행 불가, C4 상한 낮음 |
| 프런트 진입점 실행: Next.js가 build·serve(next build/dev) + 데모 화면 라우트 | Code Generation | ⏳ | package.json/*.tsx 없음. 스택=Next.js+Tailwind+shadcn/Tremor. **조치: 라우팅된 화면 생성** | 실행 UI 없으면 필수 스크린샷 불가 |
| **양 스택 deps 선언 + lockfile 커밋**: Cargo.toml+Cargo.lock, package.json+JS lockfile | Code Generation | ⏳ | 전무. **조치: 매니페스트+lockfile 커밋** | C4가 'lockfile' 명시 → 없으면 재현 불가 직접 감점 |
| okc-core를 특정 commit에 pin + pinned 빌드 실제 사용(바인딩 미배포 → source 빌드) | Infrastructure + Build & Test | 🟡 | NFR-PORT-1가 의도만 진술, pin 매니페스트/CI 없음. **조치: commit pin(submodule/rev) + source 빌드 실체화** | interop 직접 링크라 unpinned/unbuildable okc-core = 빌드 실패 |
| 양 스택(Rust 백엔드 + Next.js 프런트)을 build·test하는 CI 설정 파일 | Build & Test | 🔴 | CI 파일 없음(NFR-PORT-1 의도만). **조치: CI 워크플로를 Build&Test exit 산출물로 추가** | CI는 명시적 C4 서브체크 → 없으면 빌드 건전성 미검증 신호 |
| 백엔드 핸들러가 okc-interop 링크 + **실제** okc-core 함수 호출(add_source, approve_taxonomy, approve_cluster, regenerate_cluster, compile, verify, explain) — mock/하드코딩 응답 아님 | Code Generation | ⏳ | typed-DTO 경로 명세(§0 C-5/C-7, E4-S2)되나 코드 없음. **조치: 각 핸들러를 interop에 wiring, 호출이 엔진 도달 확인(fake 아님)** | okc-core가 fake면 플랫폼 존재 이유가 스텁 |
| §9 end-to-end가 하나의 연결 flow로 코드 실행: 토큰 업로드→admin login/create-project/freeze→run 루프(provider+remote-disclosure consent)→taxonomy approve→cluster synthesis/critic→compile(no-clobber)→read-only serve | Build & Test | ⏳ | §9 + §9 map(22 Must) 완전 명세. **조치: Build&Test에서 전 체인 실행 시연** | 어느 한 곳 단절 시 미완성 구현으로 읽힘 |
| RBAC 게이트가 **실행 중** 미들웨어로 강제: contributor/미인증의 모든 mutating op가 okc-core 호출 前 403/401 | Code Generation | ⏳ | E1-S3 + C-1(코어 auth 없음). **조치: 실제 미들웨어 게이트 구현** | 요구 (2)='admin만 통합'이 okc-web 단독 책임 → 비작동 시 헤드라인 기능 미동작 + C6 감점 |
| compile-거부 게이트(E4-S6)가 실제 강제 코드 경로: 미해결 Major/Critical 또는 미승인 cluster로 compile 시 APPROVAL_REQUIRED류 코드 + 미해결 항목 리스트 | Code Generation | ⏳ | E4-S6 AC + FR-INT-5. **조치: 서버 게이트로 강제(UI copy만 아님)** | 요구 (3) human-review 중심 → 스텁이면 마퀴 동작이 가짜 |
| 전역 에러 핸들러 + OkcError{code,category} 분기가 main path 커버: 401/403/404/405/APPROVAL_REQUIRED HTTP 매핑, PROJECT_BUSY=retry 액션, message-string 파싱 의존 금지 | NFR Design + Code Generation | 🟡 | 에러 계약 명세(FR-INT-8, §0, E3-S7)되나 전역 핸들러 미설계/미구현. **조치: NFR Design이 핸들러 형태 명세, Code Gen 구현** | 미처리 panic/문자열 분기 = 감점 |
| read-only serving이 실제 compiled-vault 내용 반환: knowledge/+legacy/+.okc/ list+body, write verb=405, path-escape/nonexistent=404; verify()/explain()=구조화 integrity+provenance | Code Generation | ⏳ | E5-S2/S3 AC. **조치: 실제 compiled 디렉터리 serving 엔드포인트 + 실제 compiled-vault fixture** | 요구 (5) serving 계약 비작동 시 actually-works + okc-mcp 차별 약화 |
| README가 스크린샷이 시연하는 기능과 정확히 일치(과대주장 없음), okc-mcp RAG=contract-only/deferred 명시 | Build & Test | 🔴 | README 없음. **조치: 시연 화면=기능 리스트인 README 작성 + okc-mcp contract-only 명시** | 스크린샷↔README 정합 서브체크 위반 = 직접 감점 |
| 22 Must-story 핵심 경로에 TODO/스텁/빈 함수 없음(특히 E4 approve_cluster/regenerate, E5 verify/explain) | Code Generation + Build & Test | ⏳ | MoSCoW에 22 Must 열거. **조치: 데모 캡처 전 모든 Must 경로 코드리뷰** | 핵심 경로 스텁 = 쉽게 발견되는 감점 |
| Application Design이 구체 화면 인벤토리 + 백엔드 모듈/okc-interop 경계 산출(진입점→실제 구현 경로를 코딩 前 설계) | App Design + Units Generation | 🔴 | state line 8은 'running' 주장하나 디렉터리 없음. **조치: 설계 산출물 랜딩 + state 정합** | design→code 추적성 없으면 C4 완성도 + C1 연결성 붕괴 |

### C5 — 온보딩 / 사용성

**요지**: inception 단계치곤 이례적으로 강한 원재료(§9 시나리오, §9 커버리지 맵, 화면 내 안내 AC — E4 자가설명, 토큰 전용 업로드). 그러나 실제 화면/README/시드계정/상태 매트릭스/screenshots는 전무. C5는 '문서 존재'가 아니라 **'화면 앞에서 다음 행동을 안다'** 로 채점.

| 점검 | 단계 | 상태 | 근거·조치 | 미준수 리스크 |
|------|------|------|-----------|---------------|
| README/quickstart로 양 tier(Rust axum+interop 백엔드 + Next.js 프런트) 설치·실행: deps 선언·lockfile·dev-server 명령·포트 | Code Gen; Build&Test 검증 | ⏳ | 스택 고정되나 코드/README 없음. **조치: Code Gen에서 tier별 install/run README + lockfile 커밋** | '시작 경로' 서브체크 0점 + C4 빌드 견인 |
| 필요한 자격증명/계정 구체 명시: AI embedding/synthesis provider env-var **이름**(UI에 비밀값 금지)과 운영자 서버측 제공법 | Infra/NFR Design; README 표면화 | 🟡 | 메커니즘(A-2, E3-S4)은 있으나 어떤 env var를 세팅해야 하는지 통합 리스트 없음. **조치: Infra Design + README 'needed keys'에 구체 env-var 이름 열거** | provider 설정에서 막힘 → 시작 실패 |
| pinned okc-core fetch/빌드 안내(바인딩 미배포, source 빌드)로 clean machine에서 interop 링크 가능 | Infra Design; Build&Test CI | 🟡 | NFR-PORT-1/C-7이 요구만, 구체 스텝 없음. **조치: commit pin+빌드 명령 문서화 + CI wiring** | 의존성 빌드 불가 → 실행·온보딩 차단 |
| first-run 경로가 작동하는 admin login + contributor가 계정 없이 upload URL+token 획득(시드/데모 데이터 문서화) | Code Gen(시드+문서) | 🔴 | E1-S2 MoSCoW 노트 '시드 대체 가능'만 힌트, 강제 스토리/문서 없음. **조치: 시드 계정 + '첫 upload token 발급' 스텝 문서화** | 빈 시스템에 admin 자격 없음 → 즉시 막힘 |
| launch 후 핵심 시나리오 매뉴얼(upload→login→create→freeze→run→review→compile→serve), 데모 시나리오 미러 | Code Gen(README), App Design 근거 | 🟡 | §9 + §9 map 우수 원재료이나 screen-referenced 실행 매뉴얼 아님. **조치: §9를 스크린샷 포함 단계별 walkthrough로 변환** | requirements의 서사는 usable walkthrough 아님 |
| 트러블슈팅이 OkcError code/category를 구체 사용자 액션에 매핑: 최소 PROJECT_BUSY(retry)·APPROVAL_REQUIRED(compile 차단)·업로드 거부(format/size/path-symlink/non-Markdown) | Functional Design(카탈로그); Code Gen(문서) | 🟡 | 코드는 열거(§0, E3-S7, E4-S6, E2-S4/S6)되나 사용자향 메시지/트러블슈팅 카탈로그 없음. **조치: code→user-message→next-action 카탈로그** | raw 코드만 노출 → 트러블슈팅·피드백 손상 |
| 통합-루프 화면이 현재 IntegrationCheckpoint 렌더 + 지금 필요한 결정만 노출(이후 단계 잠금 = 항상 다음 액션 안내하는 stepper) | App Design→Code Gen | ⏳ | E3-S3 AC 시딩. application-design/ 없음. **조치: checkpoint stepper(NeedsProvider→…→Verified)를 화면으로 명세** | 다단계 human-in-loop 중심; 다음 행동 미표시 시 C5 실패 |
| E4 conflict/critic review 화면이 okc-core 반직관 모델을 화면 내 자가설명: winner 선택 없음(contradiction 보존)·Major/Critical un-waivable→regenerate only·Minor는 필수 rationale로만 waive, severity 배지 | App Design(focal 'wow')→Code Gen | 🟡 | E4-S2/S3/S5가 화면 내 설명 명시적 요구. 화면 설계 없음. **조치: 의미 안내를 레이아웃에 내장한 focal 화면 설계** | 데모 중심 + 최난해 개념; '왜 하나 못 고르나' 안 가르치면 막힘 + C3 상실 |
| 토큰 전용 contributor 업로드 화면이 업로드 前 accepted formats(zip/tar.zst)·size limit·Markdown-only 자가설명(계정 없이 토큰만으로 도착한 사용자) | App Design→Code Gen | 🔴 | E2-S3 token-only 확립하나 제약이 post-hoc 검증(E2-S4)·사후 피드백(E2-S6)으로만 — 업로드 前 안내 요구 스토리 없음. **조치: App Design에 화면 내 'accepted input' 안내 요구 추가** | contributor는 온보딩 맥락 0인 유일 페르소나 → dead-end |
| 소스/설정 변경이 이전 승인 무효화(stale) 시 UI 경고 + 영향받은 checkpoint부터 재승인 안내 | App Design→Code Gen | ⏳ | E3-S5/E5-S5(Should) 시딩. **조치: stale 배너/안내 설계, 데모 명료성 위해 승격 고려** | hash-bound 무효화가 비가시 → admin이 옛 승인 loop, 고장난 것처럼 보임 |
| 장기 통합/compile job에 live progress·loading(job-event polling)으로 수 분 대기가 멈춘 듯 안 보이게 | App Design→Code Gen | ⏳ | E3-S7 + NFR-OBS-1/NFR-AVAIL-1 시딩. **조치: progress/loading 상태 명세** | '클릭 후 무엇이 일어났나' 서브체크; 진행표시 없는 수 분 compile=hang |
| App Design이 화면별 상태 매트릭스(empty/loading/success/error) 정의 — no-projects, no-sources, no-findings, no-compiled-vault, one-time-token-shown, revoked/expired-token | App Design→Code Gen | 🔴 | success/feedback AC는 흩어져 있으나 통합 empty-state/매트릭스 요구 없음. **조치: 화면별 empty/loading/success/error 매트릭스 강제** | 'empty states' 명시 서브체크; fresh install 빈 화면 = 감점 |
| 업로드 후 contributor가 receipt id+token으로 accept/reject-with-reason 조회(no-login 페르소나 피드백 루프 폐쇄) | App Design→Code Gen | 🟡 | E2-S6 완전 명세하나 'Should'. **조치: 설계 + Must 승격 고려(contributor의 유일 '클릭 후' 신호)** | 빠지면 contributor가 void로 업로드 |
| 실제 앱에서 하나의 핵심 시나리오(§9)가 막힘없이 end-to-end 실행되고 시연됨 | Build & Test(실행) + screenshots | ⏳ | §9 + §9 map(coverage_ok=true). 실행 대기. **조치: Build&Test에서 전 경로 실행·dead-end 없음 확인** | 'user-flow completeness' 요구; 중간 정지(consent/blocked compile) 시 C5·C4 실패 |
| okc-mcp consumer(비인간) 온보딩 등가물: consume 위치 + Markdown 레이아웃(knowledge/+legacy/+.okc/) + RAG/re-embedding은 consumer 몫 + read-only만을 자가기술하는 discovery/contract 엔드포인트 + 예제 client 호출 | Functional Design(계약)+Code Gen(예제) | 🟡 | E5-S4 명세, personas P3. 실제 엔드포인트/예제 없음. **조치: discovery 엔드포인트 구현 + 예제 GET(list/body/verify/explain)** | API consumer엔 'API+예제'=온보딩; 예제 없으면 통합 대상 사용 불가(요구 ⑤ 약화) |
| 화면 자체가 사용법 설명 — App Design 스펙에 화면 내 helper/guidance copy 포함 + Build&Test가 자가설명 E4·E5 provenance·contributor 업로드 화면 screenshots(screenshots/ 또는 result/) 캡처 | App Design(copy) + Build&Test(screenshots) | 🔴 | application-design/ 없음, screenshots/·result/ 없음. **조치: 화면 내 안내 copy 포함 화면 랜딩 후 캡처** | 웹서비스 C5는 '화면이 사용법 설명'으로 채점; 스펙·스크린샷 없으면 근거 0(C4 필수 스크린샷도 실패) |

### C6 — 유지보수성

**요지**: 4개 서브체크 중 2개(인증·인가, 시크릿 관리)는 스토리 레벨에서 airtight — okc-core에 RBAC가 0(C-1)이라 서버측 게이팅-before-any-okc-core-call이 okc-web 단독·중앙 책임이고, 비밀번호/토큰 해싱·env-var 이름 참조가 잘 명세됨. 입력검증도 강함(E2-S4). 3대 실질 리스크는 미설계 영역: (1) 모듈/컴포넌트 맵 부재(application-design/ 비어있음), (2) 설정분리는 암시만, (3) **로깅·관측이 가장 약함**(NFR-OBS-1 하나, audit.md도 보강 필요 자인).

| 점검 | 단계 | 상태 | 근거·조치 | 미준수 리스크 |
|------|------|------|-----------|---------------|
| epic-정렬 모듈/컴포넌트 맵: Rust 백엔드↔Next.js 프런트 분리 + 백엔드를 5개 관심사(E1 auth/RBAC·E2 upload+token·E3 orchestration·E4 review·E5 serving)로 분할(단일 monolith 핸들러 아님) | Application Design | 🔴 | requirements §8이 5 seam 열거·E1–E5 매핑 깔끔하나 application-design/ 없음(wf 시작만). **조치: ui-screens.md + E1–E5 keyed 백엔드 모듈/크레이트 맵 산출** | '코드 구조·모듈화' 명시 채점; 맵 없으면 undifferentiated blob |
| 모든 okc-core/interop 호출을 ONE 어댑터 모듈에 격리(typed-DTO interop schema v2 경계 소유, 스키마 버전 검증·미지원은 OkcError 코드로 reject, pinned commit 중앙화) | Application Design | 🟡 | §0 C-5/C-7 + ADR-0002 + E4-S2 + NFR-PORT-1. 격리 설계 없음. **조치: interop-adapter 단일 모듈 + commit pin 단일 위치 지정** | 0.3.0 pre-stable; 호출 누출 시 re-pin이 전역 파급 |
| 인가를 단일 중앙 미들웨어/guard로 강제(모든 okc-core 호출 前), 모든 mutating 엔드포인트(project create, add_source, provider/consent, approve_*, regenerate_cluster, compile, serving publish, token issue/revoke)가 통과 — per-handler ad-hoc 아님 | Application Design | ✅ | FR-AUTH-3 + E1-S3(401/403 AC + mutating-op 전체 리스트) + C-1. 스펙 완전. **조치(Code Gen): scatter 아닌 단일 guard 구현** | 코어가 게이팅 0 → 게이트 안 된 엔드포인트 1개 = 보안+유지보수 함정 |
| 재사용 입력검증 모듈이 적대적 업로드 차단(format allowlist zip/tar.zst, size cap, zip-bomb/expansion guard, path-traversal '../', absolute path, symlink entry, Markdown-only)하고 실패 시 바이트 미착지·add_source 미호출 보장 | Application Design | ✅ | E2-S4 테스트가능 AC. **조치(Code Gen): 업로드 엔드포인트 재사용 단일 검증 모듈, OkcError 코드 reject** | 업로드=적대적 입력면; inline/중복 시 신규 경로에서 우회 |
| 비밀번호=salted one-way hash만, session/upload token=hash/secret-ref만, 토큰 1회 표시·이후 read는 masked — 평문 자격증명 미저장 | NFR Design | ✅ | NFR-SEC-1 + E1-S5 + E2-S1. **조치(Code Gen): 실제 KDF(argon2/bcrypt), 평문 미저장 검증** | '시크릿 관리' 명시; 원시 토큰 저장 시 스펙이 미이행 약속 |
| AI provider 자격증명을 서버측 env-var **이름**으로만 참조; 비밀값 입력/echo/저장 안 함(E3-S4 provider-selection 엔드포인트에서 강제) | Infrastructure Design | ✅ | A-2 + E3-S4 AC. **조치: env-var provisioning + names-only .env.example 커밋, Code Gen이 비밀값 입력 reject** | 하드코딩 provider 키=C6 canonical auto-fail; 키 리터럴 grep이 비어야 함 |
| 설정분리 매니페스트가 모든 배포별 값 외부화 — local absolute landing/output path, source cap(=10), okc-core commit pin, provider env-var 이름, serving root — env/config로, 소스 하드코딩 없음 | Infrastructure Design | 🟡 | 값들이 requirements 전반(FR-UP-4, E3-S6, C-3, NFR-PORT-1, A-2)에 참조되나 외부화 통합 없음. **조치: 단일 config 매니페스트** | '설정 분리' 명시; 하드코딩 절대경로=비이식성, 첫 clone에서 막힘 |
| 구조화 로깅/관측 NFR이 log format·levels·request/job correlation id 정의, 모든 에러 로깅이 OkcError{code,category} keyed(message-string 파싱 금지), PROJECT_BUSY 별도 처리 | NFR Requirements | 🟡 | NFR-OBS-1 하나 + FR-INT-8/E3-S7/PROJECT_BUSY. audit.md도 '보강 필요' 자인. **조치: 1급 구조화-로깅/관측 NFR 추가, NFR Design이 format/levels/correlation 상세** | 명시 서브체크이자 최약 영역; 관측 요구/설계 부재 = 직접 감점 |
| 직렬화 single-engine 통합의 job state 영속화 + 라이프사이클 이벤트 로깅으로, 멈춘/실패 수 분 job을 운영자가 진단 가능(사용자향 progress polling과 별개) | NFR Design | 🟡 | E3-S7(사용자향) + NFR-AVAIL-1/NFR-CONC-1(project.lock)이 접하나 운영자측 미설계. **조치: job-state 영속화 + 이벤트 로깅 명세** | 수 분 human-in-loop; job 관측 없으면 hung job 진단 불가(C6+C5) |
| 큐레이터 결정(taxonomy 승인·cluster 승인·minor waiver·omission rationale·regenerate feedback)을 curator_id·rationale·timestamp와 함께 append-only 영속화(감사 가능 상태변경 이력) | Application Design | 🟡 | E1-S4·E4-S1·E4-S3가 결정 기록 명세하나 append-only store/timestamp 형태 미설계. **조치: 결정-감사 store 정의(audit.md append-only 패턴 미러)** | '누가 언제 무엇을 승인/waive했나' 답 없음 → 과거 통합 감사 불가 |
| 단일 OkcError-code/category → HTTP-status + UI-copy 매핑 모듈(APPROVAL_REQUIRED, PROJECT_BUSY→retry, 401/403/404/405)로 일관 에러 처리, message-string 파싱 없음 | Application Design | 🟡 | FR-INT-8/E3-S7/E4-S2/E4-S6/E5-S2/E5-S3에 규칙 진술되나 단일 모듈 미설계. **조치: code→status/copy 매핑을 단일 테이블/모듈로 산출** | 흩어진 매핑=불일치 응답·취약; C6 구조·모듈화 훼손 |
| 리포지토리에 커밋된 시크릿 없음: .env.example(이름만), 실제 env/자격증명 제외 .gitignore, 의존성 lockfile 커밋(Cargo.lock + package-lock/pnpm-lock)으로 재현·시크릿 위생 검증 가능 | Code Generation | ⏳ | 코드/빌드 없음. A-2·E1-S5·E2-S1이 정책 설정. **조치: .env.example·.gitignore·lockfile scaffold, 데모 전 secret scan** | C6 '시크릿 비하드코딩' + C4 lockfile 동시 실패; 커밋된 키=최악 발견 |

---

## 2. 스테이지별 게이트 체크리스트 (스테이지 순)

각 AI-DLC 스테이지 **완료 게이트**에서 검증할 기준 항목. 완료 메시지·audit.md에 6기준 self-check 요약을 첨부한다(C1 정책).

### Cross-cutting (모든 게이트에서 매번)
- **[C1]** aidlc-state.md 주장 ↔ 디스크 실체 정합 확인. 없는 산출물을 'running/complete'로 두지 말 것(없으면 'launched, not yet landed' 표기).
- **[C1]** 본 문서(`hackathon-judging-criteria.md`)를 실제로 참조하고, 게이트 결정에 반영했는지 audit.md에 기록.
- **[C1]** req-③ conflict-semantics 재검증 상태 확인 — 미해결 동안 requirements/stories/personas에 '확정'으로 서술된 부분과의 모순을 게이트마다 점검.
- **[C1]** 완료 메시지에 6기준 요약(compliant/N/A/gap) 첨부.

### 게이트 1 — Workflow Planning (다음, ALWAYS)
- **[C1·C4·C5]** screenshots/(또는 result/) 캡처, 루트 README, CI 설정, lockfile 커밋을 **Build&Test exit 산출물로 명시 예약** — AI-DLC 기본 흐름은 이들을 생성하지 않으므로 지금 계획에 못박을 것.
- **[C1]** application-design/ 산출물이 실재하도록 Application Design을 계획에 포함 + state line 8/12 dangling ref 해소를 planning 산출물로.
- **[C2]** Requirements 재방문(Problem Statement 추가)을 계획에 포함(WHEN/HOW-OFTEN/status-quo 결손).
- **[C1]** 이후 모든 게이트에서 6기준 self-check를 남기는 규칙을 workflow 계획에 명문화.

### 게이트 2 — Application Design (+ Units Generation)
- **[C1·C4]** ui-screens.md 실재 + **화면→스토리ID(E1–E5) 행렬**, 특히 focal E4(conflict/critic review)·E5(provenance/verify).
- **[C3]** **'기존 도구 대비 구조적 차별성' 섹션**(named 대안: Obsidian Publish/Sync, Notion/Confluence, LlamaIndex/Mem0, git-merge) + **differentiator→artifact→(future)module 행렬**.
- **[C2]** 랜딩/대시보드/빈 화면에 문제 프레이밍 copy 요구.
- **[C5]** checkpoint stepper(다음 액션만 노출), E4 자가설명(승자 선택 없음·un-waivable regenerate·Minor waive+rationale), 토큰 전용 업로드 화면의 **업로드 前** format/size/Markdown 안내, stale 경고, 장기 job progress/loading, **화면별 empty/loading/success/error 상태 매트릭스**, contributor 결과 조회(E2-S6 승격 고려), 화면 내 helper copy.
- **[C6]** epic-정렬 백엔드 **모듈 맵**(격리된 okc-interop 어댑터 + 단일 authz guard + 단일 OkcError→HTTP 매핑 모듈 + 재사용 입력검증 모듈 + curator-decision append-only store).
- **[C1]** UI 방향(clean/minimal, E4·E5 focal 'wow') 결정을 산출물에 인코딩.

### 게이트 3 — Functional Design (per-unit)
- **[C5]** OkcError code→user-message→next-action **트러블슈팅 카탈로그**(PROJECT_BUSY retry, APPROVAL_REQUIRED, 업로드 거부 사유).
- **[C5]** okc-mcp discovery/contract 내용(consume 위치·Markdown 레이아웃·re-embed는 consumer 몫·read-only) 명세.
- **[C6]** curator-decision append-only store 형태(curator_id·rationale·timestamp) 확정.

### 게이트 4 — NFR Requirements & NFR Design (per-unit)
- **[C6]** **1급 구조화-로깅/관측 NFR 추가**(format·levels·request/job correlation id, OkcError{code,category} keyed) — 현재 최약 영역.
- **[C6]** job-state 영속화 + 라이프사이클 이벤트 로깅(운영자 진단).
- **[C6]** **설정분리 매니페스트**(landing/output path, cap=10, commit pin, provider env-var 이름, serving root).
- **[C4]** 전역 에러 핸들러 + OkcError 분기 형태 명세(401/403/404/405/APPROVAL_REQUIRED, PROJECT_BUSY=retry, message-string 파싱 금지).
- **[C3·C6]** typed-DTO 직접-링크 결정 + 스키마 버전 검증 반영.
- **[C6]** 비밀번호/토큰 해싱(실제 KDF) 설계 확정.

### 게이트 5 — Infrastructure Design (per-unit)
- **[C4·C5]** okc-core **commit pin + source 빌드** 스텝 문서화(바인딩 미배포) + clean-machine 재현.
- **[C5·C6]** provider **env-var 이름 리스트** + names-only `.env.example` + 서버측 provisioning.
- **[C6]** 설정분리 매니페스트 배포 세부(env/config 파일) 확정.

### 게이트 6 — Code Generation (per-unit, ALWAYS)
- **[C4]** 백엔드 axum 진입점(main+router+bind) + 프런트 Next.js 진입점(라우트된 화면).
- **[C4]** deps 선언 + **lockfile 커밋**(Cargo.lock + JS lockfile).
- **[C4·C3]** 핸들러가 **실제 okc-interop** 호출(mock 아님): add_source/approve_taxonomy/approve_cluster/regenerate_cluster/compile/verify/explain.
- **[C4·C6]** **RBAC 게이트를 실행 미들웨어로**(okc-core 호출 前 403/401), 단일 guard.
- **[C4·C3]** **compile-거부 게이트**(APPROVAL_REQUIRED, 미해결 항목 리스트) 서버 강제.
- **[C4]** read-only serving이 실제 compiled-vault(knowledge/+legacy/+.okc/) 반환, write=405, escape=404, verify/explain 구조화.
- **[C4]** 전역 에러 핸들러 구현.
- **[C1]** 각 모듈/유닛에 구현하는 story ID/FR 태깅(commit/PR 본문 인용).
- **[C5]** README quickstart(tier별 install/run·포트), 시드 계정 + 첫 토큰 발급 문서, §9 walkthrough, discovery 엔드포인트 예제 호출.
- **[C6]** `.env.example`(이름만)·`.gitignore`·비밀값 입력 reject, 재사용 검증/에러-매핑 모듈 구현.
- **[C4]** 22 Must 핵심 경로 스텁/TODO 없음(E4 approve_cluster/regenerate, E5 verify/explain 특히).

### 게이트 7 — Build & Test (ALWAYS, 최종)
- **[C4]** **screenshots/ 또는 result/ 디렉터리 + 실제 실행 화면 캡처**(필수 MUST) — 최소: E4 review, E2 token upload, E5 provenance/verify, 403 게이트.
- **[C4]** 양 스택 build·test **CI 설정 파일**.
- **[C4]** okc-core pinned 빌드 실제 사용 + **§9 end-to-end** 전 체인 실행 시연.
- **[C4·C2]** **README**: 기능 리스트 = 시연 화면(과대주장 없음), okc-mcp=contract-only 명시, **문제 섹션**(Requirements Problem Statement verbatim).
- **[C1]** repo 루트 **AI-DLC PROCESS 서사**(capability→Q&A→req→stories→design→code 링크).
- **[C3]** 스크린샷이 구조적 차별점(contradiction preservation·un-waivable·freeze-then-run·provenance·RBAC-before-core)을 실제로 보여줌.
- **[C5]** 하나의 핵심 시나리오 막힘없이 end-to-end + 자가설명 화면 스크린샷.
- **[C2·60%]** **전시 페이지**에 문제 서술 + 데모 미러.
- **[C6]** 커밋된 시크릿 없음(secret scan), lockfile 커밋 확인.

---

## 3. 우선순위 조치 (즉시 → 다음 스테이지, 영향순)

### 즉시 (Cross-cutting / 다음 게이트 전)
1. **본 `hackathon-judging-criteria.md` 실체화**(현재 작업) + **aidlc-state.md line 8/12 dangling ref 해소** — application-design/ 산출물을 랜딩하거나 state 주장을 'launched, not yet landed'로 하향. (C1 신뢰도 직격 — 판정자가 state=있다는데 repo=없음을 보면 날조로 읽힘.)
2. **req-③ conflict-semantics 재검증 종결 후 lockstep 전파** — requirements §4.3 / stories E4 / personas P1 / 양 추적행렬 동시 갱신 + audit 항목. (교차-산출물 모순 제거; C1·C3 focal 'wow'의 전제.)
3. **Workflow Planning 게이트에서 screenshots/·README·CI·lockfile을 Build&Test exit 산출물로 명시 예약.** (순수 AI-DLC가 놓치는 해커톤 필수; 지금 안 박으면 끝에서 누락.)

### 다음 스테이지(Application Design 중심)
4. **Requirements에 독립 Problem Statement 추가**(WHOSE/WHAT/**WHEN**/**HOW-OFTEN**/status-quo 고통/how-solved) → README·전시 페이지에 verbatim 재사용. (C2의 명시 감점 3종 + 40%·60% 표면화.)
5. **Application Design: ui-screens.md + 화면→스토리ID 행렬 + focal E4(자가설명, 승자 선택 없음)·E5(provenance) 화면 + 화면별 empty/loading/success/error 상태 매트릭스 + 화면 내 문제·안내 copy.** (C1·C4·C5 동시 — 데모 중심이자 최난해 개념.)
6. **Application Design: 'named 도구 대비 구조적 차별성' 섹션 + differentiator→artifact→module 행렬** → README 미러. (C3의 '주장만=0점' 방지.)
7. **Application Design: epic-정렬 백엔드 모듈 맵** — 격리 okc-interop 어댑터 + 단일 authz guard + 단일 OkcError→HTTP 매핑 + 재사용 입력검증 + curator-decision append-only store. (C6 구조·모듈화; 강력한 RBAC/시크릿 스펙이 착지할 구조.)

### NFR / Infrastructure
8. **NFR: 1급 구조화-로깅/관측 NFR 추가**(correlation id, code/category keyed) + job-state 영속화 + **설정분리 매니페스트**(paths/cap/commit-pin/provider env 이름). (C6 최약 영역 보강.)
9. **Infrastructure: okc-core commit pin + source 빌드 스텝 + env-var 이름(.env.example names-only)** 문서화. (C4 재현 빌드 + C5 시작 경로.)

### Code Generation / Build & Test
10. **Code Gen: 핸들러를 실제 okc-interop에 wiring**(mock 금지)로 §9 flow 구동, **RBAC-before-core 403 + APPROVAL_REQUIRED compile 게이트** 강제, `.gitignore`·`.env.example`·lockfile 커밋, 22 Must 경로 스텁 없음. (C4 핵심 + C6 시크릿/인가.)
11. **Build & Test: §9 end-to-end 실행 + 실제 screenshots(E4·E2·E5·403) 캡처 → screenshots/**, README(기능=스크린샷, okc-mcp contract-only) + AI-DLC PROCESS 서사, secret scan. (C4 필수 스크린샷·C1 판정자 가독 서사·C2 README 문제.)