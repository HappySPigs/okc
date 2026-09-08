# okc-mcp

> **상태: Inception 검토 대기 중.** 아래의 기능 및 설치 문서는 아직 승인되지 않은 구현 초안을 설명합니다.
> Construction이 요구사항 검토보다 먼저 시작되었으며, 이 프로세스 오류는 기록되었고 이후 구현은 중단된 상태입니다.
> 먼저 [Inception 검토 제안서](aidlc-docs/inception/review.md)에서 제품 범위와 성공 기준을 확인하세요.

**OKC를 위한 고품질 지식 입력을 만드는 로컬 Obsidian Vault 작성용 MCP.**

Obsidian 앱, 플러그인, API 키 없이 Markdown 노트를 생성하고, 기존 노트를 읽고 검색하며, frontmatter·링크·중복·입력 품질을 감사합니다. 실제 통합, 검토, 컴파일은 여전히 OKC의 책임입니다.

현재 로컬 구현은 `0.1.0-alpha.1`이며 npm에 게시되지 않았습니다. Node.js **22.13+**가 필요합니다. 실제 검증 환경과 제약 사항은 해당 단계가 승인된 후 Construction 검증 로그에 기록됩니다.

## 설치 및 연결

이 저장소에서 패키지를 빌드하여 로컬에 설치합니다.

```sh
npm ci
npm run check
npm pack
npm install -g ./okc-mcp-0.1.0-alpha.1.tgz
```

기존 Vault의 절대 경로를 사용해 설정을 생성합니다. **설정은 Vault 밖에 저장하세요.**

```sh
okc-mcp config --vault /absolute/path/to/MyVault > /absolute/path/to/okc-mcp.json
okc-mcp doctor --config /absolute/path/to/okc-mcp.json
okc-mcp client-config --config /absolute/path/to/okc-mcp.json
```

`client-config`가 출력한 `mcpServers` 항목을 MCP 클라이언트 설정에 병합하세요. 다른 서버 설정은 보존합니다. 생성 명령은 다른 애플리케이션의 설정을 자동으로 편집하지 않습니다. 설정 화면과 래퍼 객체는 클라이언트마다 다르지만, 연결은 표준 stdio를 사용합니다. 출력에는 설치된 Node 실행 파일과 서버 파일의 절대 경로가 포함되어 GUI 애플리케이션에서의 PATH 차이를 줄여 줍니다.

설치하지 않고 개발 트리에서 실행하려면 위 명령에서 `okc-mcp` 대신 `node dist/cli.js`를 사용하세요. 서버를 직접 실행하려면 아래 명령을 사용합니다. 시작 이후 stdin과 stdout은 MCP 프로토콜 전용으로 예약됩니다.

```sh
okc-mcp serve --config /absolute/path/to/okc-mcp.json
```

## 첫 사용

연결된 AI 클라이언트에 다음과 같이 요청해 보세요.

> 작성 가이드를 읽고 내 Vault에서 HTTP 캐싱에 관한 노트를 찾아줘.
> 내가 제공한 출처와 주장을 구분한 새 노트를 초안으로 만들고,
> 적용한 다음, OKC 입력으로서의 품질을 감사해줘. 없는 출처는 지어내지 마.

작성 가이드는 `okc://guide/authoring` 리소스와 `capture_knowledge` 프롬프트를 통해 제공됩니다. 작성 도구들은 `dryRun` 기본값이 `true`입니다. `applied: false` 결과는 미리보기일 뿐이며, 변경을 저장하려면 `dryRun: false`로 도구를 호출하세요. 이 인자가 별도의 사람 승인을 증명하지는 않습니다.

| 도구 | 용도 |
|---|---|
| `vault_info` | 연결 모드, 파일 수, 제한, 참조된 OKC 버전 |
| `list_notes` | 정렬된 상대 경로, 페이지네이션 지원 |
| `read_note` | 내용 범위와 **파일 전체**의 SHA-256 |
| `search_notes` | 한국어 텍스트를 포함한 리터럴 검색, 짧은 발췌 |
| `create_note` | 최소 frontmatter를 가진 새 노트, 기존 파일은 절대 대체하지 않음 |
| `replace_note` | 해시 확인과 외부 백업 후 본문 전체 교체 |
| `patch_frontmatter` | 본문·기존 키·주석을 보존하며 선택한 YAML 키만 설정 |
| `audit_vault` | YAML, 링크, 중복, 운영 노이즈, 미지원 형식 감사 |

부분 읽기에 사용하는 `offset`과 `length`는 바이트나 줄 번호가 아니라 JavaScript 문자열 문자를 셉니다. 목록·검색·감사는 `offset`과 `limit`을 사용하며, `nextOffset`으로 이어갑니다. 긴 노트를 편집하기 전에 전체 범위를 읽고 검토하세요. `read_note`가 반환한 `expectedHash`를 사용하세요.

## OKC에 좋은 Vault란

기존 Vault를 재구성할 필요는 없습니다. 새 Vault라면 `inbox/`, `notes/`, `sources/`, `maps/`와 같은 얕은 구조가 합리적인 출발점입니다. 중요한 것은 **문단마다 하나의 명확한 주장, 가까이 둔 출처, 모호하지 않은 링크, 불필요하게 반복되지 않는 메타데이터**입니다.

`title`, `aliases`, `tags` 외의 `source`, `status`, `type` 같은 필드는 선택적인 작성 관례입니다. OKC가 이를 승인, 게시 허가, 분류 정책으로 해석한다고 가정하지 마세요. 템플릿, MCP 설정, 백업, 운영 문서는 입력 지식이 되지 않도록 Vault 밖에 두세요.

