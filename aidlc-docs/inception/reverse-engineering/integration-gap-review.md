# OKC 네 모듈 통합 누락 검토

검토일: 2026-09-09 KST. 범위: 저장소 루트 통합 프로젝트. 방법: 현행 코드, 모듈별 AI-DLC 요구사항과 설계, 기존 테스트 대조. 구현 변경은 요청되지 않았으며 수행하지 않았다.

## 결론

네 모듈의 큰 역할 구분은 적절하다. 그러나 현재 구현은 그 역할들이 하나의 연속된 흐름으로 연결된 상태가 아니다. 가장 먼저 해결할 부분은 hooks/web 업로드 프로토콜, 같은 Vault의 반복 업데이트, MCP의 웹 조회 경로다. “충돌 시 고르기”는 기존에 승인된 모순 보존 정책과 의미를 확인해야 한다.

아래에서 “확인된 불일치”는 소스 코드로 확인한 사실이고, “결정 필요”는 사용자 설명을 완성하기 위한 제안이다. 제안을 승인된 요구사항으로 취급하지 않는다.

## 확인된 불일치와 책임 공백

### G1. hooks가 보내는 요청을 web이 받을 수 없음 — 우선

- hooks는 임시 CBOR 프로토콜로 negotiate → blob 분할 전송 → commit을 수행한다.
- web은 multipart 파일을 받는 `POST /u/{token}/upload`를 구현했다. 응답도 hooks의 commit 결과와 다르다.
- 인증 전달도 hooks의 Authorization Bearer 헤더와 web의 경로 토큰 해석을 맞춰야 한다.
- 따라서 현재 설정값만 연결해 자동 업로드를 완성할 수 없다.

근거: [hooks protocol.rs:1](../../../okc-hooks/crates/upload-client/src/protocol.rs), [인증 조립:46](../../../okc-hooks/crates/auth-consent/src/transport/assembler.rs), [web upload router:86](../../../okc-web/backend/app/upload/router.py), [web upload models:92](../../../okc-web/backend/app/upload/models.py), [web upload auth:109](../../../okc-web/backend/app/shared/authz.py).

필요 계약: 버전이 있는 업로드 프로토콜 하나, 토큰 전달 방식, payload/content hash의 의미, 재시도 가능한 오류, 수락과 최종 반영의 구별, 업로드 클라이언트의 작업 상태 확인 권한. 전체 스냅샷 업로드와 분할 전송 중 어느 쪽을 채택할지는 구현 계획에서 결정한다.

### G2. 같은 Vault의 수정이 새 Vault 추가로 처리됨 — 우선

- web은 변경된 업로드마다 새로운 `src_ULID`와 소스 슬롯을 만들고 `add_source`를 호출한다.
- 프로젝트 소스 상한은 10개다. 업로드 계약을 먼저 연결하더라도 동일 Vault가 성공적으로 10번 다른 내용으로 등록된 뒤 다음 변경은 상한에 걸릴 수 있다.
- 이전 버전이 독립 소스로 남으므로 삭제·이동된 노트가 과거 소스에서 다시 병합될 수 있다.
- 같은 내용의 중복 거절은 일부 구현되어 있지만, 최신 revision 교체나 멱등 성공 영수증과 같은 의미가 아니다.
- core Python API에는 이미 `rebind_source`와 `replace_sources`가 있다. web의 소스 식별과 업데이트 연동이 핵심이다.

근거: [ingest.py:368,385,440](../../../okc-web/backend/app/upload/ingest.py), [SOURCE_CAP:13](../../../okc-web/backend/app/upload/models.py), [Python source API:257](../../../okc-core/bindings/python/okc/__init__.py).

필요 계약: 안정적인 project/vault/source 식별자, revision/base revision, 최신 성공 스냅샷, 삭제·rename 표현, 동시 업로드 및 재전송 처리. 보관 이력과 현재 병합 대상은 구분해야 한다.

### G3. MCP의 웹 우선 조회와 통합 Vault reader가 없음 — 우선

