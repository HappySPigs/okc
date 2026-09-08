# OKC 사용자를 위한 저장소·설치·운영 UX

설계 기준: 2026-09-06. 이 제품의 목적은 **OKC 입력에 적합한 Obsidian Vault를 작성하고 정리하는 것**이다. 아래의 초기 구현 목표와 이후 배포 권고를 구별하며 실제 지원 옵션과 검증 결과는 루트 README 및 Construction 기록을 따른다.

## 제품의 약속

사용자는 로컬 MCP 패키지를 설치하고, 편집할 Vault를 등록한 뒤, 노트 생성·수정과 입력 품질 점검을 사용한다. Obsidian plugin이나 REST API 키, native OKC 패키지를 기본 의존성으로 추가하지 않는다. Obsidian이 열려 있지 않아도 Markdown 파일로 작업할 수 있어야 한다.

로컬 stdio에서는 MCP 클라이언트가 서버 프로세스를 실행한다. 프로토콜 메시지는 stdout, 진단은 stderr로 분리한다. [MCP stdio transport](https://modelcontextprotocol.io/specification/2025-11-25/basic/transports)

첫 성공은 노트 하나를 만들고 입력 품질 점검을 실행한 다음 Obsidian에서 그 노트를 열어 확인하는 경험이다. 사용자는 컴파일러 프로젝트나 provider 설정을 먼저 만들 필요가 없다.

## 저장소는 독립적으로 관리

`okc-mcp`를 별도 저장소와 패키지로 유지한다. 소비자인 OKC의 입력 계약을 참고하지만 MCP를 쓰기 위해 Rust workspace나 형제 저장소의 특정 경로를 요구하지 않는다.

| 책임 | 소유 위치 |
| --- | --- |
| 노트 작성·수정, metadata·링크·중복 audit, 준비 변경 preview/apply | `okc-mcp` |
| 로컬 config, 경로 제한, 응답 상한, 설치·진단, MCP 연결 | `okc-mcp` |
| immutable snapshot, canonical IR, 승인 정책, compilation, artifact 검증 | OKC |
| 개인 노트·Obsidian 설정·cache·선택적 백업 | 사용자 로컬 저장소 |

OKC 입력 호환성은 지원 대상으로 문서화하고 fixture로 검증한다. OKC의 project journal이나 내부 SQLite를 MCP가 직접 관리하지 않는다. OKC의 현행 core는 source snapshot을 수집하고 이를 기반으로 통합·컴파일하므로, 편집 중인 source Vault와 수집된 snapshot의 시점을 구별한다. [OKC pipeline](https://github.com/dolgogae/okc/blob/7f87f7c81a72701710d8668b2569e51478dccc75/docs/specs/vault-compilation-pipeline.md)

## 사용자가 이해할 파일 경계

| 공간 | 의미 | 관리 방식 |
| --- | --- | --- |
| Editable Source Vault | 사용자가 Obsidian과 MCP로 작성하는 노트 | 명시적으로 등록한 경로 안에서 생성·수정 |
| MCP config | 접근할 Vault와 동작 범위 | 개인 절대 경로이므로 Git에 올리지 않음 |
| Preparation preview | 품질 점검 후 제안한 노트 변경 | 초기 구현은 각 쓰기 도구의 `dryRun`; 수정 시 전체 내용의 기준 hash 확인 |
| OKC snapshot/project | 특정 시점의 원본 수집과 통합 작업 | OKC가 소유하며 MCP authoring으로 변경하지 않음 |
| Compiled output | 통합 결과와 provenance | 이 source authoring MCP의 편집 대상으로 사용하지 않음 |

사용자의 기존 폴더 방식을 처음부터 전면 변경하지 않는다. 좋은 기본 구조와 최소한의 metadata를 제안하고, 기존 Vault에는 audit부터 적용한다. Markdown과 frontmatter만으로 의미를 보존하며 특정 Obsidian plugin의 실행 결과를 지식 본문으로 간주하지 않는다. 구체적인 note type·필드·폴더 권고는 Vault 설계 문서에서 정의한다.

동시에 열린 Obsidian과 MCP가 같은 노트를 수정할 수 있으므로 조회 시 받은 content hash를 수정 요청에 전달한다. 기준 hash가 달라졌으면 최신 내용을 다시 읽고 변경안을 재작성한다. 초기 Node 구현은 관측한 변경을 충돌로 거부하지만 외부 편집기와 OS 수준의 compare-and-swap을 공유하지 않는다. 최종 확인 직후의 동시 쓰기까지 방지한다고 보장할 수 없으므로 동일 노트의 동시 편집을 피하도록 안내하고 외부 state의 원본 백업 경로를 제공한다.

## 첫 설치의 흐름

초기 패키지 목표는 Node.js `>=22.13`의 `okc-mcp@0.1.0-alpha.1`이다. 공개 registry 배포 전이므로 첫 설치는 로컬 tarball로 검증한다. 아래 명령은 초기 CLI 계약이며 실제 완료 여부는 README와 테스트 결과로 확인한다.

1. 검증한 `okc-mcp-0.1.0-alpha.1.tgz`를 만든다.
2. `npm install -g ./okc-mcp-0.1.0-alpha.1.tgz`로 설치한다.
3. `okc-mcp config --vault /absolute/path/to/Vault`의 JSON을 config 파일로 저장한다.
4. `okc-mcp doctor --config /absolute/path/to/okc-mcp.json`으로 경로와 준비 상태를 확인한다.
5. `okc-mcp client-config --config /absolute/path/to/okc-mcp.json`이 출력한 `mcpServers` 설정을 클라이언트에 등록한다.
6. synthetic 또는 사용자가 선택한 Vault에서 목록 조회, 노트 생성, 입력 품질 점검을 수행한다.

서버 실행은 `okc-mcp serve --config /absolute/path/to/okc-mcp.json`이다. 생성한 클라이언트 설정은 Node 실행 파일과 entry 파일의 절대 경로를 사용하여 GUI 앱의 PATH 차이를 줄인다. 설정 생성은 내용을 출력하며 다른 앱의 설정을 자동 수정하지 않는다.

최초 사용자 흐름에는 OKC native SDK 빌드와 형제 레포 checkout을 넣지 않는다. OKC로 넘기는 단계는 별도 사용법으로 이어져야 하며, MCP의 준비 상태 점검과 실제 컴파일 완료 상태를 구별한다.

## 작성과 정리의 UX

작업의 데이터 변경 범위가 사용자와 MCP client에 드러나도록 도구를 나눈다.

| 작업 | 사용자 경험 | 데이터 변경 경계 |
| --- | --- | --- |
| 조회·검색·audit | 현재 노트와 발견한 문제를 확인 | 읽기만 수행 |
| 노트 생성 | 새 경로와 내용 확인 후 생성 | 기존 파일이 있으면 거부 |
| 노트 수정 | 현재 내용 기준의 변경 요청 | 기준 content hash가 일치할 때만 수정 |
| 노트 변경 preview | `dryRun: true`의 예상 결과·hash·경고 확인 | 적용 전에는 Vault를 변경하지 않음 |
| 노트 변경 apply | 동일한 요청에 `dryRun: false`로 적용 | 수정 대상의 기준 hash가 다르면 충돌 처리 |

metadata가 없다는 이유로 작성 의도를 추측해 의미를 바꾸거나 중복 후보를 자동 삭제하지 않는다. audit은 문제의 근거와 수정 선택지를 제공한다. 대규모 재배치나 의미 병합은 후속 기능으로 분리해 실제 복구 방식과 링크 보존을 먼저 설계한다.

preview와 hash 검사는 변경의 일관성 장치다. 현재 preview는 도구별 단일 노트 작업이며 preview token이나 batch transaction을 발행하지 않는다. 큰 노트 생성 preview는 잘릴 수 있고 frontmatter·전체 교체 preview는 요약과 hash를 반환하므로, 전체 diff UI나 여러 파일의 원자적 적용은 후속 기능이다. 자연어 확인 문자열이나 도구 인자만으로 실제 사람이 승인했다는 인증을 주장하지 않는다. MCP client의 도구 승인 UI와 사용자의 요청 범위가 실행 판단을 담당한다.

## 진단과 오류 메시지

문제와 다음 행동을 함께 표시한다. 아래 표는 UX 요구사항이며 모든 항목의 구현 여부는 테스트 근거로 확인한다.

| 상황 | 보여줄 핵심 | 다음 행동 |
| --- | --- | --- |
| Vault 경로 없음 | 등록 이름과 존재하지 않는 경로 | 실제 절대 경로로 config 수정 |
| 경로 이탈 또는 symlink | 등록 경로 안의 허용된 파일만 처리 가능 | 허용된 상대 경로 선택 |
| 생성 경로가 이미 존재 | 기존 노트를 덮어쓰지 않았음 | 기존 노트를 읽거나 다른 이름 선택 |
| 수정 hash 충돌 | 읽은 뒤 파일이 바뀌었음 | 최신 내용을 읽고 변경안 재작성 |
| 오래된 수정 요청 | preview 이후 대상의 hash가 바뀌었음 | 최신 내용을 읽고 새 preview 생성 |
| 불완전한 metadata | 어떤 필드·값이 입력 품질에 영향을 주는지 | 권고를 확인하고 수정 |
| 깨진 링크·중복 후보 | 대상과 근거를 제한된 목록으로 제공 | 대상 확인 후 명시적으로 정리 |
| 결과가 큼 | 잘린 범위와 결과 상한 | 더 좁은 검색 또는 다음 범위 요청 |

`doctor`는 파일 접근과 설정 준비 상태를 진단한다. 문제 진단 과정에서 사용자의 Vault를 재구성하지 않는다. 로그와 이슈 첨부에는 노트 본문, 토큰, 개인 링크를 기본 포함하지 않는다.

## 저장소에서 사용자가 먼저 보는 것

README는 **할 수 있는 일 → 지금 설치하기 → 첫 노트 작성 → 입력 품질 점검 → 제약과 문제 해결** 순서로 구성한다. 연구와 AI-DLC 기록은 별도 링크로 이어진다. alpha 상태와 실제 지원 플랫폼을 앞부분에서 밝힌다.

| 관리 자료 | 목적 |
| --- | --- |
| README | 설치·연결·첫 작성·현재 제한 |
| example config | 개인 경로가 없는 최소 설정 |
| synthetic Vault | 실제 개인 노트 없이 생성·수정·audit 재현 |
| CHANGELOG | 도구 계약·config·runtime·입력 profile 변화 |
| CONTRIBUTING | 작은 변경과 파일 쓰기 경계의 검사 방법 |
| issue templates | 설치·충돌·audit 문제를 최소 사례로 보고 |
| AI-DLC 기록 | 조사 근거, 요구사항, 결정, 구현·검증 상태 |

개인 Vault, `.obsidian`, OKC project, 토큰, 개인 절대 경로, cache와 백업을 npm 배포물에 포함하지 않는다. npm `files` allowlist와 tarball 목록 검사로 설치 범위를 검증한다. 예제 fixture는 배포 검증용으로 의도적으로 작성한 합성 데이터만 사용한다.

MCP package 버전과 권장 Vault profile 버전은 구별한다. profile 변경은 기존 노트를 자동 이관하지 않고 어떤 권고가 바뀌었는지 changelog와 새 preview로 설명한다.

## 배포와 업데이트

초기 단계는 로컬 alpha 설치 검증까지 수행한다. 이후 공개 배포는 다음 근거를 갖춘 릴리스 후보로 진행한다.

- `npm pack` 산출물을 빈 디렉터리에 설치하고 CLI 진단·stdio handshake·노트 생성·충돌 거부·audit을 확인한다.
- macOS arm64/x64, Linux x64, Windows x64는 실행 근거가 있는 조합만 지원 표에 올린다. 같은 JavaScript라는 사실로 플랫폼 검증을 대신하지 않는다.
- OKC 입력 호환성은 참조한 compiler 버전과 source fixture 수집 검증을 기록한다. 실제 compiler를 실행하지 않은 경우 문서·코드 분석에 근거한 설계임을 명시한다.
- 릴리스 노트에 바뀐 행동, 필요한 사용자 조치, 검증한 조합, 알려진 제한을 적는다.
- public npm 배포 시 OIDC trusted publishing과 provenance를 적용한다. provenance는 빌드 출처 연결이며 기능 정확성을 보증하지 않는다. [npm provenance 문서](https://docs.npmjs.com/generating-provenance-statements/)

공개 후에도 클라이언트는 사용자가 선택한 정확한 패키지 버전을 실행하도록 안내한다. 서버 시작 때 자동 업데이트하지 않는다. 이전 버전 재설치는 프로그램 복구이며 이미 적용한 노트 수정을 되돌리는 작업과 다르다고 설명한다. 파일 변경의 복구는 사용자의 기존 version history 또는 명시적으로 구현한 백업 범위에 맞춰 안내하고, 아직 없는 undo 기능을 약속하지 않는다.

공개 패키지 이름의 registry 가용성, npm 소유자, 공개 URL과 릴리스 상태는 실제 확인 전 확정 사실로 쓰지 않는다. 유지관리 문서는 선택한 라이선스와 `package.json`의 현재 값에 맞춘다.

## 유지관리의 성공 기준

아래는 측정할 항목이며 성과 수치가 아니다.

| 지표 | 측정 방식 |
| --- | --- |
| 첫 노트 작성까지의 마찰 | 깨끗한 환경에서 README만으로 완료한 단계와 장애 기록 |
| 충돌 방지 | 외부 편집 후 기존 hash 수정이 거부되는지 검증 |
| 준비 변경 이해도 | preview만 보고 어떤 파일이 왜 바뀌는지 설명할 수 있는지 검토 |
| 입력 품질 개선 | audit 전후 broken links·metadata 문제와 처리 결과 비교 |
| 업데이트 부담 | 설정 변경 필요 여부, 이전 패키지 재설치 가능 여부 |
| 지원 주장 정확성 | 플랫폼과 OKC 입력 profile별 실제 실행 근거 연결 |

원격 telemetry 없이 synthetic 재현과 최소 진단으로 초기 문제를 해결한다. 연결·충돌·입력 품질 피드백을 다음 Bolt의 요구사항으로 반영한다.
