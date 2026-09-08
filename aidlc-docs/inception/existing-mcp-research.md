# 기존 Obsidian MCP 구현 조사

조사일: 2026-09-06. 상태: Inception 조사 근거. 목적: **OKC가 처리하기 좋은 입력 Vault를 로컬에서 작성·관리하는 MCP**의 설계 근거를 얻는다. 컴파일 결과나 OKC 프로젝트 조작을 주목적으로 삼지 않는다.

## 조사 범위와 판정 기준

대표적인 REST bridge 2종과 직접 파일 접근 서버 2종을 GitHub에서 내려받아 아래 commit의 실제 진입점, 도구 등록, 파일 처리, 검색, 설정, 라이선스를 읽었다. README의 설치 설명과 코드에서 확인한 동작을 구분했다. 타 프로젝트 테스트 실행, 성능 측정, 배포 바이너리 검증, 전체 보안 감사는 수행하지 않았다. 문서에 있는 성능 수치를 실측으로 인용하지 않으며 소스 코드는 복사하지 않는다.

| 프로젝트 | 조사한 commit | 소스 내 버전 | 라이선스 확인 | 기본 구조 |
|---|---|---|---|---|
| MarkusPfundstein/mcp-obsidian | `5ee0b84fa8319fd2fdf0db0ee1febb065e712a15` | 0.2.2 | MIT, LICENSE 확인 | Python stdio → HTTP → Local REST API plugin |
| cyanheads/obsidian-mcp-server | `2d4e8d1114d650b45568caa9a163e6e853423266` | 3.5.1 | Apache-2.0, LICENSE 확인 | TypeScript MCP → service → Local REST API plugin |
| bitbonsai/mcpvault | `c5abeda9bed11864079f70ae7f33d134e294aad2` | 0.16.0 | MIT, LICENSE 확인 | Node.js stdio → 파일 서비스 → Vault |
| lstpsche/obsidian-mcp | `fea2e1f50a8a76d5232b40d07b654ae8037985c7` | 2.5.0 | MIT, LICENSE 확인 | Rust MCP → Vault/index → 파일시스템 |

버전은 package manifest 기준이다. npm/crates.io의 현재 게시 버전과 동일하다는 주장은 아니다. 근거: [Markus manifest][markus-package], [cyanheads manifest][cyan-package], [MCPVault manifest][bonsai-package], [Rust manifest][rust-package]. 라이선스: [Markus][markus-license], [cyanheads][cyan-license], [MCPVault][bonsai-license], [Rust][rust-license].

## 1. MarkusPfundstein: 작은 REST wrapper

**코드 확인.** `server.py`가 도구 15개를 등록하고 stdio 요청을 각 handler의 동기 `run_tool()`로 보낸다. `obsidian.py`는 `requests`로 `/vault/`, `/search/simple/`, JSONLogic `/search/`, periodic endpoint를 호출한다. 별도 로컬 검색 index는 없고 검색과 frontmatter 해석은 Obsidian 측 API가 맡는다. API key는 환경변수에서 받아 Bearer header로 전달한다. 기본 host는 loopback이며 HTTPS certificate 검증은 기본 비활성, 연결/읽기 timeout은 3초/6초다. [서버][markus-server], [HTTP 서비스][markus-service]

**편집 로직.** append는 POST, 전체 쓰기는 PUT, 부분 편집은 PATCH다. PATCH는 heading/block/frontmatter locator를 header로 전달한다. 중첩 heading의 bare name이 실패하면 원문 heading hierarchy를 읽어 후보가 하나일 때만 재시도한다. 중복 heading은 모호성 오류를 낸다. 삭제 도구는 `confirm=true`를 요구하지만 이는 모델이 제출하는 도구 인자이므로 별도 사람 승인의 증거로 볼 수 없다. [HTTP 서비스][markus-service], [도구 정의][markus-tools]

**경로·동시성.** wrapper는 문자열 경로를 URL에 결합한다. Vault containment와 filesystem 원자성은 upstream 구현에 의존한다. 확인한 도구/서비스에는 사용자가 읽은 revision의 `expectedHash` 검사가 없다. 코드 확인 범위 밖인 REST plugin의 보장까지 추정하지 않는다. [HTTP 서비스][markus-service]

