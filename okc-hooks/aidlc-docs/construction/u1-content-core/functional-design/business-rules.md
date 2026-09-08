# U1 Deterministic Content Core — Business Rules (결정 규칙 / 검증 로직 / 제약)

**단계**: CONSTRUCTION -> Per-Unit Loop -> **U1 `content-core`** -> Functional Design -> 산출물 2/3 (`business-rules.md`)
**작성일**: 2026-09-08
**크레이트**: `content-core` (lib) · **소속 컴포넌트**: `ContentAddressing`, `VaultScanner`, `ManifestBuilder`, `ManifestDiffer`, `SafetyLimitsValidator`
**전제(드롭리스트)**: FQ-1=A · FQ-2=A · Q2=A(SafetyLimits 상수 = U0) · U0 `CoreTypes`/코덱/`RelativePath` 정규화 구현 완료.

> **문서 성격**: U1이 소유하는 **결정 규칙·검증 로직·제약·엣지 케이스 판정**(R-*) + NFR별 **Testable Properties**(PROP-U1-*). 타입은 `domain-entities.md`, 알고리즘/흐름은 `business-logic-model.md`가 소유하며 그 이름을 그대로 재사용한다.
>
> **표기 규약**: 기술중립. Rust 시그니처는 참고용. ASCII 화살표(`A -> B`)만, 박스/유니코드 없음. Rust 타입은 backtick.

---

## 1. ContentAddressing 규칙 (해싱 결정성 — NFR-08)

**R-CA-01 (표준 SHA-256, FQ-1=A)**: `hash_stream`은 입력 바이트 스트림에 대해 **표준 SHA-256**을 계산하며, 그 결과는 `sha256sum` 및 okc-core `raw_sha256`과 **바이트 단위로 일치**한다. okc-core 콘텐츠 주소 스킴을 재현하지 않는다(권위 `vault_content_id`는 서버).

**R-CA-02 (결정성)**: 동일한 바이트 스트림은 반복 계산·환경·플랫폼과 무관하게 항상 동일한 `Sha256Digest`를 산출한다. 빈 스트림(0바이트 파일)은 SHA-256의 표준 빈-입력 다이제스트를 낸다(엔트리로 정상 포함).

**R-CA-03 (스트리밍 메모리 바운드, NFR-02)**: 해시는 고정 버퍼로 증분 계산하며 파일 전체를 메모리에 적재하지 않는다 — 100k 파일 / 20 GiB 상한에서도 무한정 메모리를 쓰지 않는다. 구체 버퍼 크기는 NFR Design 이월.

**R-CA-04 (`manifest_digest` 인코딩, D1)**: `manifest_digest(entries)`는 다음을 강제한다:
- 엔트리를 `relative_path` 바이트 사전식 오름차순으로 정렬(tie-break `(raw_sha256, size)`).
- 각 엔트리를 **길이-프리픽스 프레이밍**으로 직렬화: `len(path)`(고정폭 BE) + path 바이트 + `raw_sha256`(32바이트) + `size`(고정폭 `u64` BE). 전체 concat 위에 표준 SHA-256.
- 이 프레이밍은 U0 CBOR 지속 코덱과 **분리**되어(버전 간 안정) no-op 판정을 진화에 견고하게 한다(`domain-entities.md` §3).

**R-CA-05 (`manifest_digest` 순서 무관 결정성)**: 논리적으로 동일한 엔트리 집합(같은 `{path, raw_sha256, size}` 다중집합)은 **발견/삽입 순서와 무관하게** 동일한 `manifest_digest`를 낸다(canonical 정렬 후 계산이므로). 이것이 no-op 판정(FR-10)과 diff 결정성의 기반이다.

---

## 2. VaultScanner 규칙 (열거 — 엣지 케이스)

**R-SCAN-01 (경로 정규화 위임)**: 열거된 각 파일시스템 경로는 볼트 루트 기준 상대경로로 변환 후 U0 `RelativePath::normalize`로 정규화한다. 정규화 실패(`..`/절대/드라이브 접두 — 정상 볼트 내부 열거에서는 발생하지 않아야 함)는 `ScanError::Io`(해당 경로)로 취급하지 않고 방어적으로 배제하되, 발생 시 진단 가능해야 한다(정상 열거 불변식: 루트 하위 경로는 항상 정규화 성공).

