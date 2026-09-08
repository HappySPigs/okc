# OKC 입력에 적합한 Obsidian Source Vault 설계

상태: inception 설계 제안. 조사일: 2026-09-06. 조사 대상은 `../ObsidianKnowlegeComplication`, commit `7f87f7c81a72701710d8668b2569e51478dccc75`이다. 이 문서의 작성 관습과 MCP 기능은 OKC에 이미 구현된 기능이라는 의미가 아니다.

## 1. 제품의 역할

이 MCP의 사용자는 Obsidian에서 지식을 수집하고 다듬으며, 나중에 여러 Vault를 OKC로 통합하려는 사람이다. 핵심 작업은 **좋은 원천 노트 작성 → 연결 및 출처 정리 → 통합 전 점검**이다. 프로젝트 상태 조회, 승인된 plan의 compile, 결과 Vault 검증은 OKC가 담당하는 후속 작업이며 이 MCP의 MVP 중심 기능이 아니다.

따라서 권장 구현은 로컬 stdio로 실행하는 TypeScript 기반 Obsidian authoring MCP다. 직접 파일 접근으로 Obsidian 실행 여부와 독립적으로 동작한다. 작성 중인 live Vault는 사용자가 허용한 범위에서 편집한다. OKC에 전달하는 snapshot은 별도 불변 입력으로 취급한다. 원천 Vault를 컴파일러가 수정하지 않는 `REQ-SNP-001`과, 작성 도구를 통한 일상적인 편집을 구분한다.

OKC의 Node 바인딩은 프로젝트/통합/승인/컴파일을 위한 API이며 `list/read/write/search/lint` 같은 노트 작성 API가 없다. 이 MCP에 native addon을 필수 설치하게 만들면 사용자가 요청한 작성 경험과 무관한 설치 부담이 생긴다. 향후 실제 corpus 검증이 필요하면 공개 Rust `CorpusBuilder::build`를 사용하는 선택적 호환성 검사기를 추가한다. JS lint의 통과를 코어 검사 통과와 동일시하지 않는다.

근거: [제품 범위와 요구사항](../../../ObsidianKnowlegeComplication/docs/specs/product-and-scope.md), [공개 SDK](../../../ObsidianKnowlegeComplication/docs/specs/public-sdk-and-cli.md), [Node API 선언](../../../ObsidianKnowlegeComplication/bindings/node/index.d.ts), [CorpusBuilder](../../../ObsidianKnowlegeComplication/crates/okc-core/src/corpus.rs).

## 2. 현재 OKC가 실제로 읽는 것