**설치 UX—README 확인.** Python 3.11+, uvx, Obsidian Local REST API plugin 활성화, API key 설정이 필요하다. Docker 설정도 제공한다. Python MCP SDK는 1.x로 제한되어 있다. OKC 입력 작성 도구가 이를 기반으로 하면 Obsidian 실행, plugin, transport 설정까지 장애 원인이 늘어난다. [README][markus-readme], [manifest][markus-package]

**가져올 교훈.** 작은 도구→서비스 구조와 모호한 heading을 자동 선택하지 않는 원칙은 유용하다. REST 의존, 무조건적인 동기 I/O, 모델이 채울 수 있는 확인 boolean은 신규 MCP의 기본 구조로 채택하지 않는다.

## 2. cyanheads: 구조화된 편집과 사용 가능한 기능의 노출

**코드 확인.** MCP 도구 정의와 `ObsidianService`가 분리되어 있다. 실제 데이터 접근은 undici HTTP client로 Local REST API에 위임한다. startup에서 Omnisearch 연결을 검사한 뒤 성공했을 때만 검색 schema에 해당 mode를 넣는다. text/JSONLogic 검색 결과는 읽기 경로 정책을 적용한 후 cursor pagination한다. 자체 영속 index가 검색의 주체인 구조는 아니다. [진입점][cyan-index], [서비스][cyan-service], [검색 도구][cyan-search]

**편집 UX.** 전체 쓰기는 `overwrite=false`가 기본이고 기존 파일이 있으면 오류를 반환한다. `document-map`으로 heading/block/frontmatter 위치를 조회한 뒤 section을 편집할 수 있다. 단일 heading 이름은 유일한 경우만 허용한다. frontmatter는 `yaml.parseDocument`를 사용하여 변경하지 않은 YAML 노드의 주석·인용 스타일을 최대한 보존하며, 본문만 편집할 때 raw frontmatter를 그대로 이어 붙이는 별도 경로가 있다. YAML 전체 재출력을 완전한 byte 보존과 동일시해서는 안 된다. [쓰기 도구][cyan-write], [frontmatter 처리][cyan-frontmatter]

**안전·동시성.** read/write 경로 allowlist와 read-only 모드가 service에서도 적용된다. command palette 도구는 별도 opt-in이다. 삭제는 framework의 입력 요청과 응답으로 확인 절차를 구성한다. HTTP retry는 GET/PUT/DELETE에 제한하고 POST/PATCH를 제외하여 append 중복 실행을 피한다. 확인한 서비스/도구에는 revision hash나 `If-Match` 기반 사용자 편집 충돌 검사가 없다. 파일 존재 확인 후 PUT은 원자적인 create-if-absent와 다르다. [경로 정책][cyan-policy], [삭제 도구][cyan-delete], [서비스][cyan-service], [쓰기 도구][cyan-write]

**설치 UX—문서·manifest 확인.** npx 설정 외에 `.mcpb` bundle manifest와 클라이언트 설치 링크가 있다. Node.js 24+ 또는 Bun과 Obsidian plugin/API key가 필요하다. bundle의 사용자 설정 항목에는 필요한 env 변수가 선언되어 있다. bundle이 실제 모든 클라이언트에서 동작하는지는 이번 조사에서 실행 검증하지 않았다. [README][cyan-readme], [bundle manifest][cyan-bundle]

**가져올 교훈.** 생성과 기존 파일 교체를 나누는 기본값, outline→부분 편집, 제한된 응답 크기, 실제 가능한 기능만 노출하는 방식을 채택한다. 설치 폼과 복구 가능한 오류 메시지는 참고하되 HTTP/plugin 의존을 기본에 넣지 않는다.

## 3. MCPVault: 설치가 단순한 직접 파일 접근

**코드 확인.** Node stdio 진입점이 `createServer()`를 생성하며 총 18개 도구를 정의한다. read/write/patch/frontmatter/tags 외에 outline, line-range 읽기, wikilink 해석 도구가 있다. `--read-only`는 목록에서 mutation 도구를 숨기는 동시에 직접 호출도 차단한다. stdin 종료와 signal을 처리하여 클라이언트 종료 후 고아 프로세스를 줄인다. [진입점][bonsai-server], [서버와 도구][bonsai-create]