```text
MyKnowledge/
├── AuthoringVault/        ← Obsidian과 이 MCP가 편집
│   ├── inbox/
│   ├── notes/
│   ├── sources/
│   └── maps/
├── tooling/
│   └── okc-mcp.json
├── Knowledge.okc-project/ ← OKC 작업 및 검토 상태
└── artifacts/             ← 보존된 OKC 출력물
```

백업과 락은 설정된 `statePath` 아래에 위치합니다. 기본값은 사용자 홈 디렉터리 아래의 `.local/state/okc-mcp/<vault-id>`이며, Vault 내부 경로나 Vault를 포함하는 경로는 허용되지 않습니다. OKC는 작성용 Vault를 별도의 소스로 등록하고 스냅샷을 캡처합니다. 스냅샷 이후 변경한 내용은 다음 캡처의 입력이 됩니다. 그 결과로 만들어진 산출물과 `.okc-project` 디렉터리는 이 MCP의 편집 대상이 아닙니다.

자세한 예시는 [작성 가이드](docs/authoring-guide.md)와 [OKC Vault 설계](aidlc-docs/inception/okc-vault-design.md)를 참고하세요.

## 설정 및 문제 해결

[예시 설정](examples/okc-mcp.example.json)의 모든 경로는 절대 경로입니다. `readOnly: true`이면 서버는 세 가지 작성 도구를 하나도 등록하지 않습니다. 진단은 경로와 스캔 접근을 확인할 뿐, 쓰기 권한이나 컴파일러 호환성을 보장하지 않습니다.

| 오류 또는 상황 | 다음 조치 |
|---|---|
| `CONFLICT` | 노트를 다시 읽고 최신 내용과 변경 사항을 비교하세요. 이전 요청을 무작정 재시도하지 마세요. |
| 생성 경로가 이미 존재함 | 기존 노트를 읽고 수정하거나, 새 경로를 선택하세요. |
| `NOTE_INVALID` | 중복 YAML 키, 문법, 그리고 `title`·`aliases`·`tags`의 타입을 확인하세요. |
| 경로 또는 링크 거부됨 | 등록된 Vault 내의 일반 상대 `.md` 경로를 사용하세요. 숨김 경로, 심볼릭 링크, 하드 링크는 지원되지 않습니다. |
| 응답 제한 | `limit` 또는 요청한 읽기 `length`를 줄이세요. |
| 스캔 제한 | 설정된 제한을 조정하기 전에 연결된 소스를 좁히거나 크기를 확인하세요. 부분 결과를 완전한 감사로 취급하지 마세요. |
| 락 충돌 | 다른 MCP 쓰기가 끝날 때까지 기다린 뒤 최신 상태를 확인하세요. 크래시 복구는 [운영 가이드](docs/operations.md)를 따르세요. |

## 현재 보장 사항과 제약

- 모든 노트 조회 결과는 MCP 호스트로 전송됩니다. 서버 자체는 AI 서비스, 원격 검색, 텔레메트리를 호출하지 않습니다. 호스트가 원격 모델을 사용한다면 호스트의 데이터 처리 정책이 적용됩니다.
- 입력 감사는 선택된 OKC `0.3.0` 소스를 **작성 휴리스틱**으로 사용합니다. 이는 컴파일러 검증, 완전한 Obsidian 링크 해석, 민감 데이터 탐지, 사실 검증을 대체하지 않습니다.
- OKC에는 여전히 첨부 파일, Canvas, Base 파일 보존과 링크의 완전한 재작성에 관한 릴리스 요구사항이 남아 있습니다.
- 해시 확인과 파일 교체는 외부 Obsidian이나 Sync 프로세스와의 운영체제 수준 compare-and-swap을 제공하지 않습니다. 같은 노트를 동시에 편집하지 마세요. 백업과 관측된 stale-hash 확인은 복구와 충돌 검토를 돕습니다.
- Node 경로 확인은 상위 디렉터리를 악의적으로 동시 교체하는 것으로부터의 완전한 격리를 주장하지 않습니다. 통제되지 않은 프로세스가 파일시스템을 교체하는 환경은 지원되지 않습니다.
- 삭제, 이름 변경, 자동 폴더 이동, 스냅샷 내보내기, OKC 승인, 컴파일, 시맨틱 검색은 현재 도구로 노출되지 않습니다.

## 설계 및 개발

이 프로젝트에는 공식 AI-DLC 2.7.1 Codex 워크플로우가 설치되어 있습니다. 새 Codex 대화에서 `$aidlc --doctor`를 실행하고 [설정 및 사용 가이드](docs/aidlc-setup.md)를 따르세요. 제품은 아직 Inception 검토를 기다리고 있으며, 워크플로우 설치가 Construction 진입 승인을 기록하지는 않습니다.

- [AWS AI-DLC 적용 제안서](aidlc-docs/methodology.md)
- [기존 Obsidian MCP 4종 소스 검토](aidlc-docs/inception/existing-mcp-research.md)
- [요구사항](aidlc-docs/inception/requirements.md) · [구현 설계](aidlc-docs/construction/design.md)
- [저장소 및 설치 UX](aidlc-docs/inception/repository-ux.md) · [기여 안내](CONTRIBUTING.md)
- [현재 상태 및 후속 유닛](aidlc-docs/state.md) · 요구사항 추적성(Construction 중 생성 예정)

이 제품은 [MCP SDK](https://github.com/modelcontextprotocol/typescript-sdk/tree/v1.30.0)로 프로토콜을 구현합니다. Obsidian 링크와 속성에 대한 사용자 대면 의미는 [공식 Obsidian Help 문서](https://obsidian.md/help/Linking%2Bnotes%2Band%2Bfiles/Internal%2Blinks)를 따릅니다. 실제 OKC 파서 동작과의 차이는 감사 제약 사항으로 문서화됩니다.
