# ConfigProvider 애그리게이트 요약

**모듈**: `crates/foundation/src/config/` · **책임**: 단일 JSON config 로드 -> 2-pass 검증 -> lock-free 활성 스냅샷 -> 성공 스왑 직후 관찰자 팬아웃.
**공개 표면**: `ConfigProvider::new(known_keys: &[&str])`, `subscribe(&mut self, observer: Arc<dyn ConfigReloadObserver>)`, `load(&self, cli_path: Option<&Path>) -> Result<ConfigSnapshot, ConfigError>`, `current(&self) -> ConfigSnapshot`, `reload(&self) -> Result<(), ConfigError>`.

---

## 1. 값 모델 (`model.rs`)

- `WatcherConfig` — U0 소유 6개 core 필드:
  - `vault_path: String`(필수·비어있지 않은 절대경로), `server_endpoint: String`(필수·유효 URL·https-only), `token: Option<TokenSecret>`(`#[serde(default)]`, 존재 시 비-공백), `secure_store_enabled: bool`(default false), `log_level: LogLevel`(default `Info`), `notify_consecutive_failures: u32`(default 3, `>= 1`).
  - 타 단위 섹션(debounce_ms 등)은 필드가 아님(typed pass 에서 무시, `deny_unknown_fields` 미채택; 미지 키 판정은 Value 스캔이 담당).
  - Serialize/Deserialize 값-보존(PROP-BR-02).
- `ConfigSnapshot(Arc<WatcherConfig>)` — lock-free 읽기 스냅샷. `new`/`config()`/`into_arc()` + `Deref` -> `WatcherConfig`.
- `FOUNDATION_CONFIG_KEYS: &[&str]` — U0 6개 core 키(DEC-FEDERATED-KEYS, Q6=A). `watcher-bin` 이 하위 단위 키와 union 집계 -> `ConfigProvider::new(known_keys)` 주입. union 누락 시 R-CFG-STRICT-01 이 U0 자기 키를 unknown 으로 거부하므로 필수.
- R-LIMIT-01 컴파일타임 상수: `MAX_VAULT_TOTAL_BYTES`(20 GiB), `MAX_FILE_BYTES`(2 GiB), `MAX_FILE_COUNT`(100,000). 대응 config 키는 unknown 으로 거부(경계·단조성 검사는 U1).

---

## 2. 검증 체인

### `url_validator.rs` — UrlValidator (SEC-01)
- `validate_https_url(raw: &str) -> Result<Url, UrlValidationError>` — `url::Url::parse` 후 `scheme() == "https"` 강제(`starts_with` 아님). 성공 시 파싱된 `Url` 반환(U5 base-URL join 재사용).
- `UrlValidationError { Malformed(String), NotHttps(String) }`.

### `validator.rs` — ConfigValidator (2-pass, Q5=C)
- `validate(raw_json: &str, known_keys: &[&str]) -> Result<WatcherConfig, ConfigError>`:
  - **1st-pass**: `serde_json::Value` 파싱 -> object 키 중 `known_keys` 밖의 **모든** 미지 키를 전수 수집(중단 없음, 이름 + JSON pointer). 하나라도 있으면 즉시 `Err` 반환(typed pass 미수행).
  - **2nd-pass**: 미지 키 clean 시에만 `serde_json::from_value` typed deserialize -> 그 뒤 per-field 의미 검증을 필드 순서대로 수행해 **첫** 위반만 보고(bespoke full-field 누적 없음).
  - per-field(§4.1): `vault_path`(비어있지 않은 절대경로), `server_endpoint`(https URL, UrlValidator 위임), `token`(존재 시 비-공백), `notify_consecutive_failures`(>= 1; 0/음수/비정수는 typed pass 에서 거름).
- `ConfigError { issues: Vec<ConfigIssue> }` — thiserror. `new`/`single`/`issues()`/`report()`. 이슈 = 다수 미지-키 **또는** 단일 first-field 위반.
- `ConfigIssue { Parse, UnknownKey{key,pointer}, TypeError, FieldViolation{field,expected}, Discovery, Io{path,detail} }`.
- **realize**: R-CFG-STRICT-01, R-LIMIT-01, USE-01, MNT-01, PROP-BR-05.