**검색 로직.** 매 검색마다 Markdown 파일을 재귀 열거하고 path policy를 적용한 다음 5개씩 병렬 읽기한다. 본문/파일명 substring 후보를 만들고 호출 시 계산한 문서 빈도로 BM25 식을 적용해 재정렬한다. 기본 결과 5개, 최대 20개다. persistent index나 watcher가 이 검색 서비스의 전제가 아니다. 순회 단계와 허용 여부 필터가 분리되어 있으므로 결과 제한이 전체 I/O 제한을 뜻하지 않는다. [검색 서비스][bonsai-search]

**편집 로직.** append/prepend와 frontmatter patch에서 원본 YAML을 `parseDocument`로 편집한다. 그러나 전체 쓰기의 기본 mode는 overwrite이며 기존 노트를 읽지 못한 경우 append 경로가 신규 쓰기로 진행하는 catch가 있다. frontmatter parse 실패를 빈 metadata와 원문 본문으로 바꾸는 fallback도 있다. 작성용 MCP에서는 parse 실패나 권한 오류를 “없는 파일”과 구분해야 한다. 일반 note write/patch는 직접 `writeFile`하고 `expectedHash` 검사나 파일별 transaction은 확인되지 않았다. [파일 서비스][bonsai-fs], [frontmatter][bonsai-frontmatter]

**경로 로직.** Vault root를 realpath로 고정하고 lexical containment 및 기존 대상 symlink의 실제 위치를 검사한다. Vault 내부를 가리키는 symlink는 허용한다. 신규 대상은 직계 parent의 realpath를 시도하지만 parent도 없으면 해당 오류를 일부 무시한다. 이는 모든 기존 ancestor 검사 또는 symlink 전면 거부의 근거가 되지 않는다. `.obsidian`, `.git`, `node_modules` 등의 경로 제한은 별도 `PathFilter`가 처리한다. [파일 서비스][bonsai-fs], [경로 필터][bonsai-filter]

**설치 UX—README·manifest 확인.** Node.js 20+에서 npx + Vault 경로로 실행한다. Obsidian 앱이나 plugin을 필수로 하지 않는다. 경로를 생략하면 cwd를 사용한다. 간편 실행은 참고하되 OKC 작성 MCP는 엉뚱한 디렉터리 변경을 막기 위해 명시적인 Vault 경로를 요구하는 편이 낫다. [README][bonsai-readme], [진입점][bonsai-server]

**가져올 교훈.** 직접 FS + stdio, 부분 읽기, 간결한 결과, 서버 수준 read-only는 잘 맞는다. permissive한 경로 보정, parse 오류 은폐, 기본 overwrite는 채택하지 않는다. 작은 Vault의 MVP는 bounded scan으로 시작하고 실제 규모 측정 이후 index를 결정할 수 있다.

## 4. lstpsche: Rust index와 실시간 갱신

**코드 확인.** `Vault::open`에서 metadata index를 만들고 선택적으로 Tantivy index를 구성한다. metadata에 note/tag/link/backlink 구조를 유지한다. Tantivy는 memory index이며 제목/heading/body/frontmatter 필드, English `en_stem` tokenizer와 필드별 가중치를 사용한다. notify watcher는 500ms debounce와 용량 256의 channel을 통해 변경 파일을 갱신하고 삭제 파일을 index에서 제거한다. 한 batch의 Tantivy 변경 후 flush한다. [Vault startup][rust-vault], [metadata index][rust-index], [Tantivy][rust-tantivy], [watcher][rust-watcher]

**편집·경로.** 경로 component별 canonicalization, NFC key 비교, 신규 파일의 가장 가까운 기존 ancestor 확인이 있다. 모호한 Unicode 정규화 이름은 오류다. Vault 안 symlink를 무조건 배제하는 정책은 아니다. 파일 쓰기는 `fs::write`, append는 `OpenOptions`, delete는 직접 제거다. frontmatter 수정은 JSON 값으로 parse 후 다시 serialize하므로 YAML 주석 보존에 적합한 기본값은 아니다. 일반 write에 optimistic hash 조건은 확인되지 않았다. [경로][rust-path], [파일 처리][rust-fs], [frontmatter][rust-frontmatter]

**운영 특성.** `OBSIDIAN_TOOLS=read` 등의 profile은 tool router를 제한한다. 하지만 startup은 `.obsidian-mcp`와 ignore 파일을 만들며 외부 data directory 설정을 해도 Vault 내부 `.obsidian-mcp` 생성은 남는다. “읽기 도구만 노출”과 “Vault에 한 바이트도 쓰지 않음”은 다른 성질이다. [tool 구성][rust-tools], [startup][rust-vault]

