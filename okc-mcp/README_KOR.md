# okc-mcp

코딩 에이전트의 세션 지식을 관련 로컬 Obsidian 노트에 반영하고, 게시된 OKC 통합 Vault를 조회하는 stdio MCP입니다. Node.js 22.13 이상이 필요합니다. 현재 로컬 alpha 패키지이며 npm에는 게시하지 않았습니다.

`web` 설정이 있으면 통합 Vault를 기본 조회하고, 설정이 없을 때만 로컬을 사용합니다. 웹 연결 실패·권한 오류·게시 취소는 오류로 반환합니다. 자동으로 다른 로컬 지식으로 전환하지 않습니다.

## 코딩 세션 기록

세션은 **사용자가 선택해서 요청한 경우에만** 기록합니다. 시작·작업 완료·대화 압축·종료 시점에 자동 실행하지 않으며, 한 번 저장했다고 이후 내용까지 계속 기록하지 않습니다.

쓰기 가능한 로컬 Vault를 연결한 뒤 에이전트에게 **“이번 세션을 관련 노트에 기록해줘”**라고 요청하세요. 에이전트가 주제를 나누고 기존 노트와 구역을 확인해 반영 위치를 선택합니다. 사용자가 파일 경로나 생성·수정 여부를 고를 필요가 없습니다.

기존 주제는 해당 구역을 보강하고, 독립적인 주제는 기존 폴더 관례에 맞춰 새 노트를 만들고 연결합니다. 같은 세션의 항목을 다시 기록하면 기존 블록을 갱신해 중복을 줄입니다. 다른 내용과 YAML은 보존하며 변경 전 백업과 저장 후 해시 확인을 수행합니다.

사용자가 기록할 세션을 고르면 서버 지침과 `capture_session` 프롬프트가 그 세션의 반영 절차를 제공합니다. 호출하는 에이전트는 현재 사용자 선택을 `userSelected: true`로 명시해야 하며, 이 값의 기본값은 `false`입니다. `prepare_session_capture`는 로컬 후보·폴더·구역·이전 기록을 찾고, `apply_session_capture`는 에이전트가 선택한 항목들을 미리 검사한 뒤 저장합니다. 후보 점수는 문자열 근거이고 최종 관련성 판단은 연결된 코딩 에이전트가 합니다. 전체 지침은 `okc://guide/session-capture`에서 읽을 수 있습니다.

## 설치와 시작

```sh
npm ci
npm run check
node dist/cli.js config --vault /absolute/ExistingVault
node dist/cli.js doctor --config /absolute/okc-mcp.json
node dist/cli.js client-config --config /absolute/okc-mcp.json
```

설정 파일은 Vault 밖에 저장하세요. 새 원본 Vault는 `node dist/cli.js init --vault /absolute/NewVault`로 만듭니다. 부모 디렉터리는 미리 있어야 하며, 기존 대상은 변경하지 않습니다. 빈 `inbox/`, `notes/`, `sources/`, `maps/` 폴더만 생성하고 실제 지식은 작성 도구로 추가합니다. 선택적인 노트 템플릿은 `okc://templates/source-note` 리소스로 제공합니다.

## 한 번에 설치하기

설정 파일을 채우고 등록할 코딩 에이전트를 `agents` 목록(Claude Code는 `"claude"`, Codex는 `"codex"`)에 적은 뒤 실행합니다.

```sh
node dist/cli.js setup --config /absolute/okc-mcp.json
node dist/cli.js unregister --config /absolute/okc-mcp.json   # 해제(선택). --purge를 붙이면 생성된 설정 파일도 삭제
```

`setup`은 설정을 `~/.config/okc-mcp/config.json`(또는 `$XDG_CONFIG_HOME`)에 `0600` 권한으로 기록한 뒤, 각 에이전트의 공식 CLI(`claude mcp add --scope user okc-mcp -- …`, `codex mcp add okc-mcp -- …`)로 okc-mcp를 등록합니다. 에이전트 CLI가 `PATH`에 없으면 해당 에이전트 파일을 직접 편집하지 않고 `client-config` 스니펫을 출력합니다. `web`이 설정되어 있으면 읽기 전용 도달성 검사를 best-effort로 수행하고 실패 시 경고만 남깁니다. 읽기 토큰은 `0600` 설정 파일에만 기록되며 출력이나 로그에 절대 남기지 않습니다. `unregister`는 등록된 에이전트에서 okc-mcp를 제거(`claude mcp remove` / `codex mcp remove`)하며 Vault는 건드리지 않습니다.

