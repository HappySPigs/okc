# U1 Deterministic Content Core — NFR Design Patterns (NFR 실현 설계 패턴)

**단계**: CONSTRUCTION -> Per-Unit Loop -> **U1 `content-core`** -> NFR Design -> 산출물 1/2 (`nfr-design-patterns.md`)
**작성일**: 2026-09-08
**크레이트**: `content-core` (lib) · **소속 컴포넌트**: `ContentAddressing`, `VaultScanner`, `ManifestBuilder`, `ManifestDiffer`, `SafetyLimitsValidator`
**입력 아티팩트**: `nfr-requirements/nfr-requirements.md`·`nfr-requirements/tech-stack-decisions.md`(U1 NFR Requirements 산출물, AUTOPILOT 확정 2026-09-08) · `functional-design/`(domain-entities §1~§4 · business-rules R-CA/SCAN/BUILD/DIFF/LIMIT · business-logic-model §1~§8) · U0 산출물 `u0-foundation/nfr-design/{nfr-design-patterns.md,logical-components.md}`(스타일 템플릿 + 소비 계약, FROZEN) · 활성 확장 `property-based-testing.md`(ON, Full)·`resiliency-baseline.md`(ON)
**규칙**: `construction/nfr-design.md` Step 6 · `common/content-validation.md`(ASCII 화살표 `A -> B`, 박스 문자 미사용, Rust 제네릭 백틱) · `common/ascii-diagram-standards.md`

> **문서 성격**: 이 문서는 U1이 확정한 **품질 속성(NFR Requirements)** 을 **어떤 설계 패턴으로 실현하는가** 를 기록한다. 여기서 새로 결정하는 것은 NFR Requirements가 명시적으로 NFR Design으로 이월한 항목(스트리밍 버퍼 구체 크기, D3)뿐이며, 나머지는 확정된 NFR(카테고리별) · tech-stack-decisions(구체 크레이트 `sha2`/`std`/`thiserror`/`proptest`) · Functional Design(규칙 ID R-*, 속성 ID PROP-U1-*)을 **설계 패턴으로 실현**한다. 각 패턴은 (a) 패턴/결정 진술, (b) 실현하는 NFR·규칙 근거, (c) 구조 노트/불변식, (d) 명시적 트레이드오프로 기술한다. 논리 컴포넌트 분해 상세 맵은 자매 산출물 `logical-components.md`가 소유하며, 이 문서는 각 패턴이 어느 논리 컴포넌트에 안착하는지만 §11 추적표에서 참조한다.
>
> **표기 규약**: 화살표는 ASCII `A -> B`만 사용한다(유니코드 화살표 금지). 박스/선-그리기 문자를 쓰지 않는다. Rust 제네릭/타입/식별자(예: `Box<dyn Read>`, `Result<Manifest, BuildError>`, `u64::saturating_add`, `FileType::is_symlink`)는 백틱으로 감싼다.

---

## 0. 설정 값 취득 = 생성자 주입 (federated-config 정합, U0 FROZEN)

**(a) 패턴/결정**: U1은 **U0 `ConfigProvider`를 직접 읽지 않는다**. U1이 필요로 하는 모든 설정 값은 U8 composition root(`watcher-bin`)가 조립 시점에 **해소된 타입드 값으로 생성자 주입(downward injection)** 한다(wave-level DEC-FEDERATED-KEYS Q6=A). U1은 config 스냅샷을 보관하지 않고 순수 계산 값만 반환한다.

**(b) 주입 대상 표**:

| U1이 필요로 하는 값 | 종류 | 원천 / 주입 방식 |
|---|---|---|
| 볼트 루트 경로 | U0 **6 CORE 필드** 중 하나(`PathBuf`) | U8이 U0 `ConfigSnapshot`에서 해소한 `PathBuf`를 `VaultScanner` 생성자에 주입 |
| SafetyLimits(20 GiB / 2 GiB / 100k) | **U0 컴파일타임 상수**(config 아님) | `MAX_VAULT_TOTAL_BYTES`/`MAX_FILE_BYTES`/`MAX_FILE_COUNT` 직접 참조(D13, Q2=A) |
| 스트리밍 버퍼 크기 | **U1 내부 컴파일타임 상수**(§1, config 아님) | U1 소유 `const` — non-core federated 키로 노출하지 않음 |
| exclude/ignore 패턴 | **없음**(D9/MVP-trim) | 미지원 — U1은 federated config 키를 등록하지 않음 |

**(c) 구조 노트/불변식**: FD 계획 D9로 exclude_patterns가 MVP에서 제거되어 **U1은 U0 known-key set에 등록하는 federated 설정 키가 전무**하다. 따라서 U1에는 request_timeout_s/backoff류 non-core 파라미터 주입 대상도, 볼트 루트 라이브 리로드 관찰자도 없다(볼트 루트 변경은 재시작으로 반영, live-reload는 MVP 범위 밖). U0 core-field 리로드(token 등) 팬아웃은 U0 관찰자 소관이며 U1은 그 대상이 아니다. 불변식: U1 공개 생성자는 `ConfigProvider` 핸들이 아니라 **이미 해소된 타입드 값**(예: 볼트 루트 `PathBuf`)만 받는다 -> U0 FROZEN 유지·DAG 비순환.

