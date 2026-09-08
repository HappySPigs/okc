# Requirements 명확화 질문 — okc-web (Obsidian Vault 통합 웹 플랫폼)

아래 질문에 답해 주시면 요구사항 문서(`requirements.md`)를 정확히 작성할 수 있습니다.
각 질문의 `[Answer]:` 태그 **뒤**에 문자(A, B, C…)를 적어 주세요. 보기 중 맞는 것이 없으면 마지막 **X) 기타** 를 고르고 `[Answer]:` 뒤에 직접 설명해 주세요.
답이 여러 개면 쉼표로(`A, C`) 적어도 됩니다. 다 마치면 "완료" 라고 알려 주세요.

> **✅ 답변 상태**: 2026-09-08, 사용자 요청으로 **해커톤 권장값(recommended for hackathon)** 을 채웠습니다. (Q1은 사용자가 직접 A로 선택)
> 변경하고 싶은 항목이 있으면 해당 `[Answer]:` 를 수정하세요. 특히 **Q8(백엔드 스택)** 은 팀 숙련도에 따라 재검토를 권장합니다.

> **왜 이 질문들이 필요한가 (okc-core 분석 결과 핵심)**
> - okc-core에는 **인증/권한/RBAC/멀티테넌시가 전혀 없습니다** → 권한 관리(req 1·2)는 okc-web이 100% 새로 구현.
> - okc-core에는 **HTTP 서버·업로드·토큰·서빙·RAG/vector store가 없습니다** → 업로드(req 4)·서빙(req 5)은 전부 greenfield.
> - okc-core는 conflict를 "**승자 선택**" 하지 않습니다. 모순은 **보존**되고, 구조적 conflict는 내부에서 **폐기**되며, critic의 Major/Critical 지적은 **waive 불가**(재생성만 가능) → req 3의 의미를 정해야 함.
> - **프로젝트당 소스(Vault) 최대 10개** 상한이 있습니다 → 부서/개인 규모와 충돌.
> - 승인은 **hash-bound 단일 실행**: 소스가 하나라도 바뀌면 이전 승인이 전부 무효화됩니다.
> - **okc-mcp는 아직 존재하지 않습니다** → req 5의 RAG 서빙 주체를 정해야 함.
> - okc-core는 **0.3.0 개발 트리**(stable release 금지)이고 materializer는 **Markdown 전용**입니다.
>
> 자세한 근거: `aidlc-docs/inception/requirements/okc-core-capability-analysis.md`

---

## A. 배포 · 테넌시 · 권한

## Question 1
okc-web은 어떤 형태의 서비스인가요? (멀티테넌트 vs 단일 조직)

A) 단일 조직 · 단일 관리자 팀 · 공유 엔진 프로세스 1개 (사내 배포)

B) 멀티테넌트: 여러 조직을 **강하게 격리**하는 호스티드 서비스

C) 단일 조직이지만 **여러 부서를 소프트하게 격리**

X) 기타 (please describe after [Answer]: tag below)

[Answer]: A

## Question 2
관리자/개인은 어떻게 **인증**하며, 인증된 관리자를 okc-core의 (검증되지 않는) `curator_id` 문자열에 어떻게 바인딩하나요?

A) okc-web이 완전한 인증/인가(SSO/OIDC + RBAC)를 소유하고, 인증된 관리자 id를 `curator_id` 라벨로 기록

B) okc-web 내 단순 토큰/비밀번호 인증 + 역할(role) 플래그로 변경성 호출 게이팅

C) 인증을 앞단 게이트웨이/리버스 프록시에 위임하고 okc-web은 상위 identity 헤더를 신뢰

X) 기타 (please describe after [Answer]: tag below)

[Answer]: B

## Question 3
**부서/개인 Vault가 10개를 초과**할 때(okc-core는 프로젝트당 소스 10개 상한) 어떻게 처리하나요?

