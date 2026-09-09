# U1 Deterministic Content Core — NFR Requirements (비기능 요구사항 / 품질 속성)

**단계**: CONSTRUCTION -> Per-Unit Loop -> **U1 `content-core`** -> NFR Requirements -> 산출물 1/2 (`nfr-requirements.md`)
**작성일**: 2026-09-08
**크레이트**: `content-core` (lib) · **소속 컴포넌트**: `ContentAddressing`, `VaultScanner`, `ManifestBuilder`, `ManifestDiffer`, `SafetyLimitsValidator`
**입력 아티팩트**: `functional-design/domain-entities.md`·`business-rules.md`·`business-logic-model.md`(U1 FD 3/3, AUTOPILOT 작성 2026-09-08) · `plans/u1-content-core-functional-design-plan.md`(§3 결정 표 D1~D19 + §4 확장 컴플라이언스 + §5 드롭리스트) · `inception/requirements/requirements.md`(§5.1 NFR-02 / §5.5 NFR-08·12·13·14 / §12 FQ 오버레이) · U0 산출물 `u0-foundation/nfr-requirements/{nfr-requirements.md,tech-stack-decisions.md}`(스타일 템플릿 + 소비 계약) · 활성 확장 `property-based-testing.md`·`resiliency-baseline.md`
**전제(AUTOPILOT 확정, 재오픈 금지)**: FQ-1=A(표준 SHA-256, 권위 `vault_content_id`는 서버) · FQ-2=A(최신-상태-대체, 재스냅샷 diff) · Q2=A(SafetyLimits = U0 컴파일타임 상수) · D1~D19(FD 계획 §3) · U0 `foundation` **빌드·테스트·clippy 완료(FROZEN)** — `CoreTypes` 값 타입·CBOR 코덱·`RelativePath::normalize`·SafetyLimits 상수·`proptest-support` 제너레이터를 소비하되 **재정의 금지**.

> **문서 성격**: 이 문서는 U1이 소유하는 타입(`domain-entities.md`)·규칙(`business-rules.md`)·흐름(`business-logic-model.md`) **위에 얹는 NFR(품질 속성) 계층**이다. FD의 비즈니스 규칙을 재기술하지 않고 **규칙 ID(R-*)·속성 ID(PROP-U1-*)로 참조**하며, 각 NFR에 근거(아티팩트 + 섹션 + NFR/FR/규칙/PBT ID)와 수용 기준을 붙인다. 구체 크레이트·툴체인 선택의 정본은 자매 산출물 `tech-stack-decisions.md`이며, 이 문서는 각 품질 속성을 **실현하는 메커니즘**으로 그 결정을 참조한다.
>
> **표기 규약**: 기술중립을 지향하되(품질 속성 중심), tech-stack-decisions.md가 위임한 곳에서만 구체 크레이트를 인용한다. 다이어그램은 유니코드 박스 없이 **화살표 표기(`A -> B`)** 와 표/목록으로만 기술한다. English 식별자(크레이트명·타입·rule ID·NFR ID)는 원문 유지, Rust 제네릭/타입(예: `Box<dyn Read>`, `Result<Manifest, BuildError>`)은 백틱으로 감싼다.

---

## 1. NFR 개요 및 U1 특성

U1 `content-core`는 볼트 콘텐츠를 **결정적으로 지문화**하고 매니페스트를 재계산·비교하며 전송 전 안전 한도를 검사하는 **순수 코어 lib**다. `VaultScanner`의 파일 읽기(U1 유일 I/O)를 제외하면 지속·네트워크·시간 부수효과가 없다. 자체 런타임·스레드 풀·처리량 축이 없어 확장성/가용성/성능 수치 목표의 상당수가 **N/A**(§7)다. 반대로 U1이 실현하는 **결정성 계약(해싱·다이제스트·diff·경계 판정) + 무손실성 + 스트리밍 메모리 바운드**는 동기화 사이클의 정확성 기반이므로 이 문서의 핵심이다.

**본 문서가 확정하는 U1 NFR 카탈로그(카테고리별 ID)**:

| 카테고리 | NFR ID | 요약 |
|---|---|---|
| 신뢰성 | U1-NFR-REL-01 | 콘텐츠 해싱 결정성 + `sha256sum` 일치(스트리밍) |
| 신뢰성 | U1-NFR-REL-02 | `manifest_digest` 순서 무관 결정성 + injective 프레이밍 |
| 신뢰성 | U1-NFR-REL-03 | 정확 diff(누락 0·오탐 0·서로소 + no-op 동치) |
| 신뢰성 | U1-NFR-REL-04 | SafetyLimits 경계-정확 + 단조 + actionable 위반 |
| 신뢰성 | U1-NFR-REL-05 | U1 산출값(Manifest/ChangeSet) 무손실 round-trip(U0 코덱) |
| 신뢰성 | U1-NFR-REL-06 | fail-fast build + 루트 오류 반환(파괴적 부분/빈 커밋 방지) |
| 신뢰성 | U1-NFR-REL-07 | 순수 표면 전체 panic-free total |
| 성능/리소스 | U1-NFR-PERF-01 | 스트리밍 메모리 바운드(전량 적재 없음, 정성 계약; 수치 게이트 없음) |
| 유지보수성 | U1-NFR-MNT-01 | 오류 타입 파생 전략(thiserror 운영 오류 + 순수 판정 값 타입) |
| 유지보수성 | U1-NFR-MNT-02 | PBT 제너레이터 재사용 + U1 전용 제너레이터 노출(`proptest-support`) |
| 유지보수성 | U1-NFR-MNT-03 | 커버리지/문서 정책(no %-게이트 + PBT + 예제 앵커 + `deny(missing_docs)`) |

