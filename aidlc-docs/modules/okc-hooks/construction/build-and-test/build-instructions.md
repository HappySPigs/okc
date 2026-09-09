# Build Instructions

okc-hooks Watcher 워크스페이스(10개 크레이트, 단일 배포 바이너리 `watcher-bin`)의 빌드 지침이다.
모든 명령/결과는 실제 툴체인 실행으로 확인된 사실에 근거한다.

## Prerequisites

- **Build Tool**: Cargo/rustc **1.97.1**(rustup 1.97.1 관리). Rust **edition 2024**, 워크스페이스 **MSRV 1.89**
  (단일 인스턴스 락이 std `File::try_lock` 를 쓰며, 이 API 는 1.89 에서 안정화됨).
- **Dependencies**: 워크스페이스 핀 크레이트 — `serde`/`serde_json`/`ciborium`/`url`/`thiserror`/`arc-swap`/
  `proptest`. 외부 런타임 의존: `sha2`(U1 해시), `notify`(U2 파일시스템 감시), `ureq`+`rustls`/`ring`(U5 전송),
  `ctrlc`(U8 신호 핸들러). 모두 `cargo build` 가 자동 해소한다(`Cargo.lock` 체크인됨).
- **Environment Variables**: 없음(빌드 시점). cargo/rustc/clippy 가 PATH 에 없으므로 아래 PATH export 필요.
- **System Requirements**: macOS/Linux/Windows. **검증 플랫폼은 macOS aarch64**(Apple Silicon). Windows/Linux
  전용 cfg arm 은 컴파일 대상이나 이 환경에서 실행 검증하지 않았다(빌드/테스트/clippy 는 macOS aarch64 기준).

## Build Steps

### 1. Configure Environment (툴체인 PATH)

cargo/rustc/clippy 는 PATH 에 노출돼 있지 않다. 세 도구가 모두 들어 있는 1.97.1 툴체인 bin 을 PATH 앞에 둔다
(clippy 는 1.97.1 툴체인에 존재):

```bash
export PATH="$HOME/.rustup/toolchains/1.97.1-aarch64-apple-darwin/bin:$PATH"
```

확인:

```bash
cargo --version    # cargo 1.97.1
rustc --version    # rustc 1.97.1
```

### 2. Install Dependencies

별도 설치 명령은 없다. 다음 빌드 단계에서 `Cargo.lock` 기준으로 자동 해소·컴파일된다. 오프라인 검증이 필요하면:

```bash
cargo fetch
```

### 3. Build All Units

워크스페이스 전체(10개 크레이트)를 한 번에 빌드한다:

```bash
cargo build --workspace
```

### 4. Verify Build Success

- **Expected Output**: 모든 크레이트 `Compiling ...` 후 `Finished` — 확인된 결과는 **성공(10개 크레이트 전부
  컴파일, 에러 0)**.
- **Build Artifacts**: 단일 배포 바이너리 `target/debug/watcher-bin`(RESILIENCY-01: 9개 라이브러리 크레이트를
  이 하나가 링크). 나머지 9개는 라이브러리 크레이트다.
- **크레이트 목록(빌드 대상)**: `foundation`(U0), `content-core`(U1), `change-detect`(U2), `upload-client`(U3),
  `sync-state`(U4), `auth-consent`(U5), `observability`(U6), `lifecycle-deploy`(U7a), `ops-control`(U7b),
  `watcher-bin`(U8, 바이너리).
- **Common Warnings**: 확인된 빌드에서 경고 없음. clippy `-D warnings` 게이트도 CLEAN(unit-test-instructions.md 참조).

## Troubleshooting

### Build Fails with `command not found: cargo`

- **Cause**: cargo/rustc 가 PATH 에 없음(이 환경 기본 상태).
- **Solution**: 위 "1. Configure Environment" 의 PATH export 를 먼저 실행한다.

### Build Fails with Edition/MSRV Errors

- **Cause**: edition 2024 또는 MSRV 1.89 미만 툴체인 사용(예: `File::try_lock` 미해결).
- **Solution**: 1.97.1 툴체인을 사용한다(위 PATH export). 워크스페이스 `rust-version` 은 1.89 로 고정돼 있으므로
  1.89+ 툴체인이면 컴파일된다.

### Build Fails with Dependency Errors

- **Cause**: 네트워크 부재 상태에서 미다운로드 크레이트 접근.
- **Solution**: 온라인에서 `cargo fetch` 로 선행 다운로드 후 재빌드. `Cargo.lock` 은 체크인돼 있어 버전은 재현된다.
