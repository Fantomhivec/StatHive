# Local release build: main (tauri) + updater -> dist-release/
# thin LTO: first Windows release often ~5-15 min; later builds faster.
#
# OOM/stack overflow with parallel rustc (opt-level=3 + thin LTO):
# default CARGO_BUILD_JOBS=1. Faster: $env:UR_CARGO_JOBS=2
# Keep this file ASCII-only (Windows PowerShell 5.1 + UTF-8 without BOM breaks quotes).
$ErrorActionPreference = "Stop"
Set-Location $PSScriptRoot

function Step([string]$title) {
  Write-Host ""
  Write-Host "============================================================"
  Write-Host " $(Get-Date -Format 'HH:mm:ss')  $title"
  Write-Host "============================================================"
}

function Get-ReleaseCargoJobs {
  if ($env:UR_CARGO_JOBS -match '^\d+$' -and [int]$env:UR_CARGO_JOBS -ge 1) {
    return [int]$env:UR_CARGO_JOBS
  }
  # Windows + release LLVM: many parallel rustc -> OOM / STACK_OVERFLOW.
  # jobs=1 is safe on 8-16 GB RAM; stronger machines: UR_CARGO_JOBS=2..4
  return 1
}

function Set-ReleaseCargoJobs([int]$jobs) {
  $env:CARGO_BUILD_JOBS = "$jobs"
  Write-Host ("CARGO_BUILD_JOBS={0}  (override: `$env:UR_CARGO_JOBS=N)" -f $jobs)
}

function Invoke-PruneReleaseTarget {
  param([string]$TargetDir)
  Write-Host "Prune release intermediates (free RAM/disk before retry)..."
  & (Join-Path $PSScriptRoot "scripts\clean-rust-tmp.ps1") `
    -TargetDir $TargetDir -MinAgeMinutes 0 -DropReleaseIntermediates
}

# Quiet npm under PowerShell (no spinner that looks "stuck").
# Do NOT set CI=1 - Tauri CLI interprets it as --ci=1 and fails (wants true/false).
$env:npm_config_progress = "false"
$env:npm_config_fund = "false"
$env:npm_config_audit = "false"
Remove-Item Env:CI -ErrorAction SilentlyContinue
# If CARGO_TARGET_DIR points outside the repo, pack/clean may pick a stale exe.
# Local release always uses src-tauri/target under the repo.
Remove-Item Env:CARGO_TARGET_DIR -ErrorAction SilentlyContinue
$cargoTarget = Join-Path $PSScriptRoot "src-tauri\target"

$jobs = Get-ReleaseCargoJobs
Set-ReleaseCargoJobs $jobs

Step "[1/4] npm ci  (dependencies)"
Write-Host "If this hangs >2 min - check network / antivirus on node_modules."
npm ci --no-fund --no-audit --loglevel=info
if ($LASTEXITCODE -ne 0) { throw "npm ci failed (exit $LASTEXITCODE)" }

Step "[2/4] tauri build  (frontend + Rust release)"
Write-Host ("First time often 5-15 minutes (Rust thin LTO, jobs={0}). Please wait." -f $jobs)
npm run tauri build
if ($LASTEXITCODE -ne 0) {
  Write-Host ""
  Write-Host ("tauri build failed (exit {0}) - typical OOM with parallel rustc." -f $LASTEXITCODE)
  Write-Host "Retry: prune release target + CARGO_BUILD_JOBS=1"
  Invoke-PruneReleaseTarget -TargetDir $cargoTarget
  Set-ReleaseCargoJobs 1
  npm run tauri build
  if ($LASTEXITCODE -ne 0) { throw "tauri build failed after retry (exit $LASTEXITCODE)" }
}

Step "[3/4] cargo build -p stathive_updater --release"
# Separate updater/ crate: no Tauri; icon icons/updater.ico (winresource).
Push-Location src-tauri
cargo build -p stathive_updater --release
$cargoExit = $LASTEXITCODE
if ($cargoExit -ne 0) {
  Write-Host "updater failed - retry after prune, jobs=1"
  Pop-Location
  Invoke-PruneReleaseTarget -TargetDir $cargoTarget
  Set-ReleaseCargoJobs 1
  Push-Location src-tauri
  cargo build -p stathive_updater --release
  $cargoExit = $LASTEXITCODE
}
Pop-Location
if ($cargoExit -ne 0) { throw "updater build failed (exit $cargoExit)" }

Step "[4/4] pack dist-release + zip"
$out = Join-Path $PSScriptRoot "dist-release"
$relDir = Join-Path $cargoTarget "release"
New-Item -ItemType Directory -Force -Path $out | Out-Null
$main = Get-ChildItem -LiteralPath $relDir -Filter "stathive.exe" -File -EA SilentlyContinue |
  Select-Object -First 1
if (-not $main) {
  $main = Get-ChildItem -Recurse -LiteralPath $relDir -Filter "stathive.exe" -File -EA SilentlyContinue |
    Where-Object { $_.FullName -notmatch '\\deps\\' } |
    Select-Object -First 1
}
if (-not $main) { throw "stathive.exe not found in $relDir" }
$upd = Join-Path $relDir "stathive_updater.exe"
if (-not (Test-Path -LiteralPath $upd)) { throw "stathive_updater.exe not found in $relDir" }
Copy-Item $main.FullName (Join-Path $out "StatHive.exe") -Force
Copy-Item $upd (Join-Path $out "StatHive.Updater.exe") -Force
$zip = Join-Path $PSScriptRoot "StatHive-win-x64.zip"
if (Test-Path $zip) { Remove-Item $zip -Force }
Compress-Archive -Path (Join-Path $out "*") -DestinationPath $zip -Force

Step "clean target leftovers (tmp + incremental + release deps)"
& (Join-Path $PSScriptRoot "scripts\clean-rust-tmp.ps1") `
  -TargetDir $cargoTarget -MinAgeMinutes 0 -PruneIncremental -DropReleaseIntermediates

Write-Host ""
Write-Host "OK:"
Write-Host ("  {0}\StatHive.exe" -f $out)
Write-Host ("  {0}\StatHive.Updater.exe" -f $out)
Write-Host ("  {0}" -f $zip)