---

## 2. 신뢰성 (Reliability)

U1의 신뢰성은 "**상위 단위가 신뢰할 수 있는 결정적·무손실 순수 계약**"을 제공하는 것이다 — 재현 가능한 해시, 순서 무관 다이제스트, 정확한 diff, 경계에서 흔들리지 않는 한도 판정, 어떤 입력에도 죽지 않는 순수 표면.

### 2.1 U1-NFR-REL-01 — 콘텐츠 해싱 결정성 + `sha256sum` 일치 (NFR-08)

- **요구**: `ContentAddressing::hash_stream<R: Read>(reader)`는 임의 바이트 스트림에 대해 **표준 SHA-256**을 계산하며, 그 결과 `Sha256Digest`는 (a) 반복 계산·환경·플랫폼과 무관하게 항상 동일하고, (b) `sha256sum`(= okc-core `raw_sha256` 값) 과 **바이트 단위로 일치**하며, (c) 스트림 **청크 분할 경계와 무관**하다(`hash_stream(concat(chunks)) == hash_stream(whole)`). 빈 스트림(0바이트 파일)은 표준 빈-입력 SHA-256을 낸다.
- **근거**: `business-rules.md` §1 R-CA-01(표준 SHA-256)·R-CA-02(결정성)·R-CA-03(스트리밍 메모리 바운드) · `business-logic-model.md` §2.1 · requirements.md §5.5 NFR-08 + §12.1 FQ-1=A 정정(okc 재현 삭제, `sha256sum` 일치) · US-E7-01. 실현 크레이트 = `sha2`(RustCrypto, 정본 tech-stack-decisions.md) — 순수 Rust·`Update`/`Finalize` 증분 API가 스트리밍 결정성에 직접 맞는다.
- **수용 기준**:
  - (AC-1) PROP-U1-01(카테고리 Idempotence): 임의 `Vec<u8>`(빈·1바이트·경계·대용량 축소본)에 대해 반복 `hash_stream`이 동일 `Sha256Digest`를 내고, 임의 청크 분할 시퀀스에 대해 `hash_stream(chunks) == hash_stream(whole)`가 반례 없이 통과한다.
  - (AC-2) PROP-U1-01(카테고리 Oracle): 임의 `b`에 대해 `hash_stream(b) == 참조 sha256(b)`(표준 sha256 오라클과 바이트 일치).
- **연계**: 이 해시는 U3 `UploadProtocolDriver`가 전송 직전 blob 재검증(FR-22/Q8=A)에서 `hash_stream`을 **재사용**하는 값이므로, 결정성은 TOCTOU 폐쇄의 기반이기도 하다.

### 2.2 U1-NFR-REL-02 — `manifest_digest` 순서 무관 결정성 + injective 프레이밍 (NFR-08)

- **요구**: `ContentAddressing::manifest_digest(entries)`는 (a) 논리적으로 동일한 엔트리 집합(같은 `{relative_path, raw_sha256, size}` 다중집합)에 대해 **발견/삽입 순서와 무관하게** 동일한 `ManifestDigest`를 내고(canonical 정렬 후 계산), (b) **길이-프리픽스 프레이밍**으로 injective(단사) 인코딩을 강제해 서로 다른 논리 집합이 같은 바이트열을 만들지 않는다(예: `["ab","c"]` vs `["a","bc"]` 충돌 없음).
- **근거**: `business-rules.md` §1 R-CA-04(프레이밍 인코딩)·R-CA-05(순서 무관 결정성) · `domain-entities.md` §3(다이제스트 도메인 인코딩 규약, D1) · `business-logic-model.md` §2.2 · requirements.md §5.5 NFR-08 · US-E7-01. **CBOR 지속 코덱과 분리**된 프레이밍(D1)이라 지속 스키마 진화(NFR-13 신규 필드)와 무관하게 no-op 판정(FR-10)이 버전 간 안정하다. 실현 = `sha2` + std 바이트 프레이밍(`u64::to_be_bytes` 등, 신규 의존 없음).
- **수용 기준**:
  - (AC-1) PROP-U1-02(Invariant): 임의 엔트리 집합 `E`와 그 순열 `perm(E)`에 대해 `manifest_digest(perm(E)) == manifest_digest(E)`.
  - (AC-2) PROP-U1-02(Oracle/injective 보조): 경로 프리픽스가 겹치는 케이스를 포함하는 제너레이터로 실행해 서로 다른 논리 집합이 서로 다른 프레이밍 바이트열을 산출하고, 참조 오라클(canonical 정렬 후 §3 프레이밍 재계산)과 일치한다.

