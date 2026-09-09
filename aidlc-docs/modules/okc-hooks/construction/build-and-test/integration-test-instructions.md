# Integration Test Instructions

## Integration repair addendum

The latest module tests include actual threaded retry scheduling, temporary-file snapshot spooling, and persistent target identity checks. Run `PROPTEST_RNG_SEED=20260909 cargo test --workspace --features proptest-support` for the full 250-test suite. Use the `upload-client` example `protocol-fixture` to generate exact Rust CBOR requests for the root/web receiver tests; see [watcher-setup.md](watcher-setup.md) for the agreed contract and commands. Earlier statements below that the server does not exist describe the original construction baseline, not the current root integration implementation.

## Purpose

유닛 간 상호작용(트리거 -> 사이클 -> 전송 -> 상태/히스토리 표면화, 제어면 위임, config 합류)이 함께 올바로
동작하는지 검증한다. 이 프로젝트의 크로스-유닛 검증은 **in-tree 통합 테스트 + seam 계약(기록 fake/mock)** 로
성립하며, 실제 네트워크·파일시스템·라이브 서버 없이 결정적으로 돈다. 라이브 okc 서버 대상 end-to-end 는
서버 부재로 이연 상태다(아래 "Blocked/Deferred" 참조).

## Prerequisite: 툴체인 PATH

```bash
export PATH="$HOME/.rustup/toolchains/1.97.1-aarch64-apple-darwin/bin:$PATH"
```

## 존재하는 크로스-유닛 커버리지 (in-tree, 확인됨)

### Scenario 1: U8 조립 루트가 9개 라이브러리를 전부 조립

- **Description**: `watcher-bin`(U8)이 U0..U7b 아홉 크레이트를 하위 -> 상위 단방향으로 조립하는 단일 배포
  바이너리다(RESILIENCY-01). 배선 그래프(`WIRING_EDGES`, 35간선)에 역엣지가 없음을 `wiring_is_acyclic` 로
  검증한다(PROP-U8-03) — 컴파일 성공 자체가 9개 공개 API 계약 합류의 통합 증거다.
- **검증 명령**: `cargo build --workspace` (성공, 10개 크레이트) + `cargo test -p watcher-bin --features proptest-support`.
- **Expected**: 빌드 성공 + 배선 비순환 property 통과.

### Scenario 2: `SyncCycleCoordinator` 크로스-유닛 사이클(U1/U2/U4/U5/U6/U7b)

- **Description**: `watcher-bin/tests/coordinator_cycle.rs` 가 코디네이터를 6개 port seam(U4 store / U2 guard /
  U1 manifest-source / U5 consent / U3 driver / U7b run-state)에 record-fake 로 배선해 한 사이클을 구동한다.
  paused-skip -> vault 가용성 -> 매니페스트 재계산 -> guard_diff -> diff -> consent 게이트 -> 드라이버 위임 ->
  U6 status/history/critical push 의 유닛 간 순서·배타성을 검증한다(R-U8-06/07/08/09).
- **Setup**: 없음(in-memory 기록 fake). `--features proptest-support` 로 property 분기까지 포함.
- **검증 명령**: `cargo test -p watcher-bin --features proptest-support` (예제 7 + property 관련 분기).
- **Expected**: 정확히 하나의 배타적 `CoordinatorOutcome`, 드라이버 조건부 정확히 1회 호출, hold 시 status 조건 1회 raise.

### Scenario 3: U3 -> U5 전송 계약(HttpTransport mock)

- **Description**: `upload-client`(U3) 테스트가 `auth-consent`(U5)의 `HttpTransport`/전송 seam 을 mock 으로
  구동해 업로드 프로토콜(preflight -> have-want -> 전송/청크 -> 재검증 -> commit)의 유닛 간 계약을 검증한다.
- **Setup**: 없음(mock transport). 라이브 HTTP 없음.
- **검증 명령**: `cargo test -p upload-client --features proptest-support` (16 통과).
- **Expected**: mock 계약 상에서 프로토콜 단계 전이·오류 분류가 규격대로.

### Scenario 4: U7b 제어면 프로토콜 round-trip

- **Description**: `ops-control`(U7b) 테스트가 IPC 프레이밍 + 요청/응답 프로토콜 + 디스패치 핸들러를
  record-fake 로 검증한다(U8 `DaemonControlHandlers` 가 위임하는 대상). 버전 핸드셰이크, health -> 종료코드,
  라우팅 이분법 포함.
- **검증 명령**: `cargo test -p ops-control --features proptest-support` (23 통과).
- **Expected**: 프로토콜 무손실 round-trip + 부작용 0/1회 + 배타 라우팅.

## Run Integration Tests (요약)

전체 크로스-유닛 커버리지는 크레이트별 테스트 스위트에 내장돼 있으므로, 단위 테스트 실행이 곧 통합 검증이다:

```bash
# 크로스-유닛 통합 커버리지가 포함된 크레이트만 우선 실행
cargo test -p watcher-bin    --features proptest-support   # 조립 + 코디네이터 크로스-유닛(24)
cargo test -p upload-client  --features proptest-support   # U3 -> U5 전송 계약(16)
cargo test -p ops-control    --features proptest-support   # U7b 제어면 프로토콜(23)
```

- **Logs Location**: cargo 표준 출력. 실패 시 assert 메시지에 유닛 경계와 기대 계약이 드러난다.
- **Cleanup**: 없음(in-memory). `instance_lock`/일부 테스트는 임시 파일을 `std::env::temp_dir()` 아래 만들고
  각 테스트가 스스로 정리한다.

## Blocked / Deferred 크로스-유닛 상호작용 (정직한 미커버 목록)

아래는 현재 자동 통합 검증에 **포함되지 않은** 유닛 간/외부 상호작용이다. mock 계약으로 대체 검증했거나 MVP
범위에서 이연했다.

- **라이브 okc 서버 end-to-end (DEP-01/02/05/06/07: blocked-on-server)**: 실제 인증(U5 -> 서버) 및 업로드 커밋
  (U3 -> 서버) 왕복은 서버 부재로 미검증이며 **mock transport 계약으로만** 검증했다. 서버 가용 시 라이브 계약
  테스트(contract test)로 승격 필요.
- **Windows 명명 파이프 IPC**: U7b IPC seam 뒤 Windows named-pipe 는 이연 스텁이며, **검증 경로는 Unix 도메인
  소켓**뿐이다(CLI <-> 데몬 제어면 왕복은 UDS 로만 확인).
- **OS 보안 저장소(secure-store)**: 이연 null-object 로, 토큰 1차 경로는 요구사항 §13 대로 config/env 평문
  토큰이다. OS 키체인/자격증명 관리자 연동은 미검증.
- **AutoUpdater 백그라운드 루프(U7a)**: MVP 이연 — `UpdateSource` 미배선이라 U7a 자동 업데이트 폴링/롤백의
  런타임 통합은 미검증(설정 해소만 확인).
- **크로스-OS 빌드/clippy**: cfg'd-out Windows/Linux arm 은 macOS aarch64 에서 실행 검증하지 않았다. 멀티-OS
  CI 매트릭스(+ MSRV 1.89 최소 버전 잡) 및 정확한 크레이트 patch-pin 은 이연.
