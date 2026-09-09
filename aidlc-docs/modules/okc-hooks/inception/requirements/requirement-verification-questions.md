# 요구사항 확인 질문 (Requirements Verification Questions)

**대상 프로젝트**: `okc-hooks` — 로컬 설치형 **Watcher** 앱 (Obsidian vault를 주기적으로 모니터링 → 변경 시 OKC 웹서비스 API로 업로드)

**작성 방법**: 각 질문의 `[Answer]:` 태그 뒤에 해당하는 **알파벳(A, B, C ...)** 을 적어주세요. 보기 중 맞는 것이 없으면 마지막 `X) Other`를 고르고 뒤에 자유롭게 설명해 주세요. 다 작성하시면 "완료" 또는 "done"이라고 알려주세요.

---

## 배경: okc-core 분석 요약 (질문 이해용)

Watcher가 업로드할 대상 웹서비스는 `../okc-core/`(OKC — Obsidian Knowledge Compilation) 위에 세워집니다. 프로토콜 질문에 답하시기 전에 알아두면 좋은 핵심 제약을 먼저 정리했습니다.

- **통짜 스냅샷 방식**: okc-core는 vault를 **불변(immutable) 원자 스냅샷** 하나로 받습니다. 입력 단위는 로컬 경로(디렉터리 / `.zip` / `.tar.zst`)이며, **증분(delta) 수집이나 바이트 스트림 업로드 API는 없습니다.** 웹서비스는 업로드받은 바이트를 **서버 로컬 디스크에 먼저 풀어놓은 뒤** 경로를 바인딩해야 합니다.
- **전면 SHA-256 콘텐츠 주소화**: 파일별 `raw_sha256`(외부 `sha256sum`과 동일)과 vault 전체 `vault_content_id`가 이미 계산됩니다. → **"서버에 없는 blob만 전송"하는 dedup 프로토콜이 가능**합니다. 단, dedup 최소 단위는 **파일 하나 전체**입니다(파일 내부 청크 분할 기능은 없음).
- **크기 상한(SafetyLimits)**: 총 20 GiB / 파일당 2 GiB / 최대 10만 파일 / vault(source) 최대 10개. 이 한계는 **업로드 시점이 아니라 컴파일 시점에** 검사됩니다.
- **보안 모델**: 현재 okc-core에는 **인바운드 인증/멀티테넌시 개념이 전혀 없습니다.** vault를 원격으로 보내는 것은 "소스 유출(disclosure)"로 취급되어, **로컬 민감정보 사전검사(sensitive preflight)** 와 **호출마다 새로 받는(persist 안 되는) 명시적 동의**의 대상이 됩니다. → **백그라운드 자동 동기화 Watcher는 이 동의 모델과 충돌**하므로 제품 결정이 필요합니다.

---

## 섹션 1. 확장 규칙 적용 여부 (Extensions)

### Question 1
이 프로젝트에 **보안(Security) 확장 규칙**을 강제할까요?

A) 예 — 모든 SECURITY 규칙을 차단성(blocking) 제약으로 강제 (실서비스/프로덕션 수준 권장)

B) 아니오 — SECURITY 규칙 생략 (PoC·프로토타입·실험용에 적합)

X) Other (please describe after [Answer]: tag below)

> 참고: 이 Watcher는 로컬 파일을 원격으로 업로드하고 인증을 새로 설계해야 하므로 보안 확장(A)을 권장합니다.

[Answer]: B

### Question 2
이 프로젝트에 **복원력(Resiliency) 베이스라인**을 적용할까요?

이 확장은 AWS Well-Architected(신뢰성 기둥) 기반의 **설계 단계 모범사례(내결함성·가용성·관측성·복구성)** 를 요구사항/설계/코드에 반영하도록 안내합니다. 프로덕션 준비 완료를 보장하는 것은 아니며 **좋은 출발점**입니다.

A) 예 — 복원력 베이스라인을 방향성 있는 설계 지침으로 적용 (업무상 중요한 워크로드 권장)

B) 아니오 — 복원력 베이스라인 생략 (빠른 반복이 더 중요한 PoC·프로토타입에 적합)

X) Other (please describe after [Answer]: tag below)

> 참고: 네트워크 불안정/오프라인/재시도/백프레셔가 Watcher의 핵심 관심사이므로 적용(A)을 권장합니다.

[Answer]: A

### Question 3
이 프로젝트에 **속성 기반 테스트(Property-Based Testing, PBT)** 규칙을 강제할까요?

