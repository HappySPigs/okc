# 요구사항 검토 질문 (Requirements Verification)

이 파일은 Inception 요구사항 검토를 위한 질문입니다. 각 질문의 `[Answer]:` 뒤에 **선택한 letter(A, B, C...)** 를 적어 주세요. 보기 중 맞는 게 없으면 마지막 **X) Other** 를 고르고 `[Answer]:` 뒤에 직접 설명해 주세요. 다 작성하시면 "완료" 라고 알려 주세요.

배경 문서: [requirements.md](../requirements.md) (초안, REQ-001~012), [review.md](../review.md) (미승인 가정 7건 + 성공 기준 5건).
현재 파악된 방향은 각 질문에 "(현재 초안)" 으로 표시해 두었으니, 그대로 확정할지 / 바꿀지 결정해 주시면 됩니다.

---

## Question 1 — Vault 개수 (단일 vs 다중)
현재 초안은 한 번에 **하나의 Vault**를 연결해 저작합니다(REQ-001, first Unit). 첫 버전에서 여러 Vault를 동시에 저작·비교해야 합니까?

A) 아니오 — 첫 버전은 단일 Vault로 한정 (현재 초안)

B) 예 — 첫 버전부터 다중 Vault 동시 저작·비교가 필요

C) 단일 Vault로 시작하되, 다중 Vault는 명시적 후속 Unit으로만 계획

X) Other (please describe after [Answer]: tag below)

[Answer]: C

## Question 2 — 접근 방식 (파일시스템 직접 vs Obsidian 런타임 연동)
현재 초안은 Obsidian 앱·REST 플러그인·API 키 없이 **stdio로 파일시스템에 직접 접근**합니다(REQ-001). Obsidian이 실행 중일 때의 UI·Dataview·플러그인 기능이 첫 버전에 필수입니까?

A) 아니오 — 파일시스템 직접 접근만으로 충분 (현재 초안)

B) 예 — Obsidian 런타임/Dataview/플러그인 연동이 필수

C) 파일시스템 우선, 일부 Obsidian 연동은 후속 Unit

X) Other (please describe after [Answer]: tag below)

[Answer]: A

## Question 3 — 설치 경로 (Node/TypeScript + npm)
현재 초안은 **Node/TypeScript stdio MCP + 로컬 tarball 설치**입니다(REQ-001, REQ-010). 대상 사용자와 OS에 이 설치 경로가 충분히 간단합니까?

A) 예 — Node/npm 로컬 설치로 진행 (현재 초안)

B) 아니오 — 더 간단한 배포가 필요 (예: 단일 실행 바이너리)

C) 예, 단 npm 외 설치 옵션(바이너리/패키지 매니저)은 후속 Unit

X) Other (please describe after [Answer]: tag below)

[Answer]: A

## Question 4 — 첫 Unit 우선 여정
첫 Unit은 **저작 · 업데이트 · 품질 감사**로 한정되어 있습니다. 우선순위가 가장 높은 사용자 여정은 무엇입니까?

A) 지식 캡처(새 노트 작성) 중심

B) 정리·구조화(기존 노트 정돈) 중심

C) 중복 병합 중심

D) 근거(source) 보강 중심

E) 현재 초안대로 저작+업데이트+품질감사 균형 유지 (현재 초안)

X) Other (please describe after [Answer]: tag below)

[Answer]: B

## Question 5 — 기존 Vault 개선 vs 새 Vault 생성 (+ 마이그레이션)
현재 초안은 **기존 구조를 존중**하고 얕은 폴더 구조는 **선택**입니다(REQ-002, REQ-005). 기존 Vault 개선과 새 Vault 생성 사이의 균형은?

A) 기존 Vault 개선 우선 — 강제 마이그레이션 없음 (현재 초안)

B) 새 Vault 생성·스캐폴딩 우선

C) 둘 다 동등하게 지원

X) Other (please describe after [Answer]: tag below)

[Answer]: A

## Question 6 — 품질 감사(휴리스틱) vs 실제 OKC 수집 테스트
현재 초안의 품질 감사는 **휴리스틱**이며 컴파일러 검증 통과를 주장하지 않습니다(REQ-007). 실제 OKC 수집(ingestion) 테스트는 언제 포함되어야 합니까?

A) 첫 Unit은 휴리스틱만, 실제 OKC 수집 테스트는 후속 Unit (현재 초안)

B) 첫 Unit부터 실제 OKC 수집 테스트 통합이 필요

C) 첫 Unit에 최소한의 스모크(smoke) 수집 테스트만 포함

X) Other (please describe after [Answer]: tag below)

[Answer]: A

## Question 7 — 변경 관리 (정리 권한 · 백업 · 검토 단위)
현재 초안은 **해시 불일치 거부 + Vault 외부 백업**이며 삭제·이동·이름변경 도구는 없습니다(REQ-004, REQ-008, REQ-011). 어느 수준의 정리 권한이 허용되고, 사용자 변경 검토는 어느 단위로 이뤄져야 합니까?

A) 삭제·이동·이름변경 없음, 안전한 부분 업데이트만 + 외부 백업 (현재 초안)

B) 사용자 확인을 전제로 제한적 정리(이름변경·이동)를 첫 버전에 포함

C) 파일 단위가 아닌 변경(diff) 단위의 사용자 검토가 필요

X) Other (please describe after [Answer]: tag below)

[Answer]: A

## Question 8 — 성공 기준 5가지 채택 여부
review.md의 제안 성공 기준: ①문서만으로 설치→첫 노트 작성 ②강제 마이그레이션 없이 문제·개선안 이해 ③근거·충돌·링크 손실 없이 추가·업데이트 ④OKC 인계 시 입력 구조 문제·수집 노이즈 감소 ⑤변경 검토·충돌 식별·이전 내용 복구. 이 5가지를 그대로 채택합니까?

A) 예 — 5가지 성공 기준 그대로 채택

B) 대체로 동의하나 일부 수정 필요 (X)에 기술)

C) 채택하되 측정 방식을 더 구체화 필요 (합성/동의 사용자 시나리오 지표)

X) Other (please describe after [Answer]: tag below)

[Answer]: A

## Question 9 — Security Extensions
Should security extension rules be enforced for this project? (보안 확장 규칙을 이 프로젝트에 강제할까요?)

A) Yes — enforce all SECURITY rules as blocking constraints (production-grade 권장)

B) No — skip all SECURITY rules (PoC·프로토타입·실험 프로젝트에 적합)

X) Other (please describe after [Answer]: tag below)

[Answer]: B

## Question 10 — Resiliency Extensions
Should the resiliency baseline (AWS Well-Architected 신뢰성 기둥 기반의 설계-시점 모범사례) be applied? 이 로컬 단일 프로세스 저작 도구에 적용할까요?

A) Yes — apply the resiliency baseline as directional design-time guidance

B) No — skip the resiliency baseline (rapid iteration 우선, PoC·프로토타입에 적합)

X) Other (please describe after [Answer]: tag below)

[Answer]: A

## Question 11 — Property-Based Testing Extension
Should property-based testing (PBT) rules be enforced? 이 프로젝트에는 YAML/frontmatter 파싱·부분 업데이트·해시 같은 데이터 변환 로직이 있습니다.

A) Yes — enforce all PBT rules as blocking constraints (business logic·serialization·stateful 권장)

B) Partial — pure function과 serialization round-trip 에만 PBT 적용

C) No — skip all PBT rules (단순 CRUD·UI-only·thin 통합 계층에 적합)

X) Other (please describe after [Answer]: tag below)

[Answer]: B