**(d) 트레이드오프**: U1이 `ConfigProvider`를 직접 읽어 non-core 키(예: 스캔 튜닝)를 자체 해석하는 방식은 **미채택** — U0 typed 6-field 경계를 침범하고 U1을 config 표면에 결합시킨다. 생성자 주입은 U1을 순수 계산 lib로 유지하고 config 파싱/해소를 U8 단일 지점에 집중시킨다. `federated_config_handled = true`.

---

## 1. 스트리밍 고정 버퍼 해싱 (PERF-01 / REL-01) — `sha2` `Update`/`Finalize` + U1 소유 `const` 64 KiB 읽기 버퍼

**(a) 패턴/결정**: `ContentAddressing::hash_stream<R: Read>(reader)`는 **U1 소유 컴파일타임 `const` 고정 크기 읽기 버퍼**로 `reader`를 반복 읽어(`Read::read`) `sha2`의 `Sha256::update`에 증분 공급하고 EOF에서 `finalize`로 32바이트 `Sha256Digest`를 얻는다. **NFR Requirements가 NFR Design으로 이월한 구체 버퍼 크기(D3)를 여기서 확정한다: `const HASH_BUFFER_BYTES: usize = 64 * 1024`(64 KiB) 단일 고정 상수.** 파일 전체를 메모리에 적재하지 않으며 파일 크기와 무관하게 상수 메모리를 쓴다.

**(b) NFR/규칙 실현**:
- **U1-NFR-PERF-01**(스트리밍 메모리 바운드, 정성 계약): 고정 버퍼가 "전량 적재 없음 · 파일 크기에 비례하지 않는 상수 메모리"를 실현한다. SafetyLimits 상한(파일당 2 GiB, 총 20 GiB, 100k 파일)에서도 해시 단계 메모리는 버퍼 크기로 유계다.
- **U1-NFR-REL-01**(해싱 결정성 + `sha256sum` 일치, NFR-08): `Update`/`Finalize` 증분 API는 청크 분할 경계와 무관하게 동일 다이제스트를 내므로(R-CA-02) `hash_stream(concat(chunks)) == hash_stream(whole)`이 성립하고, 표준 SHA-256 구현이라 `sha256sum` 바이트 일치(R-CA-01)를 신뢰한다.
- **규칙 R-CA-03**(스트리밍 메모리 바운드) · **R-BUILD-02**(순차 스트리밍, 동시 다수 미적재).

**(c) 구조 노트/불변식**: 버퍼는 `hash_stream` 호출 스택-지역(또는 재사용 가능한 지역 `Vec<u8>`)으로 확보되며 파일당 순차 처리되어 동시 다수 파일 바이트가 적재되지 않는다(R-BUILD-02). 불변식: 임의 `Read` 스트림에 대해 `hash_stream`의 최대 상주 바이트는 `HASH_BUFFER_BYTES` + `Sha256` 내부 상태(고정 64바이트 블록)로 유계다. `hash_stream`은 `VaultScanner` 의존 없이 임의 `Read`를 받으므로(§4 seam) U3 전송 재검증(FR-22/Q8=A)이 재사용한다.

**(d) 트레이드오프**: (1) config/federated 키로 노출되는 **튜닝 가능 버퍼**는 **미채택**(MVP-trim) — U1은 federated 키를 등록하지 않고(§0), 단일 사용자 로컬 데몬에 버퍼 튜닝 요구가 없다. (2) 파일 크기 적응형 버퍼(작은 파일 = 작은 버퍼)도 미채택 — 복잡도 대비 이득이 없고 상수 메모리 계약을 흐린다. (3) 64 KiB는 `BufReader` 기본(8 KiB)보다 크게 잡아 대용량 파일 syscall 횟수를 줄이되 100k 파일 순차 처리에서도 상주 메모리가 무시할 만한 절충값이다. 수치 성능 게이트는 두지 않는다(§7, U0 정성-성능 스탠스 미러).

---

## 2. injective 길이-프리픽스 프레이밍 (REL-02) — `std` 바이트 프레이밍 + CBOR 코덱과 분리

**(a) 패턴/결정**: `ContentAddressing::manifest_digest(entries)`는 엔트리를 canonical 정렬(`relative_path` 바이트 사전식, tie-break `(raw_sha256, size)`)한 뒤 각 엔트리를 **`std` 바이트 길이-프리픽스 프레이밍**(`len(path)`를 `u64::to_be_bytes`로 고정폭 BE 프리픽스 + path 바이트 + `raw_sha256` 32바이트 + `size`를 `u64::to_be_bytes` BE)으로 직렬화해 concat한 바이트열을 `sha2`에 흘려 `ManifestDigest`를 계산한다. 이 프레이밍은 **U0 CBOR 지속 코덱(`encode`/`decode`)과 별개**이며 신규 직렬화 의존을 도입하지 않는다(D1).