| 관찰 | 작성 MCP에 주는 의미 | 로컬 근거 |
|---|---|---|
| Markdown은 strict UTF-8이며 frontmatter는 JSON으로 표현 가능한 YAML mapping이다. 중복 YAML key는 실패한다. | 새 파일은 UTF-8/LF로 쓰고 metadata는 mapping, 중복 key, 자료형을 검사한다. 기존 문서 전체를 불필요하게 포맷하지 않는다. | [parse.rs](../../../ObsidianKnowlegeComplication/crates/okc-core/src/parse.rs) `parse_markdown`, `parse_frontmatter` |
| 일부 잘못된 YAML 또는 mapping이 아닌 frontmatter는 opaque 경고 후 typed metadata에서 빠진다. | authoring lint는 이를 오류로 보고한다. 자동으로 빈 metadata로 바꾸면 출처를 잃을 수 있다. | [parse.rs](../../../ObsidianKnowlegeComplication/crates/okc-core/src/parse.rs) `opaque_frontmatter`; [corpus.rs](../../../ObsidianKnowlegeComplication/crates/okc-core/src/corpus.rs) `prepare_from_plan` |
| `title` 문자열과 `aliases`, `tags`의 문자열/문자열 배열이 특별히 추출된다. 배열의 문자열 아닌 항목은 lookup에서 빠진다. | aliases/tags는 항상 문자열 배열을 권장한다. 제목은 frontmatter title → 첫 heading → filename 순서이다. | [parse.rs](../../../ObsidianKnowlegeComplication/crates/okc-core/src/parse.rs) `string_list_field`, `parse_markdown` |
| 다른 metadata도 typed 값으로 corpus에 보존된다. 최상위 배열은 원소마다 별도 metadata disposition 대상이다. | authoring 관습 필드는 선택적으로 넣는다. 필드를 늘리면 검토해야 하는 지식도 늘어난다. | [corpus.rs](../../../ObsidianKnowlegeComplication/crates/okc-core/src/corpus.rs), `REQ-INT-002` |
| 모든 Markdown 문서는 정확히 한 taxonomy cluster에 포함되며 singleton도 합성·critic·승인이 필요하다. | 빈 템플릿, 운영 README, generated index를 원천 지식 폴더에 대량 생성하지 않는다. folder 이름이나 `status: draft`가 자동 제외를 뜻하지 않는다. | [제품 범위](../../../ObsidianKnowlegeComplication/docs/specs/product-and-scope.md), `REQ-INT-001/004` |
| `.obsidian/**`, `.git/**`, 비밀 파일 유형은 제외되지만 Vault/상위/global ignore는 입력 구성에 영향을 주지 않는다. | `.gitignore`로 template나 draft를 OKC에서 제외한다고 설명하면 안 된다. 템플릿과 MCP 작업 데이터는 Vault 외부에 둔다. | [snapshot.rs](../../../ObsidianKnowlegeComplication/crates/okc-core/src/snapshot.rs) `collect_directory`, `is_excluded`; [보안 명세](../../../ObsidianKnowlegeComplication/docs/specs/security-and-trust-boundaries.md) |
| 현재 블록 scanner는 ATX heading, 빈 줄로 나뉜 문단, fenced code를 evidence block으로 나눈다. | `## 주장`, `## 근거`와 명확한 문단 경계를 사용한다. 한 블록에 여러 독립 주장을 빽빽하게 섞지 않는다. | [parse.rs](../../../ObsidianKnowlegeComplication/crates/okc-core/src/parse.rs) `scan_blocks` |
| Markdown links, wikilinks, embeds, heading과 block suffix를 분석한다. code span/fence 안의 `[[...]]`는 링크가 아니다. | link/rename 작업은 문법과 범위를 인식해야 한다. 문서 전체 regex 치환은 금지한다. | [ALG-NRM-001](../../../ObsidianKnowlegeComplication/docs/algorithms/stable/markdown-canvas-and-link-normalization.md) |
| lookup은 source 안의 경로, 정규화된 경로, filename/title/alias 후보를 사용한다. 후보가 여러 개면 추측하지 않는다. | 서로 다른 뜻의 동일 제목을 구체화하고, 필요할 때 문서 상대 경로를 쓴다. 충돌한 alias를 자동 병합하지 않는다. | [plan.rs](../../../ObsidianKnowlegeComplication/crates/okc-core/src/plan.rs) `resolve_document_candidates`, `fragment_matches` |
| `^block-id`는 마지막 토큰의 ASCII 영숫자, `-`, `_`를 인식한다. | anchor가 필요한 근거에만 간결한 명시적 ID를 붙인다. 이 ID를 코어의 hash 기반 BlockId와 혼동하지 않는다. | [parse.rs](../../../ObsidianKnowlegeComplication/crates/okc-core/src/parse.rs) `explicit_block_id` |
| attachments/Canvas/Base는 입력 안전성 처리에 존재하지만 현재 출력 materializer는 Markdown-only다. | 첨부 설명과 핵심 근거를 Markdown에도 적는다. 그래프와 Canvas만으로 지식을 표현하면 현 출력에서 보장할 수 없다. | [현재 상태](../../../ObsidianKnowlegeComplication/docs/CURRENT_STATE.md), `REQ-PAR-002/003`, `REQ-INT-006` |

## 3. 권장 Vault 구조

폴더 구조는 시작을 돕는 기본값이며 기존 사용자에게 강제할 migration 규칙이 아니다. PARA 등 기존 구성을 읽고 lint할 수 있어야 한다. OKC가 기대하는 것은 특정 폴더 이름이 아니라 안전한 원천 파일, 명확한 문서/블록, 근거와 보존 가능한 metadata다.