**R-SCAN-02 (심링크 SKIP, D8/MVP-trim)**: 심볼릭 링크(파일·디렉터리 모두)는 **따라가지 않고 엔트리에서 제외**한다. 근거: 루프(`a -> b -> a`)·볼트 루트 이탈(`link -> /etc`)을 최단 안전하게 차단. 심링크 대상 콘텐츠 동기화는 MVP 범위 밖.

**R-SCAN-03 (숨김 파일 포함, D10)**: dotfile/dotdir(`.obsidian` 등)은 **필터링 없이 포함**한다 — 정당한 볼트 콘텐츠다. 특수 처리 없음.

**R-SCAN-04 (exclude 패턴 없음, D9/MVP-trim)**: config 구동 제외 패턴을 지원하지 않는다. 데몬 산출물(SyncState/히스토리/로그)은 볼트 외부 플랫폼 상태 경로에 저장되어 self-churn이 발생하지 않으므로 MVP에서 제외 규칙이 불필요하다.

**R-SCAN-05 (루트 가용성 -> 값 반환, 판정은 U2)**: 루트 부재/언마운트/접근 불가는 `ScanError::RootUnavailable`로 **오류 반환**하며, 0-파일 매니페스트를 만들지 않는다(파괴적 빈 커밋 방지의 U1측 근거, US-E1-06). vault-unavailable **해석·표면화는 U2 `VaultAvailabilityGuard`/U8**이 수행하고 U1은 push하지 않는다(§0 노트2).

**R-SCAN-06 (열거 결정성)**: `VaultSnapshot.files`는 항상 `relative_path` 오름차순 canonical 정렬로 반환한다 — 파일시스템 walk 순서(디렉터리 반환 순서)와 무관하게 결정적 출력.

---

## 3. ManifestBuilder 규칙 (조립 — fail-fast)

**R-BUILD-01 (fail-fast, D12)**: `build`는 scan -> 파일별 `hash_stream` -> `ManifestEntry` -> 정렬 -> `manifest_digest` -> `Manifest`를 수행하며, **어느 파일이라도 해시 I/O 실패 시 전체를 중단**하고 `BuildError`를 반환한다. **부분 매니페스트(일부 파일 스킵)를 반환하지 않는다.** 근거: 스킵된 파일은 diff에서 "삭제"로 오인되어 파괴적 커밋을 유발할 수 있다 — 사이클 중단 후 다음 사이클 재스냅샷이 안전(FQ-2=A).

**R-BUILD-02 (스트리밍/메모리 바운드, NFR-02)**: 파일을 순차 스트리밍 해시하여 동시에 다수 파일을 메모리에 적재하지 않는다. 매니페스트 자체(엔트리 메타)는 파일 수 상한(100k)에 비례하는 유한 크기다.

**R-BUILD-03 (결정적 출력)**: 동일 볼트 상태(같은 파일 집합·내용)에 대해 `build`는 항상 동일한 `Manifest`(같은 엔트리·같은 `manifest_digest`)를 낸다 — R-CA-05 + R-SCAN-06 승계.

**R-BUILD-04 (파생값 일관)**: 산출 `Manifest`는 U0 불변식 `manifest_digest == digest(canonical(entries))`를 만족한다(U1이 계산으로 강제).

---

## 4. ManifestDiffer 규칙 (정확 diff — NFR-12)

**R-DIFF-01 (변경 종류 = added/modified/deleted만, D4/MVP-trim)**: `diff(last_committed, current)`는 세 종류만 산출한다:
- `added`: `current`에만 존재하는 경로의 `ManifestEntry`.
- `modified`: 양쪽에 존재하나 `(raw_sha256, size)` 중 하나라도 다른 경로의 **`current` 값** `ManifestEntry`.
- `deleted`: `last_committed`에만 존재하고 `current`에 없는 경로의 `RelativePath`.

**rename/content-move 감지는 하지 않는다** — 이름 변경은 "이전 경로 deleted + 새 경로 added"로 나타난다(MVP).

