# One-command local installer for okc-hooks + okc-mcp (Windows / PowerShell).
#
# Fill okc-install.config.json (copy from okc-install.config.example.json), then run:
#   ./install.ps1                          # uses ./okc-install.config.json
#   ./install.ps1 -Config C:\path\cfg.json # or an explicit path
#   $env:SKIP_BUILD=1; ./install.ps1       # skip the cargo/npm build step
#
# Renders your one combined config into per-module configs, builds each module, and runs
# each module's `setup` (hooks auto-start service via SCM; okc-mcp into Claude Code / Codex).
# Tokens live only in 0600-equivalent temp files that are deleted on exit.
param([string]$Config)

$ErrorActionPreference = 'Stop'
$Root = Split-Path -Parent $MyInvocation.MyCommand.Path
if (-not $Config) { $Config = Join-Path $Root 'okc-install.config.json' }
$Example = Join-Path $Root 'okc-install.config.example.json'

function Log($m)  { Write-Host "[okc-install] $m" -ForegroundColor Blue }
function Fail($m) { Write-Host "[okc-install] $m" -ForegroundColor Red; exit 1 }

if (-not (Test-Path $Config)) {
  Fail "설정 파일이 없습니다: $Config`n  먼저 예제를 복사해 값을 채우세요:`n    Copy-Item `"$Example`" `"$(Join-Path $Root 'okc-install.config.json')`"`n  그런 다음 다시 ./install.ps1 를 실행하세요."
}
if (-not (Get-Command node -ErrorAction SilentlyContinue)) { Fail "node(>=22) 가 필요합니다." }

$Tmp = Join-Path ([System.IO.Path]::GetTempPath()) ("okc-install." + [System.IO.Path]::GetRandomFileName())
New-Item -ItemType Directory -Path $Tmp | Out-Null
try {
  Log "설정을 모듈별 config 로 변환 중…"
  & node (Join-Path $Root 'scripts/okc-install-render.mjs') $Config $Tmp
  if ($LASTEXITCODE -ne 0) { Fail "설정 변환 실패." }

  $installed = 0
  $skipBuild = $env:SKIP_BUILD -eq '1'

  $hooksCfg = Join-Path $Tmp 'hooks.json'
  if (Test-Path $hooksCfg) {
    if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) { Fail "okc-hooks 설치에는 cargo(Rust) 가 필요합니다." }
    if (-not $skipBuild) { Log "okc-hooks 빌드 중 (cargo build --release)…"; Push-Location (Join-Path $Root 'okc-hooks'); & cargo build --release; Pop-Location }
    $bin = Join-Path $Root 'okc-hooks/target/release/watcher-bin.exe'
    if (-not (Test-Path $bin)) { Fail "watcher-bin.exe 가 없습니다: $bin" }
    Log "okc-hooks 설치 중 (자동시작 서비스 등록)…"
    & $bin setup --config $hooksCfg
    $installed++
  }

  $mcpCfg = Join-Path $Tmp 'mcp.json'
  if (Test-Path $mcpCfg) {
    if (-not (Get-Command npm -ErrorAction SilentlyContinue)) { Fail "okc-mcp 설치에는 npm 이 필요합니다." }
    if (-not $skipBuild) { Log "okc-mcp 빌드 중 (npm ci && npm run build)…"; Push-Location (Join-Path $Root 'okc-mcp'); & npm ci; & npm run build; Pop-Location }
    $cli = Join-Path $Root 'okc-mcp/dist/cli.js'
    if (-not (Test-Path $cli)) { Fail "dist/cli.js 가 없습니다: $cli" }
    Log "okc-mcp 설치 중 (coding agent 등록)…"
    & node $cli setup --config $mcpCfg
    $installed++
  }

  if ($installed -eq 0) { Fail "설치할 모듈이 없습니다 (둘 다 enabled=false 입니까?)." }
  Log "완료! $installed 개 모듈 설치가 끝났습니다."
}
finally {
  Remove-Item -Recurse -Force $Tmp -ErrorAction SilentlyContinue
}
