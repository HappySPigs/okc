<!--
  DRAFT — 병합 시 이 파일 내용을 <monorepo-root>/README.md 로 옮겨라.
  현재 위치(aidlc-docs/integration/)는 모노레포 루트가 아직 없기 때문의 임시 보관용이다.
  갱신 근거: module-integration-guide.md(§1 생태계·§4 레이아웃), aidlc-state.md(현재 스테이지), execution-plan.md(7-유닛).
-->

# OKC — Obsidian Knowledge Compilation (통합 모노레포)

개인·부서별 **Obsidian Vault**를 권한 관리 하에 **하나의 병합 Vault**로 통합하고, 그 결과를 **RAG 소스**로 제공하는 플랫폼. 통합 과정의 모순(conflict)은 승자 선택이 아니라 **관리자 결정 표면**(승인 / minor waive / omission / regenerate)으로 다루고, 모순 자체는 **불변·hash-bound**로 보존한다.

> **이 저장소를 방금 clone 했다면?** 아래 **[Getting Started](#getting-started)**의 "첫 프롬프트"를 AI 코딩 어시스턴트(Claude Code 등)에 그대로 붙여넣어라. 이 프로젝트는 **AI-DLC(AI-Driven Development Lifecycle)** 워크플로로 진행되며, **INCEPTION(설계) 단계까지 완료**된 상태다.

---

## 1. 모듈 생태계

| 모듈 | 언어/런타임 | 역할 | 상태 |
|---|---|---|---|
| **okc-core** | Rust (workspace) | Vault 컴파일 엔진(인제스트·AI 파이프라인·critic/curator 게이트·모순 보존·compile/verify/explain) | 기존(핀된 의존성) |
| **okc-web** | Rust(axum) + Next.js | 플랫폼: 인증/RBAC, 토큰 업로드, 통합 오케스트레이션, 리뷰(결정 표면), 서빙 | **설계 완료(INCEPTION), 코드 미착수** |
| **okc-mcp** | (미정) | 병합 Vault를 RAG 소스로 소비(청킹·임베딩·vector index, MCP 툴) | 미구현(계약만) |
| **obsidian-hook** | TypeScript (Obsidian 플러그인) | 로컬 Vault → okc-web 자동 업로드 | 미구현(계약만) |

**데이터 파이프라인**: Obsidian Vault → hook → okc-web 업로드 → okc-core `add_source` → 통합/리뷰/`compile` → okc-web 서빙 → okc-mcp RAG.

계약·경계·모노레포 레이아웃 상세: [`okc-web/aidlc-docs/integration/module-integration-guide.md`](okc-web/aidlc-docs/integration/module-integration-guide.md)

---

## 2. 지금 이 저장소에 있는 것 / 없는 것

- ✅ **있음**: `okc-web/aidlc-docs/` — AI-DLC INCEPTION 전 산출물(요구사항·스토리·워크플로 계획·Application Design 5종·Units Generation 3종·의사결정 기록·감사 로그). `okc-web/CLAUDE.md` + `okc-web/.aidlc-rule-details/` — 워크플로 규칙.
- ⬜ **없음(다음 단계)**: `okc-web/backend/`·`okc-web/frontend/` 실제 코드 — **CONSTRUCTION 단계에서 생성** 예정. `okc-mcp`·`obsidian-hook` 구현.

즉, **"clone 후 바로 빌드·실행"이 아니라, AI-DLC를 이어받아 CONSTRUCTION(코드 생성)부터 진행**하는 상태다.

---

## 3. Getting Started

### 사전 요구사항
- **Rust** (stable; `okc-core` 핀 커밋과 호환되는 버전)
- **Node.js** LTS + npm (Next.js 프론트)
- (선택) **Obsidian** — hook 데모용
- `okc-core`를 **서브모듈/vendored**로 편입한 경우: `git submodule update --init --recursive`

### ▶ 첫 프롬프트 (AI-DLC 이어받기 — 권장)

AI 어시스턴트를 **`okc-web/`** 작업 디렉터리에서 열고, 아래를 그대로 붙여넣어라:

```text
이 저장소는 AI-DLC 워크플로로 진행 중인 기존 프로젝트다(신규 아님). INCEPTION 단계가 이미 완료돼 있다.

[먼저 읽어라]
- okc-web/CLAUDE.md (워크플로 규칙)
- okc-web/aidlc-docs/aidlc-state.md (현재 진행 상태 = 진실의 원천)
- common/session-continuity.md (세션 이어받기 지침)

[절대 하지 말 것]
- 웰컴 메시지 출력
- 요구사항/스토리/설계를 처음부터 다시 만드는 INCEPTION 재시작
- aidlc-state.md · audit.md 덮어쓰기(반드시 append-only)

[할 것]
aidlc-state.md의 Current Stage가 "INCEPTION 완료 → CONSTRUCTION 준비"임을 확인한 뒤,
CLAUDE.md의 CONSTRUCTION Phase(Per-Unit Loop)를 U0부터 시작하라.
각 유닛은 execution-plan.md의 depth 결정(U3/U4=comprehensive · U1/U2=standard · U5/U6=Functional Design skip)과
application-design.md / unit-of-work*.md를 근거로 진행하고, 각 스테이지 완료 게이트에서 내 승인을 받아라.
막히면 추측하지 말고 실제 파일/에러를 보여준 뒤 최소 변경만 제안하라.
```

**짧은 버전** (AI-DLC 자동 resume 신뢰):
```text
okc-web/aidlc-docs/aidlc-state.md를 읽고 AI-DLC를 저장된 상태에서 이어서 진행해줘.
INCEPTION은 완료됐으니 재시작하지 말고 CONSTRUCTION(U0)부터 가자.
```

### CONSTRUCTION 완료 후(코드가 생성된 뒤) 빌드·실행
> 아래는 CONSTRUCTION이 끝나 `backend/`·`frontend/`가 생긴 이후에 유효하다.
1. 환경설정: `cp .env.example .env` 후 빈 값(자격증명) 채우기 — **`.env`는 커밋 금지**.
2. okc-core: 핀 커밋 소스 빌드.
3. 백엔드: `cd okc-web/backend && cargo run`.
4. 프론트: `cd okc-web/frontend && npm install && npm run dev`.
5. 데모 스파인: 관리자 로그인 → 업로드 토큰 발급 → `.md` 업로드 → 통합/리뷰 → 서빙 API 확인.

---

## 4. 유닛 구성 (CONSTRUCTION 순서)

`U0` Foundation(adapter/큐·shared) → `U1` Auth → `U2` Upload → `U3` Orchestration → `U4` Review → `U5` Serving, **`U6` Frontend 인터리브**. 전 유닛은 U0에 의존(단방향 DAG). 상세: [`unit-of-work.md`](okc-web/aidlc-docs/inception/application-design/unit-of-work.md) · [`unit-of-work-dependency.md`](okc-web/aidlc-docs/inception/application-design/unit-of-work-dependency.md) · [`unit-of-work-story-map.md`](okc-web/aidlc-docs/inception/application-design/unit-of-work-story-map.md).

---

## 5. 설정·시크릿 위생

- provider 자격증명은 **env-var 이름으로만** 참조(값 커밋 금지). 루트 [`.env.example`](.env.example)에 **이름만** 둔다.
- 비밀번호·세션·업로드 토큰은 **해시 저장**(평문 저장 금지). okc-core는 호출자를 인증하지 않음 → 인증/인가는 100% okc-web.
- okc-core는 **수정 금지**(ADR-0002), 특정 commit에 **핀** 후 CI에서 소스 빌드.

---

## 6. AI-DLC PROCESS narrative (심사 참고)

이 프로젝트는 요구사항 → 스토리 → 워크플로 계획 → Application Design → Units Generation으로 **각 단계 결정이 다음 단계에 그대로 이어지도록** 진행됐다(추적성). 전 과정 기록: [`okc-web/aidlc-docs/audit.md`](okc-web/aidlc-docs/audit.md), 상태: [`aidlc-state.md`](okc-web/aidlc-docs/aidlc-state.md).
