# 설치, 복구, 업데이트

이 제품의 운영은 MCP 호스트가 실행하는 로컬 프로세스와 패키지 관리다. 서버 포트,
AWS 계정, API 키, 자동 provider 호출은 필요하지 않다.

## 연결과 진단

README의 `config` → `doctor` → `client-config`를 수행한다.
Vault는 이미 존재하는 디렉터리여야 하며 config는 밖에 둔다.
`doctor`는 파일 목록을 확인하고 Vault나 없는 state 디렉터리를 만들지 않는다.
첫 쓰기 때 state가 만들어진다. stdout은 `serve`에서 MCP 전용이고 정상 동작의 노트 본문은 로그에 남기지 않는다.

`maxFiles`는 숨김 항목과 디렉터리를 포함한 방문 entry 수의 한도다.
`maxScanBytes`는 스캔한 일반 파일 크기의 합에 적용되므로 큰 PDF나 미디어도 스캔 한도에 영향을 준다.
노트별 기본값은 1 MiB, 스캔은 64 MiB, entry는 10,000이다. 100k/20GB Vault 성능은 검증하지 않았다.
한 번의 검색·audit은 현재 파일을 순회하며 Vault 전체의 원자적인 스냅샷을 만드는 동작이 아니다.

## 변경 전 백업

`replace_note`와 `patch_frontmatter` 적용은 기존 내용을 다음 위치에 저장하고 `backupId`를 반환한다.

```text
<statePath>/backups/<backupId>
<statePath>/locks/<vault-hash>.lock
```

백업은 이전 Markdown의 정확한 내용이다. 사용한 도구 결과의 `path`, `backupId`와 해시를
복구 시 함께 확인한다. 현재 백업 목록/자동 복원 도구나 별도 전체 버전 이력 DB는 없다.
백업 ID와 원래 노트 경로의 대응을 도구 결과에서 보존해야 한다.

되돌릴 때는 해당 노트의 편집과 MCP 쓰기를 멈추고 현재 버전도 별도로 보존한다.
백업 파일과 원래 경로를 확인하여 외부 편집기로 비교·복원하거나, 백업 내용을 검토한 뒤
현재 `read_note` 해시로 `replace_note`에 전달한다. 이전 해시를 재사용하지 않는다.
백업은 개인 자료이므로 공개 저장소에 올리지 않는다. 자동 만료/정리는 아직 없으므로
보존 정책에 따라 별도로 보관·정리한다.

## 비정상 종료와 잠금

동일한 canonical Vault와 **동일한 statePath**를 사용하는 MCP writer는 잠금으로 조정한다.
서로 다른 statePath나 Obsidian/Sync는 이 잠금을 공유하지 않는다.
오래된 것처럼 보이는 잠금도 서버가 자동 삭제하지 않는다.

`VAULT_BUSY`가 지속되면 관련 MCP 호스트와 편집을 멈추고 lock 파일의 `pid`, `startedAt`을 확인한다.
해당 writer가 종료되었고 다른 writer가 없는 것을 확인한 후에만 잔존 lock 파일을 수동으로 제거한다.
PID는 재사용될 수 있으므로 숫자만 보고 실행 중인 프로세스를 종료하지 않는다.
재연결 후 현재 노트와 backup을 확인하고 필요한 변경을 다시 검토한다.

파일 교체 중 비정상 종료로 `.okc-mcp-*.tmp`가 남을 수 있다. 모든 관련 writer를 멈춘 후
내용을 비교하고 필요한 복구가 끝나면 잔존 임시파일만 수동 정리한다.
아직 전원 차단·파일시스템 장애·모든 OS의 crash recovery를 보증하지 않는다.

## 업데이트와 제거

업데이트 전 새 버전의 CHANGELOG와 도구/설정 변경을 확인한다. 정확한 버전의 tarball을
설치하고 `doctor`, client-config를 다시 확인한 뒤 MCP 호스트를 재시작한다.
Node나 설치 위치가 바뀌면 생성한 절대 경로를 다시 등록한다. 서버는 시작 시 자동 업데이트하지 않는다.

패키지 되돌리기는 이전 tarball을 다시 설치하는 일이며 노트의 내용을 자동으로 되돌리지 않는다.
제거할 때는 호스트의 해당 MCP 항목을 제거하고 `npm uninstall -g okc-mcp`를 실행한다.
Vault와 config/state는 패키지 제거로 지워지지 않는다. 백업을 확인한 뒤 별도로 관리한다.

## 공개 릴리스 전 남은 검증

로컬 검사와 별도로 native macOS/Linux/Windows의 실제 CI 실행, 설치 UX 사용자 시험,
Obsidian/Sync 동시 편집·프로세스 중단 시험, 큰 Vault 측정이 필요하다.
CI 파일의 존재는 그 환경에서 통과했다는 증거가 아니다.
registry 패키지 이름/소유권과 배포 절차는 공개 후보 단계에서 확정하며 현재 공개 배포는 하지 않았다.
