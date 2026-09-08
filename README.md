# okc-mcp

> **진행 상태: Inception 검토 중.** 아래 기능·설치 문서는 승인 전 구현 초안의 설명입니다.
> 요구사항 검토 전에 Construction을 진행한 절차를 정정하고 추가 구현을 멈췄습니다.
> 제품 범위와 성공 기준은 [Inception 검토안](aidlc-docs/inception/review.md)에서 먼저 확정합니다.

**OKC에 넣을 지식을 잘 쌓기 위한, 로컬 Obsidian Vault 작성 MCP.**

Obsidian 앱·플러그인·API 키 없이 Markdown 노트를 만들고, 기존 노트를 읽고 검색하며,
프런트매터와 링크·중복·입력 품질을 점검합니다. 실제 OKC 통합·검토·컴파일은 OKC가 담당합니다.

현재 `0.1.0-alpha.1` 로컬 구현입니다. npm에 공개 배포하지 않았습니다.
Node.js **22.13+**가 필요하며 실제 검증 환경과 한계는 [검증 기록](aidlc-docs/construction/validation.md)에 남깁니다.

## 설치와 연결

이 저장소에서 패키지를 만들고 로컬 설치합니다.

```sh
npm ci
npm run check
npm pack
npm install -g ./okc-mcp-0.1.0-alpha.1.tgz
```

기존 Vault의 절대 경로로 설정을 생성합니다. **설정 파일은 Vault 밖에 저장**합니다.

```sh
okc-mcp config --vault /absolute/path/to/MyVault > /absolute/path/to/okc-mcp.json
okc-mcp doctor --config /absolute/path/to/okc-mcp.json
okc-mcp client-config --config /absolute/path/to/okc-mcp.json
```

`client-config`가 출력한 `mcpServers` 항목을 사용하는 MCP 클라이언트의 설정에 합칩니다.
다른 서버 설정은 그대로 보존하세요. 생성 명령은 다른 앱의 설정을 자동 변경하지 않습니다.
클라이언트마다 설정 화면과 감싸는 구조는 다를 수 있으나 연결은 표준 stdio입니다.
출력에는 설치된 Node와 서버 파일의 절대 경로가 들어가 GUI 앱의 PATH 차이를 줄입니다.

설치 없이 개발 트리에서 실행하려면 `node dist/cli.js`를 위의 `okc-mcp` 대신 사용합니다.
직접 서버를 실행하는 명령은 다음과 같으며, 이후 stdin/stdout은 MCP 프로토콜 전용입니다.

```sh
okc-mcp serve --config /absolute/path/to/okc-mcp.json
```

## 첫 사용

MCP를 연결한 AI 클라이언트에 다음처럼 요청합니다.

> 작성 지침을 읽고 내 Vault에서 HTTP 캐시 관련 노트를 찾아줘.
> 내가 제공한 자료의 주장과 출처를 구분해서 새 노트 초안을 만들고,
> 적용한 뒤 OKC 입력 품질을 점검해 줘. 없는 출처는 만들지 마.

`okc://guide/authoring` 리소스와 `capture_knowledge` 프롬프트에 작성 지침을 제공합니다.
작성 도구의 `dryRun` 기본값은 `true`입니다. 결과의 `applied: false`는 미리보기이며,
실제로 저장하려면 `dryRun: false`로 호출합니다. 이 인자가 별도의 사람 승인을 증명하지는 않습니다.

| 도구 | 역할 |
|---|---|
| `vault_info` | 연결 모드·파일 수·제한·OKC 참고 버전 |
| `list_notes` | 정렬된 상대 경로와 페이지 |
| `read_note` | 본문 범위와 **전체 파일** SHA-256 |
| `search_notes` | 한글을 포함한 리터럴 검색, 짧은 본문 발췌 |
| `create_note` | 최소 프런트매터가 있는 새 노트, 기존 파일 교체 금지 |
| `replace_note` | 해시 검사와 외부 백업 후 전체 본문 수정 |
| `patch_frontmatter` | 본문·기존 키·주석을 보존하며 지정한 YAML 키 설정 |
| `audit_vault` | YAML·링크·중복·운영 문서 잡음·미지원 형식 점검 |

부분 읽기의 `offset`/`length`는 JavaScript 문자열 문자 단위이며 바이트나 줄 번호가 아닙니다.
목록·검색·점검은 `offset`/`limit`, 다음 페이지는 `nextOffset`으로 이어갑니다.
수정할 긴 노트는 전체 범위를 읽고 검토하세요. `expectedHash`는 `read_note`가 반환한 값을 씁니다.

## 어떤 Vault가 OKC에 좋은가

기존 폴더를 옮길 필요는 없습니다. 새로 만든다면 `inbox/`, `notes/`, `sources/`, `maps/`의
얕은 구조로 시작할 수 있습니다. 중요한 것은 **한 문단의 명확한 주장, 가까운 출처,
모호하지 않은 링크, 불필요하게 반복하지 않는 메타데이터**입니다.

`title`, `aliases`, `tags` 외의 `source`, `status`, `type` 등은 선택적인 작성 관습입니다.
OKC가 이를 승인·공개 허가·분류 정책으로 해석한다고 가정하지 않습니다.
템플릿·MCP 설정·백업·운영 문서는 Vault 밖에 두어 입력 지식에 섞이지 않게 합니다.