**(b) NFR/규칙 실현**:
- **U1-NFR-REL-02**(순서 무관 결정성 + injective 프레이밍, NFR-08): 계산 내부에서 canonical 정렬을 강제하므로 발견/삽입 순서와 무관하게 동일 다이제스트(R-CA-05). 길이 프리픽스가 `["ab","c"]` vs `["a","bc"]` 같은 경계 모호성을 제거해 injective(단사) 인코딩을 보장한다(R-CA-04).
- **규칙 R-CA-04**(프레이밍 인코딩) · **R-CA-05**(순서 무관 결정성). no-op 판정(FR-10)이 지속 스키마 진화(NFR-13 신규 필드 추가)와 무관하게 버전 간 안정한 것은 프레이밍이 CBOR 코덱과 분리되어 있기 때문이다(D1).

**(c) 구조 노트/불변식**: 불변식 — (1) 논리적으로 동일한 엔트리 다중집합은 항상 동일한 프레이밍 바이트열 -> 동일 `ManifestDigest`. (2) 서로 다른 논리 집합은 서로 다른 프레이밍 바이트열을 만든다(다이제스트 충돌은 SHA-256 충돌 확률로만 발생). (3) 프레이밍은 순수 함수이며 I/O·상태가 없다. 정렬 키는 U0 `ManifestEntry`의 derive `Ord`와 일치한다.

**(d) 트레이드오프**: (1) CBOR 지속 코덱을 다이제스트 입력으로 재사용하는 방식은 **미채택** — CBOR는 값 복원(round-trip)용이라 지속 스키마 진화 시 바이트열이 흔들려 no-op 판정을 깨뜨릴 수 있다. 별도 안정 프레이밍이 버전 간 콘텐츠 지문의 안정성을 준다. (2) 구분자(delimiter) 기반 concat은 미채택 — 경로에 구분자 바이트가 나타나면 모호성이 재발한다. 길이-프리픽스가 delimiter-free로 injective를 보장한다. (3) 신규 직렬화 크레이트 도입은 미채택 — `u64::to_be_bytes` 등 `std`로 충분(신규 의존 0).

---

## 3. 정확 diff = canonical 투 포인터 merge-join + O(1) no-op short-circuit (REL-03)

**(a) 패턴/결정**: `ManifestDiffer::diff(last_committed, current)`는 (1) `current.manifest_digest == last_committed.manifest_digest`이면 전체 엔트리 비교 없이 **O(1)로 빈 `ChangeSet`을 조기 반환**하고(R-DIFF-04), (2) 그렇지 않으면 두 매니페스트가 canonical 정렬(U0 불변식)임을 전제로 **투 포인터 merge-join**으로 `added`/`modified`/`deleted`를 단일 패스 계산한다. `modified` 판정은 동일 경로에서 `(raw_sha256, size)` 차이로만 한다(R-DIFF-02, mtime 미사용).

**(b) NFR/규칙 실현**:
- **U1-NFR-REL-03**(정확 diff, 누락 0·오탐 0·서로소 + no-op 동치, NFR-12): merge-join이 두 정렬 시퀀스의 정확한 대칭차/교차-변경을 산출하고, 각 경로가 정확히 한 목록에만 나타나 서로소(disjoint)를 보장한다. `apply(prev, diff) == current`(PROP-U1-03).
- **규칙 R-DIFF-01**(범위 = a/m/d) · **R-DIFF-03**(정확성) · **R-DIFF-04**(no-op 동치) · **R-DIFF-05**(순수·결정적 merge-join, O(n+m)).

**(c) 구조 노트/불변식**: 불변식 — (1) 세 목록은 서로소이고 각각 `relative_path` 오름차순(merge-join 특성상 자동 정렬). (2) 지목되지 않은 경로는 양쪽 `(raw_sha256, size)`가 동일. (3) `diff(m, m).is_empty() == true`이고 `current.manifest_digest == prev.manifest_digest <=> diff.is_empty()`. (4) `last_committed`는 U1이 지속하지 않는 인자다 — **U4 `SyncStateStore`가 소유·전달**하며 최초 사이클 취급은 호출자(U8)가 결정(R-DIFF-06). `diff`는 순수·무결이며 I/O·상태가 없다.

**(d) 트레이드오프**: (1) rename/content-move 감지는 **미채택**(D4/MVP-trim) — 이름 변경은 deleted+added로 표현. 유사도 매칭은 MVP 범위 밖 복잡도. (2) 해시맵 기반 diff(정렬 전제 없이 `HashMap` 조인)는 미채택 — 두 매니페스트가 이미 canonical 정렬(U0 불변식)이라 merge-join이 추가 할당 없이 O(n+m)로 충분하고 출력 정렬도 자연 충족된다. (3) no-op O(1) short-circuit은 다이제스트 동치성(R-DIFF-04)에 의존하며, 다이제스트가 다르면 정확 비교로 폴백해 정확성을 훼손하지 않는다.

---

## 4. 스트리밍 seam = `R: Read` 제네릭 + `Box<dyn Read>` 어댑터 (포터빌리티 / REL-01 재사용 경계)