```text
KnowledgeWorkspace/
├── PersonalVault/               # 평소 Obsidian에서 편집하는 원천 Vault
│   ├── .obsidian/               # 사용자 Obsidian 설정; MCP 편집 대상 아님
│   ├── inbox/                   # 새 메모; 아직 덜 정리된 실제 지식
│   ├── notes/                   # 개념·경험·결정 노트
│   │   └── architecture/
│   │       └── Immutable snapshots.md
│   ├── sources/                 # 문헌·회의·관찰별 출처 노트
│   ├── maps/                    # 사람이 해설하는 소수의 주제별 길잡이
│   └── assets/                  # 원문 PDF·이미지 등; 출력 제한 점검 대상
├── authoring-config/            # MCP 설정·작성 profile·템플릿, Vault 밖
├── authoring-state/             # 재생성 가능 index·변경 backup·점검 report
├── snapshots/                   # 후속 export로 생성하는 고정 입력
└── compilation/                 # 후속 OKC 프로젝트와 출력
```

`notes/`를 OKC 출력의 `knowledge/`와 같게 만들 이유는 없다. 작성 중인 분류는 바뀔 수 있고, OKC taxonomy는 별도 제안과 승인으로 결정된다. `inbox/`와 `maps/`도 선택한 source 안에 있으면 통합 대상이다. 단순 파일 목록을 수백 개의 MOC Markdown으로 자동 생성하지 않는다. 실제 문맥을 설명하는 길잡이만 노트로 남긴다.

snapshot export는 별도 후속 기능으로 둔다. 작성 Vault와 snapshot 경로를 같은 root로 지정할 수 없게 하고, export 이후의 live 편집은 기존 snapshot을 바꾸지 않는다. 작성 중 여러 파일이 동시에 변하면 단순 recursive copy로 일관된 시점을 보장할 수 없다. 향후 구현에서는 파일 hash 검증/변경 감지 후 재시도 또는 실패, 새 목적지에만 게시하는 규칙이 필요하다. Vault symlink는 순회하지 않으며 `.obsidian/plugins`나 실행 파일을 실행하지 않는다. `sourceId`는 snapshot/프로젝트의 Vault 식별자이며 모든 노트에 반복할 필드가 아니다. 실제 문법은 `[A-Za-z0-9_.-]`, 1–128 bytes, `.`와 `..` 제외이다.

근거: [snapshot 알고리즘](../../../ObsidianKnowlegeComplication/docs/algorithms/stable/snapshot-identity-and-hashing.md), [source.rs](../../../ObsidianKnowlegeComplication/crates/okc-core/src/source.rs), `REQ-SNP-001/002`, `REQ-SRC-001/002`, `REQ-APP-002`.

## 4. 노트 작성 profile

기본은 작은 metadata와 자립적인 본문이다. 아래 `type`, `created`, `source`, `license`는 **이 MCP의 선택적 작성 관습**이며 OKC가 요구하거나 승인으로 해석하는 key가 아니다. OKC의 `document_id`, `snapshot_id`, `content_hash`, `approved` 등을 수동으로 만들어 넣지 않는다.

일반 노트 예시:

```markdown
---
title: Immutable snapshots
aliases:
  - 불변 스냅샷
tags:
  - knowledge/compilation
type: concept
---
# Immutable snapshots

원천을 바꾸지 않고 특정 시점의 내용을 고정하면 통합 결과가 어떤 입력에서 나왔는지 확인할 수 있다.

## 근거

OKC의 source 계약은 compiler가 원천의 bytes, permissions, timestamps를 수정하지 못하게 한다. 근거: [[sources/OKC source contract#원천 불변성]]. ^source-immutability

## 적용 범위

작성 중인 노트의 편집과, 이미 통합에 사용한 snapshot의 보존은 서로 다른 작업이다.

## 열린 질문

동시에 편집 중인 Vault를 일관된 snapshot으로 넘기는 방법은 별도 export 설계가 필요하다.
```

