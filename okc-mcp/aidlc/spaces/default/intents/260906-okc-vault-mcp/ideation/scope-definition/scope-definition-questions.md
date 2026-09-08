# Scope Definition Questions

## Sources

- [intent] [`intent-statement`](../intent-capture/intent-statement.md): 단일 source Vault, 지속적 작성, 입력 점검, 사용자 통제라는 승인된 제품 신호.
- [market] [`competitive-analysis`](../market-research/competitive-analysis.md), [`market-trends`](../market-research/market-trends.md), [`build-vs-buy`](../market-research/build-vs-buy.md): 지속 쓰기는 table stakes이며 OKC 안내·충돌 안전성·명시적 경계가 차별점이라는 승인된 조사 결론.

## Q1. 첫 사용자가 “설치할 가치가 있다”고 느끼게 할 최소 완결 흐름은 어디까지인가요?

기능을 따로 나열하기보다, 설치부터 실제 Vault 가치 확인까지 끊기지 않는 MVP 경계를 정합니다.

- A. 설치·연결 확인·새 노트 생성·기존 노트 읽기와 안전한 수정까지를 완결 흐름으로 본다.
- B. A에 OKC용 구조 안내와 advisory 입력 점검까지 포함한다.
- C. B에 업데이트·진단·제거 문서까지 포함하여 공개 배포 가능한 첫 경험으로 만든다.
- X. Other (please specify)

[Answer]: C. B에 업데이트·진단·제거 문서까지 포함하여 공개 배포 가능한 첫 경험으로 만든다.

## Q2. 첫 버전의 Vault 변경 기능에서 어디에 절단선을 둘까요?

생성·수정은 승인됐지만, 삭제·이동·이름 변경까지 같은 범위에 넣으면 복구와 링크 영향 범위가 크게 달라집니다.

- A. 생성, 읽기, 검색, append, 기존 노트의 안전한 부분 수정만 포함하고 삭제·이동·이름 변경은 제외한다.
- B. A에 새 경로로의 안전한 이동·이름 변경을 포함하되 삭제는 제외한다.
- C. B에 휴지통 또는 복구 가능한 삭제까지 포함하여 전체 authoring lifecycle을 다룬다.
- X. Other (please specify)

[Answer]: A. 생성, 읽기, 검색, append, 기존 노트의 안전한 부분 수정만 포함하고 삭제·이동·이름 변경은 제외한다.

## Q3. 기존 노트를 수정하기 전에 내용이 바뀐 경우 첫 버전은 어떻게 행동해야 하나요?

Obsidian과 MCP가 같은 파일을 편집할 수 있으므로, 조용한 덮어쓰기를 허용할지 명시해야 합니다.

- A. 변경을 거부하고 현재 내용과 충돌 사실을 알려 사용자가 다시 읽은 뒤 재시도하게 한다.
- B. 변경 제안을 별도 미리보기로 보여 주고 사용자가 적용 여부를 정하게 한다.
- C. 충돌이 난 변경만 자동 병합을 시도하고 불가능할 때 사용자에게 묻는다.
- X. Other (please specify)

[Answer]: A. 변경을 거부하고 현재 내용과 충돌 사실을 알려 사용자가 다시 읽은 뒤 재시도하게 한다.

## Q4. OKC용 Vault 구조 안내는 첫 버전에서 얼마나 강제해야 하나요?

Obsidian source Vault는 사용자가 소유하며, 안내와 compiler 규칙을 혼동하지 않는 제품 경계가 필요합니다.

- A. 추천 폴더·메타데이터·링크 예시만 제공하고 사용자가 모두 바꿀 수 있게 한다.
- B. 기본 starter 구조를 제안하되 사용자가 승인하거나 재정의한 뒤 작성한다.
- C. OKC가 확인한 제약만 차단하고, 나머지 근거·상태·MOC 패턴은 수정 가능한 권고로 제공한다.
- X. Other (please specify)

[Answer]: C. OKC가 확인한 제약만 차단하고, 나머지 근거·상태·MOC 패턴은 수정 가능한 권고로 제공한다.

## Q5. 첫 버전의 OKC 입력 점검 결과는 어디까지 책임져야 하나요?

이 제품은 authoring MCP이며 실제 compiler나 승인 권한이 아니라는 경계를 사용자에게 명확히 보여야 합니다.

- A. frontmatter·경로·링크·구조 문제를 advisory로 보여 주고 실제 compiler 검증은 주장하지 않는다.
- B. A에 사용자가 선택한 문제의 안전한 수정 제안까지 제공한다.
- C. B에 사용자가 승인한 수정의 직접 적용까지 포함한다.
- X. Other (please specify)

[Answer]: C. B에 사용자가 승인한 수정의 직접 적용까지 포함한다.

## Q6. 공개 저장소의 첫 릴리스에 반드시 포함할 사용자 경험은 무엇인가요?

코드가 존재하는 것과 개인 사용자가 안심하고 설치·유지할 수 있는 제품은 다르므로 배포 UX 경계를 정합니다.

- A. 빠른 설치와 MCP host 설정 예시만 포함한다.
- B. A에 연결 확인, synthetic 예제 Vault, 첫 노트 walkthrough를 포함한다.
- C. B에 지원 버전표, 보안 경계, 문제 해결, 업데이트·제거 안내를 포함한다.
- X. Other (please specify)

[Answer]: C. B에 지원 버전표, 보안 경계, 문제 해결, 업데이트·제거 안내를 포함한다.

## Q7. 첫 제품의 작업 순서는 어떤 판단 기준을 우선해야 하나요?

지속 작성 가치와 파일 안전성 모두 중요하므로, 무엇을 먼저 입증할지 결정해야 이후 backlog 순서가 일관됩니다.

- A. 위험 우선 — Vault 경계, 충돌 거부, 원문 보존을 먼저 입증한다.
- B. 가치 우선 — 설치부터 첫 구조화 노트까지의 end-to-end 흐름을 먼저 입증한다.
- C. walking skeleton 우선 — 최소 연결·읽기·쓰기·확인을 세로로 관통한 뒤 위험도와 의존성을 반영해 안전성·안내를 확장한다.
- X. Other (please specify)

[Answer]: C. walking skeleton 우선 — 최소 연결·읽기·쓰기·확인을 세로로 관통한 뒤 위험도와 의존성을 반영해 안전성·안내를 확장한다.

## Q8. 첫 릴리스에 외부 마감이나 고정된 범위 제약이 있나요?

기한이 없다면 품질 게이트 중심으로 계획하고, 기한이 있다면 Must/Should 경계를 더 강하게 잘라야 합니다.

- A. 고정 마감은 없고, 승인된 안전성·사용성 기준 충족을 릴리스 조건으로 둔다.
- B. 가까운 시범 사용 일정이 있으므로 최소 end-to-end 흐름을 먼저 제공해야 한다.
- C. 특정 OKC 릴리스와 맞춰야 하므로 호환성 확인 시점이 마감이다.
- X. Other (please specify)

[Answer]: A. 고정 마감은 없고, 승인된 안전성·사용성 기준 충족을 릴리스 조건으로 둔다.

## Consolidated Summary Confirmation

Does this all look correct before I generate the artifact?

- Looks correct
- Request changes

[Answer]: Looks correct