**R-DIFF-02 (modified 판정 기준, D5)**: 같은 경로에서 `modified` 여부는 `raw_sha256` 또는 `size`의 차이로 판정한다. mtime은 사용하지 않는다(U0 `ManifestEntry` 결정 승계 — 콘텐츠 아이덴티티는 `raw_sha256`이 결정).

**R-DIFF-03 (정확성 — 누락 0·오탐 0, NFR-12)**: 세 목록의 합집합은 `last_committed`/`current` 경로 집합의 정확한 대칭차/교차-변경을 반영한다. `diff`가 지목하지 않은 경로는 양쪽에서 `(raw_sha256, size)`가 동일하고, 지목한 경로만 실제로 달라진다. 세 목록은 서로소(disjoint) — 같은 경로가 둘 이상의 목록에 나오지 않는다.

**R-DIFF-04 (no-op 동치성, D7)**: `current.manifest_digest == last_committed.manifest_digest` 이면 `diff`는 **빈 `ChangeSet`**(`is_empty() == true`)을 반환한다(전체 엔트리 비교 없이 O(1) 조기 종료). 역으로 diff가 비면 두 다이제스트가 같다(U0 R-NOOP-02 승계). 동일 매니페스트 diff `diff(m, m)`는 항상 empty.

**R-DIFF-05 (순수·무결·결정적)**: `diff`는 I/O·상태·부수효과가 없다. 출력 3목록은 각각 `relative_path` 오름차순 정렬로 결정적 표면화한다. 두 입력 매니페스트가 canonical 정렬(U0 불변식)임을 전제로 투 포인터 merge-join으로 계산한다(`business-logic-model.md` §4).

**R-DIFF-06 (마지막 커밋 소유 경계)**: `last_committed`는 U1이 지속하지 않는다 — **U4 `SyncStateStore`가 소유·전달**하는 인자다. 최초 사이클(마지막 커밋 없음)의 취급(예: 빈 매니페스트 대비 diff = 전부 added)은 호출자(U8)가 결정하며, U1 `diff`는 넘겨진 두 매니페스트만 비교한다.

---

## 5. SafetyLimitsValidator 규칙 (경계-정확·단조 — NFR-14)

**R-LIMIT-01 (3한도, 값은 U0 상수, D13)**: `validate(manifest)`는 세 한도를 검사한다 — 총 볼트 크기 <= 20 GiB(`MAX_VAULT_TOTAL_BYTES`), 파일당 <= 2 GiB(`MAX_FILE_BYTES`), 파일 수 <= 100k(`MAX_FILE_COUNT`). 값은 U0 컴파일타임 상수 참조(재정의 없음). ≤10-sources는 서버 권위(DEP-04)라 미포함.

**R-LIMIT-02 (경계-정확, D14 — off-by-one 없음)**: `actual == limit`은 **수용(WithinLimits)**, `actual > limit`만 **거부(Exceeded)**. 즉 정확히 20 GiB / 2 GiB / 100k는 통과하고, 그 값을 1바이트/1파일이라도 초과하면 거부한다.

**R-LIMIT-03 (단조성, US-E7-07)**: 이미 `Exceeded`인 매니페스트에 파일 또는 바이트를 추가하면 판정은 **절대 `WithinLimits`로 뒤집히지 않는다**. accept/reject boolean은 총량·파일수·최대파일크기에 대해 단조 증가하는 판정이다(단일 위반 리포트 선택 D15와 무관하게 성립).

**R-LIMIT-04 (첫 위반 short-circuit, D15/MVP-trim)**: 초과 시 `Exceeded`는 **첫 발견 위반 하나**만 담는다(모든 위반 누적 리스트 아님). 검사 순서(R-LIMIT-05)에 의해 결정적으로 정해진 위반을 보고한다.

**R-LIMIT-05 (검사 순서·오버플로, D16)**: 단일 패스로 (1) `FileCount`(O(1)) 먼저, (2) 엔트리 순회하며 각 엔트리 `FileBytes` 검사 + `total`을 `saturating_add`로 누적, 누적 중 `total > max_total_bytes`가 되는 즉시 `TotalBytes` 위반. `u64` 오버플로는 saturating으로 방지(panic-free-total). 위반이 없으면 `WithinLimits`.