A) 페더레이션: 소스 ≤10개짜리 프로젝트를 여러 개 운영하고 산출물을 조합/링크

B) 사전 집계: 여러 개인 Vault를 소스 아카이브 몇 개로 먼저 합쳐서 인제스트

C) okc-core에 신규 ADR + 코어 변경으로 **상한을 상향** (upstream 의존)

X) 기타 (please describe after [Answer]: tag below)

[Answer]: X — 해커톤 데모는 **소스 10개 이하로 제한**하고 federation은 미구현(요구사항에는 향후 확장 "설계 방향"으로만 명시). 10개 초과 필요 시 B(사전 집계)로 폴백.

---

## B. 통합 · 충돌 · 워크플로우

## Question 4
**req 3 "관리자가 conflict를 확인·선택해서 통합"** 을, 승자 선택 API가 없는 OKC 모델에서 구체적으로 어떻게 정의하나요?

A) **기존 API로 재해석**: 클러스터 승인 / minor 지적 waive / omission 제안 / 피드백으로 재생성(regenerate) — (신규 코어 작업 불필요, 권장)

B) **진짜 승자 선택 + 구조적 path-conflict 선택**을 요구 (okc-core에 **신규 public API 필요** — 현재 내부 conflict는 폐기됨)

C) 관리자가 병합 결과를 **직접 수기 편집** (미구현 manual-amendment 기능 ADR-0029 의존)

X) 기타 (please describe after [Answer]: tag below)

[Answer]: A

## Question 5
업로드는 **계속 흘러 들어오나요**, 아니면 소스를 **고정한 뒤** 1회 통합+리뷰 패스를 도나요?

A) **Freeze-then-run**: 소스 고정 → 통합 + 리뷰 + 컴파일 1회 (okc-core의 hash-bound 단일 실행 모델에 부합, 권장)

B) **증분(incremental)**: 업로드마다 재통합/부분 병합 허용 (변경 시마다 전 승인 재수행 비용)

C) **스케줄 배치**: 정해진 시각에 소스를 스냅샷 후 컴파일

X) 기타 (please describe after [Answer]: tag below)

[Answer]: A

## Question 6
**개인별 업로드**(req 4)의 토큰 보관과 업로드 바이트 검증/등록 방식은?

A) okc-web이 **토큰 스토어 + 검증**을 소유, 검증된 바이트를 테넌트별 디스크 경로에 착지 후 `add_source`(owner_display_name 부여)

B) 오브젝트 스토리지(S3/blob)에 착지 후, 엔진이 읽을 수 있는 로컬 절대경로로 동기화

C) 서명된 직접 업로드 URL + 서버측 검증 후 등록

X) 기타 (please describe after [Answer]: tag below)

[Answer]: A

---

## C. req 5 서빙/RAG · 통합 방식

## Question 7
**okc-mcp는 지금 범위에 포함되나요? req 5(병합 Vault를 RAG로 서빙)의 주체는 누구인가요?** (okc-mcp는 현재 미구현)

A) okc-web이 지금 컴파일된 디렉터리 위에 **자체 read-only HTTP/파일 API**를 제공, okc-mcp는 이연

B) okc-web은 Vault **생성/검증만** 하고, 서빙 + RAG 리트리벌은 (앞으로 만들) **okc-mcp가 소유**

C) 리트리벌을 기존 외부 엔진(예: obsidian-mcp)에 위임하고 okc-web은 디렉터리만 노출

X) 기타 (please describe after [Answer]: tag below)

[Answer]: A

## Question 8
okc-web **백엔드를 okc-core에 붙이는 방식**은? (분석 권장: Rust 백엔드가 `okc-interop` 를 직접 링크)

A) **Rust 백엔드(예: axum)** 가 `okc-interop`/`okc-app` 를 path dependency로 직접 링크 (타입드 DTO·async Job·구조화 에러 확보, 권장)

B) **Node.js 백엔드** + okc-core Node 바인딩 (payload는 불투명 JSON, 미배포 소스빌드)

