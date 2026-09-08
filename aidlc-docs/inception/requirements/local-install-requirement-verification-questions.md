# Local Config-Driven Install — 요구사항 확인 질문

> **응답 방식**: 사용자가 2026-09-09에 "어지간하면 권장안으로, 설계·구현 범위가 너무 커지는 선택은 우선순위를 낮추고, autopilot으로 진행"을 지시함. 아래 `[Answer]:` 는 그 지침에 따라 **AI가 대리 결정**한 값이며, 각 답 아래 근거를 1줄로 남김. 사용자는 언제든 특정 답을 바꿔 재지시할 수 있음.

이 초기 이니셔티브의 목표: **okc-hooks**와 **okc-mcp** 각각에 "값만 채우면 되는 config 파일"을 두고, 채운 뒤에는 각 모듈이 **로컬에 스스로 설치**되도록 하는 것입니다. (okc-hooks → 자동 실행 데몬, okc-mcp → 사용하는 coding agent에 전역 등록, 어떤 agent인지도 config로 지정)

## 현재 구현 상태 (참고용, 조사 결과 요약)

- **okc-hooks**: JSON config(스키마 정의됨) + `install`/`uninstall` 명령으로 OS 서비스(launchd/systemd/Windows SCM) 등록·자동시작이 **이미 동작**합니다. 다만 config를 **생성하거나 토큰을 넣어주는 명령이 없어**, config JSON이 `/etc/okc-watcher/config.json`(root 권한) 또는 `OKC_WATCHER_CONFIG` 경로에 **미리 존재**해야 합니다. okc-web 연동은 `Authorization: Bearer <upload-token>` 으로 이미 구현되어 있습니다.
- **okc-mcp**: JSON config + okc-web serving 조회(`Bearer <read-token>`)는 구현됨. 하지만 `client-config`는 등록 스니펫을 **출력만** 하고 어떤 agent 설정 파일에도 **쓰지 않습니다**. "어떤 coding agent를 쓰는지"라는 개념/필드도 아직 없습니다. npm 미배포(로컬 alpha).
- **okc-web**: 토큰은 **관리자가 콘솔에서 발급**하며 발급 시 **1회만 노출**됩니다. 종류가 둘로 나뉩니다 — hooks용 **upload token**, mcp(비공개 프로젝트)용 **serving read token**. 공개(public) 프로젝트는 mcp에 토큰이 불필요합니다.

---

## Question 1
config 파일을 어떻게 구성할까요?

A) 모듈별로 각각 (okc-hooks용 1개 + okc-mcp용 1개) — 각 모듈의 기존 스키마를 그대로 사용

B) 루트에 통합 config 1개 → 설치 시 각 모듈 형식으로 자동 분배(공통값인 okc-web 주소/토큰은 한 곳에만)

C) 통합 config 1개를 두 모듈이 직접 읽음(공유 스키마 신설)

X) Other (please describe after [Answer]: tag below)

[Answer]: A
> 근거: 기존 스키마 재사용 = 최소 변경. 통합 config(B/C)는 신규 분배/공유 로더가 필요해 범위 증가. 토큰도 hooks(upload)·mcp(read)가 서로 달라 한 곳에 합칠 실익이 적음.

## Question 2
"값만 채우면 자동으로 설치"를 어떤 방식으로 원하세요?

A) 값을 채운 뒤 **설치 명령을 한 번 실행**(예: `okc-hooks setup`, `okc-mcp setup`) → config 배치 + 데몬 등록/agent 등록까지 한 번에 처리

B) config 파일을 저장하면 **파일을 감시(watch)해 자동 재설치/재등록** (별도 명령 불필요)

C) 저장소 루트의 **스크립트 하나**(예: `./install.sh`)를 실행하면 두 모듈 모두 설치

X) Other (please describe after [Answer]: tag below)

[Answer]: A
> 근거: 한 번의 `setup` 명령이 "값 채움→설치"를 충족하는 최소 구현. 파일 워처(B)는 상주 프로세스가 추가로 필요해 범위 과다. 루트 스크립트(C)는 A 위에 나중에 얇게 얹을 수 있는 선택(우선순위 낮춤).

## Question 3
okc-hooks의 config 파일을 새 설치 흐름에서 어디에 둘까요? (데몬은 이미 user-level launchd/systemd로 실행됩니다)

