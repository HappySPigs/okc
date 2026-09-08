# 이 저장소의 AI-DLC 설정

이 프로젝트는 **AI-DLC v1.0.1**(클래식, 규칙 기반 배포판)을 **Claude Code**용으로
사용합니다. `awslabs/aidlc-workflows`의 `v1.0.1` 태그를 기준으로 하며,
공식 설치 방식 중 **Option 1(프로젝트 루트)**를 따랐습니다.

- 진입점: 루트의 `CLAUDE.md` (= v1 `aws-aidlc-rules/core-workflow.md`와 동일)
- 규칙 세부: `.aidlc-rule-details/` (v1 `aws-aidlc-rule-details/*`)
- [공식 저장소](https://github.com/awslabs/aidlc-workflows/tree/v1.0.1)

v1은 **프롬프트/규칙 기반**입니다. 실행 엔진·TypeScript 도구·훅·상태 파일 도구가
없으며, Claude Code가 세션 시작 시 `CLAUDE.md`를 읽고 필요할 때
`.aidlc-rule-details/`의 세부 규칙을 로드해 워크플로우를 진행합니다. 상태·산출물은
클래식 관례대로 `aidlc-docs/`에 기록합니다.

## 사용 방법

1. Claude Code를 이 프로젝트 디렉터리에서 시작(또는 **완전히 재시작**)합니다.
   `CLAUDE.md`가 바뀌었으므로 새 규칙을 반영하려면 재시작이 필요합니다(`/clear`로는 부족).
2. `/config`로 활성 설정을 확인하고, "현재 이 프로젝트에서 활성화된 지침이 무엇이냐"고
   물어 규칙 로딩을 검증합니다.
3. 제품 작업은 아래 **제품 게이트**를 지킨 상태에서 요청합니다.

## 2.7.1 하네스에서 v1.0.1로 전환한 기록 (2026-09-08)

- 2026-09-06 ~ 09-07: 이 저장소에는 처음에 `awslabs/aidlc-workflows` **2.7.1**
  하네스 배포판(Codex + Claude Code 공존)이 설치되어 있었습니다. `.codex/`,
  `.agents/`, `.claude/`(TypeScript 엔진·훅·스킬), `AGENTS.md`, `aidlc/spaces/`
  워크스페이스로 구성된 버전입니다.
- 2026-09-08: 사용자의 지시("v1.0.1 태그로 설치해라", "이제 2.0은 안 쓴다")에 따라
  2.7.1 하네스를 제거하고 v1.0.1 클래식 규칙 배포판으로 전환했습니다.
  - **제거**: `.claude/`, `.codex/`, `.agents/`, `AGENTS.md`, `aidlc/`(2.7.1
    워크스페이스·워크플로우 기록·CodeKB·감사 로그), `docs/aidlc-upstream/`.
  - **설치**: `CLAUDE.md`(v1 core-workflow.md), `.aidlc-rule-details/`.
- **복구 가능성**: 제거 직전의 2.7.1 전체 상태는 체크포인트 커밋
  `5c89366` ("checkpoint: preserve AI-DLC 2.7.1 harness state before v1.0.1 switch")에
  그대로 보존되어 있습니다. 필요하면 그 커밋에서 되살릴 수 있습니다.
- 2.7.1 워크플로우가 진행 중이던 `okc-vault-mcp` 인텐트의 이데이션·리버스
  엔지니어링 산출물도 위 체크포인트에 포함됩니다. v1에서 참고가 필요하면 거기서
  꺼내 `aidlc-docs/`로 옮길 수 있습니다.

## 제품 게이트 — 유효

제품은 여전히 **Inception 요구사항 검토 대기** 상태입니다. 프레임워크 설치·전환은
요구사항 승인이나 Construction 진입 인가가 아닙니다. `src/`, `tests/`, 패키지 설정은
미승인 초안입니다. 사용자가 요구사항을 검토하고 Construction 전환을 인가하기 전에는
제품 구현·테스트·패키징을 재개하지 않습니다. 승인 이벤트를 임의로 만들어내지 않습니다.

## 재설치·업데이트

1. `awslabs/aidlc-workflows`의 원하는 릴리스(현재 `v1.0.1`)를 프로젝트 밖에 받습니다.
2. `aws-aidlc-rules/core-workflow.md`를 `./CLAUDE.md`로 복사하고,
   `aws-aidlc-rule-details/*`를 `.aidlc-rule-details/`로 복사합니다.
3. `aidlc-docs/`의 기존 기록과 `src/`·`tests/` 제품 초안을 덮어쓰지 않습니다.
4. Claude Code를 재시작해 새 `CLAUDE.md`를 로드합니다.