C) **Python 백엔드** + okc-core Python 바인딩 (result() 블로킹, threadpool 필요)

X) 기타 (please describe after [Answer]: tag below)

[Answer]: A  (⚠️ 팀이 Rust에 익숙하지 않으면 C(Python)로 변경 권장 — 단 네이티브 바인딩 소스 빌드 셋업 비용 감수)

---

## D. 프로세스 토폴로지 · 안정성

## Question 9
런타임 **프로세스 토폴로지**는? (scheduler/reservation은 프로세스 전역, cross-process는 `project.lock`만 보호)

A) **단일 장수 엔진 프로세스**가 모든 프로젝트 소유, okc-web이 통합 실행을 직렬화 (권장·단순)

B) 다중 워커 프로세스 + `project.lock` 위에 **외부 분산 락/큐** 추가

C) **테넌트별 격리된 엔진 프로세스**

X) 기타 (please describe after [Answer]: tag below)

[Answer]: A

## Question 10
okc-web은 **pre-stable 0.3.0 코어**(stable release 금지, Markdown 전용 출력, 바인딩 미배포) 위에서 빌드하는 것을 수용하나요?

A) 수용: **Markdown 전용 출력**을 받아들이고 특정 okc-core commit에 핀 고정

B) okc-core가 release blocker(첨부/Canvas/link-rewrite/OKCPack, semantic scale)를 해소할 때까지 **GA 대기**

C) 지금은 **Markdown 전용 한정 베타**로 출시, GA는 이후

X) 기타 (please describe after [Answer]: tag below)

[Answer]: A

---

## E. 확장(Extensions) 옵트인

## Question 11: 보안(Security) 확장
이 프로젝트에 보안 확장 규칙을 강제할까요?

A) 예 — 모든 SECURITY 규칙을 **차단성 제약(blocking)** 으로 강제 (프로덕션급 애플리케이션 권장)

B) 아니오 — 모든 SECURITY 규칙 생략 (PoC·프로토타입·실험 프로젝트에 적합)

X) 기타 (please describe after [Answer]: tag below)

[Answer]: B  (해커톤 PoC. 단 업로드 파일 크기/확장자 기본 검증은 상식선에서 유지)

## Question 12: 복원력(Resiliency) 확장
이 프로젝트에 resiliency baseline을 적용할까요?

**이 확장이란**: 활성화하면 AWS Well-Architected(신뢰성 기둥) 기반의 **설계 시점 best practice** 세트를 적용해 요구사항·설계·코드를 결함 허용, 고가용성, 관측가능성, 복구성 방향으로 유도합니다(15개 실천 영역).
**이 확장이 아닌 것**: 워크로드를 프로덕션 레디로 만들거나 특정 가용성/RTO/RPO를 보장하지 않습니다. 정식 Well-Architected 리뷰의 대체가 아닌 **좋은 초안**입니다.

A) 예 — resiliency baseline을 방향성 best practice·설계 시점 가이드로 적용 (비즈니스 크리티컬 워크로드 권장)

B) 아니오 — resiliency baseline 생략 (빠른 반복이 중요한 PoC·프로토타입에 적합)

X) 기타 (please describe after [Answer]: tag below)

[Answer]: B

## Question 13: 속성 기반 테스트(Property-Based Testing) 확장
PBT 규칙을 이 프로젝트에 강제할까요?

A) 예 — 모든 PBT 규칙을 차단성 제약으로 강제 (비즈니스 로직·데이터 변환·직렬화·상태 컴포넌트가 있는 프로젝트 권장)

B) 부분 — 순수 함수와 직렬화 round-trip에만 PBT 강제 (알고리즘 복잡도가 제한적인 프로젝트)

C) 아니오 — 모든 PBT 규칙 생략 (단순 CRUD·UI 전용·얇은 통합 계층)

X) 기타 (please describe after [Answer]: tag below)

[Answer]: C