### 2.3 U1-NFR-REL-03 — 정확 diff (누락 0·오탐 0·서로소 + no-op 동치) (NFR-12)

- **요구**: `ManifestDiffer::diff(last_committed, current)`는 (a) `added`/`modified`/`deleted` 세 목록의 합집합이 두 매니페스트 경로 집합의 정확한 대칭차/교차-변경을 반영하고(누락 0·오탐 0), (b) 세 목록이 서로소(disjoint)이며, (c) 지목되지 않은 경로는 양쪽 `(raw_sha256, size)`가 동일하고, (d) `current.manifest_digest == last_committed.manifest_digest`이면 O(1)로 빈 `ChangeSet`을 반환한다(no-op 동치, FR-10).
- **근거**: `business-rules.md` §4 R-DIFF-01(범위 = a/m/d)·R-DIFF-02(modified 판정 = `(raw_sha256, size)`)·R-DIFF-03(정확성)·R-DIFF-04(no-op 동치)·R-DIFF-05(순수·결정적 merge-join) · `business-logic-model.md` §5 · requirements.md §5.5 NFR-12 · §12.2 FQ-2=A(최신-상태-대체, 재스냅샷 diff) · US-E7-05. **경계(D4/MVP-trim)**: rename 감지는 하지 않으며 이름 변경은 deleted+added로 표현된다.
- **수용 기준**:
  - (AC-1) PROP-U1-03(Oracle/재구성): 임의 상관 매니페스트 쌍 `prev`/`current`에 대해 `apply(prev, diff(prev, current)) == current`(canonical 형상)가 반례 없이 통과한다.
  - (AC-2) PROP-U1-03(Invariant): `added`/`modified`/`deleted` 경로는 서로소이고, 지목되지 않은 경로는 양쪽 `(raw_sha256, size)`가 동일하다.
  - (AC-3) no-op 동치: `diff(m, m).is_empty() == true`이고 `current.manifest_digest == prev.manifest_digest <=> diff.is_empty()`.
- **경계**: `last_committed`는 U1이 지속하지 않는다 — **U4 `SyncStateStore`가 소유·인자 전달**(R-DIFF-06). 최초 사이클 취급은 호출자(U8)가 결정한다.

### 2.4 U1-NFR-REL-04 — SafetyLimits 경계-정확 + 단조 + actionable 위반 (NFR-14)

- **요구**: `SafetyLimitsValidator::validate(manifest)`는 (a) 세 한도(총 <=20 GiB / 파일당 <=2 GiB / 파일 수 <=100k, **U0 상수** `MAX_VAULT_TOTAL_BYTES`/`MAX_FILE_BYTES`/`MAX_FILE_COUNT`)를 **경계-정확**하게 검사하고(`actual <= limit` accept, `actual > limit`만 reject; off-by-one 없음), (b) **단조**하며(이미 `Exceeded`인 매니페스트에 파일/바이트를 추가해도 `WithinLimits`로 뒤집히지 않음), (c) 초과 시 `Exceeded(LimitViolation)`에 **어떤 한도를 얼마나** 넘었는지(한도 종류·`limit`·`actual`, `FileBytes`는 `path` 포함) 담아 **actionable**(FR-05)하다.
- **근거**: `business-rules.md` §5 R-LIMIT-01(3한도, 값=U0 상수)·R-LIMIT-02(경계-정확)·R-LIMIT-03(단조)·R-LIMIT-04(첫 위반 short-circuit, MVP-trim)·R-LIMIT-05(검사 순서 + `saturating_add`)·R-LIMIT-06(순수) · `business-logic-model.md` §6 · requirements.md §5.5 NFR-14 + §3.2 FR-05(로컬 reject + actionable 리포트) · US-E7-07.
- **수용 기준**:
  - (AC-1) PROP-U1-04(Invariant/경계): 세 한도 경계를 조준하는 제너레이터(limit-1 / limit / limit+1)로 `actual == limit`은 accept, `actual == limit + 1`은 reject가 반례 없이 통과한다.
  - (AC-2) PROP-U1-04(Monotonicity): reject 상태에 임의 파일/바이트를 증분 추가한 모든 결과가 여전히 reject다.
  - (AC-3) 초과 시 `Exceeded`는 초과한 한도의 종류·`limit`·`actual`(해당 시 `path`)을 보고한다(FR-05 actionable). OverLimit 표면화·halt는 U3/U8 소관이며 U1은 판정 값만 반환한다.
- **MVP-trim(D15)**: `Exceeded`는 **첫 발견 위반 하나**만 담는다(모든 위반 누적 리스트 아님). 검사 순서(count -> per-file -> total)로 결정적으로 정해진 위반을 보고한다. accept/reject boolean 단조성은 이와 무관하게 성립한다.