A) 예 — 모든 PBT 규칙을 차단성 제약으로 강제 (비즈니스 로직·데이터 변환·직렬화·상태 저장 컴포넌트가 있는 프로젝트 권장)

B) 부분 — 순수 함수와 직렬화 라운드트립에만 PBT 적용 (알고리즘 복잡도가 제한적인 프로젝트에 적합)

C) 아니오 — PBT 규칙 생략 (단순 CRUD·UI 전용·얇은 통합 계층에 적합)

X) Other (please describe after [Answer]: tag below)

> 참고: 스냅샷 해싱/매니페스트 비교/dedup 협상 등 결정론적 로직이 있어 최소한 부분 적용(B) 이상을 권장합니다.

[Answer]: A

---

## 섹션 2. 범위와 대상 (Scope)

### Question 4
Watcher가 **설치·실행될 운영체제(OS)** 는 무엇인가요? (복수 선택은 X에 적어주세요)

A) macOS 전용

B) Windows 전용

C) macOS + Windows + Linux (크로스플랫폼 데스크톱)

X) Other (please describe after [Answer]: tag below)

[Answer]: C

### Question 5
Watcher가 업로드하는 **대상**은 무엇인가요? (요청하신 표현은 "obsidian vault 업로드"였습니다)

A) **원본(raw) Obsidian vault** — 서버(okc-core)가 이후 컴파일 수행 *(요청 취지에 부합)*

B) 로컬에서 이미 **컴파일된 산출물(compiled artifact)** 만 업로드 (원본 소스는 전송하지 않음 — 유출 위험 최소화)

C) 선택 가능 — 사용자가 원본/컴파일본 중 선택

X) Other (please describe after [Answer]: tag below)

> 참고: A(원본 업로드)는 요청 취지와 맞지만, okc-core 보안 모델상 "소스 유출" 위협을 다시 도입하므로 **전송 전 로컬 민감정보 사전검사(Q11)** 와 **인증(Q12)** 설계가 반드시 동반되어야 합니다.

[Answer]: 컴파일 하는 로직이 있어?

---

## 섹션 3. 모니터링 & 트리거

### Question 6
Watcher가 vault 변경을 감지하고 업로드를 **트리거하는 방식**은?

A) **주기적 폴링(timer)** — N분마다 vault 해시를 다시 계산해 변경 시 업로드 *(요청하신 "특정 시간 주기" 방식)*

B) **파일시스템 이벤트 감시 + 디바운스** — 변경 즉시 감지 후 조용해지면(예: 30초) 업로드

C) **하이브리드** — 파일 이벤트로 감지하되 최소 간격 타이머로 병합/상한 적용 *(권장: 반응성 + 과도한 업로드 방지)*

D) **수동 트리거만** — 사용자가 명령할 때만 업로드

X) Other (please describe after [Answer]: tag below)

> 폴링 주기/디바운스 기본값 선호가 있으면(예: "5분", "변경 후 30초") `[Answer]:` 뒤에 함께 적어주세요.

[Answer]: B

---

## 섹션 4. 업로드 프로토콜 (핵심 — 대용량 대응)

### Question 7
"용량이 적지 않다"는 점을 고려해 **어떤 업로드 전략**을 채택할까요? (okc-core 실제 지원 기준으로 정리)

A) **통짜 아카이브 업로드** — vault 전체를 `.tar.zst` 하나로 묶어 매번 전송 (가장 단순, okc-core 입력 방식과 1:1 일치, 단 변경이 작아도 전량 재전송)

B) **콘텐츠 주소화 dedup (have/want)** — 파일별 `{경로, sha256}` 매니페스트를 먼저 보내고 **서버에 없는 파일만** 업로드 *(권장: 큰 정적 첨부/미디어 재전송 방지, okc-core가 이미 파일 해시 보유)*

C) **B + 재개형(resumable) 청크 전송(tus 방식)** — 위 dedup에 더해 대용량/불안정 네트워크용 중단-재개 지원

D) **파일 내부 델타(rsync 유사)** — 파일 하나 내부의 바뀐 바이트만 전송

X) Other (please describe after [Answer]: tag below)

> 참고: okc-core는 **파일 단위 dedup**까지만 자연스럽게 지원합니다(파일 내부 청크 분할/롤링 해시 없음). 따라서 **D는 전부 신규 구현**이 필요하고 이득이 제한적입니다. 권장 조합은 **B를 기본, 대용량/불안정 시 C를 추가**입니다.

[Answer]: B를 기본, 대용량/불안정 시 C