**R-LIMIT-06 (순수·무결)**: `validate`는 I/O·상태·부수효과 없음. 동일 매니페스트는 동일 판정. OverLimit 상태 표면화(StatusService push)·halt는 U3/U8 소관이며 U1은 판정 값만 반환.

---

## 6. Testable Properties (PBT-01, Full) — U1 NFR별

> **확장 강제(PBT ON Full)**: U1의 각 NFR을 코드-gen이 proptest로 구현 가능하도록 속성을 명세한다. 각 속성에 카테고리 라벨 {Round-trip, Invariant, Idempotence, Commutativity, Oracle, Induction, Easy verification}과 도메인 제너레이터(PBT-07) 요구를 기재한다. 프레임워크 = **proptest**(D19/US-E7-10). U0 `proptest_support` 제너레이터(`arb_manifest_entry`/`arb_relative_path`/`arb_sha256_digest`/`arb_byte_count`/`arb_manifest`)를 재사용한다.

### 6.1 PROP-U1-01 — 해싱 결정성 + sha256sum 일치 (NFR-08, US-E7-01)

- **카테고리**: **Idempotence**(반복 결정성) + **Oracle**(표준 sha256 대조).
- **속성(결정성)**: 임의 바이트열 `b`(빈 파일 포함)에 대해 `hash_stream(b)`를 반복 계산하면 항상 동일 `Sha256Digest`. 스트림 청크 분할 경계와 무관(`hash_stream(concat(chunks)) == hash_stream(whole)`).
- **속성(Oracle)**: 임의 `b`에 대해 `hash_stream(b) == 참조 sha256(b)`(표준 sha256sum 결과와 바이트 일치).
- **제너레이터(PBT-07)**: 임의 `Vec<u8>`(빈·1바이트·경계·대용량 축소본), 임의 청크 분할 시퀀스.

### 6.2 PROP-U1-02 — manifest_digest 순서 무관 결정성 (NFR-08, US-E7-01)

- **카테고리**: **Invariant** + **Oracle**.
- **속성(Invariant)**: 임의 엔트리 집합 `E`와 그 순열 `perm(E)`에 대해 `manifest_digest(perm(E)) == manifest_digest(E)`(canonical 정렬 후 계산).
- **속성(injective, 보조 Invariant)**: 서로 다른 논리 엔트리 집합은 서로 다른 프레이밍 바이트열을 만든다(길이-프리픽스로 `["ab","c"]` vs `["a","bc"]` 충돌 없음) -> 다이제스트 충돌은 SHA-256 충돌 확률로만 발생.
- **속성(Oracle)**: 참조 오라클 = canonical 정렬 후 §3 프레이밍 재계산과 일치.
- **제너레이터(PBT-07)**: 경로 유일성 보장 엔트리 집합 + 순열, 경계(빈 집합·단일·100k 근접 축소본), 경로 프리픽스가 겹치는 케이스(프레이밍 injective 검증용).

### 6.3 PROP-U1-03 — diff 정확성 오라클 (NFR-12, US-E7-05)

- **카테고리**: **Oracle** + **Easy verification** + **Invariant**.
- **속성(Oracle/재구성)**: 임의 두 매니페스트 `prev`, `current`에 대해 `apply(prev, diff(prev, current)) == current`(canonical 형상). 즉 `prev`에서 `deleted` 제거 + `added`/`modified` 반영 -> `current` 정확 재구성(누락 0·오탐 0).
- **속성(Invariant)**: `added`/`modified`/`deleted` 경로는 서로소. 지목되지 않은 경로는 양쪽 `(raw_sha256, size)` 동일.
- **속성(no-op 동치, Round-trip 특수)**: `diff(m, m).is_empty() == true` 그리고 `current.manifest_digest == prev.manifest_digest <=> diff.is_empty()`.
- **제너레이터(PBT-07)**: 상관된 매니페스트 쌍 — 기저 매니페스트 + 파생(무작위 add/modify/delete 적용), 경계(빈 볼트·전량 삭제·전량 신규·동일 매니페스트).

### 6.4 PROP-U1-04 — SafetyLimits 경계-정확 + 단조 (NFR-14, US-E7-07)