**(a) 패턴/결정**: 해싱은 파일시스템에 결합하지 않는다. `ContentAddressing::hash_stream<R: Read>`는 **임의 `Read` 스트림을 제네릭으로 받고**, `VaultScanner::open_reader`는 파일별 스트리밍 리더를 **`Box<dyn Read>` 어댑터**로 반환한다. 파일시스템 walk·심링크 판별은 `VaultScanner`(U1 유일 I/O)에 격리하며 **`std::fs`만** 사용한다(신규 walk 크레이트 없음, T2). U1은 U0의 push-only 싱크 계약(`Logger`/`StatusSink` 등)을 **구현하지 않는다** — 관측·표면화는 값 반환 후 U8이 수행한다.

**(b) NFR/규칙 실현**:
- **U1-NFR-REL-01 재사용 경계**: `hash_stream`이 `R: Read`로 추상화되어 `VaultScanner` 의존 없이 U3 `UploadProtocolDriver`의 전송 재검증(FR-22/Q8=A)에서 임의 blob 스트림에 재사용된다.
- **포터빌리티(NFR-05)**: `std::fs::read_dir` 재귀 + `DirEntry::file_type()` -> `FileType::is_symlink()`(심링크 SKIP, R-SCAN-02) + `std::fs::File`은 macOS/Windows/Linux 플랫폼 중립이라 크로스플랫폼 재현 빌드에 별도 U1 포터빌리티 통제가 불필요하다. Edition 2024 + 고정 MSRV 1.85는 `crate.workspace = true`로 U0에서 상속한다.
- **규칙 R-SCAN-02**(심링크 SKIP) · **R-SCAN-03**(dotfile 포함) · **R-SCAN-04**(exclude 패턴 없음) · **business-logic-model §3.2**(`open_reader`).

**(c) 구조 노트/불변식**: 불변식 — (1) 파일 콘텐츠 I/O는 `VaultScanner`에만 존재하고 나머지 4개 컴포넌트(`ContentAddressing`·`ManifestBuilder` 조립부·`ManifestDiffer`·`SafetyLimitsValidator`)는 순수·무결(단, `ManifestBuilder`는 `VaultScanner` 경유 I/O를 조율). (2) `hash_stream`은 `Read` 계약만 요구해 파일/메모리/네트워크 스트림에 동형으로 동작. (3) U1은 U0 싱크 트레이트를 import는 하되 **구현하지 않으며**, 판정/값(`Manifest`/`ChangeSet`/`LimitVerdict`/`ScanError`)만 반환한다(push 안 함).

**(d) 트레이드오프**: (1) `walkdir` 크레이트 도입은 **미채택**(MVP-trim) — 심링크 SKIP·정렬은 `std` `FileType`/canonical 정렬로 직접 제어해야 하고 exclude 패턴이 없어(D9) walk 커스터마이즈 필요가 낮다. (2) `hash_stream`을 파일 경로 인자로 특수화하는 방식은 미채택 — `R: Read` 제네릭이 U3 재검증 재사용을 가능케 하는 핵심 seam이다. (3) `Box<dyn Read>`의 동적 디스패치 비용은 파일당 1회로 무시할 만하며, 스캐너 리더 생성 지점을 단일 어댑터로 통일한다.

---

## 5. fail-fast build + 루트 오류 값 반환 (REL-06) — U0 오류 taxonomy 경유 파괴적 커밋 방지

**(a) 패턴/결정**: `ManifestBuilder::build`는 scan 또는 파일별 `hash_stream`에서 **어느 파일이라도 실패하면 전체를 중단**하고 `Err(BuildError)`를 반환한다 — **부분 매니페스트(일부 파일 스킵)를 반환하지 않는다**(D12). `VaultScanner::scan`은 루트 부재/언마운트/접근 불가를 `Err(ScanError::RootUnavailable)`로 **값 반환**하며 0-파일 `VaultSnapshot`을 만들지 않는다(R-SCAN-05). 오류는 U0 관례를 승계한 `thiserror` 파생 타입(`ScanError`/`BuildError`)으로 `source`(원인 `io::Error`)를 보존한다.

**(b) NFR/규칙 실현**:
- **U1-NFR-REL-06**(fail-fast build + 루트 오류 반환): 스킵된 파일이 diff에서 "삭제"로 오인되어 파괴적 커밋을 유발하는 것과, 0-파일 매니페스트가 파괴적 빈 커밋을 유발하는 것을 방지한다.
- **U1-NFR-MNT-01**(오류 타입 파생 전략): 운영 오류(`ScanError`/`BuildError`) = `thiserror` 파생(`Display`/`std::error::Error` + `source`). `BuildError::Scan(ScanError)`·`Hash{path, source}`로 오류 국소성 보존.
- **규칙 R-BUILD-01**(fail-fast, D12) · **R-SCAN-05**(루트 가용성 -> 값 반환).
- **Resiliency Baseline(ON)**: fail-fast + 루트 오류 반환이 파괴적 부분/빈 커밋을 방지해 사이클 정확성에 기여(RESILIENCY-01 부분 적용).

**(c) 구조 노트/불변식**: 불변식 — (1) `build`는 `Ok(Manifest)` 또는 `Err(BuildError)`만 반환(부분 성공 산출물 없음). (2) `scan`은 루트 도달 불가 시 0-엔트리 `VaultSnapshot`을 절대 산출하지 않는다. (3) vault-unavailable **해석·표면화는 U2 `VaultAvailabilityGuard`/U8** 소관이며 U1은 값만 반환(push 안 함, 로그/상태에 직접 쓰지 않음). RTO/RPO 수치·재시도 정책은 U8/U3 소관(U1 순수 로직에 N/A).