**설치 UX—README 확인.** cargo install 또는 OS별 release binary를 안내한다. stdio 외에 공유 HTTP server/daemon 모드도 제공하며 semantic embedding은 추가 feature와 runtime 선택이 있다. startup 시간에 대한 README 수치는 이번 조사에서 재현하지 않았다. 한국어 Vault 품질은 English tokenizer의 존재만으로 보장할 수 없다. [README][rust-readme], [Tantivy][rust-tantivy]

**가져올 교훈.** 향후 대규모 검색은 metadata/index/watcher 분리와 갱신 상태 노출을 참고한다. MVP부터 semantic daemon·모델 다운로드·HTTP 운영을 추가하지 않는다. cache, lock, backup은 사용자의 노트와 분리된 local state directory에 둔다.

## 5. OKC 입력 작성 MCP에 적용할 결정

다음은 조사 결과에 근거한 **신규 프로젝트 설계 제안**이며 기존 서버의 기능 주장과 구분한다.

| 항목 | 적용 제안 | 이유 |
|---|---|---|
| 기본 연결 | 설치형 Node.js CLI + stdio + 명시적 Vault root | Obsidian을 실행하지 않아도 작성·조회 가능. API key와 별도 listener 불필요 |
| 관리 대상 | 살아 있는 authoring Vault만 명시적으로 연결 | OKC가 확보한 immutable snapshot, project 내부, compiled artifact는 별도 영역 |
| 최소 작성 기능 | create note, read, search, outline, section patch, frontmatter patch | 입력의 구조와 근거를 점진적으로 개선하는 데 집중 |
| 원본 보존 | 알려지지 않은 properties·YAML 주석·CRLF·본문 공백 보존, malformed YAML은 수정 중지 | 사용자·plugin metadata를 임의 변환하지 않음 |
| 충돌 | 읽을 때 content hash 반환, 수정 시 expected hash 요구, 동일 파일 MCP 쓰기 직렬화 | 오래된 모델 응답이 새로운 노트를 조용히 덮는 경우 감지 |
| 쓰기 게시 | 새 파일 exclusive create, 기존 파일은 같은 filesystem staging 후 원자 교체·직전 상태 재검사 | 생성 충돌과 중단 중 partial write를 줄임 |
| 파일 경계 | path traversal·특수파일·모든 symlink·Unicode/case 충돌 거부, root identity 재검사 | OKC 입력 경계와 맞추고 모든 도구에 같은 검사 사용 |
| 검색·연결 | 제한된 결과+outline→부분 읽기, ambiguous wikilink 보고 | 작은 문맥으로 필요한 근거를 찾고 잘못된 자동 연결 방지 |
| 준비도 점검 | `readiness`에서 경로·UTF-8·YAML·링크/첨부 한계 등을 근거와 함께 보고 | 폴더를 자동 이동하지 않고 OKC 입력 장애를 수정 가능하게 안내 |
| 도구의 권한 | mutation 요청과 준비도 점검을 구분; read-only 옵션은 handler에서도 강제 | tool annotation은 enforcement나 human consent의 대체가 아님 |
| 사용자 경험 | 진단 명령, 버전 고정 설치 예시, 설정 snippet, 외부 state 위치 안내 | 설치→연결 확인→첫 노트 작성→readiness까지 설명 가능 |

**동시 편집의 실제 한계.** hash 확인 + atomic rename은 OS 차원의 compare-and-swap이 아니다. 외부 Obsidian/Sync 프로세스가 마지막 검사 직후 편집하는 경쟁을 완전히 제거하지 못한다. MVP는 보장 범위를 “MCP 내부 직렬화와 관측 가능한 stale hash 충돌 감지”로 한정하고 충돌 시 자동 overwrite/retry를 금지해야 한다. backup·복구 정책과 외부 writer 협조 없이는 “어떤 동시 편집도 잃지 않는다”는 주장을 하지 않는다. symlink에 대해서도 path 검사만으로 적대적인 concurrent 교체에 완전 격리를 주장하지 않는다.

