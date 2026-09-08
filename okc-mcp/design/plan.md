# okc-mcp 프로젝트 구성 안내

> 이 문서는 저장소를 처음 살펴보는 사람을 위해 "이 프로젝트가 어떻게 짜여 있는가"를
> 정리한 오리엔테이션 문서다. 조사 시점: 2026-09-07. 소스 코드와 문서를 직접 읽고
> 기록한 것이며, 새로운 요구사항 승인이나 AI-DLC 단계 진행 결과가 아니다.

## 1. 한 줄 요약

이 저장소에는 **두 개의 서로 다른 것**이 한 폴더에 함께 들어 있다.

1. **제품** — `okc-mcp`. Obsidian Vault를 OKC(Obsidian Knowledge Complication)에
   좋은 입력이 되도록 작성/편집하는 로컬 MCP 서버 (`src/`, `tests/`).
2. **개발 프로세스 프레임워크** — AWS **AI-DLC** 워크플로우 설치본. 이 제품을
   "아이디어→요구사항→설계→구현" 순서로 통제하며 진행하기 위한 도구·규칙·기록
   (`.claude/`, `.codex/`, `.agents/`, `aidlc/`, `aidlc-docs/`).

즉, 저장소의 파일 대부분(약 590개 추적 파일 중 590개 중 `.claude`+`.codex`+`.agents`
만 ~590개)이 **제품 코드가 아니라 프로세스 프레임워크**다. 실제 제품 소스는
`src/` 6개 파일(약 1,170줄)뿐이다.

## 2. 최상위 디렉터리 지도

| 경로 | 성격 | 내용 |
|---|---|---|
| `src/` | **제품 코드** | MCP 서버 구현 (TypeScript, 6개 파일) |
| `tests/` | **제품 테스트** | `node --test` 기반 유닛 테스트 4개 파일 |
| `examples/` | 제품 | `okc-mcp.example.json` 설정 예시 |
| `docs/` | 제품 + 프로세스 | 사용자 문서(`authoring-guide.md`, `operations.md`) + AI-DLC 설치 기록 |
| `package.json` / `tsconfig*.json` | 제품 빌드 | Node ≥22.13, `npm run check`(typecheck+test+build) |
| `README.md` / `CHANGELOG.md` / `CONTRIBUTING.md` | 제품 | 사용자·기여자 안내 |
| `aidlc-docs/` | **과거 프로세스 기록** | 초기(비공식) 연구·설계·요구사항 초안 + 상태/감사 로그 |
| `aidlc/` | **현행 프로세스 상태** | 공식 AI-DLC 워크스페이스(스페이스/인텐트/메모리/감사) |
| `.claude/` | 프로세스 하네스 | Claude Code용 AI-DLC 스킬·에이전트·툴·훅·지식 |
| `.codex/` `.agents/` `AGENTS.md` | 프로세스 하네스 | 같은 워크플로우의 Codex 하네스 (병행 설치) |
| `CLAUDE.md` `.github/` | 설정 | 프로젝트 지침 및 GitHub 설정 |

## 3. 제품(okc-mcp)의 실제 구조

`package.json`: name `okc-mcp`, version `0.1.0-alpha.1`, "Local Obsidian authoring
MCP for evidence-rich OKC source Vaults". 미배포(npm 게시 안 됨) 상태.

### 3.1 소스 파일 (`src/`)

| 파일 | 줄수 | 역할 |
|---|---|---|
| `cli.ts` | 61 | CLI 진입점. `config` / `doctor` / `client-config` / `serve` 4개 명령. stdio 전송만 사용. |
| `config.ts` | 54 | 설정 스키마(zod). **절대 경로 강제**, Vault ID 해시, 설정 파일이 Vault 밖에 있도록 강제. |
| `vault.ts` | 484 | Vault 파일 I/O의 안전 계층. symlink/hardlink 거부, 경로 순회 차단, 제외 폴더(`.git`,`.obsidian`,`.okc`,`.trash`,`node_modules`), 백업/락. |
| `notes.ts` | 359 | 노트 파싱·검증·감사. YAML frontmatter, 링크 추출, 중복/품질 finding, `__proto__` 등 위험 키 차단. |
| `server.ts` | 177 | MCP 서버 정의. 아래 도구/리소스/프롬프트 등록. |
| `guide.ts` | 35 | `okc://guide/authoring` 리소스에 실리는 작성 가이드 텍스트. |

### 3.2 MCP 인터페이스 (server.ts에서 등록)

**읽기 전용 도구** (항상 등록):
`vault_info`, `list_notes`, `read_note`, `search_notes`, `audit_vault`

**쓰기 도구** (`readOnly:false`일 때만 등록):
`create_note`(기존 파일 대체 안 함), `replace_note`(전체 교체, 해시 확인+백업),
`patch_frontmatter`(지정 키만 변경, 본문 보존)

- 모든 쓰기 도구는 `dryRun` 기본값 **true**(미리보기만). `false`여야 실제 저장.
- 쓰기는 `read_note`가 준 전체 파일 SHA-256(`expectedHash`)을 요구하고, 값이
  다르면 `CONFLICT`로 거부 → **외부(Obsidian) 편집을 조용히 덮어쓰지 않음**.