**(d) 트레이드오프**: (1) `anyhow`식 타입소거 오류는 **미채택**(U0 상속) — `LimitViolation`/`ScanError` 구조화 변이를 소실시켜 actionable 리포트(FR-05)와 vault-unavailable 판정(U2 소비)을 불가능하게 한다. (2) best-effort 부분 매니페스트(실패 파일 스킵 후 진행)는 미채택 — 파괴적 diff를 유발한다. fail-fast 후 다음 사이클 재스냅샷이 안전(FQ-2=A). (3) `ScanError`의 vault-unavailable **해석을 U1이 수행**하는 방식은 미채택 — 판정 결합을 피하고자 값 반환에 국한(§0 노트2 소유 경계).

---

## 6. panic-free-total = 순수 표면 clippy lint-gate + `saturating_add` + PBT no-panic (REL-07)

**(a) 패턴/결정**: U1 순수 표면(`hash_stream`·`manifest_digest`·`diff`·`validate`)이 위치한 **순수 모듈 상단에 컴파일타임 clippy lint-gate**를 건다: `deny(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing, clippy::panic)`(U0 §1 패턴 미러). `SafetyLimitsValidator::validate`의 총량 누적은 `u64::saturating_add`(std)로 오버플로 패닉을 방지한다(R-LIMIT-05). 여기에 NFR Requirements가 요구한 PBT no-panic 속성(§2.7 AC-1/2)을 병행한다. 런타임 방어 래퍼는 두지 않는다(런타임 비용 0).

**(b) NFR/규칙 실현**:
- **U1-NFR-REL-07**(순수 표면 panic-free total): "모든 입력에 대해 패닉 없이 종결" 요구를 코드 규율이 아니라 **컴파일타임 lint 게이트로 승격**해 `unwrap`/슬라이스 인덱싱/`panic!` 재도입을 차단한다. `validate`는 100k/20 GiB 근접 대값에서도 `saturating_add`로 산술 패닉이 없다.
- **U1-NFR-REL-04**(SafetyLimits) 및 **U1-NFR-REL-02/03**(diff/digest): 임의 엔트리 집합에 대해 `unwrap`/인덱싱 패닉 없이 값을 낸다. `hash_stream`의 파일 I/O 실패는 패닉이 아니라 `Result`(상위에서 `BuildError::Hash`)로 표면화.
- **규칙 R-LIMIT-05**(saturating 오버플로 방지) · **R-LIMIT-06**(순수). U0-NFR-REL-02(panic-free total, 워크스페이스 규율)와 정합.

**(c) 구조 노트/불변식**: lint-gate는 **순수 모듈에만** 적용한다(`ContentAddressing`·`ManifestDiffer`·`SafetyLimitsValidator` + `ManifestBuilder`의 순수 조립부). `VaultScanner`는 파일 I/O를 수행하므로 순수 표면이 아니나, I/O 실패는 여전히 `Result`(`ScanError`)로만 표면화하며 `unwrap` 패닉 경로가 없다(규율 + 예제 테스트로 검증). 불변식: 순수 표면의 모든 실패/경계는 `Result::Err` 또는 total 판정 값으로만 나가고 `panic!`/`unwrap`/인덱싱 패닉 경로가 타입·lint 수준에서 부재한다.

**(d) 트레이드오프**: (1) `decode`류 경계 런타임 `catch_unwind` 방어 래퍼는 **미채택**(U0 미러) — total 순수 함수에 unwind-catch는 panic-free-total 계약과 상충하고 버그를 은폐한다. (2) lint 없이 코드 규율 + PBT만도 미채택 — 구조적 강제가 약해 재도입 여지. (3) `checked_add` + 오류 반환 대신 `saturating_add`를 택한 이유: 총량이 `u64` 상한을 넘으면 그 자체가 명백한 SafetyLimits 초과이므로 saturate 후 `TotalBytes` 위반으로 판정하는 것이 actionable하고 total function을 유지한다.

---

## 7. 수치 성능 게이트 부재 = 정성 계약만 (PERF-01) — U0 정성-성능 스탠스 미러

**(a) 패턴/결정**: U1에 **throughput·latency·peak-memory 수치 게이트를 두지 않는다**. §1 고정 버퍼 스트리밍 + §3 O(n+m) merge-join + O(n) 단일 패스 validate라는 **정성 계약(전량 적재 없음 · 선형·유계 비용)** 만 문서화한다. 구체 버퍼 크기(64 KiB, §1)만 수치로 확정하고, 절대 처리량/지연 임계는 설정하지 않는다.

**(b) NFR/규칙 실현**:
- **U1-NFR-PERF-01**(스트리밍 메모리 바운드, 수치 목표 N/A): (a) 처리 비용은 볼트 파일 수·바이트에 선형·유계라 절대 수치 목표를 정의할 도메인 근거가 없다. (b) requirements NFR-02는 "무한정 메모리 없이 스트리밍/증분 해시"라는 정성 바운드만 요구한다. (c) 데몬은 단일 사용자·로컬 프로세스라 처리량 SLA가 없다(RESILIENCY-02).
- **규칙 R-CA-03**·**R-BUILD-02**.