**OKC와의 경계.** sibling `ObsidianKnowlegeComplication`의 README와 보안 계약은 컴파일러가 source snapshot을 불변으로 다루며, UTF-8/NFC·symlink·resource bounds를 검증한다고 명시한다. 이는 사용자가 authoring Vault를 편집하지 못한다는 뜻이 아니다. MCP는 사용자가 명시적으로 연 **작성 공간**을 개선하고, OKC가 source로 확보한 **불변 snapshot**을 수정하지 않는다. 준비도 결과도 OKC compiler의 실제 ingestion/verification을 대체하는 인증서가 아니다. 현재 OKC는 Markdown materialization 단계이므로 첨부·Canvas·Base의 완전 호환을 약속해서는 안 된다. [로컬 OKC README](../../../ObsidianKnowlegeComplication/README.md), [보안 계약](../../../ObsidianKnowlegeComplication/docs/specs/security-and-trust-boundaries.md)

## 6. 구현 전후 검증으로 연결할 사례

- 노트를 읽은 뒤 사용자가 수정하면 이전 hash로 section/frontmatter patch 시 충돌 오류가 나고 현재 bytes가 유지된다.
- 생성 대상이 이미 있으면 overwrite 인자를 묵시적으로 보정하지 않고 거부한다.
- 수정하지 않은 frontmatter 주석·quoted value·unknown property, 본문과 CRLF가 보존된다. malformed YAML 수정은 실패한다.
- 중복 heading·동명이인 wikilink·정규화 충돌 경로는 후보를 보여 주고 자동 선택하지 않는다.
- Vault 밖 symlink, 내부 symlink, symlink parent, `.git`/plugin 경로는 create/read/search/patch 모두에서 같은 정책으로 처리된다.
- 준비도 점검과 startup이 Vault를 바꾸지 않는다. 캐시·로그·잠금·백업의 허용 위치를 확인한다.
- 한국어·한영 혼합 제목/본문, 대형 노트, 많은 파일에서 결과 크기·scan budget·응답 취소가 동작한다.
- MCP mutation, 파일 삭제/이동, 원격 model 호출 범위는 문서화된 기능에 한정한다. 노트 안 명령문은 tool authority로 처리하지 않는다.

