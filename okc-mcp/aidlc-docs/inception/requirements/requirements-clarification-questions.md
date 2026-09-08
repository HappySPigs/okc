# 요구사항 명확화 질문 (Clarification)

답변에서 확정 전 짚어야 할 지점 2가지가 나왔습니다. 각 `[Answer]:` 뒤에 letter를 적고, 다 되면 "완료"라고 알려 주세요.

---

## 충돌 1 — "정리·구조화"(Q4=B) vs "파일 재구성 없음"(Q7=A)

Q4에서 **정리·구조화(기존 노트 정돈)**를 첫 Unit 최우선 여정으로 고르셨습니다.
Q7에서는 **이동·이름변경·삭제 없음, 안전한 부분 업데이트만 + 외부 백업(현재 초안)**을 고르셨습니다.

그런데 "노트 정돈"은 보통 **폴더 이동·이름변경·중복 병합** 같은 파일 단위 재구성을 포함합니다. 이건 Q7=A가 배제하는 동작이라, 첫 Unit에서 "정리·구조화"가 구체적으로 어디까지인지 확정해야 합니다.

### Clarification Question 1
첫 Unit에서 "정리·구조화"는 어디까지 포함합니까?

A) **노트 내부 정돈만** — frontmatter(title/aliases/tags) 표준화, 잘못된 YAML 수정, 근거·링크 보강, 품질 문제 감사·플래그. 파일 이동·이름변경·병합은 **없음** (Q7=A 유지, 파일 재구성은 후속 Unit). ← Q7 답변과 일관됨

B) **노트 내부 정돈 + 제한적 파일 재구성** — 사용자 확인을 전제로 이름변경·이동까지 첫 Unit에 포함. (이 경우 Q7을 A→B로 조정)

C) **파일 재구성이 첫 Unit의 핵심** — 이동·이름변경·중복 병합을 첫 Unit으로 끌어옴. (Q7을 B로 바꾸고, 현재 exclusions의 rename/move/dedup을 첫 Unit으로 승격)

X) Other (please describe after [Answer]: tag below)

[Answer]: A

---

## 확인 2 — 보안 확장 끔(Q9=B)의 범위

Q9에서 **보안 확장 규칙(AWS security-baseline 룰셋)을 끄기로** 하셨습니다.

확인: 이건 "추가적인 보안 엔지니어링 룰셋을 blocking constraint로 강제하지 않는다"는 뜻이고, **제품 자체의 보안 요구사항은 그대로 유지**된다는 전제입니다 — REQ-008(Vault 경계 강제, symlink/hardlink·숨김 경로 거부, 파일 크기·개수·응답 크기 제한), REQ-011(노트를 신뢰 불가 데이터로 취급, shell·임의 HTTP·삭제 도구 미노출). 이 제품의 핵심 가치가 신뢰 경계라서 명시적으로 확인합니다.

### Clarification Question 2
보안 확장을 끄는 범위가 맞습니까?

A) **맞음** — 확장 룰셋만 끄고, REQ-008 / REQ-011 제품 보안 요구사항은 **유지**

B) 아니오 — 제품 보안 요구사항도 일부 완화하고 싶음 (X에 구체적으로 기술)

X) Other (please describe after [Answer]: tag below)

[Answer]: A