### 2.5 U1-NFR-REL-05 — U1 산출값 무손실 round-trip (NFR-13, U0 코덱 경유)

- **요구**: `ManifestBuilder`가 산출한(= `manifest_digest`가 실제 계산된) 임의 `Manifest` `m`과 `ManifestDiffer`가 산출한 임의 `ChangeSet`은 **U0 CBOR 코덱**으로 `decode(encode(v)) == v`(무손실 round-trip)를 만족한다. U1은 코덱을 소유하지 않으며, **자기 컴포넌트가 산출한 값**(계산된 다이제스트·정렬된 엔트리·빈/단일/다수 목록)이 U0 계약을 통과함을 재확인한다.
- **근거**: `business-rules.md` §6.5 PROP-U1-05 · `domain-entities.md` §1(U0 타입 소비)·`business-logic-model.md` §8 · requirements.md §5.5 NFR-13 + §12.2 FQ-2=A(대상 = manifest + sync-state + history; queue entries 제거) · U0-NFR-REL-01(R-CODEC-01, 코덱 무손실 계약 = U0 소유·FROZEN). 코덱 백엔드 = `ciborium`(U0 소유); U1은 신규 의존을 추가하지 않고 U0 `encode`/`decode`를 호출한다.
- **수용 기준**:
  - (AC-1) PROP-U1-05(Round-trip): "digest-consistent" 제너레이터(엔트리 집합에서 실제 `manifest_digest` 계산)로 만든 `Manifest`가 U0 코덱 round-trip을 반례 없이 통과한다.
  - (AC-2) 빈/단일/다수 `ChangeSet`이 U0 코덱 round-trip을 통과한다.
- **소유 경계**: 코덱 자체의 무손실성 증명은 U0(PROP-DE-01/PROP-BL-01) 소유다. U1은 자기 산출값 형상에 대한 재확인만 담당한다. `ScanError`/`BuildError`/`LimitVerdict`는 지속되지 않으므로 NFR-13 대상이 아니다(FD §6.6).

### 2.6 U1-NFR-REL-06 — fail-fast build + 루트 오류 반환 (파괴적 부분/빈 커밋 방지)

- **요구(회복력 기여의 신뢰성 계약화)**: (a) `ManifestBuilder::build`는 scan 또는 파일별 `hash_stream`에서 **어느 파일이라도 실패하면 전체를 중단**하고 `BuildError`를 반환한다 — **부분 매니페스트(일부 파일 스킵)를 반환하지 않는다**(스킵 파일이 diff에서 "삭제"로 오인되어 파괴적 커밋을 유발하는 것을 방지). (b) `VaultScanner::scan`은 루트 부재/언마운트/접근 불가를 `ScanError::RootUnavailable`로 **오류 반환**하며 **0-파일 매니페스트를 만들지 않는다**(파괴적 빈 커밋 방지).
- **근거**: `business-rules.md` §3 R-BUILD-01(fail-fast, D12)·§2 R-SCAN-05(루트 가용성 -> 값 반환) · `business-logic-model.md` §4·§3.1 · requirements.md §5.2 NFR-03(zero-loss 의도)·§6 RESILIENCY-02 · US-E1-06(vault-unavailable) · §12.2 FQ-2=A(다음 사이클 재스냅샷이 미반영 편집 흡수).
- **수용 기준**:
  - (AC-1) 임의 파일 해시 I/O 실패에 대해 `build`는 `Ok`가 아니라 `Err(BuildError)`를 반환하며 부분 성공 산출물이 없다(예제/오류-주입 테스트).
  - (AC-2) 루트 도달 불가 시 `scan`은 `Err(ScanError::RootUnavailable)`을 반환하고 0-엔트리 `VaultSnapshot`을 산출하지 않는다.
- **경계**: vault-unavailable **해석·표면화는 U2 `VaultAvailabilityGuard`/U8** 소관이며 U1은 값만 반환한다(push 안 함). RTO/RPO 수치·재시도 정책은 U8/U3 소관.

### 2.7 U1-NFR-REL-07 — 순수 표면 전체 panic-free total

- **요구**: U1 순수 표면(`hash_stream`·`manifest_digest`·`diff`·`validate`)은 **모든 입력에 대해 패닉 없이 종결**한다. 특히 `validate`의 총량 누적은 `u64` 오버플로를 `saturating_add`로 방지해 어떤 매니페스트에도 산술 패닉을 내지 않고(panic-free-total, R-LIMIT-05), `diff`/`manifest_digest`는 임의 엔트리 집합에 대해 `unwrap`/슬라이스 인덱싱/정수 변환 패닉 없이 값을 낸다. `hash_stream`의 파일 I/O 실패는 패닉이 아니라 `Result`(상위에서 `BuildError::Hash`)로 표면화된다.
- **근거**: `business-rules.md` §5 R-LIMIT-05(saturating 오버플로 방지)·R-LIMIT-06(순수) · `business-logic-model.md` §6 · U0-NFR-REL-02(panic-free total 계약, 워크스페이스 규율) 정합. 이 계약은 100k/20 GiB 상한 근접 입력에서도 성립한다.
- **수용 기준**:
  - (AC-1) `validate`에 대한 PBT no-panic: 경계·대값 `size`를 포함하는 매니페스트 제너레이터로 실행해 어떤 입력도 패닉 없이 `LimitVerdict`로 종결한다(총량 누적이 `u64` 상한을 넘어도 `saturating_add`로 무패닉).
  - (AC-2) `diff`/`manifest_digest`는 임의 엔트리 집합(빈·단일·대량 축소본·경로 프리픽스 중첩)에 대해 패닉 없이 값을 낸다(PROP-U1-02/03 제너레이터 재사용).