출처 노트에는 사용자에게서 받은 사실만 기록한다. URL, 저자, 발행일, 열람일, 라이선스를 추측하여 채우지 않는다. 개인정보나 토큰을 `source` 필드에 복사하지 않으며 URL의 credential/query 민감값도 점검한다.

```markdown
---
title: OKC source contract
type: source
source: local:ObsidianKnowlegeComplication/docs/specs/product-and-scope.md
---
# OKC source contract

## 원천 불변성

REQ-SNP-001은 compiler가 원천 Vault의 bytes, permissions, timestamps를 변경하지 못하게 한다.

## 식별 정보

조사한 repository commit: 7f87f7c81a72701710d8668b2569e51478dccc75.
문서의 요구사항을 요약한 작성 노트이며 원본 명세는 해당 repository에서 확인한다.
```

운영 원칙:

- 사실, 해석, 가설과 반례를 본문에서 구별한다. 상반된 근거는 각 출처와 시점을 남긴다. `confidence: 0.9` 같은 숫자를 객관적 진실이나 검토 완료로 사용하지 않는다.
- 날짜는 필요할 때 ISO 문자열로 기록한다. MCP 읽기/검색마다 `updated`를 덮어쓰지 않는다. 불필요한 바이트 변화는 snapshot과 검토 권한을 새로 만든다.
- alias는 다른 문서의 실제 의미와 겹치지 않게 점검한다. 제목이 흔한 경우 `Caching`보다 `HTTP response caching`처럼 뜻을 드러낸다.
- 폴더 중첩은 얕게 유지하고 filename은 사람이 읽을 수 있게 한다. 한국어를 금지할 필요는 없다. NFC/case-fold collision, Windows 예약명, 끝 공백/점, 안전하지 않은 구분문자는 검사한다.
- 긴 PDF와 이미지의 핵심 주장이 Markdown 없이 attachment에만 있으면 `attachment-only-evidence` 경고를 표시한다. OCR/요약을 자동으로 사실로 확정하지 않는다.
- 태그를 모두 `#tag` 본문으로 반복하거나 빈 section을 일률적으로 채우지 않는다. 필요한 관습만 선택한다.

근거: [canonical IR](../../../ObsidianKnowlegeComplication/docs/specs/canonical-knowledge-ir.md), [근거 완결 integration](../../../ObsidianKnowlegeComplication/docs/algorithms/stable/evidence-complete-integration.md), `REQ-CNF-001`, `REQ-INT-002/003/005`.

