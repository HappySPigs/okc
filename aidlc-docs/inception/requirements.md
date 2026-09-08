# 요구사항 — OKC 입력 Vault를 만드는 Obsidian MCP

상태: 2026-09-06 요구사항 초안. assistant가 제안한 첫 범위이며 사용자 검토·승인 전이다.
Construction을 중단하고 [Inception 검토안](review.md)에서 실제 요구와 가정을 다시 확정한다.

## 의도

사용자는 기존 Obsidian MCP의 로직 조사, OKC에 가장 적합한 Vault 구성 설계,
OKC 사용자의 설치·저장소 관리 UX, AWS AI-DLC 방식의 초기 설계부터 구현까지를 요청했다.
이어 결과 Vault 조회용 어댑터가 아니라 **OKC 입력 Vault를 가장 잘 만드는 Obsidian MCP**라고 정정했다.
이 정정이 현재 제품 방향이다. 먼저 제안했던 읽기 중심 선택지는 폐기했다.

## 첫 사용자 여정

로컬 설치 → 편집할 Vault 하나를 명시적으로 연결 → 근거 있는 노트 생성 →
읽기/검색 → 기존 해시를 확인하며 본문·프런트매터 수정 → 입력 품질 점검 →
OKC에 원본 경로를 전달하고 OKC가 스냅샷·통합·검토·컴파일을 담당한다.

| ID | 요구사항 | 수용 기준 |
|---|---|---|
| REQ-001 | 설치형 로컬 stdio MCP | Obsidian 앱·REST 플러그인·API 키·OKC 바이너리 없이 동작한다. |
| REQ-002 | 기존 Vault 존중 | 폴더 재배치나 필수 커스텀 프런트매터 마이그레이션 없이 연결한다. |
| REQ-003 | 지식 작성 | 제목·본문과 선택적 aliases/tags/source로 Markdown을 생성하고 기존 파일 교체를 거부한다. |
| REQ-004 | 충돌을 고려한 수정 | expectedHash가 다르면 거부하며 수정 전 Vault 밖 백업을 보존한다. |
| REQ-005 | 구조 보존 | 프런트매터 부분 수정은 미지의 키·본문·YAML 주석을 보존한다. malformed YAML은 거부한다. |
| REQ-006 | 탐색 | 경로·한글 본문의 결정적 목록/리터럴 검색과 해시가 포함된 읽기를 제공한다. |
| REQ-007 | OKC 입력 점검 | YAML, 경로, 링크, 중복, 운영 문서 잡음, 미지원 형식을 보고한다. compiler 검증 통과라고 주장하지 않는다. |
| REQ-008 | 제한된 권한 | 등록 Vault 밖 접근, symlink/hardlink, 숨김 제어 경로를 거부하고 크기·파일 수·응답량을 제한한다. |
| REQ-009 | 지식과 도구 상태 분리 | 설정·백업·템플릿은 입력 Vault 밖에 둔다. compiled artifact와 project를 편집 Vault로 연결하지 않는다. |
| REQ-010 | 검토 가능한 설치 | config 생성, 설정 진단, 절대 경로 client-config 출력, 로컬 tarball 설치를 지원한다. |
| REQ-011 | 신뢰 경계 | 노트는 비신뢰 데이터다. 내부 AI 호출, 자동 승인, 셸·임의 HTTP·삭제 도구를 제공하지 않는다. |
| REQ-012 | 수명주기 기록 | 조사·설계·구현·테스트·운영/릴리스 상태와 후속 작업을 파일로 남긴다. |

## 제외와 후속 단위

첫 단위는 단일 Vault Markdown authoring이다. semantic 검색, Obsidian UI/Dataview 실행,
범용 삭제/rename, 자동 taxonomy 이동, 원격 HTTP, provider 실행, OKC review 승인,
Attachment/Canvas/Base 변환, 완전한 Obsidian 링크 해석은 포함하지 않는다.
원본 작성의 권한과 OKC가 봉인한 스냅샷의 불변성은 다른 수명주기 단계다.

후속 단위는 ① 링크를 보존하는 rename 및 복구 UX ② compiler corpus 검사 연동
③ 실제 사용자 Vault 성능·OS 행렬 ④ 출판·업데이트 경로다.
사용자 검토 후 우선순위를 변경할 수 있다.