- **비고**: PBT 케이스 수·shrinking·고정 시드·CI 통합(PBT-08)은 Code Generation / Build-and-Test 이월.

---

## 3. 성능 / 리소스 (Performance)

### 3.1 U1-NFR-PERF-01 — 스트리밍 메모리 바운드 (정성 계약; 수치 목표 N/A)

- **요구(정성 계약만)**: (a) `hash_stream`은 **고정 버퍼로 증분 계산**하며 파일 전체를 메모리에 적재하지 않는다 — SafetyLimits 상한(파일당 2 GiB, 총 20 GiB, 100k 파일)에서도 파일 크기에 비례하는 무한정 메모리를 쓰지 않는다. (b) `ManifestBuilder::build`는 파일을 **순차 스트리밍**하여 동시에 다수 파일 바이트를 적재하지 않는다(매니페스트 메타 자체는 파일 수 상한에 비례하는 유한 크기). (c) `diff`는 O(n+m) merge-join, `validate`는 O(n) 단일 패스다.
- **명시적 결정 — 수치 목표 없음(근거)**: U1에 **throughput·latency·peak-memory 수치 게이트를 두지 않는다**(U0의 정성-성능 스탠스 미러). 근거:
  - (a) 처리 비용은 볼트 파일 수·바이트에 선형·유계라 절대 수치 목표를 정의할 도메인 근거가 없다(입력 크기에 종속).
  - (b) requirements NFR-02는 "무한정 메모리 없이 스트리밍/증분 해시"라는 **정성 바운드**를 요구할 뿐 수치 임계를 정의하지 않는다.
  - (c) 데몬은 단일 사용자·로컬 프로세스라 처리량 SLA가 없다(§6 RESILIENCY-02).
- **근거**: `business-rules.md` §1 R-CA-03·§3 R-BUILD-02 · `business-logic-model.md` §2.1·§4·§5 · requirements.md §5.1 NFR-02 · FD 계획 §3 D3(고정 버퍼 스트리밍, 구체 버퍼 크기 = NFR Design 이월).
- **수용 기준**:
  - (AC-1) `hash_stream`이 대용량 축소본 스트림을 전량 적재 없이 완결하고(고정 버퍼), `build`가 100k 근접 파일 수에서 동시 다수 적재 없이 완결한다(스트리밍 특성은 예제/PBT 대량 경계로 간접 검증).
  - (AC-2) 어떤 절대 처리량/지연/피크메모리 임계도 U1 게이트로 설정하지 않는다(수치 게이트 부재를 명시적으로 문서화). 구체 스트리밍 버퍼 크기는 NFR Design에서 확정한다.

---

## 4. 유지보수성 (Maintainability)

### 4.1 U1-NFR-MNT-01 — 오류 타입 파생 전략 (U0 관례 승계)

- **요구**: U1 오류/판정 타입을 두 부류로 구분해 파생한다:
  - **운영 오류(`Result` 반환)**: `ScanError`·`BuildError`는 **`thiserror`** 파생으로 `Display`/`std::error::Error`를 얻고, `BuildError::Scan(ScanError)`·`Hash{path, source: io::Error}`처럼 원인(`source`)을 보존한다(오류 국소성).
  - **판정 값 타입(throw 아님)**: `LimitVerdict`·`LimitViolation`은 **순수 enum**으로 유지한다 — 이들은 `validate`가 반환하는 **값**이며 상위(U3/U8)가 소비해 표면화한다(지속·직렬화 대상 아님, FD §6.6).
- **선정 근거**: U0-NFR-MNT-01의 워크스페이스 전역 관례(운영 오류 = `thiserror`, 분류/판정 = 순수 값) 승계 — 드리프트 없음. `anyhow`식 타입소거는 `LimitViolation`의 구조화 변이(`TotalBytes`/`FileBytes`/`FileCount`)를 소실시켜 actionable 리포트(FR-05)를 불가능하게 하므로 거부한다.
- **근거**: `domain-entities.md` §2.4(`LimitVerdict`/`LimitViolation`)·§2.5(`ScanError`)·§2.6(`BuildError`) · requirements.md §5.7 NFR-17 · U0-NFR-MNT-01. 실현 크레이트(`thiserror`, 워크스페이스 상속) 정본 = tech-stack-decisions.md.
- **수용 기준**:
  - (AC-1) `ScanError`·`BuildError`는 `thiserror` 파생으로 `Display`/`Error`를 제공하고 I/O 원인을 `source`로 보존한다.
  - (AC-2) `LimitVerdict`/`LimitViolation`은 순수 enum이며 코덱 round-trip(NFR-13) 대상에 포함되지 않는다(지속 안 함).