- MCP 설정은 로컬 `vaultPath`, `statePath`, 제한값만 허용하며 모든 실행이 파일시스템 Vault를 생성한다.
- web URL/project/read credential 설정과 웹 미설정 시 로컬 선택 분기가 없다.
- 현재 Vault 구현은 읽기 전용에서도 `.okc`가 있는 컴파일 산출물과 `.okc-project`를 거절한다. 통합 Vault를 내려받아 경로만 지정하는 방식으로도 연결되지 않는다.
- 이는 현재 MCP가 로컬 원본 작성 도구로 설계되었기 때문이다.

근거: [config.ts:8](../../../okc-mcp/src/config.ts), [cli.ts:78](../../../okc-mcp/src/cli.ts), [vault.ts:115,164](../../../okc-mcp/src/vault.ts), [MCP 요구사항](../../../okc-mcp/aidlc-docs/inception/requirements/requirements.md).

필요 계약: 조회 source 선택, compiled 전용 reader, 웹 설정 없음과 인증 실패·미게시·timeout의 구분. 사용자 문장의 “설정 안됐으면 local”은 장애 발생 시에도 자동으로 local로 바꾸라는 승인으로 해석하지 않는다.

### G4. MCP가 읽는 곳과 쓰는 곳의 구분 필요

web의 게시 Vault는 읽기 전용이다. 현재 MCP의 작성과 검색은 한 Vault 객체를 공유한다. 웹 조회를 기본으로 바꿀 때 노트 생성·수정 대상도 명시해야 한다.

권고: 조회는 web 우선/local 대체, 작성은 명시된 local 원본 Vault. 작성 결과는 hooks 업로드와 통합·게시 후 웹 지식에 반영한다. 원격 작성이 필요하다면 web의 별도 원본/제안 입력 계약이 추가로 필요하다. 게시 산출물 직접 수정은 기존 core 불변 산출물 계약과 맞지 않는다.

근거: [MCP server.ts:20,171](../../../okc-mcp/src/server.ts), [web 읽기 API:65](../../../okc-web/backend/app/serving/router.py).

### G5. “충돌에서 고르기”와 기존 정책의 의미 확인 필요

기존 core/web은 모순된 주장을 보존하고 taxonomy/cluster 승인, 재생성, 명시적 omission, minor waiver를 제공한다. 상충 주장 중 하나를 진실로 선택해 다른 주장을 없애는 winner selection은 이전 요구사항에서 제외되었다.

따라서 충돌 UI 전체가 없다고 볼 수 없다. 사용자가 말한 “고르기”가 현재 승인·재생성 흐름인지, 파일 충돌의 버전 선택인지, 의미 충돌의 채택 주장 선택인지 구분해야 한다. 마지막 의미라면 기존 모듈 정책을 변경하는 결정이다. 선택의 근거·결정자·revision과 원본 provenance를 함께 남기는 기준도 필요하다.

근거: [기존 결정 기록:18](../../../okc-web/aidlc-docs/inception/requirements/decision-records.md), [review service:7](../../../okc-web/backend/app/review/service.py), [모순 렌더링:1737](../../../okc-core/crates/okc-core/src/integration.rs).

### G6. 업로드 이후 통합·검토·게시의 실행 시점이 빠져 있음

현재 업로드는 소스 등록 작업을 큐에 넣는다. freeze/integrate/compile/publish는 별도 관리자 작업이다. 사용자 요청은 hooks의 자동 업로드를 명시했지만 자동 병합·자동 게시까지 명시하지는 않았다.

필요 결정: 업로드마다 자동 실행할지, 변경을 모아 실행할지, 관리자가 실행할지. 검토가 필요한 동안 신규 업로드를 어느 revision에 포함할지, 이전 게시본 유지 여부, 취소·실패 재시작, hooks에서 확인 가능한 완료 상태도 정해야 한다. 현재 상태를 설명할 때 upload accepted, source registered, review pending, published를 구별해야 한다.

근거: [ingest.py:440](../../../okc-web/backend/app/upload/ingest.py), [orchestration routes:69,98,106](../../../okc-web/backend/app/orchestration/router.py), [publish route:46](../../../okc-web/backend/app/serving/router.py).

### G7. 게시 revision과 MCP 최신성 연결이 불완전함

web에는 live/stale, verify/explain, 게시 경로 저장이 있다. 하지만 publish가 읽는 engine manifest는 프로젝트 manifest이고, 거기서 컴파일 결과의 plan/taxonomy ID를 찾는다. 필드가 없으면 ID가 None이 되고, 산출물 경로는 compiled 하위 디렉터리의 수정 시각으로 선택한다.

