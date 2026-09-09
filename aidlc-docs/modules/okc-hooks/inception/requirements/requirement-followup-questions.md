# 요구사항 후속 질문 (Follow-up Questions)

이전 답변을 검토한 결과, `requirements.md`를 쓰기 전에 **딱 4가지**만 더 확정하면 됩니다.
각 `[Answer]:` 태그 뒤에 알파벳을 적어주세요. 다 되면 "완료"라고 알려주세요.

---

## 섹션 A. Q5 재확인 — 원본 업로드 vs 컴파일본 업로드

**배경 (당신의 질문 "컴파일 하는 로직이 있어?"에 대한 코드 기반 답변):**

네, okc-core에는 컴파일 로직이 있습니다 — okc-core 자체가 "Obsidian Vault 컴파일러"입니다 (`crates/okc-core/src/lib.rs:1`). 그런데 그 "컴파일"은 **3단계 파이프라인**이고, 백그라운드 Watcher가 대신하기 어렵습니다:

1. **수집(ingest)** — raw vault → 정규화된 IR(IntegrationCorpus). 결정론적·오프라인. (`corpus.rs:42`)
2. **integrate(AI 단계)** — LLM + 임베딩으로 클러스터링/합성/비평 → **사람이 taxonomy와 "모든 클러스터"를 승인** → `ApprovedIntegrationPlan` 봉인. 네트워크·API 키 필요. (okc-app/okc-ai)
3. **compile(산출)** — 승인된 plan → 디스크에 "Compiled Vault" 디렉터리 생성. 결정론적·오프라인. (`integration.rs:1301`)

**Watcher 설계에 결정적인 사실:**

- **컴파일은 봉인된 `ApprovedIntegrationPlan`이 있어야만 돌아갑니다** (`integration.rs:575-582`이 "sealed provider recordings" 없는 plan을 거부). 그 plan은 AI 파이프라인 + **사람의 클러스터별 승인**을 거쳐야만 생성됩니다. → **무인(unattended) 백그라운드 Watcher로는 컴파일 산출물을 만들 수 없습니다.**
- 로컬 컴파일은 **프라이버시 이점이 없습니다**: 컴파일 산출물의 `.okc/integration-plan.json`에 **원문 전체(민감정보 포함)가 그대로 임베드**됩니다(결정론적 검증용, 최대 512MB `integration.rs:1500`). 즉 컴파일본은 정제/레댁션된 export가 **아닙니다**.
- 컴파일러 산출물은 현재 **Markdown 전용** — 첨부파일/Canvas/Base는 릴리스 블로커라 **컴파일 업로드 시 원본 콘텐츠가 누락**됩니다 (`PROJECT_CONTEXT.md:76-78`).
- 설계 의도상 어댑터(웹서비스·Watcher)는 "표준 상태·승인 권한·컴파일 정책을 소유하지 않는다" (`PROJECT_CONTEXT.md:45-48`) → **컴파일은 서버(아직 미구축) 몫**입니다.

### Question A1
위 내용을 바탕으로, Watcher가 업로드하는 대상을 확정합니다.

A) **원본(raw) vault 업로드** — Watcher는 raw vault(가능하면 봉인된 VaultSnapshot)만 서버로 보내고, integrate+승인+compile은 서버가 담당 *(권장 — 코드 근거상 유일하게 무인 자동화와 양립)*

B) **로컬 컴파일본 업로드** — (주의: Watcher 머신에서 AI 파이프라인 + 사람 승인을 먼저 수행해야만 가능. 무인 자동 동기화와 충돌하며, 프라이버시 이점 없음, 비-Markdown 누락)

C) **둘 다 지원** — 기본은 원본(A), "컴파일본 발행(publish compiled vault)"은 Watcher가 이미 full OKC 노드로서 plan을 승인·생성한 경우의 **별도 opt-in 고급 기능**으로만

X) Other (please describe after [Answer]: tag below)

[Answer]: A

---

## 섹션 B. 복원력(Resiliency) 확장 — 로컬 Watcher에 해당하는 항목만

Resiliency 확장(Q2=A)을 켜셨습니다. 다만 이 확장의 규칙 대부분은 **클라우드 프로덕션 워크로드**(멀티존/멀티리전, 오토스케일, DR 전략, 서버 백업/장애조치, 리전 토폴로지)를 위한 것이라 **로컬 설치형 Watcher에는 해당되지 않습니다.** 아래 규칙들은 **N/A로 표시**하고 (필요 시) 미구축 웹서비스 설계 때 다루겠습니다:

