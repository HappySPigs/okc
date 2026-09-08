# OKC MCP의 AI-DLC 적용

> **공식 workflow 설치 (2026-09-06):** 사용자의 후속 설치 요청에 따라 2.7.1 Codex
> 배포본을 설치했다. 현재 사용법·설치 출처·진단은 [설정 안내](../docs/aidlc-setup.md)를 따른다.
> 아래 축약 방법론은 과거 미승인 제안으로 보존하며, 설치가 과거 단계의 승인이나 완료를 만들지는 않는다.

> 절차 정정: 아래 자체 축약 적용안은 사용자의 진행 방식 검토 없이 작성되었다.
> 사용자 지적에 따라 Construction을 중단하고 Inception으로 돌아갔다.
> 현재 상태는 [state.md](state.md), 다음 검토 대상은 [Inception 검토안](inception/review.md)을 따른다.
> 아래 단위·기술 선택·자율 진행 설명은 승인된 workflow가 아니다.

조사 기준: 2026-09-06. 이 문서는 당시 AWS 방법론을 작은 로컬 MCP 제품에 맞게 적용하려 했던 미승인 제안이다. 공식 설치 이후에도 아래 내용을 공식 실행 이력이나 승인 게이트 통과 기록으로 간주하지 않는다.

## 원방법론과 현재 구현 구분

