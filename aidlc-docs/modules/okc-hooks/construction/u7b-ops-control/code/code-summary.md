# U7b ops-control — Code Summary (구현 요약)

**단계**: CONSTRUCTION -> Per-Unit Loop -> **U7b Operations Control** -> Code Generation
**크레이트**: `ops-control` (lib) · **소속 컴포넌트**: `OperatorCli`, `ControlPlane`, `RunStateController`
**근거 산출물**: FD 3종(`domain-entities.md` / `business-rules.md` / `business-logic-model.md`)
**NFR 참고**: NFR 단계는 사용자 지시로 **SKIP** -> 코드는 FD + 하위/동위 크레이트(U0/U5/U6/U7a) 실제 공개 API 에 직접 근거한다.

---

## 1. 구현된 컴포넌트 + 공개 API 표면

세 개의 응집 컴포넌트를 노출한다. OS/소켓/프로세스/파일시스템 상호작용은 전부 seam 트레이트 뒤로 격리되어
특권·네트워크 없이 프로토콜/전이/라우팅 순수 로직을 결정적으로 검증한다.

- **`OperatorCli`**(모듈 `cli`): 고정 서브커맨드 집합(status/health/pause/resume/sync-now/stop/history/consent/
  reload/install/uninstall)을 std 로 손수 파싱하고 **라우팅 이분법**(R-U7B-01)으로 디스패치한다 — 데몬 대상은
  `Command -> ControlOp` 사상 후 `ControlClient`(IPC) 로 전송하고, install/uninstall 은 U7a `ServiceManager`/
  `Uninstaller` 를 in-process 로 직접 호출(IPC 우회)한다. `health` 는 종료코드로 전수·배타 매핑(`Healthy -> 0`,
  `Unhealthy -> 1`, 데몬 미도달/전송 -> `2`, R-U7B-04). 공개: `Command`/`CliArgs`/`CliResult`/`ExitCode`/`ServiceOps`/
  `NativeServiceOps`/`parse`/`run`/`dispatch_args`/`to_control_op`/`map_health`.
- **`ControlPlane`**(모듈 `control_plane`): 전송-불변 프로토콜 로직(디스패치 전 엄격 버전 검사 + op별 디스패치 테이블)을
  `ControlHandlers` seam 뒤에서 구동하며 accept 루프(`serve_forever`)/1-연결-1-요청 처리(`handle_conn`)를 포함한다.
  실 배선 `WatcherHandlers` 는 주입된 `Arc<StatusService>`/`Arc<UploadHistoryStore>`/`Arc<ConsentGate>`/
  `Arc<ConfigProvider>`/`Arc<RunStateController>` 의 **실제 공개 API 에만** 위임(R-U7B-08). 공개: `ControlPlane`/
  `ControlHandlers`/`WatcherHandlers`/`ControlClient`/`IpcControlClient`/`dispatch`/`handle_request`.
- **`RunStateController`**(모듈 `run_state`): 권위 run-state(running|paused + sync/stop 신호) 보유. `paused` 만
  crash-atomic 하게 재시작-지속(U4 파이프라인 미러: `encode -> temp -> fsync -> atomic rename -> parent dir fsync`)하고,
  pause/resume 성공 시 주입된 `Arc<dyn StatusSink>`(U6 `StatusService` 구현)에 `set_operational(Paused|Idle)` 반영
  (R-U7B-12/13). U8 이 `current()` 로 읽고 신호를 consume-and-clear 하며 컨트롤러는 U8 을 역참조하지 않는다.
  공개: `RunStateController`/`RunState`/`RunMode`/`PersistedRunState`/`RunStateError`.
- 프로토콜 표면(모듈 `protocol`, 순수 값): `ControlRequest`/`ControlOp`/`ControlResponse`/`ControlResult`/
  `ControlError`/`ConsentOp`/`ConsentView`/`HistoryFilter`/`StopMode`/`PROTO_VERSION`.
- IPC 경계(모듈 `ipc`): `IpcEndpoint`/`IpcError` + 3-트레이트 seam `IpcListener`/`IpcConnector`/`IpcStream` +
  프레이밍 헬퍼 `read_frame_from`/`write_frame_to`/`MAX_FRAME_BYTES` + 팩토리 `bind_native`/`native_connector`.

## 2. 모듈 레이아웃

- `protocol.rs` — 와이어 메시지 타입(단일 요청/응답 enum) + `HistoryFilter`/`ConsentView` 투영. 순수 값 모듈이라
  `#![deny(clippy::unwrap_used/expect_used/indexing_slicing/panic)]`.
- `ipc.rs` — 전송 seam + 8바이트 big-endian 길이-프리픽스 프레이밍(절단/과대/디코드 실패 -> 패닉 없이 `Protocol`) +
  `#[cfg(unix)]` UDS 구현(`UdsListener`/`UdsConnector`/`UdsStream`) + `#[cfg(not(unix))]` 스텁. 소켓 IO 라 순수 lint-gate 없음(단 패닉 경로 없음).