```text
MyKnowledge/
├── AuthoringVault/        ← Obsidian과 이 MCP가 편집
│   ├── inbox/
│   ├── notes/
│   ├── sources/
│   └── maps/
├── tooling/
│   └── okc-mcp.json
├── Knowledge.okc-project/ ← OKC 작업·검토 상태
└── artifacts/             ← OKC 출력 보존
```

백업·잠금은 설정의 `statePath`에 둡니다. 기본값은 사용자 홈의
`.local/state/okc-mcp/<vault-id>`이며 Vault 안이나 Vault를 감싸는 경로는 허용하지 않습니다.
OKC는 별도로 이 편집 Vault를 source로 등록하고 스냅샷을 수집합니다.
스냅샷 이후 수정은 다음 수집의 입력입니다. 결과 artifact와 `.okc-project`는 이 MCP의 편집 대상이 아닙니다.

자세한 예시: [작성 가이드](docs/authoring-guide.md), [OKC Vault 설계](aidlc-docs/inception/okc-vault-design.md).

## 설정과 문제 해결

[설정 예시](examples/okc-mcp.example.json)의 모든 경로는 절대 경로입니다.
`readOnly: true`이면 세 가지 작성 도구를 아예 등록하지 않습니다.
진단은 경로와 스캔 가능 여부를 확인하며 쓰기 권한이나 compiler 호환성을 보증하지 않습니다.

| 오류/상황 | 다음 행동 |
|---|---|
| `CONFLICT` | 노트를 다시 읽고 최신 내용에 변경을 다시 검토합니다. 이전 요청을 무조건 재시도하지 않습니다. |
| 이미 있는 생성 경로 | 기존 노트를 읽은 뒤 수정하거나 새 경로를 선택합니다. |
| `NOTE_INVALID` | 중복 YAML 키·문법·title/aliases/tags 타입을 확인합니다. |
| 경로/링크 거부 | 등록 Vault의 일반 `.md` 상대 경로를 사용합니다. 숨김 경로·symlink·hardlink는 지원하지 않습니다. |
| 응답 상한 | `limit` 또는 읽는 `length`를 줄입니다. |
| 스캔 상한 | 연결 대상을 줄이거나 크기를 확인한 뒤 설정 한도를 조정합니다. 결과를 완전한 점검으로 오인하지 않습니다. |
| 잠금 충돌 | 다른 MCP 쓰기가 끝난 뒤 다시 확인합니다. 비정상 종료 복구는 [운영 가이드](docs/operations.md)를 따릅니다. |

## 현재 보장 범위

- 모든 노트 조회 결과는 MCP 호스트에 전달됩니다. 서버 자체는 AI·원격 검색·telemetry를 호출하지 않습니다.
  사용하는 호스트가 원격 모델이면 그 호스트의 데이터 처리 범위가 적용됩니다.
- 입력 점검은 OKC `0.3.0`의 특정 소스를 참고한 **작성용 휴리스틱**입니다.
  compiler validation, 완전한 Obsidian 링크 해석, 민감정보 탐지, 사실 검증을 대체하지 않습니다.
- 현재 OKC는 Attachment/Canvas/Base 보존과 완전한 링크 재작성 등의 출시 조건이 남아 있습니다.
- 해시 검사와 파일 교체는 외부 Obsidian/Sync와 운영체제 수준의 compare-and-swap을 제공하지 않습니다.
  같은 노트의 동시 편집을 피하세요. 백업과 관측된 stale hash 검사로 복구·충돌 검토를 지원합니다.
- Node 경로 검사로 악의적인 동시 상위 디렉터리 교체를 완전히 격리한다고 주장하지 않습니다.
  제어하지 않는 프로세스가 파일시스템을 교체하는 환경은 지원하지 않습니다.
- 삭제·rename·자동 폴더 이동·스냅샷 내보내기·OKC 승인·컴파일·semantic 검색은 현재 도구에 없습니다.

## 설계와 개발

공식 AI-DLC 2.7.1의 Codex 워크플로를 이 프로젝트에 설치했습니다.
새 Codex 대화에서 `$aidlc --doctor`로 확인하고 [사용·설정 안내](docs/aidlc-setup.md)를
따라 시작합니다. 현재 제품은 계속 Inception 검토 대기이며, 이 설치는 Construction
전환을 승인한 기록이 아닙니다.

- [AWS AI-DLC 적용 방법](aidlc-docs/methodology.md)
- [기존 Obsidian MCP 네 종 소스 조사](aidlc-docs/inception/existing-mcp-research.md)
- [요구사항](aidlc-docs/inception/requirements.md) · [구현 설계](aidlc-docs/construction/design.md)
- [저장소·설치 UX](aidlc-docs/inception/repository-ux.md) · [기여 방법](CONTRIBUTING.md)
- [현재 상태와 후속 단위](aidlc-docs/state.md) · [요구사항 추적](aidlc-docs/construction/traceability.md)

이 제품은 [MCP SDK](https://github.com/modelcontextprotocol/typescript-sdk/tree/v1.30.0)로
프로토콜을 구현합니다. Obsidian 링크/프로퍼티의 사용자 의미는
[Obsidian 공식 도움말](https://obsidian.md/help/Linking%2Bnotes%2Band%2Bfiles/Internal%2Blinks)을 참고하며,
OKC의 실제 parser 동작과 차이가 있으면 점검의 한계로 명시합니다.
# OKC-MCP