### 4.2 U1-NFR-MNT-02 — PBT 제너레이터 재사용 + U1 전용 제너레이터 노출 (PBT-07)

- **요구**: U1의 PROP-U1-01~05를 구현하는 제너레이터는 (a) U0 `foundation`의 **`proptest-support`(비기본 feature)** 제너레이터(`arb_manifest_entry`/`arb_relative_path`/`arb_sha256_digest`/`arb_byte_count`/`arb_manifest`)를 **재사용**하고, (b) U1 전용 제너레이터(임의 `Vec<u8>`·청크 분할 시퀀스·경로 유일 엔트리 집합 + 순열·상관 매니페스트 쌍·경계 조준 SafetyLimits 입력·digest-consistent 매니페스트)를 **U1 자신의 비기본 `proptest-support` feature**로 노출한다(U0 미러). 하위 소비 단위(예: U3 재검증 테스트)는 dev-dependency에서 그 feature를 켜 재사용할 수 있다.
- **선정 근거**: U0-NFR-MNT-02와 동일한 단일-출처 원칙 — 제너레이터 정의 드리프트 방지 + `proptest`가 비테스트/프로덕션 빌드에 유출되지 않도록 non-default feature 게이트. U1 도메인 특화 제너레이터(바이트 스트림·경계 조준 한도 입력)는 U0에 없으므로 U1이 자기 feature로 추가한다.
- **근거**: `business-rules.md` §6(PROP-U1-* 제너레이터 요구, PBT-07 총괄)·`business-logic-model.md` §8 · property-based-testing.md PBT-07(제너레이터 재사용성) · U0-NFR-MNT-02(`proptest-support` 노출 패턴) · FD 계획 §3 D19(proptest). feature 이름·게이팅 정본 = tech-stack-decisions.md.
- **수용 기준**:
  - (AC-1) U1 제너레이터 모듈은 non-default feature 뒤에 위치하며 기본 빌드에 `proptest` 의존이 나타나지 않는다.
  - (AC-2) 제너레이터는 문서화된 도메인 제약을 존중한다(경로 유일 엔트리 집합, digest-consistent 매니페스트, 경계 조준 SafetyLimits 입력 = limit-1/limit/limit+1).

### 4.3 U1-NFR-MNT-03 — 커버리지 / rustdoc 문서 정책 (U0 정합)

- **요구**:
  - **전역 커버리지 %-게이트를 두지 않는다.** 실질 검증은 PBT(§2 속성들) + 예제 앵커(PBT-10: `VaultScanner` I/O·심링크 SKIP·hidden 포함·fail-fast 경로에 예제/통합 테스트 병행)로 확보한다.
  - **`content-core` 공개 항목에 `#![deny(missing_docs)]`를 강제**한다 — `hash_stream`(U3 재사용)·`validate`(U3 재검사)·`build`/`diff`(U8 사이클) 등 상위 단위가 소비하는 공개 API 문서화를 컴파일타임에 강제한다.
- **선정 근거**: U0-NFR-MNT-03(no %-게이트 + PBT + 예제 앵커) 정합. U1은 순수 코어이며 결정성은 속성으로, I/O 경로는 예제로 검증하는 것이 line/branch % 게이트보다 실질적이다. `deny(missing_docs)`는 U1이 다수 상위 단위에 재사용되는 공개 계약 표면을 가지므로 near-free로 채택한다.
- **근거**: property-based-testing.md PBT-10(PBT는 예제 테스트를 대체하지 않고 보완) · `business-rules.md` §6.6(VaultScanner = 예제/통합 판정) · requirements.md §5.7 NFR-17 · U0-NFR-MNT-03. 상세 CI 커버리지 통합은 Build-and-Test 이월.
- **수용 기준**:
  - (AC-1) business-critical 경로(해싱·다이제스트·diff·경계 판정)는 PBT를, I/O 경로(`VaultScanner`)는 예제/통합 테스트를 가진다(PBT-10). 어떤 핵심 경로도 검증 공백이 없다.
  - (AC-2) `content-core` 공개 항목에 문서 주석이 누락되면 `deny(missing_docs)`로 컴파일 실패한다.

> **포터빌리티(NFR-05) 연계**: 크로스플랫폼 재현 빌드(macOS/Windows/Linux)는 U0가 워크스페이스 DAG 루트로 전파하는 **Edition 2024 + 고정 MSRV**로 보장된다. U1은 `crate.workspace = true`로 이를 상속하며, `VaultScanner`의 파일시스템 walk는 `std::fs`(플랫폼 중립)를 쓰므로 별도 U1 포터빌리티 NFR을 신설하지 않는다(심링크 판별도 `std` `FileType::is_symlink()`로 플랫폼 중립). 구체 툴체인 결정은 tech-stack-decisions.md.