- **카테고리**: **Invariant**(경계) + **Induction/Monotonicity**.
- **속성(경계-정확)**: 임의 매니페스트에 대해 판정은 세 한도 경계에서 정확히 accept/reject를 가른다 — `actual == limit`은 accept, `actual == limit + 1`은 reject(off-by-one 없음).
- **속성(단조)**: 이미 reject된 매니페스트에 임의 파일/바이트를 추가한 모든 결과는 여전히 reject(accept로 뒤집히지 않음).
- **속성(위반 보고)**: 세 한도 중 하나라도 초과하면 `Exceeded`이며 그 위반은 초과한 한도의 종류·값을 보고(FR-05 actionable).
- **제너레이터(PBT-07)**: 총량/파일수/파일크기를 경계 근처(limit-1, limit, limit+1)로 조준하는 매니페스트 제너레이터, reject 상태에 증분 추가하는 시퀀스.

### 6.5 PROP-U1-05 — U1 산출 Manifest의 무손실 round-trip (NFR-13)

- **카테고리**: **Round-trip**(코덱은 U0 소유, U1 산출값으로 재확인).
- **속성**: `ManifestBuilder`가 산출한(= `manifest_digest`가 실제 계산된) 임의 `Manifest` `m`에 대해 U0 코덱으로 `decode(encode(m)) == m`. 마찬가지로 `ManifestDiffer`가 산출한 `ChangeSet`도 round-trip 무손실.
- **소유 경계**: 코덱 자체는 U0 소유(PROP-DE-01/PROP-BL-01). U1은 **자기 컴포넌트가 산출한 값**(계산된 다이제스트·정렬된 엔트리·빈/다수 목록)이 round-trip을 통과함을 자기 제너레이터로 재확인한다.
- **제너레이터(PBT-07)**: `ManifestBuilder`류 산출을 모사하는 "digest-consistent" 매니페스트 제너레이터(엔트리 집합에서 실제 `manifest_digest` 계산), 빈/단일/다수 `ChangeSet`.

### 6.6 속성 없음(No PBT properties identified) 판정

| 요소 | 판정 | 근거 |
|---|---|---|
| `VaultScanner::scan`/`open_reader` | **No PBT properties(값 속성)** — 예제/통합 테스트 대상 | 파일시스템 I/O 부수효과 — 결정성은 R-SCAN-06(정렬)이나 실제 검증은 임시 디렉터리 픽스처 기반 예제 테스트가 적합. 심링크 SKIP·hidden 포함은 시나리오 테스트 |
| `ScanError`/`BuildError` taxonomy | round-trip 대상 아님(진단 오류, 지속 안 함) | U1 내부 반환값 — 지속되지 않으므로 NFR-13 대상 아님 |

> **제너레이터(PBT-07) 총괄**: 위 속성은 도메인 타입 제너레이터를 요구한다(임의 `Vec<u8>`·경로 유일 엔트리 집합·상관 매니페스트 쌍·경계 조준 SafetyLimits 입력). shrinking·고정 시드·CI 통합(PBT-08)은 Code Generation/Build-and-Test 이월이며, 여기서는 요구 사실과 대상 타입을 명시한다.

---

## 7. 확장 컴플라이언스 요약 (완료 게이트)

| 확장 | 활성 | 이 문서 적용 판정 | 근거 |
|---|---|---|---|
| **Property-Based Testing** | ON(Full) | **준수** | §6 PROP-U1-01~05로 NFR-08/12/13/14 전부 속성화, 카테고리 라벨·제너레이터 요구·proptest(D19) 기재. VaultScanner I/O는 §6.6에 예제 테스트로 판정 |
| **Resiliency Baseline** | ON | **부분 적용 + N/A** | R-BUILD-01 fail-fast(파괴적 부분 diff 방지)·R-SCAN-05 루트 오류 반환(파괴적 빈 커밋 방지)·R-CA-03 스트리밍(OOM 회피)이 회복력 기여. RTO/RPO 수치·배포·HA/DR은 U1 순수 로직에 N/A |
| **Security Baseline** | OFF | **N/A** | 미로딩·미강제. U1은 토큰/시크릿 미취급. RISK-01은 문서화된 수용 위험 |
