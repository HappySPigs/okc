# Unit Test Execution

크레이트별 단위·예제 테스트와 property-based test(PBT) 실행 지침이다. 모든 수치는 실제 실행으로 확인됐다.

## Prerequisite: 툴체인 PATH

```bash
export PATH="$HOME/.rustup/toolchains/1.97.1-aarch64-apple-darwin/bin:$PATH"
```

## PBT feature 게이트 (중요)

proptest 기반 property 테스트는 각 크레이트의 **비-default feature `proptest-support`** 뒤에 게이트돼 있다
(PBT-07: proptest 를 프로덕션 빌드 그래프에서 배제). 따라서:

- 기본 `cargo test`(플래그 없음) -> 예제/단위 테스트만 실행(PBT 제외).
- `--features proptest-support` -> 예제/단위 + property 테스트 전부 실행.

아래 수치는 **`--features proptest-support` 기준(PBT 포함)** 이며, 크레이트 10개 전부 이 feature 를 정의한다.

## Run Unit Tests

### 1. 크레이트별 실행(검증된 경로)

property 테스트를 함께 돌리려면 크레이트마다 `--features proptest-support` 를 붙여 실행한다:

```bash
cargo test -p foundation        --features proptest-support
cargo test -p content-core      --features proptest-support
cargo test -p change-detect     --features proptest-support
cargo test -p sync-state        --features proptest-support
cargo test -p auth-consent      --features proptest-support
cargo test -p observability     --features proptest-support
cargo test -p upload-client     --features proptest-support
cargo test -p lifecycle-deploy  --features proptest-support
cargo test -p ops-control       --features proptest-support
cargo test -p watcher-bin       --features proptest-support
```

### 2. Review Test Results

**Expected: 총 237개 통과, 0 실패**(확인된 결과). 크레이트별 내역:

| 크레이트 | 유닛 | 통과 테스트 수 |
|---|---|---|
| `foundation` | U0 | 32 |
| `content-core` | U1 | 17 |
| `change-detect` | U2 | 26 |
| `sync-state` | U4 | 15 |
| `auth-consent` | U5 | 35 |
| `observability` | U6 | 24 |
| `upload-client` | U3 | 16 |
| `lifecycle-deploy` | U7a | 25 |
| `ops-control` | U7b | 23 |
| `watcher-bin` | U8 | 24 |
| **합계** | | **237** |

- **Test Coverage**: 정량 커버리지 게이트는 이 프로젝트에서 채택하지 않았다(도구 미도입). 대신 각 순수 모듈에
  대해 property-based test 로 도메인 불변식을 검증하고, 패닉-프리 모듈에는 컴파일타임 lint-gate
  (`deny(clippy::unwrap_used/expect_used/indexing_slicing/panic)`)를 둔다.
- **Test Report Location**: cargo 표준 출력(터미널). 각 크레이트 실행 끝의 `test result: ok. N passed; 0 failed`.
- **watcher-bin(U8) 24개 내역**: 예제 15개(coordinator_cycle 7 + config_keys 4 + instance_lock 4) +
  property 9개(PROP-U8-01/03/04/05/06/07). 세부는 integration-test-instructions.md 참조.

### 3. 기본(PBT 제외) 실행

property 없이 예제/단위만 빠르게 돌리려면 feature 없이 실행한다(수치는 위 표보다 작다):

```bash
cargo test --workspace
```

### 4. Fix Failing Tests

테스트가 실패하면:
1. 터미널의 실패 테스트명과 assert 메시지를 확인한다.
2. 해당 크레이트 소스에서 원인을 수정한다.
3. 해당 크레이트만 `cargo test -p <crate> --features proptest-support` 로 재실행해 통과를 확인한다.
4. 마지막에 clippy 게이트를 재확인한다: `cargo clippy --workspace --all-targets --features proptest-support -- -D warnings`.

## Lint Gate (clippy)

단위 테스트와 함께 통과해야 하는 정적 게이트다(확인된 결과: **CLEAN, 0 error**):

```bash
cargo clippy --workspace --all-targets --features proptest-support -- -D warnings
```

`--all-targets` 로 lib/bin/test/example 전부, `--features proptest-support` 로 property 코드까지 포함해
경고를 에러로 승격(`-D warnings`)한 상태에서 통과한다.