---

## 5. 보안-잔존 (Security residual)

**전제**: Security Baseline 확장 = **OFF**(requirements.md §2.3 Q1=B). U1은 **토큰/시크릿/자격증명을 취급하지 않는다** — 콘텐츠 해싱·매니페스트·한도 판정만 수행하는 순수 코어다. 따라서 U1에는 신설·잔존 보안 통제가 **없다**(§7 N/A). RISK-01(로컬 평문 산출물)과의 관계는 아래 한 줄로만 표면화한다.

- **RISK-01 관계(신규 통제 아님)**: U1이 산출하는 `Manifest`(경로 + `raw_sha256`)는 로컬 평문 산출물 집합의 일부가 되지만, **지속은 U4가 수행**하며 U1은 값만 반환한다(저장하지 않음). SHA-256 자체는 U0-NFR-SEC-01(TLS 강제)·SEC-02(토큰 redaction)와 무관하다. U1은 이 수용 위험에 대해 어떤 완화도 신설하지 않는다(Security 확장 OFF 유지).

---

## 6. 사용성 (Usability)

**판정 = N/A**: U1은 UI/CLI/트레이/config 표면이 없는 순수 lib이므로 사람-대면 사용성 요구가 없다(U0-NFR-USE-01의 config 메시지 표면조차 U1에는 없음). 다만 개발자-대면 API 계약으로서 `LimitViolation`이 **actionable 정보**(한도 종류·`limit`·`actual`·`path`)를 구조화해 반환하는 것은 U1-NFR-REL-04(AC-3)에서 이미 요구화했으며, 실제 사람-대면 표면화(알림 FR-19·상태)는 U8/U6 소관이다.

---

## 7. N/A 카테고리 근거표

아래 카테고리는 U1에 요구를 신설하지 않는다(발명 금지). 각 판정은 확정 세트(FD 계획 §3·§4, requirements RESILIENCY-02/§6)에서 온 것이다.

| 카테고리 | 판정 | 근거 |
|---|---|---|
| **확장성(Scalability)** | N/A | U1은 순수 코어 lib로 자체 런타임·스레드·처리량 축이 없음. 100k/20 GiB는 SafetyLimits **상한**(U0 상수)이며 스케일 축이 아님. 스트리밍 메모리 바운드는 U1-NFR-PERF-01의 정성 계약으로 표면화. |
| **가용성(Availability)** | N/A | lib 크레이트라 SLA 없음. 전체 Watcher는 단일 사용자·재시작 가능 로컬 프로세스라 RTO/availability-SLA가 requirements RESILIENCY-02(§6)에서 N/A 확정. |
| **성능 수치 목표(Performance numeric)** | N/A(정성 계약만) | 처리 비용은 파일 수·바이트에 선형·유계라 throughput/latency/peak-memory 수치 게이트 근거 없음. U1-NFR-PERF-01의 정성 계약(스트리밍·전량 적재 없음)만 문서화. (§3.1) |
| **Resiliency DR / RTO / RPO** | N/A(신규 결정 없음) | DR/RTO/availability는 requirements RESILIENCY-02에서 N/A 확정. RPO=0(zero-loss)은 U4 durable state가 전달하며 U1 fail-fast/루트 오류(U1-NFR-REL-06)·무손실 산출값(REL-05)은 그 **기반**일 뿐. 배포/롤백/HA는 U1(순수 로직)에 N/A. |
| **사용성(사람-대면)** | N/A | U1은 UI/CLI/config 표면 없는 lib. actionable `LimitViolation`은 REL-04로 요구화, 실제 표면화는 U8/U6 소관. (§6) |
| **보안 강제 통제(Security enforced)** | N/A | Security Baseline OFF, U1은 토큰/시크릿 미취급. 암호화 저장·키관리·시크릿 스캐닝 신설 없음. TLS(NFR-06)는 U5/U0 소관, U1과 무관. RISK-01은 문서화된 수용 위험(§5). |

---

## 8. NFR to 요구사항 추적표

각 U1 NFR을 소스 NFR/FR ID(requirements) + 규칙 ID(business-rules) + PBT 속성 ID로 매핑한다.