또 파일 목록과 파일 본문은 각각 현재 publication을 조회한다. 중간에 재게시하면 서로 다른 게시본을 읽을 가능성이 있다. 이 경쟁 상황은 정적 분석 결과이며 재현 테스트를 실행한 것은 아니다.

필요 계약: 검증된 산출물의 명시적 revision/manifest/hash, 원자적 게시 포인터, revision에 고정된 목록·본문·provenance 조회, MCP 캐시 갱신과 stale 표시. 마지막 정상 게시본 및 이전 게시본 복원 정책도 이 경계에서 정한다.

근거: [serving/service.py:144,165,192,228](../../../okc-web/backend/app/serving/service.py), [core manifest 반환:1134](../../../okc-core/crates/okc-interop/src/lib.rs), [ProjectManifest:173](../../../okc-core/crates/okc-app/src/lib.rs).

### G8. 읽기 공개 범위와 업로드 포함 범위의 결정 필요

web 게시 읽기 API는 현재 인증이 없다. 공개 지식 공유 서비스라면 가능한 정책이지만, 비공개 팀 Vault를 의도한다면 project별 read 권한과 MCP 인증이 추가로 필요하다. 업로드 토큰이 읽기 권한까지 제공한다고 가정할 수 없다.

hooks scanner는 숨김 파일을 포함한 일반 파일 전체를 수집하며 exclude 패턴이 없다. core/web의 게시 콘텐츠는 Markdown 중심이다. 원본 보존 범위와 게시 범위, `.obsidian`·`.git`·운영 파일의 전송 제외 규칙, 첨부파일 취급을 명시해야 한다.

근거: [serving/router.py:9,65](../../../okc-web/backend/app/serving/router.py), [hooks scanner:3,33,65](../../../okc-hooks/crates/content-core/src/scanner.rs), [web 게시 형식:83](../../../okc-web/backend/app/serving/models.py).

### G9. “정형화된 Vault”와 Obsidian 호환 범위의 완료 기준 필요

MCP에는 노트 생성·수정·YAML/링크 검사 등이 있다. 그러나 초기화는 기존 디렉터리를 요구하며, 폴더와 추가 metadata는 대체로 작성 관습이다. 정형화된 새 Vault를 만드는 제품 기능은 초기 디렉터리 생성, 선택적 템플릿, 공통 metadata/출처 규약과 검증 기준을 정의해야 한다.

core에는 첨부파일·Canvas/Base 전달과 전체 링크 재작성의 미완료 항목이 명시되어 있다. “Vault 병합”을 전체 Obsidian 데이터의 완전 보존으로 해석하면 현재 구현보다 넓어진다. MVP를 Markdown 지식 병합으로 할지 별도 결정이 필요하다.

근거: [MCP vault.ts:111](../../../okc-mcp/src/vault.ts), [notes.ts](../../../okc-mcp/src/notes.ts), [guide.ts](../../../okc-mcp/src/guide.ts), [core 현재 한계:291](../../../okc-core/docs/CURRENT_STATE.md).

### G10. 검색 품질과 출처 반환의 책임 정합 필요

web 문서는 청킹·임베딩·인덱스·검색을 MCP 책임으로 둔다. 현재 MCP는 로컬 문자열 검색이며 지속 인덱스와 임베딩은 범위 밖이다. 따라서 게시 통합 지식의 검색 경로는 별도 구현이 필요하다.

에이전트 지식 활용에 벡터 검색이 필수인 것은 아니다. 우선 웹 read/search와 결과의 source/project/revision/stale/provenance 반환을 정하고, 규모와 검색 요구에 따라 전문 검색 또는 의미 검색을 선택할 수 있다.

근거: [web MCP 계약:94](../../../okc-web/backend/app/serving/models.py), [MCP 검색:78](../../../okc-mcp/src/server.ts), [retrieval 요구사항](../../../okc-mcp/aidlc-docs/inception/requirements/requirements-retrieval-unit.md).

### G11. 자동 업로드 재시도 시점의 추가 점검 필요