[markus-package]: https://github.com/MarkusPfundstein/mcp-obsidian/blob/5ee0b84fa8319fd2fdf0db0ee1febb065e712a15/pyproject.toml
[markus-license]: https://github.com/MarkusPfundstein/mcp-obsidian/blob/5ee0b84fa8319fd2fdf0db0ee1febb065e712a15/LICENSE
[markus-readme]: https://github.com/MarkusPfundstein/mcp-obsidian/blob/5ee0b84fa8319fd2fdf0db0ee1febb065e712a15/README.md
[markus-server]: https://github.com/MarkusPfundstein/mcp-obsidian/blob/5ee0b84fa8319fd2fdf0db0ee1febb065e712a15/src/mcp_obsidian/server.py
[markus-service]: https://github.com/MarkusPfundstein/mcp-obsidian/blob/5ee0b84fa8319fd2fdf0db0ee1febb065e712a15/src/mcp_obsidian/obsidian.py
[markus-tools]: https://github.com/MarkusPfundstein/mcp-obsidian/blob/5ee0b84fa8319fd2fdf0db0ee1febb065e712a15/src/mcp_obsidian/tools.py
[cyan-package]: https://github.com/cyanheads/obsidian-mcp-server/blob/2d4e8d1114d650b45568caa9a163e6e853423266/package.json
[cyan-license]: https://github.com/cyanheads/obsidian-mcp-server/blob/2d4e8d1114d650b45568caa9a163e6e853423266/LICENSE
[cyan-readme]: https://github.com/cyanheads/obsidian-mcp-server/blob/2d4e8d1114d650b45568caa9a163e6e853423266/README.md
[cyan-bundle]: https://github.com/cyanheads/obsidian-mcp-server/blob/2d4e8d1114d650b45568caa9a163e6e853423266/manifest.json
[cyan-index]: https://github.com/cyanheads/obsidian-mcp-server/blob/2d4e8d1114d650b45568caa9a163e6e853423266/src/index.ts
[cyan-service]: https://github.com/cyanheads/obsidian-mcp-server/blob/2d4e8d1114d650b45568caa9a163e6e853423266/src/services/obsidian/obsidian-service.ts
[cyan-frontmatter]: https://github.com/cyanheads/obsidian-mcp-server/blob/2d4e8d1114d650b45568caa9a163e6e853423266/src/services/obsidian/frontmatter-ops.ts
[cyan-policy]: https://github.com/cyanheads/obsidian-mcp-server/blob/2d4e8d1114d650b45568caa9a163e6e853423266/src/services/obsidian/path-policy.ts
[cyan-write]: https://github.com/cyanheads/obsidian-mcp-server/blob/2d4e8d1114d650b45568caa9a163e6e853423266/src/mcp-server/tools/definitions/obsidian-write-note.tool.ts
[cyan-delete]: https://github.com/cyanheads/obsidian-mcp-server/blob/2d4e8d1114d650b45568caa9a163e6e853423266/src/mcp-server/tools/definitions/obsidian-delete-note.tool.ts
[cyan-search]: https://github.com/cyanheads/obsidian-mcp-server/blob/2d4e8d1114d650b45568caa9a163e6e853423266/src/mcp-server/tools/definitions/obsidian-search-notes.tool.ts
[bonsai-package]: https://github.com/bitbonsai/mcpvault/blob/c5abeda9bed11864079f70ae7f33d134e294aad2/package.json
[bonsai-license]: https://github.com/bitbonsai/mcpvault/blob/c5abeda9bed11864079f70ae7f33d134e294aad2/LICENSE
[bonsai-readme]: https://github.com/bitbonsai/mcpvault/blob/c5abeda9bed11864079f70ae7f33d134e294aad2/README.md
[bonsai-server]: https://github.com/bitbonsai/mcpvault/blob/c5abeda9bed11864079f70ae7f33d134e294aad2/server.ts
[bonsai-create]: https://github.com/bitbonsai/mcpvault/blob/c5abeda9bed11864079f70ae7f33d134e294aad2/src/createServer.ts
[bonsai-search]: https://github.com/bitbonsai/mcpvault/blob/c5abeda9bed11864079f70ae7f33d134e294aad2/src/search.ts
[bonsai-fs]: https://github.com/bitbonsai/mcpvault/blob/c5abeda9bed11864079f70ae7f33d134e294aad2/src/filesystem.ts
[bonsai-frontmatter]: https://github.com/bitbonsai/mcpvault/blob/c5abeda9bed11864079f70ae7f33d134e294aad2/src/frontmatter.ts
[bonsai-filter]: https://github.com/bitbonsai/mcpvault/blob/c5abeda9bed11864079f70ae7f33d134e294aad2/src/pathfilter.ts
[rust-package]: https://github.com/lstpsche/obsidian-mcp/blob/fea2e1f50a8a76d5232b40d07b654ae8037985c7/Cargo.toml
[rust-license]: https://github.com/lstpsche/obsidian-mcp/blob/fea2e1f50a8a76d5232b40d07b654ae8037985c7/LICENSE
[rust-readme]: https://github.com/lstpsche/obsidian-mcp/blob/fea2e1f50a8a76d5232b40d07b654ae8037985c7/README.md
[rust-vault]: https://github.com/lstpsche/obsidian-mcp/blob/fea2e1f50a8a76d5232b40d07b654ae8037985c7/src/vault/mod.rs
[rust-index]: https://github.com/lstpsche/obsidian-mcp/blob/fea2e1f50a8a76d5232b40d07b654ae8037985c7/src/vault/index.rs
[rust-tantivy]: https://github.com/lstpsche/obsidian-mcp/blob/fea2e1f50a8a76d5232b40d07b654ae8037985c7/src/vault/tantivy_index.rs
[rust-watcher]: https://github.com/lstpsche/obsidian-mcp/blob/fea2e1f50a8a76d5232b40d07b654ae8037985c7/src/vault/watcher.rs
[rust-path]: https://github.com/lstpsche/obsidian-mcp/blob/fea2e1f50a8a76d5232b40d07b654ae8037985c7/src/vault/path.rs
[rust-fs]: https://github.com/lstpsche/obsidian-mcp/blob/fea2e1f50a8a76d5232b40d07b654ae8037985c7/src/vault/fs.rs
[rust-frontmatter]: https://github.com/lstpsche/obsidian-mcp/blob/fea2e1f50a8a76d5232b40d07b654ae8037985c7/src/vault/frontmatter.rs
[rust-tools]: https://github.com/lstpsche/obsidian-mcp/blob/fea2e1f50a8a76d5232b40d07b654ae8037985c7/src/tools/mod.rs