**(c) 구조 노트/불변식**: 불변식 — (1) `hash_stream` 상주 메모리는 버퍼 크기로 유계(§1). (2) `build`는 파일 순차 처리로 동시 다수 파일 바이트 미적재. (3) `diff`는 O(n+m), `validate`는 O(n). 매니페스트 메타 자체는 파일 수 상한(100k)에 비례하는 유한 크기다.

**(d) 트레이드오프**: 수치 벤치마크 게이트(예: "X MB/s 이상")는 **미채택**(MVP-trim) — 입력 크기 종속이라 안정적 임계를 정의할 수 없고 단일 사용자 로컬 데몬에 SLA가 없다. 스트리밍 특성은 대량 경계 예제/PBT로 간접 검증한다(§8).

---

## 8. PBT 실현 = U0 `proptest-support` 재사용 + U1 전용 `proptest-support` 비기본 feature (MNT-02)

**(a) 패턴/결정**: PROP-U1-01~05를 구현하는 제너레이터는 (a) U0 `foundation`의 **`proptest-support`(비기본 feature)** 제너레이터(`arb_manifest_entry`/`arb_relative_path`/`arb_sha256_digest`/`arb_byte_count`/`arb_manifest`)를 **재사용**하고, (b) U1 전용 제너레이터를 **U1 자신의 비기본 `proptest-support` feature**로 노출한다(U0 패턴 미러). 프레임워크 = `proptest`(U0 상속, PBT-09/D19).

**(b) NFR/규칙 실현**:
- **U1-NFR-MNT-02**(PBT 제너레이터 재사용 + 노출, PBT-07): 단일-출처 원칙 -> 정의 드리프트 방지 + non-default feature 게이트로 `proptest`가 프로덕션 빌드에 유출되지 않는다.
- **속성 정렬**: PROP-U1-01(해싱 결정성/Oracle) · PROP-U1-02(순서 무관/injective) · PROP-U1-03(diff 오라클/재구성·서로소) · PROP-U1-04(경계-정확·단조) · PROP-U1-05(round-trip). U1 전용 제너레이터 = 임의 `Vec<u8>` + 청크 분할 시퀀스 · 경로 유일 엔트리 집합 + 순열 + 프리픽스 중첩 · 상관 매니페스트 쌍 · 경계 조준 SafetyLimits 입력(limit-1/limit/limit+1) · digest-consistent 매니페스트.

**(c) 구조 노트/불변식**: 불변식 — (1) U1 제너레이터 모듈은 non-default feature 뒤에 위치해 기본 빌드에 `proptest` 의존이 나타나지 않는다. (2) 제너레이터는 문서화된 도메인 제약(경로 유일 엔트리 집합, digest-consistent 매니페스트, 경계 조준 입력)을 존중한다. `VaultScanner` I/O는 값 속성이 없어 임시 디렉터리 픽스처 기반 예제/통합 테스트로 판정(FD §6.6, blocking 아님).

**(d) 트레이드오프**: (1) `proptest`를 기본 의존으로 두는 방식은 **미채택**(U0 미러) — 프로덕션 빌드에 테스트 크레이트 유출. (2) U1 제너레이터를 U0에 두는 방식도 미채택 — U1 도메인 특화 제너레이터(바이트 스트림·경계 조준 한도 입력)는 U0에 없고 소유 경계상 U1이 자기 feature로 추가한다. shrinking·고정 시드·CI(PBT-08)는 Code Generation / Build-and-Test 이월.

---

## 9. 문서 / 품질 린트 (MNT-03) — `#![deny(missing_docs)]` + no 커버리지 %-게이트 + 예제 앵커

**(a) 패턴/결정**: `content-core` 공개 항목에 **`#![deny(missing_docs)]`를 강제**하고, **전역 커버리지 %-게이트를 두지 않는다**. 실질 검증은 PBT(§8 속성들) + 예제 앵커(PBT-10: `VaultScanner` I/O·심링크 SKIP·hidden 포함·fail-fast 경로에 예제/통합 테스트 병행)로 확보한다.

**(b) NFR/규칙 실현**:
- **U1-NFR-MNT-03**(커버리지/문서 정책): `hash_stream`(U3 재사용)·`validate`(U3 재검사)·`build`/`diff`(U8 사이클) 등 상위 단위가 소비하는 공개 API 문서화를 컴파일타임에 강제한다.
- **PBT-10**(PBT는 예제 테스트를 대체하지 않고 보완) · U0-NFR-MNT-03 정합.

**(c) 구조 노트/불변식**: 불변식 — (1) 공개 항목에 문서 주석이 누락되면 `deny(missing_docs)`로 컴파일 실패. (2) business-critical 경로(해싱·다이제스트·diff·경계 판정)는 PBT를, I/O 경로(`VaultScanner`)는 예제/통합 테스트를 가진다 -> 검증 공백 없음.