hooks는 debounce, 시작 스캔, 감시, 주기적 reconciliation, 전송 상태 복구를 이미 갖춘다. 다만 coordinator의 retryable 분기는 지연 후 별도 재시도 trigger를 넣지 않아 다음 외부 이벤트 또는 reconciliation을 기다리는 경로가 있다. 기본 reconciliation은 900초다.

실서버 계약을 연결할 때 네트워크 복구 후 추가 파일 편집 없이도 정해진 시간에 업로드가 재개되는지 검증해야 한다. 새 영속 이벤트 큐를 필수로 추가해야 한다는 의미는 아니다. 기존 hooks 요구사항은 마지막 committed manifest + dirty + resume offset 방식을 명시적으로 채택했다.

근거: [coordinator.rs:239,379](../../../okc-hooks/crates/watcher-bin/src/coordinator.rs), [daemon config:52](../../../okc-hooks/crates/watcher-bin/src/config.rs), [hooks 승인된 요구사항](../../../okc-hooks/aidlc-docs/inception/requirements/requirements.md).

### G12. 네 모듈을 관통하는 검증이 없음

hooks 문서는 실서버 계약을 mock으로 검증했다고 명시한다. web의 실제 core 테스트는 로그인 → 토큰 → 업로드 → add_source 경로를 다룬다. MCP 통합 테스트는 로컬 stdio와 파일시스템을 다룬다. 이 증거를 합쳐 네 모듈 전체 통합이 통과했다고 볼 수 없다.

필수 통합 시나리오 제안:

1. MCP 원본 작성 → hooks 감지 → web 업로드/소스 갱신 → core 검토·컴파일 → 게시 → MCP 웹 조회.
2. 같은 Vault를 11회 이상 수정해도 독립 소스 수는 증가하지 않음.
3. 삭제·rename, 중복 재전송, 중단 후 재시도, 순서가 뒤바뀐 revision을 처리.
4. 충돌 검토 중 새 업로드와 재게시 중 MCP 조회가 각 revision의 일관성을 유지.
5. 웹 미설정, 인증 실패, 미게시, stale 상태가 구분됨.
6. 원본 작성용 MCP가 게시 산출물을 수정하지 않음.

근거: [hooks 통합 검증 범위](../../../okc-hooks/aidlc-docs/construction/build-and-test/integration-test-instructions.md), [web 실제 spine](../../../okc-web/backend/tests/test_w1_spine.py), [MCP 통합 검증 범위](../../../okc-mcp/aidlc-docs/construction/build-and-test/integration-test-instructions.md).

## 권고하는 역할 보완

| 모듈 | 사용자 설명에 보완할 책임 |
|---|---|
| okc-core | 병합 정책, 모순/중복 판정, 검토 결정 검증, provenance, 검증 가능한 불변 산출물 생성. |
| okc-hooks | 시작 전체 스캔, 변경·삭제 감지, stable Vault revision 업로드, 전송 재개, 최종 수락 확인, 상태 표시. |
| okc-mcp | 원본 Vault 초기화·작성·검증과 통합 Vault 조회를 분리; 웹 우선 source 선택, 검색, 출처·revision 반환. |
| okc-web | 인증/프로젝트/소스 revision 관리, 업로드 계약, 통합 작업과 검토·게시 조정, 버전 고정 읽기 API. |

통합 결과를 다시 local 원본으로 내려보내는 양방향 동기화는 현재 사용자 설명에 없다. 필요하면 별도 범위이며, 게시물을 다시 업로드해 반복 병합하는 루프를 막는 설계가 동반되어야 한다.

## 검증 및 한계

- 현행 코드와 모듈 요구사항을 정적으로 대조했다.
- MCP 기존 config/vault 테스트 15개를 실행했고 모두 통과했다: `node --import tsx --test tests/config.test.ts tests/vault.test.ts`.
- 이 테스트는 현재 로컬 설정과 compiled artifact 거절 동작의 증거이며, 미구현 웹 연결의 검증은 아니다.
- 실서버 업로드, provider 호출, 네 모듈 전체 end-to-end는 실행하지 않았다.
- AI-DLC extension은 opt-in 대기다. 해당 확장 규칙의 준수 판정은 N/A이며, 제품 자체 계약에서 확인한 문제를 보고했다.
- 본 문서는 검토 결과다. 기존 모듈 요구사항을 변경하거나 후속 구현을 승인하지 않는다.