## 웹 연결

```json
{
  "vaultPath": "/absolute/AuthoringVault",
  "statePath": "/absolute/okc-mcp-state",
  "readOnly": false,
  "web": {
    "baseUrl": "http://127.0.0.1:8000",
    "projectId": "your-project-id",
    "timeoutMs": 10000
  }
}
```

비공개 게시본은 okc-web이 발급한 읽기 토큰을 `web.token`에 넣습니다. 원격 서버에는 HTTPS를 사용하세요. 웹 조회만 필요하면 `vaultPath`와 `statePath`를 생략하고 `readOnly: true`로 설정합니다. 로컬을 기본으로 사용하려면 `web`을 제거하세요.

조회 도구는 `list_notes`, `read_note`, `search_notes`, `outline_note`, `list_backlinks`, `audit_vault`입니다. 기본값을 바꾸려면 `source: "local"` 또는 `source: "web"`을 명시합니다. 웹 설정 시 `verify_vault`와 `explain_note`로 무결성과 출처도 확인할 수 있습니다.

원격 응답의 `source`에는 프로젝트, `revision`, 게시 상태, `stale` 여부가 포함됩니다. 다음 페이지나 연관 조회에 같은 `revision`을 전달하면 같은 게시본을 사용합니다. 여러 파일을 읽는 한 번의 검색은 자동으로 게시본 하나에 고정됩니다.

## 원본 작성과 통합

작성 도구는 항상 로컬을 대상으로 합니다: `create_note`, `update_note`, `standardize_frontmatter`, `fix_yaml`, `reinforce_sources_links`. 수정 전 `read_note`에 `source: "local"`을 넣어 원본과 전체 SHA-256을 확인하세요. `dryRun` 기본값은 `true`이며, `false`로 호출할 때 실제로 저장합니다. 기존 파일 수정에는 현재 해시와 Vault 밖의 사전 백업을 사용합니다.

로컬 변경은 hooks 업로드, web 통합·검토·게시 후 통합 Vault에 반영됩니다. 게시 결과와 `.okc-project`는 작성 대상으로 연결할 수 없습니다. `readOnly: true`이면 작성 도구와 세션 기록 도구·프롬프트를 노출하지 않습니다. 웹이 기본 조회여도 세션 반영 위치는 항상 로컬 원본에서 찾습니다.

세션 기록은 `dryRun: true`로 사전 검사와 내용 미리보기를 제공하고, `false`로 실제 저장합니다. 여러 노트를 쓰다가 실행 중 실패하면 `partial`과 파일별 결과를 반환합니다. 성공한 파일을 확인하고 같은 세션·항목 ID와 최신 해시로 다시 준비하면 완료한 항목은 중복 저장하지 않습니다. `okc-capture` 주석은 반복 기록을 식별하므로 내용과 따로 복사·삭제하지 마세요. 세션 종료 자동 수집이나 별도 모델 API 호출은 추가하지 않습니다.

노트 구조는 최소 YAML과 출처 중심의 Markdown 관례입니다. 폴더명이나 `status/type` 같은 추가 키를 승인·게시 권한으로 해석하지 않습니다. 검색은 한국어를 지원하는 제한된 문자열 검색이고, 임베딩·시맨틱 검색은 포함하지 않습니다. 원격 캐시는 한 작업 동안만 유지되며, 요청마다 바이트·시간 제한과 취소를 적용합니다.

상세 도구와 제약은 [영문 README](README.md), [복구 안내](docs/recovery.md), [AI-DLC 상태](aidlc-docs/aidlc-state.md), [검증 결과](aidlc-docs/construction/build-and-test/web-knowledge-summary.md)를 참고하세요.