**(d) 트레이드오프**: 커버리지 % 게이트는 **미채택**(U0 정합) — 순수 코어의 결정성은 속성으로, I/O 경로는 예제로 검증하는 것이 line/branch % 게이트보다 실질적이다. `deny(missing_docs)`는 다수 상위 단위 소비 공개 표면이라 near-free로 채택한다. 상세 CI 커버리지 통합은 Build-and-Test 이월.

---

## 10. MANDATORY 카테고리 N/A 판정표

| 카테고리 | 판정 | 근거 |
|---|---|---|
| **Scalability** | N/A | U1은 순수 코어 lib로 자체 런타임·스레드·처리량 축이 없음. 100k/20 GiB는 SafetyLimits **상한**(U0 상수)이며 스케일 축이 아님. 스트리밍 메모리 바운드는 §1/§7 정성 계약으로 표면화. 신규 확장성 패턴 없음 |
| **Performance** | N/A(정성 계약만) | 처리 비용은 파일 수·바이트에 선형·유계라 throughput/latency/peak-memory 수치 게이트 근거 없음. §1 고정 버퍼(64 KiB)·§3 O(n+m) merge-join·§7 정성 계약만 확정 |
| **Availability** | N/A | lib 크레이트라 SLA 없음. 전체 Watcher는 단일 사용자·재시작 가능 로컬 프로세스(RESILIENCY-02에서 availability-SLA N/A 확정) |
| **Resilience DR / RTO / RPO** | 부분 적용 + 대체로 N/A | §5 fail-fast build + 루트 오류 반환(파괴적 부분/빈 커밋 방지) · §6 panic-free total(대값 산술 크래시 회피) · §1 스트리밍(OOM 회피) · U1 산출 `Manifest`(무손실 REL-05)가 U4 zero-loss 지속(RPO=0)의 입력. RTO/RPO 수치·DR·배포/롤백·HA는 U1 순수 로직에 **N/A**(RESILIENCY-02) |
| **Usability(사람-대면)** | N/A | U1은 UI/CLI/config 표면 없는 lib. actionable `LimitViolation`(한도 종류·`limit`·`actual`·`path`)은 U1-NFR-REL-04로 요구화, 실제 사람-대면 표면화는 U8/U6 소관 |
| **Security(강제 통제)** | N/A | Security Baseline OFF, U1은 토큰/시크릿 미취급. 암호화 저장·키관리·시크릿 스캐닝 신설 없음. TLS(NFR-06)는 U5/U0 소관. RISK-01(로컬 평문 산출물)은 문서화된 수용 위험이며 U1은 값만 반환(지속은 U4) |
| **Logical Components** | 다뤄짐(N/A 아님) | Application Design이 확정한 5개 컴포넌트의 내부 논리 분해·의존 매핑은 자매 산출물 `logical-components.md`가 소유 |

---

## 11. 추적표 (패턴 -> NFR ID -> 규칙 ID -> 논리 컴포넌트)

| 설계 패턴 | NFR ID | 규칙 ID | 안착 논리 컴포넌트 |
|---|---|---|---|
| §0 생성자 주입(federated-config 정합) | (federated Q6=A) | R-LIMIT-01, R-SCAN-04(D9) | `VaultScanner`(볼트 루트 주입) · `SafetyLimitsValidator`(U0 상수) |
| §1 스트리밍 고정 버퍼 해싱(64 KiB `const`) | U1-NFR-PERF-01, U1-NFR-REL-01 | R-CA-01/02/03, R-BUILD-02 | `ContentAddressing` -> StreamHasher |
| §2 injective 길이-프리픽스 프레이밍 | U1-NFR-REL-02 | R-CA-04, R-CA-05 | `ContentAddressing` -> DigestFramer |
| §3 merge-join 정확 diff + O(1) no-op | U1-NFR-REL-03 | R-DIFF-01..06 | `ManifestDiffer` |
| §4 `R: Read` seam + `Box<dyn Read>` 어댑터 | U1-NFR-REL-01(재사용), NFR-05(포터빌리티) | R-SCAN-02/03/04 | `VaultScanner` -> DirWalker · ReaderFactory; `ContentAddressing` -> StreamHasher |
| §5 fail-fast build + 루트 오류 값 반환 | U1-NFR-REL-06, U1-NFR-MNT-01 | R-BUILD-01, R-SCAN-05 | `ManifestBuilder` · `VaultScanner`; ErrorTaxonomy(`BuildError`/`ScanError`) |
| §6 panic-free total(clippy lint-gate + `saturating_add`) | U1-NFR-REL-07, U1-NFR-REL-04 | R-LIMIT-05, R-LIMIT-06 | `ContentAddressing` · `ManifestDiffer` · `SafetyLimitsValidator` |
| §7 수치 성능 게이트 부재(정성 계약) | U1-NFR-PERF-01 | R-CA-03, R-BUILD-02 | (전 순수 컴포넌트) |
| §8 PBT 제너레이터 재사용·노출 | U1-NFR-MNT-02 | (제너레이터 계약, PBT-07) | ProptestGenerators(test-support 논리 단위) |
| §9 문서/품질 린트(`deny(missing_docs)`) | U1-NFR-MNT-03 | (공개 계약, PBT-10) | (crate 전역 공개 표면) |

---

## 12. 확장 컴플라이언스 요약 (완료 게이트용)