AWS의 2025-07-31 소개와 Method Definition은 **Inception → Construction → Operations**를 설명한다. AI가 의도를 요구사항과 작업 단위로 구체화하고, 사람이 중요한 판단을 검토하며, 설계·코드·테스트·운영 자료를 저장소의 지속적인 맥락으로 남긴다. 작업 단위인 Unit과 짧은 구현·검증 반복인 Bolt는 구별한다. [AWS 소개](https://aws.amazon.com/blogs/devops/ai-driven-development-life-cycle/), [Method Definition, 특히 II.9–10과 III](https://prod.d13rzhkk8cj2z0.amplifyapp.com/aidlc.pdf)

2025-11-29의 AWS 공개 워크플로 설명은 작업마다 단계의 종류와 깊이를 조정하도록 한다. 작은 변경에 큰 조직의 절차를 그대로 강제하는 문제와 사람의 검토가 사라지는 문제를 함께 다룬다. [AWS adaptive workflows 설명](https://aws.amazon.com/blogs/devops/open-sourcing-adaptive-workflows-for-ai-driven-development-life-cycle-ai-dlc/)

조사 시점의 `awslabs/aidlc-workflows`는 `main`에서 Workflows 2.0 GA를 안내하며 README의 버전은 `2.7.1`이다. 재현 가능한 참조는 다음 커밋이다.

```text
repository: https://github.com/awslabs/aidlc-workflows
commit: 22ed2d101f4f01196b76d5725cf8d9aabe5fef9e
commit time: 2026-09-06T03:22:39Z
```

현재 구현은 **Initialization → Ideation → Inception → Construction → Operation**의 5개 phase, 33개 stage로 원방법론을 확장한다. 기본 `classic` scope는 Ideation을 제외하며 scope와 깊이를 별도로 조정한다. 따라서 2025년 3단계 소개, 오래된 `aidlc-docs` 관례, 현재 2.x 실행기의 구조를 같은 버전인 것처럼 섞지 않는다. [고정 README](https://github.com/awslabs/aidlc-workflows/blob/22ed2d101f4f01196b76d5725cf8d9aabe5fef9e/README.md), [고정 scope 설명](https://github.com/awslabs/aidlc-workflows/blob/22ed2d101f4f01196b76d5725cf8d9aabe5fef9e/docs/guide/05-scopes-and-depth.md)

2.x 가이드의 Construction 기본 진행은 stage-major이며, 첫 Construction 실행 단계의 검토 이후 사용자가 이후 단계의 자율 진행 여부를 선택한다. Bolt 계획과 실제 엔진의 실행 묶음도 동일하지 않다고 명시한다. 이 프로젝트에서 여러 AI 작업자가 협업했다고 해서 이 공식 엔진의 게이트·감사 이벤트가 발생했다고 기록하지 않는다. [고정 phases and stages](https://github.com/awslabs/aidlc-workflows/blob/22ed2d101f4f01196b76d5725cf8d9aabe5fef9e/docs/guide/04-phases-and-stages.md)

## 이 프로젝트의 적용 범위

`okc-mcp`는 **OKC에 좋은 입력을 제공하도록 Obsidian Vault를 작성·정리하는 로컬 MCP**다. 신규 제품 개발과 기존 소비 시스템의 입력 계약 조사를 함께 수행한다. 사용자가 편집하는 살아 있는 Vault와 OKC가 수집한 immutable snapshot을 구별한다. MCP는 전자를 돕고 컴파일러가 소유한 snapshot·canonical state·approval 정책을 변경하지 않는다.

현재 참조한 OKC는 커밋 `7f87f7c81a72701710d8668b2569e51478dccc75`의 Schema 3, `0.3.0` 개발 트리다. Markdown과 frontmatter 및 링크의 실제 수집 동작을 확인해 입력 작성 방식을 설계한다. OKC 저장소의 `normative-future` MCP 명세는 컴파일러 제어 어댑터를 설명하므로 이 source authoring 제품의 기능 목록으로 그대로 옮기지 않는다. [OKC current state](https://github.com/dolgogae/okc/blob/7f87f7c81a72701710d8668b2569e51478dccc75/docs/CURRENT_STATE.md), [Vault compilation pipeline](https://github.com/dolgogae/okc/blob/7f87f7c81a72701710d8668b2569e51478dccc75/docs/specs/vault-compilation-pipeline.md)

이 초기 작업은 다음 깊이로 수행한다. 아래 내용은 이 프로젝트를 위한 적용안이며 AWS가 지정한 별도 scope 이름이 아니다.

| 단계 | 실행할 일 | 검토할 결과 |
| --- | --- | --- |
| 시작·의도 정리 | 사용자 목표, 로컬 설치 제약, 저장소 상태와 기존 OKC 계약 확인 | 사실·사용자 요구·설계 가정을 구별한 실행 상태 |
| Inception | 기존 Obsidian MCP 비교, Vault 경계, 사용자 흐름, 최소 도구 계약, 작업 단위 | 조사 근거, 요구사항, Vault 설계, 저장소 UX |
| Construction | stdio 서버, 안전한 노트 생성·수정, 입력 품질 audit, preview/apply, 설치·진단 | 실행 가능한 로컬 패키지, 검증 결과, 남은 제약 |
| Operations 준비 | 로컬 tarball 설치 확인, 문제 진단, 릴리스·호환성·복구 절차 | 배포 가능한 후보물과 실제 공개 배포 여부를 구별한 기록 |

원격 서버·AWS 계정·클라우드 IaC·다중 사용자 인증·웹 UI는 현재 로컬 stdio 목표의 필수 요소가 아니다. 대신 설치와 업데이트가 이 제품의 운영 과정이므로 Operation 준비를 생략하지 않는다.

## 첫 구현의 Units와 Bolts

Unit은 사용자가 얻는 기능의 경계다. 다음 표는 초기 단위를 제안하며 최종 구현과 테스트 이름은 Construction 기록에 연결한다.

| Unit | 사용자 결과 | 완료 기준 |
| --- | --- | --- |
| U1 — 설치와 연결 | 로컬 설치 후 한 설정으로 MCP 클라이언트 연결 | `serve`, `doctor`, 설정 생성, 실제 stdio handshake 검증 |
| U2 — Vault 작성 | 등록된 Vault의 노트 생성·읽기·검색·수정 | 경로 경계, 생성 시 덮어쓰기 방지, 수정 시 content hash 충돌 방지, 유한한 결과 |
| U3 — OKC 입력 준비 | metadata·링크·중복 문제를 이해하고 계획한 정리를 적용 | audit은 읽기 전용, 변경 preview, 오래된 계획 거부, 적용 범위 명시 |
| U4 — 배포와 유지관리 | 소스 저장소 밖에서도 설치·진단·제거 가능 | tarball 설치 smoke, synthetic Vault 검증, 사용자용 문서, 릴리스 보류 항목 |

첫 Bolt는 U1과 U2의 가장 작은 연결·작성 경로를 끝까지 검증한다. 다음 Bolt는 U3의 audit과 안전한 준비 변경, U4의 설치 검증을 완성한다. 작업 단위마다 구현 전에 경계를 문서화하고 구현 후 테스트 근거를 연결한다. 각 Bolt의 코드는 검토 가능한 크기로 유지한다.

후속 Unit은 별도 의도로 다룬다. 예를 들어 링크를 보존하는 노트 이동, 의미 중복 후보 병합, 첨부파일 정리, 팀 동시편집은 각각 다른 데이터 변경 위험을 갖는다. 첫 제품에는 native OKC 설치나 compiler 제어가 필요하지 않다. 입력 품질 audit을 OKC 전체 컴파일 성공 보증으로 표시하지 않는다.

## 사람의 판단과 감사 기록

사용자의 이번 요청은 조사·설계·로컬 구현을 진행할 근거다. 이를 근거로 가역적인 파일 작성, 테스트, 설치 후보 패키지 생성까지 진행한다. 반면 구체적인 설계안에 대한 사후 승인이 이미 있었다고 기록하지 않는다.

기록은 다음 상태를 분리한다.

- **사용자 요구:** 대화에서 실제로 요청한 내용과 출처.
- **확인한 사실:** 특정 커밋·API·문서·실행 결과가 뒷받침하는 내용.
- **설계 가정:** AI가 구현을 진행하기 위해 택한 기본값과 수정 시 영향.
- **구현·검증:** 실제 생성된 파일과 실행된 검사, 통과 또는 실패.
- **사용자 결정:** 실제 후속 응답으로 확인된 선택만 기록.

제품 방향이나 변경 권한을 넓히는 결정은 구체적인 차이와 영향이 검토 가능할 때 제시한다. source authoring 도구 구현은 현재 범위에 포함된다. 개발 중에는 synthetic Vault로 쓰기와 실패를 검증하며 사용자의 실제 개인 Vault를 예시 데이터로 수정하지 않는다. canonical approval 기록, 원격 provider 호출, 공개 패키지 배포는 현재 작업의 자동 후속 단계로 취급하지 않는다. 이는 현재 범위에 따른 선택이며 모든 문서마다 형식적인 승인 질문을 강제하는 규칙이 아니다.

당시에는 외부 공식 워크플로 파일을 참고 자료로만 읽었다. 이후 사용자의 명시적 설치 요청으로 공식 2.7.1 Codex 배포본과 훅을 연결했다. 해당 커밋·로컬 조정·기존 프로젝트 기록의 관계는 [설정 안내](../docs/aidlc-setup.md)에 기록한다. scope와 제품 단계 승인은 설치와 별도로 실제 워크플로에서 다룬다.

## 변경을 다시 이어가기 위한 최소 기록

문서를 구현 옆에 버전 관리한다. `aidlc-docs`는 이 프로젝트가 선택한 기록 위치다. 공식 2.x가 사용하는 `aidlc/spaces/<space>/intents/...` 엔진 상태를 생성하거나 모방하지 않는다.

| 기록 | 담을 내용 |
| --- | --- |
| 상태와 결정 기록 | 현재 단계, 실제 사용자 결정, 가정, 미해결 항목 |
| Inception 문서 | 조사·요구사항·Vault 설계·설치와 관리 UX |
| Construction 문서 | 도구 계약, 구현 경계, 테스트 명령과 결과 |
| Operations 문서 | 로컬 설치 방법, 지원 환경, 업데이트·복구, 공개 배포 여부 |

검증 종료 시 요구사항과 코드·테스트의 연결을 점검한다. 테스트 통과와 제품 승인을 구분하고, 로컬 패키지 생성과 registry publication을 구분한다. 다음 세션은 이 기록을 읽고 남은 일을 이어간다.