### Question 8
okc-core의 **크기 상한(총 20 GiB / 파일당 2 GiB / 10만 파일)** 초과 상황을 어떻게 다룰까요? (상한은 서버 컴파일 시점에 검사됨)

A) **업로드 전 사전 검증** — Watcher/서버가 전송 전에 크기·개수를 검사해 초과 시 즉시 거부 (대용량 전송 낭비 방지) *(권장)*

B) 일단 업로드 후 서버가 컴파일 시점에 판단 (구현 단순, 대신 큰 전송이 헛수고가 될 수 있음)

C) 상한을 넘는 vault는 애초에 지원 대상 아님으로 명시 (경고만)

X) Other (please describe after [Answer]: tag below)

[Answer]: A

---

## 섹션 5. 보안 · 동의 · 인증

### Question 9
**동의(consent) 모델** — okc-core는 원격 전송 동의를 "호출마다 새로 받고 저장하지 않음"이 기본입니다. 백그라운드 자동 동기화와 충돌합니다. 어떻게 할까요?

A) **상시(standing) 동의** — 최초 1회 사용자가 승인하면 이후 자동 업로드 (편의 최우선, 보안 모델과는 절충 — 승인 범위/철회 방법 명시 필요)

B) **업로드마다 재확인** — 매 업로드 전 명시적 확인 (보안 모델 충실, 자동화 편의는 낮음)

C) **배치/세션 동의** — 예: "오늘 하루" 또는 "이 세션 동안"처럼 기간·범위 한정 동의 *(권장: 자동화와 보안의 절충)*

X) Other (please describe after [Answer]: tag below)

[Answer]: A

### Question 10
**민감정보(개인키·API키·클라우드 자격증명·커넥션 문자열·이메일·전화번호 등) 처리** — okc-core는 민감정보가 포함된 문서의 원격 노출을 "무조건 금지(override 불가)"합니다. Watcher는 어떻게 할까요?

A) **전송 전 로컬 사전검사 후 해당 문서 제외/보류** — 민감 문서는 업로드에서 빼고 사용자에게 보고 *(권장, okc-core 규칙과 일치)*

B) **마스킹/레댁션 후 전송** — 민감 부분만 가리고 나머지 전송

C) **큐레이터 예외(SensitiveException) 기록 시에만 전송** — 사용자가 명시적으로 승인한 항목만

D) 민감정보 검사 없이 그대로 전송 *(보안 확장 A 선택 시 선택 불가)*

X) Other (please describe after [Answer]: tag below)

[Answer]: D

### Question 11
아직 만들지 않은 웹서비스가 **Watcher를 인증**하는 방식은? (okc-core엔 인바운드 인증이 없어 새로 정의해야 함)

A) **API 토큰/키** — 사용자가 발급받아 Watcher에 설정 (가장 단순)

B) **OAuth 2.0 / OIDC 로그인** — 브라우저 기반 사용자 로그인 후 토큰 발급

C) **상호 TLS(mTLS) / 클라이언트 인증서** — 강한 상호 인증

D) 지금은 미정 — 요구사항엔 "인증 필요"만 명시하고 상세는 웹서비스 설계 시 확정

X) Other (please describe after [Answer]: tag below)

[Answer]: A

---

## 섹션 6. 오프라인 · 버전 관리

### Question 12
**오프라인/일시적 실패/백프레셔**(서버 응답 지연, `PROJECT_BUSY`, 큐 초과 등) 시 Watcher 동작은?

A) **로컬 큐잉 + 지수 백오프 재시도**, 대기 중 변경은 최신본으로 병합(coalesce) *(권장)*

B) 실패 시 다음 주기까지 그냥 대기 (단순)

C) 실패 즉시 사용자에게 알림 후 수동 재시도

X) Other (please describe after [Answer]: tag below)

[Answer]: A

### Question 13
**버전/이력 관리** — okc-core는 제출마다 append-only run 저널에 기록하고, 변경된 vault 제출 시 이전 승인은 무효화(fail-closed)됩니다. Watcher/요구사항 수준에서 필요한 것은?

A) **업로드 이력(스냅샷 해시·시각·성공/실패) 로컬 보관 + 조회** (권장)

B) 최소한의 "마지막 성공 업로드"만 기억 (단순)

C) 이력 불필요 — 매번 상태 없는(stateless) 업로드

X) Other (please describe after [Answer]: tag below)

[Answer]: A

---

*(이 질문들에 답해주시면, 답변을 분석해 모순/모호함이 있으면 추가 질문을 드리고, 없으면 `requirements.md`(요구사항 문서)를 작성합니다.)*