A) 사용자 레벨(권장, sudo 불필요) — 예: `~/.config/okc-watcher/config.json`

B) 시스템 레벨 유지 — `/etc/okc-watcher/config.json` (sudo 필요)

X) Other (please describe after [Answer]: tag below)

[Answer]: A
> 근거: 데몬이 이미 user-level로 실행되므로 sudo 없는 설치가 자연스러움. discovery 기본경로를 건드리지 않기 위해, `setup`이 생성한 user 경로를 서비스 등록 시 `--config`/`OKC_WATCHER_CONFIG`로 명시 주입(코어 loader 변경 최소화).

## Question 4
okc-hooks 설치 명령의 형태는? (현재 `install`은 OS 서비스 등록만 하고 config는 미리 있어야 함)

A) 채운 config를 **검증 → 발견 경로에 배치 → OS 서비스 등록/자동시작**까지 하는 신규 `setup` 명령 추가(기존 `install`은 유지)

B) 기존 `install`을 확장해 config 생성/배치까지 포함

X) Other (please describe after [Answer]: tag below)

[Answer]: A
> 근거: 신규 `setup`이 내부적으로 기존 `install`의 서비스 등록 로직을 호출 = 기존 동작/테스트 보존(외과적). `install` 확장(B)은 기존 계약을 바꿔 회귀 위험.

## Question 5
okc-mcp를 등록할 coding agent는? **(복수 선택 가능)**

A) Claude Code (CLI) — 전역/user scope

B) Claude Desktop 앱 (`claude_desktop_config.json`)

C) Codex (`~/.codex/config.toml`)

X) Other (please describe after [Answer]: tag below)

[Answer]: A,C
> 근거: 사용자가 명시한 "claude나 codex" 두 대상만 v1에 지원. Claude Desktop(B)은 별도 대상이라 범위만 늘어 v1 제외(추후 확장).

## Question 6
okc-mcp를 위 agent에 **등록하는 방식**은? (현재는 스니펫 출력만)

A) 선택한 agent의 전역 설정 파일에 **직접 병합**해서 써넣기(기존 항목 보존 + 백업)

B) 각 agent의 **공식 CLI 사용**(예: `claude mcp add --scope user ...`), 설정 파일 직접 편집은 안 함

C) 지금처럼 **출력만** 하고 사용자가 붙여넣기(등록 자동화 안 함)

X) Other (please describe after [Answer]: tag below)

[Answer]: B
> 근거: 각 agent의 공식 CLI(`claude mcp add --scope user`, `codex mcp add`)에 쓰기를 위임하면 JSON/TOML을 직접 파싱·병합하다 설정을 깨뜨릴 위험이 없고 코드량도 최소. CLI 미탐지 시 스니펫 출력(C 동작)으로 안전하게 폴백.

## Question 7
okc-mcp `setup`이 모듈의 로컬 config JSON(vaultPath / web.baseUrl / projectId / token 등)도 **생성·관리**하나요?

A) 예 — `setup`이 config JSON을 생성/갱신하고, 그 경로를 agent 등록에 사용

B) 아니요 — config는 사용자가 직접 두고, `setup`은 agent 등록만 수행

X) Other (please describe after [Answer]: tag below)

[Answer]: A
> 근거: "값 채움→설치"의 핵심. 사용자가 채우는 값이 곧 mcp config(vaultPath/web/token/codingAgent)이므로, setup이 이 파일을 쓰고 그 경로로 등록해야 흐름이 완결됨.

## Question 8
okc-web 토큰(발급은 okc-web 콘솔에서, 1회 노출)을 설치 흐름에서 어떻게 다룰까요?

A) 사용자가 발급받은 토큰을 config에 **붙여넣기 → 설치 시 okc-web에 실제 호출로 endpoint/token 유효성 검증**

B) 붙여넣기만, **검증 없이** 그대로 기록

C) 설치 흐름이 okc-web에 접속해 **토큰까지 자동 발급** 시도(관리자 인증 필요)

X) Other (please describe after [Answer]: tag below)