- `run_state.rs` — `RunStateController` + `std::fs` 원자 지속 파이프라인 + `Mutex` 임계구역(poison 시 회수). IO/상태 모듈.
- `control_plane.rs` — `dispatch`/`handle_request` 순수 로직 + `ControlPlane` 서버 루프 + `WatcherHandlers` 실 배선 + `IpcControlClient`.
- `cli.rs` — 손수 파서 + 라우팅 + health 종료코드 매핑 + `NativeServiceOps`(U7a in-process 조립). 예약어 `gen` 미사용.
- `tests/example_ops_control.rs` / `tests/prop_ops_control.rs` — 예제·property 테스트(§5).

## 3. 의존성

**신규 외부 크레이트 0건.** `foundation`(U0 코덱 `encode`/`decode` + `StatusSnapshot`/`Health`/`UploadHistoryRecord`/
`ConsentState`/`StatusSink`/`OperationalState`/`ConfigProvider`/`ReadJudgment` 소비), `observability`(U6
`StatusService`/`UploadHistoryStore`/`HistoryQuery`), `auth-consent`(U5 `ConsentGate`), `lifecycle-deploy`(U7a
`ServiceManager`/`Uninstaller`/`ServiceSpec`/`UninstallOptions`/`native_controller` 등)에만 의존한다 — U8(조립 루트)은
참조하지 않는다. IPC 는 `std::os::unix::net`(Unix 도메인 소켓)을 3-트레이트 seam 뒤에 두어 구현하며 Windows 명명
파이프는 동일 seam 뒤 스텁(이연, un-defer = `interprocess` 크레이트)이라 non-unix 에서도 컴파일된다. CLI 파싱은 std 로
손수 구현(clap 등 미도입). `thiserror`(오류 taxonomy), `serde`/`serde_json`(직렬화·`--json` 렌더), `proptest`(optional).
`proptest` 는 비-default feature(`proptest-support`) 게이트라 프로덕션 빌드 그래프에 유입되지 않는다(PBT-07).

## 4. MVP 트림 (FD 대비 편차 — 코드 주석에 명시)

- **합리적 편차**: U6 `HistoryQuery{since, only_failures}` 가 serde 를 구현하지 않아(동결 API) 와이어에는 동일 두 필드를
  실어 나르는 U7b 소유 `HistoryFilter` 로 투영하고, `WatcherHandlers.history` 가 디스패치 시 `HistoryQuery` 로 재조립한다.
- `ConsentGate.view()` 가 반환하는 U5 내부 타입(`ConsentLifecycle`/`ConsentGrant`) 결합을 피해 와이어에는
  `ConsentView{consent: ConsentState, acknowledged}` 두 필드로 투영(D-U7B-13).
- Windows 명명 파이프 미구현(스텁 커넥터는 항상 `Connect` 오류, 리스너는 `Bind` 오류).
- peer 자격증명 검사(SO_PEERCRED) 이연 -> 소유자-전용 디렉터리(0700) + 소켓 best-effort `chmod 0600` 이 1차 접근 통제(R-U7B-07).
- 1 연결 = 1 요청/응답 동기 처리(블로킹). `MAX_FRAME_BYTES` 는 고정 상수(8 MiB, config 화 이연).
- `--json` 렌더는 `serde_json`, 비-json 은 `Debug` 포맷 텍스트(D-U7B-14 해소; 얇은 뷰 미도입).
- 파싱/전송 실패는 종료코드 2 로 통합 매핑(별도 usage 코드 미도입).

## 5. 테스트 커버리지 (총 23개 통과, 0 실패)

- **예제/단위 테스트 12개**(`tests/example_ops_control.rs`, 기본 `cargo test`): run-state 전이+지속+`StatusSink` 반영
  (R-U7B-10/11/12/13), 휘발성 신호 미지속, 손상 상태 파일 `Running` 안전 회복, 프레임 round-trip, 절단 프레임 `Protocol`
  (패닉 없음), 서버 디스패치 round-trip(기록 핸들러), 버전 불일치 시 부수효과 0회(R-U7B-06), history/reload 도메인 오류의
  응답-프레임 내 전달(R-U7B-09), health->exit-code 전수, 라우팅 이분법(install=service / status=ipc).
- **property 테스트 11개**(`tests/prop_ops_control.rs`, `proptest-support` 게이트)로 PROP-U7B-01..07 를 커버:
  PROP-U7B-01(요청/응답 무손실 round-trip), PROP-U7B-02(임의 바이트 프레이밍 회복력 — 절단/손상에 패닉 없이 `Protocol`),
  PROP-U7B-03(버전 핸드셰이크 + 부작용 0회/1회), PROP-U7B-04(run-state 전이·멱등), PROP-U7B-05(paused-persist +
  휘발성 리셋 + 손상 안전 회복), PROP-U7B-06(health->exit-code 배타·전수), PROP-U7B-07(라우팅 이분법 배타·전수).

## 6. 검증 사실 (확인됨)

- 최초 빌드 성공 — 컴파일 버그 0건(first-try).
- U7b 테스트 23개(예제 12 + property 11) 전부 통과, 0 실패.
- `cargo clippy --all-targets --features proptest-support -- -D warnings` CLEAN.