| 확장 | 활성 | 이 단계 판정 | 근거 |
|---|---|---|---|
| **Property-Based Testing** | ON (Full) | **준수 — blocking 없음** | §8이 각 패턴을 확정 속성(PROP-U1-01~05)에 정렬하고 U0 `proptest-support` 재사용 + U1 전용 feature 노출(PBT-07)을 설계로 확정. §6 panic-free는 PBT no-panic(PROP-U1-02/04 제너레이터 재사용)과 병행. PBT-09는 NFR-Req 충족(재선택 없음). `VaultScanner` I/O는 예제/통합 판정(FD §6.6, blocking 아님). PBT-08은 Code Generation/Build-and-Test 이월 |
| **Resiliency Baseline** | ON | **준수(부분 적용 + 대체로 N/A) — blocking 없음** | RESILIENCY-01: U1은 결정적 코어(사이클 정확성 기반). §5 fail-fast build + 루트 오류 반환(파괴적 부분/빈 커밋 방지) · §6 panic-free total(대값 산술 크래시 회피) · §1 스트리밍(OOM 회피) · U1 산출 `Manifest`(무손실 REL-05)가 U4 zero-loss 지속(RPO=0, NFR-03)의 입력. RTO/RPO 수치·DR·배포/롤백·HA는 U1(순수 로직)에 N/A(RESILIENCY-02). 신규 U1 resiliency 결정 없음 |
| **Security Baseline** | OFF | **N/A — 미로딩·미강제** | U1은 토큰/시크릿 미취급(§10). RISK-01(로컬 평문 산출물)은 문서화된 수용 위험이며 U1은 값만 반환(지속은 U4). 신설 통제 없음 |

**블로킹 판정**: 모든 적용 가능한 활성 확장 규칙이 준수(PBT)되거나 부분 적용 + N/A(Resiliency)/N/A(Security)로 판정됨 — **blocking finding 없음.**

---

## 13. Autopilot Decisions (질문 대체 — RECOMMENDED · MVP 편향)

AUTOPILOT(사용자 승인, 게이트 waived) 하에 이 NFR Design 단계에서 저자가 확정한 열린 항목이다. FD 계획(D1~D19)·NFR Requirements 결정(T1~T9)은 재오픈하지 않으며, 아래는 **설계 패턴 실현 선택**에 국한한다.

| # | 주제(topic) | 선택(chosen) | MVP-trim? | 근거 |
|---|---|---|---|---|
| P1 | 스트리밍 해시 구체 버퍼 크기(D3 이월) | **`const HASH_BUFFER_BYTES = 64 * 1024`(64 KiB) 단일 고정 상수** | **예** | config/federated 튜닝·적응형 버퍼 미채택 — 단일 고정 상수가 정성 메모리 바운드(PERF-01)를 실현하고 튜닝 요구가 없다(§1). |
| P2 | diff 알고리즘 | **canonical 투 포인터 merge-join + O(1) no-op short-circuit** | 아니오 | 두 매니페스트가 이미 정렬(U0 불변식)이라 `HashMap` 조인 없이 O(n+m)·추가 할당 최소·출력 정렬 자연 충족(§3). |
| P3 | panic-free 강제 방식 | **순수 모듈 clippy lint-gate + `saturating_add` + PBT no-panic**(U0 §1 미러) | 아니오 | `catch_unwind` 런타임 래퍼 미채택(계약 상충·버그 은폐). 컴파일타임 강제가 재도입 차단(§6). |
| P4 | 해싱/스캔 결합도 | **`R: Read` 제네릭 + `Box<dyn Read>` 어댑터**(파일 I/O를 `VaultScanner`에 격리) | 아니오 | `hash_stream`을 파일시스템에 결합하지 않아 U3 전송 재검증 재사용을 가능케 하는 핵심 seam(§4). |
| P5 | 관측 표면화 | **U0 싱크 계약 미구현 — 판정/값만 반환**(push 안 함) | **예** | 로그/상태 push를 U1이 하지 않고 U8이 소비(§4/§5 소유 경계). U1 표면·의존 최소화. |
| P6 | 설정 값 취득 | **생성자 주입**(볼트 루트 = U0 core-field, U8 주입); federated 키 없음 | **예** | wave-level federated-config(Q6=A) 정합, U0 FROZEN 유지, exclude_patterns 제거(D9)(§0). |
| P7 | 총량 오버플로 처리 | **`u64::saturating_add`**(saturate 후 `TotalBytes` 위반 판정) | 아니오 | `checked_add`+오류 대신 saturate가 total function 유지 + actionable 판정(§6). |
| P8 | 성능 수치 게이트 | **없음**(정성 계약만; 버퍼 크기만 수치 확정) | **예** | 처리 비용 입력 크기 선형·유계, 단일 사용자 로컬(U0 정성-성능 스탠스 미러, §7). |

**federated_config_handled**: **true** — §0에서 U1이 config 값을 U8 생성자 주입(해소된 타입드 값)으로만 받고 `ConfigProvider`를 직접 읽지 않음(U0 core-field 리로드 대상도 아님)을 명시했으며, exclude_patterns 제거(D9)로 U1은 federated 키를 전혀 등록하지 않음을 확정했다.