| U1 NFR ID | 요약 | 소스 NFR/FR ID | 규칙 ID | PBT ID |
|---|---|---|---|---|
| U1-NFR-REL-01 | 해싱 결정성 + `sha256sum` 일치 | NFR-08, FR-02/FR-22, US-E7-01 | R-CA-01/02/03 | PROP-U1-01 (Idempotence + Oracle) |
| U1-NFR-REL-02 | `manifest_digest` 순서 무관 + injective | NFR-08, FR-10, US-E7-01 | R-CA-04/05 | PROP-U1-02 (Invariant + Oracle) |
| U1-NFR-REL-03 | 정확 diff + no-op 동치 | NFR-12, FR-10, US-E7-05 | R-DIFF-01..06 | PROP-U1-03 (Oracle + Invariant) |
| U1-NFR-REL-04 | SafetyLimits 경계-정확·단조·actionable | NFR-14, FR-05, US-E7-07 | R-LIMIT-01..06 | PROP-U1-04 (Invariant + Monotonicity) |
| U1-NFR-REL-05 | 산출값 무손실 round-trip | NFR-13(U0 코덱), US-E7-06(기반) | R-BUILD-04 | PROP-U1-05 (Round-trip) |
| U1-NFR-REL-06 | fail-fast build + 루트 오류 반환 | NFR-03(zero-loss 의도), US-E1-06 | R-BUILD-01, R-SCAN-05 | (예제/오류-주입, 값 속성 아님) |
| U1-NFR-REL-07 | 순수 표면 panic-free total | NFR-14(파생), NFR-03(기반) | R-LIMIT-05/06 | PBT no-panic(PROP-U1-02/04 제너레이터 재사용) |
| U1-NFR-PERF-01 | 스트리밍 메모리 바운드(수치 N/A) | NFR-02 | R-CA-03, R-BUILD-02 | PROP-U1-01/05 대량 경계(간접) |
| U1-NFR-MNT-01 | 오류 타입 파생 전략 | NFR-17 | (오류 taxonomy §2.4~2.6) | N/A(파생 관례) |
| U1-NFR-MNT-02 | PBT 제너레이터 재사용·노출 | NFR-17, NFR-08..14(기반) | (제너레이터 계약) | PBT-07 |
| U1-NFR-MNT-03 | 커버리지/문서 정책 | NFR-17 | (공개 계약) | PBT-10 |
| (포터빌리티 연계) | Edition 2024 + 고정 MSRV(U0 상속) + `std` FS | NFR-05 | (tech-stack 상속) | N/A(빌드 재현성) |

---

## 9. 확장 컴플라이언스 요약 (완료 게이트용)

| 확장 | 활성 | 이 단계 판정 | 근거 |
|---|---|---|---|
| **Property-Based Testing** | ON (Full) | **강제·준수** | PBT-09(프레임워크)는 U0에서 `proptest`로 확정되어 워크스페이스 상속(재선택 없음); U1은 FD 계획 D19로 이를 확정. PBT-01(속성 식별) = FD에서 PROP-U1-01~05로 완료. PBT-07(제너레이터 재사용)은 U1-NFR-MNT-02(U0 `proptest-support` 재사용 + U1 전용 feature 노출)로 실현. PBT no-panic은 U1-NFR-REL-07(§2.7). PBT-10(예제 병행)은 U1-NFR-MNT-03. `VaultScanner` I/O는 값 속성이 없어 예제/통합 테스트로 판정(FD §6.6, blocking 아님). shrinking/시드/CI(PBT-08)는 Code Generation/Build-and-Test 이월. **blocking 없음.** |
| **Resiliency Baseline** | ON | **부분 적용 + 대체로 N/A** | RESILIENCY-01: U1은 결정적 코어(사이클 정확성 기반). (a) U1-NFR-REL-06 fail-fast build + 루트 오류 반환이 파괴적 부분/빈 커밋을 방지, (b) U1-NFR-REL-07 panic-free total이 대용량 입력에서 산술 크래시 회피, (c) U1-NFR-PERF-01 스트리밍이 OOM 회피, (d) U1 산출 `Manifest`(REL-05 무손실)가 U4 zero-loss 지속(RPO=0, NFR-03)의 입력. **RTO/RPO 수치·DR·배포/롤백·HA는 U1(순수 로직)에 N/A**(RESILIENCY-02, requirements §6). 신규 U1 Resiliency 결정 없음. |
| **Security Baseline** | OFF | **N/A** | 미로딩·미강제. U1은 토큰/시크릿 미취급(§5). RISK-01(로컬 평문 산출물)은 문서화된 수용 위험이며 U1은 값만 반환(지속은 U4). 신설 통제 없음. |

**블로킹 판정**: 모든 적용 가능한 활성 확장 규칙이 준수(PBT)되거나 부분 적용 + N/A(Resiliency)/N/A(Security)로 판정됨 — **blocking finding 없음.**

---

## 10. 후속 단계 이월 항목 (참고)

- 스트리밍 해시 **구체 버퍼 크기**(U1-NFR-PERF-01) -> NFR Design.
- PBT 케이스 수·shrinking·고정 시드·CI 통합(PBT-08) -> Code Generation / Build-and-Test.
- 상세 CI 커버리지 통합(U1-NFR-MNT-03) -> Build-and-Test.
- 구체 크레이트/feature 결정(SHA-256 = `sha2`·파일 읽기 = `std::io` 스트리밍·오류 = `thiserror`·PBT = `proptest` + `proptest-support` feature)의 정본 -> 자매 산출물 `tech-stack-decisions.md`.