- 리소스: `okc://guide/authoring` · 프롬프트: `capture_knowledge`.
- 서버는 AI 서비스·원격 검색·텔레메트리를 호출하지 않음.

### 3.3 설계상의 경계 (제품이 하지 않는 것)

delete / move / rename, 멀티 Vault, Obsidian 앱·CLI·플러그인 필수 연동,
원격/호스팅 MCP, **실제 OKC 컴파일러 실행·통과 판정·승인**, Canvas/Bases/첨부
완전 작성, 영속 full-text/semantic index — 모두 범위 밖. 감사(`audit_vault`)는
**advisory**(권고)일 뿐 컴파일러 검증이 아님.

## 4. 프로세스 프레임워크(AI-DLC)와 현재 상태

이 제품은 AWS **AI-DLC**(AI-Driven Development Life Cycle) 방법론으로 진행된다.
사람이 각 단계마다 승인해야 다음으로 넘어가는 게이트형 워크플로우다.

### 4.1 두 종류의 기록 — 매우 중요

- `aidlc-docs/` = **과거의 비공식 초안**. 예전에 AI가 방법론을 축약 해석해
  연구·설계·구현을 **요구사항 검토 전에 앞서** 진행한 흔적. 이 안의 코드/문서는
  "승인된 baseline이 아니라 참고용 초안"이다.
- `aidlc/spaces/default/` = **현행 공식 워크스페이스**. 실제 사용자 지시에서
  유도한 방법(memory)과 진행 기록(intents/audit)이 여기 쌓인다.

### 4.2 프로세스 상태 (`aidlc-docs/state.md` + 저장소 지침 기준)

- **제품 게이트가 잠겨 있음(IN FORCE):** 요구사항 검토와 Construction 전환이
  승인되기 전까지 **제품 구현·테스트·패키징을 재개하지 않는다.**
- 현재 `src/`, `tests/`의 코드는 **너무 일찍 작성된 미승인 초안**이지 확정된
  제품 baseline이 아니다.
- 현행 공식 인텐트 `okc-vault-mcp`(scope `okc-local-mcp`)가 진행 중이며,
  **Ideation 후반**에 있다. 최근까지 완료된 흐름:
  - Intent Capture → Market Research → Scope Definition 승인
  - 다음 단계로 **Rough Mockups**(저-fidelity 화면/흐름 설계)를 진행하려던 참
    (이 문서를 쓰느라 잠시 멈춤).
- 승인 이벤트를 임의로 만들어내지 않으며, 상태 전이는 전적으로 도구가 소유한다.

### 4.3 대화·작성 언어

이 워크플로우의 확립된 대화 언어는 **한국어**다 (사람이 읽는 산출물·질문·설명 모두).

## 5. 개발·검증 방법

```sh
npm ci
npm run check      # = typecheck (tsc --noEmit + 테스트 tsconfig) && test && build
npm test           # node --import tsx --test tests/*.test.ts
npm run build      # tsc → dist/
```

- 런타임: Node.js **22.13+**. 의존성: `@modelcontextprotocol/sdk` 1.30.0,
  `yaml` 2.9.0, `zod` 4.5.4.
- AI-DLC 하네스 도구·훅은 별도로 **bun**을 요구한다(제품 런타임과 무관).
- 검증은 반드시 **합성(synthetic) 임시 Vault**로 한다. 개인 노트·설정은 커밋 금지.

## 6. 처음 보는 사람이 기억할 핵심 3가지

1. **파일의 대부분은 제품이 아니라 프로세스 프레임워크다.** 제품을 이해하려면
   `src/` + `README.md` + `package.json`만 보면 된다.
2. **제품 게이트가 잠겨 있다.** `src/`/`tests/`는 미승인 초안이며, 요구사항 검토와
   Construction 승인 전에는 구현을 재개하지 않는다.
3. **`aidlc-docs/`(과거 초안)와 `aidlc/`(현행 공식 기록)는 다른 것이다.** 과거
   초안을 완료된 단계나 승인 증거로 오인하면 안 된다.

## 7. 더 읽을거리 (저장소 내부)

- 제품: `README.md`, `docs/authoring-guide.md`, `docs/operations.md`
- 제품 설계 초안(참고용): `aidlc-docs/inception/requirements.md`,
  `aidlc-docs/construction/design.md`, `aidlc-docs/inception/okc-vault-design.md`,
  `aidlc-docs/inception/existing-mcp-research.md`, `aidlc-docs/inception/repository-ux.md`
- 프로세스 상태: `aidlc-docs/state.md`, `aidlc-docs/inception/review.md`
- 하네스/방법론: `.claude/CLAUDE.md`, `docs/aidlc-setup.md`,
  `aidlc/spaces/default/memory/`(org/team/project + 단계별 규칙)
- 현행 인텐트 산출물:
  `aidlc/spaces/default/intents/260906-okc-vault-mcp/ideation/`
