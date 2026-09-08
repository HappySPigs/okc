# 이 저장소의 공식 AI-DLC 설정

`awslabs/aidlc-workflows` **2.7.1**의 Codex 배포본을 이 프로젝트에 설치했다.
고정 커밋은 `22ed2d101f4f01196b76d5725cf8d9aabe5fef9e`이다.
[공식 설치 안내](https://github.com/awslabs/aidlc-workflows/blob/22ed2d101f4f01196b76d5725cf8d9aabe5fef9e/docs/guide/harnesses/codex-cli.md),
[설치 출처와 파일 해시](aidlc-upstream/installation.json),
[upstream 라이선스](aidlc-upstream/LICENSE).

사용자의 2026-09-06 요청 “이거 여기서 사용가능하게 세팅해”에 따른 개발 워크플로 설정이다.
제품은 계속 **Inception 검토 대기**다. 설치를 요구사항 승인이나 Construction 진입으로
기록하지 않는다. 기존 `src/`, `tests/`, 패키지 설정은 승인 전 초안이다.

## 사용하기

이 프로젝트를 작업 폴더로 선택한 **새 Codex 대화**에서 다음을 입력한다.
이미 열린 대화는 설치 전 스킬·훅 목록을 가지고 있을 수 있다.

```text
$aidlc --doctor
```

설정 확인 후 프로젝트 검토를 시작할 때는 다음처럼 요청한다.

```text
$aidlc aidlc-docs/state.md와 aidlc-docs/inception/review.md를 참고해 OKC 입력 Vault를 만드는 로컬 Obsidian MCP의 요구사항을 공식 절차로 검토하자. 기존 코드와 문서는 승인 전 초안이며, 내가 요구사항을 검토하고 Construction 전환을 승인하기 전에는 제품 구현·테스트·패키징을 진행하지 마.
```

상태 확인은 `$aidlc --status`, 도움말은 `$aidlc --help`다. `$aidlc`가 목록에 없다면
Codex를 다시 열고 프로젝트 경로 및 `/skills`의 `aidlc` 항목을 확인한다.
설치 작업에서는 워크플로 intent나 단계 완료·승인 기록을 생성하지 않는다.
첫 제품 워크플로가 공식 초기화부터 기록을 만들며 기존 초안은 참고 자료로 사용한다.

셸에서도 모델 호출 없이 설치 상태를 확인할 수 있다.

```sh
bun .codex/tools/aidlc-utility.ts version
bun .codex/tools/aidlc-utility.ts doctor
bun .codex/tools/aidlc-utility.ts status
```

## 설치 구성과 기존 기록

| 경로 | 역할 |
|---|---|
| `.agents/skills/` | 공식 `$aidlc` 및 단계·보조 스킬 42개 |
| `.codex/tools/`, `.codex/hooks/`, `.codex/hooks.json` | 공식 엔진과 Codex 훅 |
| `.codex/agents/` | 공식 역할 14개의 Markdown 설명과 TOML 설정 |
| `.codex/config.toml` | 이 프로젝트의 AI-DLC 규칙 경로와 게이트 설정 |
| `aidlc/spaces/default/memory/` | 공식 규칙과 실제 사용자 지시에서 가져온 프로젝트 제약 |
| `aidlc/spaces/<space>/intents/` | 향후 공식 실행기가 생성할 intent·상태·감사·산출물 |
| `aidlc-docs/` | 설치 전에 작성된 조사·초안·절차 정정 이력 |
| `docs/aidlc-upstream/` | 고정 버전의 설치 안내, 원본 설정 예시, 라이선스, 출처·해시 |

`aidlc-docs/state.md`의 제품 승인 상태와 향후 공식 실행기의 상태 파일은 구별한다.
기존 문서를 완료된 공식 단계로 복사하거나 승인 이벤트를 소급 생성하지 않는다.
공식 엔진의 TypeScript, 스킬, 단계 그래프와 공유 훅 본문은 배포본 그대로 보존한다.
Codex 전용 어댑터의 로컬 호환성 수정은 아래에 별도로 기록하며 upstream 출처 해시를
바꾸어 공식 배포본인 것처럼 표시하지 않는다.

## Codex 환경에 맞춘 조정

배포본의 기본 AWS Bedrock 연결 대신 **기존 Codex 모델·제공자·로그인**을 사용한다.
프로젝트 설정에서 모델명·context 크기·reasoning 기본값과 Bedrock 제공자 블록을 생략했다.
공식 5개 역할이 지정한 `openai.gpt-5.6-terra`는 OpenAI용 `gpt-5.6-terra`로 바꾸고,
역할별 reasoning 설정은 유지했다. 나머지 역할은 현재 세션 모델을 상속한다.
별도 AWS 계정이나 새 API 키를 추가하지 않는다.

기존 sandbox·네트워크·TUI 설정을 상속한다. 공식 배포본의 규칙 경로,
위임 깊이 제한과 질문 도구 설정은 유지한다. `default_mode_request_user_input`은
설치 당시 Codex에서 개발 중인 기능으로 표시되며, 제공되지 않는 환경의 질문 표시는
공식 워크플로의 대체 절차와 호스트의 도구 제약을 따른다.

훅은 Codex가 신뢰한 항목만 실행한다. 이 프로젝트의 `.codex/hooks.json`에 대한
15개 항목만 사용자 `~/.codex/config.toml`의 `[hooks.state]`에 등록했다.
사용자 설정은 같은 디렉터리의 `config.toml.before-okc-mcp-aidlc-*.bak`으로 백업했다.
모델·로그인·MCP·다른 프로젝트 설정을 보존했으며, 사용자 설정과 백업을 이 저장소에
복사하지 않았다.

공식 `package.ts codex trust` 출력 중 matcher가 지정된 **6개 항목의 해시**가
Codex CLI 0.153.4의 실제 해시와 달랐다. 훅 파일은 수정하지 않고, Codex의 읽기 전용
`hooks/list`가 설치된 공식 훅에 대해 반환한 `currentHash`로 15개 항목을 등록했다.
새 app-server 프로세스로 다시 조회하여 전부 `trusted`, `enabled: true`임을 확인했다.
공식 생성기 출력만 붙여 넣고 신뢰가 완료됐다고 가정하지 않는다.

[OpenAI Hooks 문서](https://learn.chatgpt.com/docs/hooks.md)에 따라 Codex CLI 0.153.4는
성공한 `Stop`과 `SubagentStop` 훅도 stdout에 JSON을 반환해야 한다.
공유 AI-DLC 훅은 허용 경로에서 성공 코드 0과 빈 stdout을 반환하므로 Codex가 이를
`Hook failed`로 표시했다. `.codex/hooks/aidlc-codex-adapter.ts`에서 이 두 이벤트의
빈 성공 응답만 `{}`로 정규화한다. 차단 응답과 종료 코드는 그대로 전달하고, 공유 훅
본문이나 워크플로 상태 전이는 변경하지 않는다.

같은 문서의 `PreToolUse` 계약은 `updatedInput`을 반환할 때
`permissionDecision: "allow"`를 함께 요구한다. 세션 바인딩과 단계 규칙 전달 경로가
기존 `updatedInput`만 반환해 발생하던 `Hook failed`를 막기 위해 Codex 전용 어댑터에서
누락된 allow 결정을 보완한다. deny·exit 2 차단 응답은 변경하지 않는다.

Codex의 프로젝트 설정은 신뢰한 프로젝트에서 읽힌다.
[OpenAI의 설정 설명](https://learn.chatgpt.com/docs/config-file/config-reference#configtoml).
훅 등록·신뢰 확인은 설치 확인이며, 현재 대화에서 실제 제품 워크플로 훅이 실행되었다는
증거로 취급하지 않는다. 새 Codex 대화의 첫 실행에서 실제 훅 이력이 시작된다.

## 검증 기록

2026-09-06, macOS arm64, Codex CLI `0.153.4`, Bun `1.4.0`.

- 임시 설치본에서 공식 `doctor`: **44 passed, 0 failed**.
- 실제 프로젝트에서 공식 `doctor`: **44 passed, 0 failed**; 버전 `aidlc 2.7.1`.
- TOML 설정 15개 및 공식 훅 신뢰 항목 15개의 구문 확인 완료.
- 실제 프로젝트를 대상으로 Codex app-server의 strict config 로딩 성공.
  기존 `gpt-6-astra` 모델, AI-DLC 규칙 경로, 질문 기능 설정을 확인했다.
- Codex의 `skills/list`: 스킬 42개, 활성화된 `aidlc`, 프로젝트 스킬 오류 0개.
- Codex의 `hooks/list`: 훅 15개 모두 신뢰·활성화, 오류·경고 0개.
- 설치 시 배포 파일 331개의 설치 해시 일치. 제품 소스·테스트·패키지 등 15개 파일의
  설치 전후 해시가 같고, 개인 런타임 제외와 공유 기록 보존용 gitignore를 확인했다.
- 공식 `status`: `No active AI-DLC workflow found.` 실제 intent를 초기화하지 않았다.
- 제품의 `npm run check`, 테스트, 빌드, 패키징은 실행하지 않았다.

2026-09-07 추가 검증, Codex CLI `0.153.4`:

- 활성 intent가 없는 실제 `Stop` 입력에서 수정 전 `exit 0 + 빈 stdout`을 재현했다.
- Codex 어댑터 수정 후 같은 입력이 `exit 0 + {}`를 반환하는지 확인했다.
- `SubagentStop`의 빈 성공 경로도 `exit 0 + {}`를 반환하도록 같은 계약을 적용했다.
- `PreToolUse` 세션 바인딩 입력에서 `updatedInput`과
  `permissionDecision: "allow"`가 함께 반환되는지 확인했다.
- 공유 단계 규칙 훅의 `updatedInput`에도 같은 allow 결정을 추가하는 Codex 경계를
  적용했다.
- 공식 `doctor`를 다시 실행해 훅 heartbeat, drop, lock 및 설치 상태를 확인했다.
- 이 로컬 호환성 수정으로 Codex 전용 어댑터 한 파일은 upstream 설치 해시와 의도적으로
  달라졌으며, 고정 upstream 커밋이나 설치 출처 기록은 변경하지 않았다.

공식 진단은 기존 scope의 advisory 47개, 프로젝트 규칙과 org 규칙의 키워드 중복
1개, 아직 커밋하지 않은 method 파일 8개를 안내했다. 키워드 중복은 승인 전 제품 작업
중단 지시와 org의 trunk 개발 문구 사이에서 검출되었으며 서로 다른 제약이다.
훅 heartbeat가 아직 없는 것은 제품 워크플로를 시작하지 않은 상태와 일치한다.
이 설치 진단을 실제 단계 실행이나 제품 검증 완료로 표시하지 않는다.

## 재설치와 업데이트

1. [설치 출처](aidlc-upstream/installation.json)의 커밋과 공식 설치 안내를 확인한다.
2. 새 배포본의 `.codex/`, `.agents/`, `aidlc/`를 비교한다. 기존 intent, 감사 기록,
   사용자 지식과 수정한 `memory/`를 통째로 덮어쓰지 않는다.
3. 프로젝트용 모델 조정과 루트 `AGENTS.md`의 승인 경계를 유지한다.
4. upstream `.gitignore`의 `# AI-DLC` 부분만 기존 프로젝트 규칙에 합친다.
   상태·감사·산출물은 공유하고 사용자 커서·로컬 런타임은 제외한다.
5. 공식 소스 체크아웃에서 `bun install --frozen-lockfile` 후
   `bun scripts/package.ts codex trust --project <절대 프로젝트 경로>`로 새 신뢰 항목을
   생성한다. 해당 훅 파일의 기존 항목은 교체하고 중복 TOML 테이블을 만들지 않는다.
   새 Codex가 보고하는 훅 해시·신뢰 상태도 확인한다. 해시가 달라 `modified`로
   표시되면 설치된 훅 명령을 검토하고 Codex의 훅 신뢰 화면에서 다시 신뢰한다.
   0.153.4에서 확인된 생성기 차이는 위 검증 기록을 참고한다.
6. 공식 `doctor`와 Codex의 스킬·훅 인식을 확인하고, 출처·해시·실제 검증 기록을 갱신한다.

이 저장소에는 설치 출처·배포 파일을 두므로 다른 컴퓨터에서 프레임워크 개발용
의존성을 설치할 필요는 없다. Codex CLI 0.145.0 이상과 Bun, 해당 프로젝트 및 훅의
신뢰 설정은 각 컴퓨터에서 준비한다.