링크 경로에는 실제 호환성 차이가 있다. [Obsidian 공식 내부 링크 문서](https://obsidian.md/help/Linking%2Bnotes%2Band%2Bfiles/Internal%2Blinks)는 폴더를 포함한 wikilink를 Vault root 기준으로 설명한다. 조사한 OKC resolver는 현재 문서 상대 경로를 먼저 검사하고 filename/title/alias 후보로 넘어간다. 그래서 nested note의 `[[sources/B]]`가 두 위치에 존재하면 의미가 갈릴 수 있다. 작성 MCP는 두 해석을 점검하고 서로 다른 파일이면 모호성 경고를 낸다. 유효한 Obsidian root 경로를 무조건 깨진 링크로 표시하거나 사용자의 링크를 일괄 변경하지 않는다. 위 예시도 Obsidian 작성 문법을 사용한다. 명시적 block ID는 두 도구의 공통 범위인 영문·숫자·하이픈을 기본값으로 사용한다.

## 5. 작성 MCP의 도구 계약 제안

초기 설치에서는 고정된 하나의 live Vault root를 받는다. 도구는 상대 경로만 받고, source/output/project 위치를 도구 호출로 임의 확대하지 못한다. 인자와 응답은 엄격한 schema, 크기 제한과 구조화된 오류를 사용한다. 다음은 구현 전 계약 후보이며 실제 구현 여부는 construction 문서에 기록한다.

| 도구 | 입력 핵심 | 동작과 결과 |
|---|---|---|
| `vault_status` | 없음 | 등록 root의 논리 이름, profile/version, 지원/제한 capability, bounded 파일 수; 민감한 절대 경로는 기본 응답에서 숨김 |
| `list_notes` | `folder?`, `cursor?`, `limit?` | 정렬된 note path/title 목록, 다음 cursor; 파일 내용은 반환하지 않음 |
| `read_note` | `path`, `section?` | bounded 내용과 SHA-256 revision; note 내용은 신뢰할 수 없는 자료로 표시 |
| `search_notes` | `query`, `limit?`, `cursor?` | 로컬 lexical 검색; path/heading/짧은 snippet/revision. metadata/text index는 삭제해도 다시 만들 수 있음 |
| `create_note` | `path`, `content` | 경로/UTF-8/size/frontmatter 검사 후 부재한 새 파일에만 작성; overwrite 금지 |
| `update_note` | `path`, `expectedRevision`, `content` | 기존 revision 확인, 검증, 변경 전 backup와 안전한 교체. stale revision은 conflict로 종료 |
| `lint_vault` | `paths?`, `cursor?`, `limit?` | 오류/경고, path/span, 근거 rule ID, 수정 안내; compiler 승인이나 자동 수정 없음 |
| `get_note_template` | `kind` | concept/source/decision 등 작은 선택적 template; Vault에 빈 노트를 쓰지 않음 |

section patch, rename/backlink rewrite, snapshot export, semantic retrieval은 이후 단위로 나눈다. 링크를 고치는 기능은 inline/fenced code, display text, percent encoding, heading/block suffix를 보존하는 parser가 준비된 뒤 넣는다. 전체 문서 regex 치환으로 먼저 출시하지 않는다. rename은 연결된 모든 변경의 preview, revision 검사, rollback이 없는 동안 노출하지 않는 것이 낫다.

`expectedRevision`은 본문 hash로 오래된 편집을 감지하지만 Obsidian과 운영체제 사이에 원자적 compare-and-swap을 자동 제공하지 않는다. 같은 프로세스의 write는 직렬화하고, 외부 편집을 직전 재확인하며, race/복구의 실제 보장 범위를 문서화해야 한다. 원래 note를 backup 없이 삭제하거나 실패한 write를 성공으로 답하지 않는다. backup은 Vault 외부 private state에 두어 지식으로 재유입되지 않게 한다.

원천 Markdown의 “이 명령을 실행하라”는 도구 호출 권한이 아니다. 일반 shell/HTTP/delete, arbitrary binary write, `.obsidian` 설정 및 plugin write는 작성 MCP 범위에 넣지 않는다. 외부 URL은 출처 데이터로 취급하며 로컬 검색이 네트워크 요청을 만들지 않는다.

## 6. OKC 호환성 lint

rule severity는 `error`(현재 작성 계약상 저장/준비를 막음), `warning`(손실·모호성·정리 필요), `info`(지원 상태)로 분리한다. 호환성 profile 버전과 검사한 OKC commit을 함께 표시한다.

| 제안 rule ID | 수준 | 검사 | 연결된 OKC 요구사항 |
|---|---|---|---|
| `OKC_PATH_UNSAFE` | error | traversal, absolute/drive/UNC, symlink, 예약명, case/NFC 충돌, 경로 제한 | `REQ-SEC-001`, `REQ-SNP-002` |
| `OKC_TEXT_ENCODING` | error | non-UTF-8, NUL/제어문자, oversized note | `REQ-PAR-001`, `REQ-SEC-001` |
| `OKC_FRONTMATTER_INVALID` | error | duplicate key, malformed YAML, non-mapping, canonical JSON 비표현 값 | `REQ-PAR-001`, `REQ-INT-002` |
| `OKC_METADATA_TYPE` | warning | title 비문자열, aliases/tags 비문자열 항목, 빈 metadata/boilerplate 과다 | `REQ-PAR-001`, `REQ-INT-002` |
| `OKC_LINK_UNRESOLVED` | warning | 없는 note/heading/block/attachment | `REQ-PAR-001`, `REQ-PRV-001` |
| `OKC_LINK_AMBIGUOUS` | warning | 중복 title/stem/alias와 명확하지 않은 link | `REQ-DED-001`, `REQ-CNF-001` |
| `OKC_EVIDENCE_MISSING` | warning | 출처 노트의 식별 근거 없음, attachment-only 주장, 주장/관찰 문맥 부족 | `REQ-INT-003`, `REQ-PRV-001` |
| `OKC_NONMARKDOWN_OUTPUT` | warning | attachment/Canvas/Base가 현재 compiled output에 보존된다고 기대하는 내용 | `REQ-PAR-002/003`, `REQ-INT-006` |
| `OKC_INGEST_NOISE` | warning | templates/자동 index/운영 문서/빈 note가 선택 source에 있음 | `REQ-INT-001/002` |
| `OKC_SENSITIVE_CANDIDATE` | warning | 민감 내용 가능성; matched secret 없이 위치/유형만 반환 | `REQ-SEC-002/003` |

`OKC_EVIDENCE_MISSING`는 단순한 작성 안내이다. MCP가 모든 주장의 진실성이나 저작권을 판정한다는 의미가 아니다. JS의 Unicode lowercasing은 코어가 pin한 full case folding과 다를 수 있으므로 그 차이를 숨기지 않는다. 위 lint는 전체 compiler, semantic integration, critic, human approval의 대체물이 아니다.

## 7. 구현 검증에 재사용할 로컬 자료

- [기본 Vault fixture](../../../ObsidianKnowlegeComplication/tests/fixtures/basic_vault): frontmatter, wikilink, embed, inline-code 비링크, Canvas/Base/asset.
- [링크 모호성 fixture](../../../ObsidianKnowlegeComplication/tests/fixtures/typed_markdown_ambiguity): 중복 `Topic.md`와 반복 모호한 link.
- [rewrite 보존 fixture](../../../ObsidianKnowlegeComplication/tests/fixtures/markdown_rewrite_integrity): 수정하지 않는 bytes 검증 참고.
- [parser tests](../../../ObsidianKnowlegeComplication/crates/okc-core/src/parse.rs): duplicate frontmatter, escaped links, code/fence와 UTF-8 span 참고.
- [canonical source 검사](../../../ObsidianKnowlegeComplication/crates/okc-core/tests): traversal/alias/Unicode/source 불변성 테스트의 경계 참고. fixture 존재는 해당 기능의 current materialization 출시를 뜻하지 않는다.

MVP에는 사용자 Vault와 분리된 fixture에서 protocol E2E, create 충돌, revision mismatch, 경로 탈출/symlink, frontmatter failure, 코드 안 가짜 링크, search/result bound, 외부 편집 보존을 검사한다. snapshot export가 추가되면 원천 hash/mtime 보존과 snapshot 후 live 변경 분리를 별도로 검사한다. native SDK cross-language artifact golden은 이 작성 MCP의 MVP 검증으로 포장하지 않는다.

## 8. 관리와 후속 확장 경계

작성 관습은 작은 versioned profile로 관리한다. 예제 Vault는 synthetic data만 포함하며 사용자 Vault를 이 저장소의 submodule로 만들지 않는다. code repository, 개인 notes, 생성된 index/backup, OKC snapshot/project는 별도 수명과 저장 정책을 갖는다. 신규 사용자는 기본 profile로 시작하고, 기존 Vault 사용자는 구조를 옮기지 않고 connect → lint → 필요한 note만 개선할 수 있어야 한다.

현재 OKC는 `0.3.0` 개발 제품으로 Markdown 통합 경로와 Node/Python 패키지가 로컬 구현되어 있으나 npm/PyPI에 발행되지 않았고 stable release는 금지되어 있다. Pack, non-Markdown materialization, semantic scale, 원격 native matrix는 미완료다. 이 MCP는 그 미완료 제품에 대한 자동 설치나 무조건적인 호환성 보장을 하지 않는다. 향후 직접 corpus 검사/OKC handoff를 별도 optional capability로 도입하고, 사용 중인 compiler version과 실제 검사 여부를 표시한다.

근거: [CURRENT_STATE](../../../ObsidianKnowlegeComplication/docs/CURRENT_STATE.md), [TRACEABILITY](../../../ObsidianKnowlegeComplication/docs/TRACEABILITY.md), [ADR-0027](../../../ObsidianKnowlegeComplication/docs/adr/0027-current-schema-single-source.md).