[Answer]: A
> 근거: "그냥 채우면 동작" 목표엔 오설정 조기 감지가 중요. 단 **best-effort·비차단(warn only)** 으로 한정 — mcp는 read-only GET(`contract`)로 검증, hooks는 부작용 없는 도달성/설정 검증만(전체 인증 핸드셰이크·세션 생성은 안 함). 자동 발급(C)은 관리자 인증 흐름이라 범위 과다 → 제외.

## Question 9
토큰을 로컬에 어떻게 저장할까요? (hooks upload token / mcp read token 공통)

A) 평문 config 파일에 저장 + 파일 권한 0600 (현재 방식)

B) OS secure-store(macOS Keychain / Linux Secret Service / Windows DPAPI)에 저장 — 단, hooks의 secure-store는 현재 미구현 stub이라 **신규 구현 필요**

C) 환경변수(`OKC_WATCHER_TOKEN` 등)로 주입, config엔 미기록

X) Other (please describe after [Answer]: tag below)

[Answer]: A
> 근거: 현재 방식 유지 + `setup`이 config 파일 권한을 0600으로 강제. secure-store(B)는 hooks stub 신규 구현이라 범위 과다. 보안 확장 적용 시에도 0600·토큰 미로깅·TLS 강제(이미 구현)로 baseline 충족, 완전한 secure-store는 후속으로 이연.

## Question 10
이번 이니셔티브의 작업 범위는?

A) 설계 + 구현(코드·테스트)까지

B) 설계/계획 문서까지만 (구현은 이후 별도)

X) Other (please describe after [Answer]: tag below)

[Answer]: A
> 근거: 사용자가 "설계 구현"을 명시. 단 각 모듈 변경은 최소·외과적으로 유지.

## Question 11
제거(uninstall)/등록 해제도 이번 범위에 포함할까요? (hooks엔 `uninstall`이 있으나 mcp 등록 해제는 없음)

A) 예 — 두 모듈 모두 대칭적인 uninstall/등록 해제 포함

B) 아니요 — 이번엔 설치/등록만

X) Other (please describe after [Answer]: tag below)

[Answer]: A
> 근거: CLI 위임(Q6=B) 덕에 mcp 등록 해제(`claude mcp remove`, `codex mcp remove`)가 저비용. hooks는 기존 `uninstall` 재사용. 단 생성한 config 제거 정도로 한정하고 Vault는 절대 건드리지 않음.

---

# 확장(Extension) 적용 여부

## Question: Security Extensions
이 프로젝트에 보안(Security) 확장 규칙을 강제할까요?

A) 예 — 모든 SECURITY 규칙을 blocking 제약으로 강제

B) 아니요 — SECURITY 규칙 생략

X) Other (please describe after [Answer]: tag below)

[Answer]: A
> 근거: 토큰/시크릿을 파일에 쓰고 OS 서비스·agent 설정을 수정하는 기능이라 보안은 권장안. 단계별로 **적용 가능한 규칙만** 강제(미해당은 N/A). 해당 핵심 규칙(시크릿 미로깅, 0600 권한, TLS 강제, 최소권한)은 이미 하려던 것과 일치해 한계 비용이 낮음.

## Question: Resiliency Extensions
resiliency baseline을 적용할까요?

A) 예 — 방향성 모범사례/설계 지침으로 적용

B) 아니요 — 생략

X) Other (please describe after [Answer]: tag below)

[Answer]: B
> 근거: 결과물은 로컬 1회성 설치 CLI이지 분산 프로덕션 워크로드가 아님. Well-Architected 신뢰성(HA/DR/RTO/RPO/관측성 15개 영역)은 대부분 N/A이고 적용 시 범위만 크게 늘어남 → 생략(범위 최소화).

## Question: Property-Based Testing Extension
property-based testing(PBT) 규칙을 강제할까요?

A) 예 — 모든 PBT 규칙을 blocking 제약으로 강제

B) 부분 — 순수 함수와 직렬화 round-trip에만 PBT 적용

C) 아니요 — PBT 생략

X) Other (please describe after [Answer]: tag below)

[Answer]: B
> 근거: config 파싱/직렬화 round-trip처럼 PBT 가치가 큰 순수 로직에만 한정 적용. okc-mcp엔 이미 PBT 인프라(`tests/properties.pbt.test.ts`)가 있어 저비용. 전면 강제(A)는 범위 과다.