> **N/A (미구축 웹서비스 몫 / 로컬 도구 무관)**: RESILIENCY-07(리질리언스 모니터링·알람), 08(멀티존/리전), 09(오토스케일), 11(DR 전략), 12(서버 데이터 백업/복제), 13(장애조치 런북).
> **요구사항에 자동 반영(질문 불필요)**: RESILIENCY-05(로컬 구조화 로깅), 06(Watcher 상태/헬스 표시), 10(업로드 호출 타임아웃+지수 백오프 — 이미 Q12=A에서 선택), 14(리질리언스 테스트는 NFR Design/Operations로 연기).

로컬 Watcher에 **실제로 결정이 필요한** 3가지만 여쭙니다.

### Question B1 (RESILIENCY-02, 로컬 맥락으로 재구성)
**재시작/크래시 복구 — 미전송 변경의 데이터 손실 허용치.** (Q6에서 파일이벤트 감시(B)를 고르셨는데, 프로세스가 죽어있는 동안 발생한 변경은 이벤트를 놓칠 수 있습니다. 재시작 시 이를 어떻게 보장할까요?)

A) **디스크 영속 큐 + 재시작 시 대조 스캔** — 미전송분을 영속 저장하고, 시작할 때 vault 전체 해시를 마지막 업로드 매니페스트와 대조해 놓친 변경까지 재감지 (손실 0 목표) *(권장)*

B) **대조 스캔만** — 영속 큐 없이, 시작 시(및 주기적) 전체 재해시로 놓친 변경을 따라잡음 (단순, 대신 다음 스캔 전까지는 미전송 상태로 남는 창이 존재)

X) Other (please describe after [Answer]: tag below)

[Answer]: A

### Question B2 (RESILIENCY-03/04, 배포·롤백)
**설치형 Watcher의 업데이트·롤백** 방식은?

A) **자동 업데이트 + 실패 시 이전 버전 자동 롤백** (사용자 개입 최소)

B) **수동 업데이트** (사용자가 새 버전 설치, 롤백은 이전 설치본 재설치)

C) **지금 정하지 않고 Operations 단계로 연기** — 요구사항엔 "업데이트/롤백 메커니즘 필요"만 명시 *(권장: 배포 인프라·웹서비스 미정)*

X) Other (please describe after [Answer]: tag below)

[Answer]: A

### Question B3 (RESILIENCY-15, 오류 처리·통지)
**업로드 실패/오류 발생 시 사용자에게 알리고 기록하는 방식**은?

A) **로컬 구조화 로그 + 상태 표시(트레이/아이콘) + 심각 오류 능동 알림** *(권장)*

B) **로컬 로그만** (사용자가 직접 확인)

C) Operations 단계로 연기

X) Other (please describe after [Answer]: tag below)

[Answer]: A

---

## (참고) 이전 답변에서 요구사항에 "수용된 위험/의존성"으로 명시할 항목

아래는 추가 질문이 아니라, 당신의 선택을 그대로 반영하되 requirements.md에 **명시적으로 기록**할 내용입니다 (제 전역 지침상 트레이드오프는 숨기지 않고 드러냅니다). 다르게 원하시면 위 X) Other나 답변 옆에 적어주세요.

- **Q1=B (보안 확장 미적용) + Q10=D (민감정보 미검사) + 원본 업로드**: okc-core가 방어하는 "소스 유출" 위협을 그대로 수용하는 선택입니다. 다만 okc-core의 민감정보 차단은 **원격 AI 제공자에게 보낼 때(integrate 단계, 서버측)** 적용되는 것이라, **업로드 자체를 막지는 않습니다.** → "명시적으로 수용된 위험"으로 기록.
- **Q9=A (상시 동의)**: okc-core 기본은 "호출마다 새로 받고 저장 안 하는 동의"입니다. 상시 동의는 **아직 안 만든 웹서비스가 동의를 영속·범위화하도록 설계**해야 성립합니다. → 웹서비스에 대한 **다운스트림 요구사항/의존성**으로 기록.

---

*(A1 + B1~B3에 답해주시면, 모순 점검 후 바로 `requirements.md`를 작성합니다.)*