### `loader.rs` — ConfigLoader (R-DISCOVER-01)
- `resolve_config_path(cli_path: Option<&Path>) -> PathBuf` — 우선순위 `--config` 플래그 > `OKC_WATCHER_CONFIG` env(비어있지 않을 때) > 플랫폼 기본 경로(Windows `%PROGRAMDATA%/okc-watcher/config.json`, 그 외 `/etc/okc-watcher/config.json`).
- `read_and_validate(path: &Path, known_keys: &[&str]) -> Result<WatcherConfig, ConfigError>` — 파일 읽기(실패 -> `ConfigIssue::Io`) 후 `validate` 위임.
- 파일 IO/env 접근하므로 순수 lint-gate 미적용. 주입된 known-key union 은 CONSUME 만(하위 단위 import 없음, DAG 루트).

---

## 3. 관찰자 레지스트리 (`observer.rs` — ObserverRegistry)

- 내부 저장 = **불변 순서** `Vec<Arc<dyn ConfigReloadObserver>>`(기동 시 고정, 런타임 변경/해제 없음 -> 결정적 순서 + registry 락 없음).
- `new()`/`Default`, `subscribe(&mut self, observer: Arc<dyn ConfigReloadObserver>)`(조립 단계 가변 참조), `notify_all(&self)`, `len`/`is_empty`.
- `notify_all` 은 등록 순서대로 `on_config_reload()` fan-out 하되, 각 호출을 `std::panic::catch_unwind(AssertUnwindSafe(...))` 로 격리 — 패닉 observer 가 나머지 K-1 통지나 활성 스냅샷을 오염시키지 못함(스왑은 팬아웃 이전 원자 커밋).
- fan-out 대상 매핑(R-OBSERVER-02): `token`/`secure_store_enabled` -> U5, `log_level` -> U6.
- **realize**: R-OBSERVER-01/02/03, R-RELOAD-05, REL-04.

---

## 4. Reload 동시성 모델 (`store.rs` — ConfigProvider)

핵심 구성:
- `active: ArcSwapOption<WatcherConfig>` — 활성 config 의 lock-free 스왑 셀(최초 load 전 비어 있음).
- `reload_mutex: Mutex<ReloadState>` — 쓰기 경로 직렬화 + 해소된 경로(`resolved_path`) 보관.
- `observers: ObserverRegistry`, `known_keys: Vec<String>`(주입된 union 을 소유 보관, `known_keys_refs()` 로 `&[&str]` 빌림).

동작:
- **`current()` = lock-free 읽기**: `active.load_full()` 로드, `reload_mutex` 미취득. 로드 전 호출(계약 위반)은 패닉 대신 검증 불가한 `unloaded_placeholder`(빈 필수 필드) 반환.
- **`load(cli_path)` = 최초 로드**: 경로 해소(R-DISCOVER-01) -> `read_and_validate` -> `Arc::new` -> `reload_mutex` 취득해 `resolved_path` 기억 + `active.store`. 성공 시 `ConfigSnapshot` 반환. 실패 시 `Err`(활성은 비어 있는 채) -> `watcher-bin` 이 non-zero exit abort(R-RELOAD-04). 최초 load 는 fan-out 미트리거.
- **`reload()` = 쓰기 경로 직렬화**: `reload_mutex` 임계구역 안에서 기억된 경로 재-읽기·검증 -> 성공 시에만 `active.store` 원자 스왑 -> 동일 구역에서 `observers.notify_all()` 순서 fan-out. 최초 load 미완료 시 `ConfigIssue::Discovery` 로 거부.
- **keep-last-good**(R-RELOAD-03/REL-05): 검증 실패 시 스왑 미도달 -> 이전 스냅샷 유지, `Err(ConfigError)` 만 표면화(부분 적용 없음).
- **뮤텍스 poison 회복**: `lock_reload()` 는 poison 시 패닉 대신 `into_inner()` 로 내부 상태 회수.
- **불변식**: 동시 reader 는 완료된 old 또는 완료된 new 스냅샷 중 정확히 하나만 관측(중간 상태 없음); 동일 유효 config 2회 적용 == 1회 관측(멱등, PROP-BR-03).

시퀀스: `reload -> lock(reload_mutex) -> read_and_validate -> (성공) active.store(swap) -> observers.notify_all(catch_unwind fan-out) -> unlock`.

- **realize**: R-RELOAD-01/02/03/04/05, R-TRIGGER-01(CLI `reload` 로만, SIGHUP/파일-watch 없음), R-OBSERVER-03, REL-03/REL-05. `ArcSwap` 상태 보유 모듈이므로 순수 clippy lint-gate 미적용.
