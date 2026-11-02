# Автоочистка src-tauri/target: хвосты rustc и лишний кэш.
# Не трогает исходники; deps debug оставляем (нужны для быстрых cargo check).
param(
    # Не трогать .tmp* новее N минут (параллельная линковка).
    [int]$MinAgeMinutes = 5,
    # Оставить только последнюю incremental-сессию на crate (режет debug/incremental).
    [switch]$PruneIncremental = $true,
    # После release-упаковки: снести release/{deps,build,.fingerprint,incremental}.
    [switch]$DropReleaseIntermediates,
    # Корень cargo target (по умолчанию src-tauri/target).
    [string]$TargetDir = "",
    [switch]$Quiet
)

$ErrorActionPreference = "SilentlyContinue"
$root = Split-Path $PSScriptRoot -Parent
$target = if ($TargetDir) { $TargetDir } else { Join-Path $root "src-tauri\target" }
if (-not (Test-Path $target)) {
    if (-not $Quiet) { Write-Host "clean-rust-target: no target/" }
    exit 0
}

function Dir-Size([string]$path) {
    $sum = 0L
    if (-not (Test-Path -LiteralPath $path)) { return 0L }
    Get-ChildItem -LiteralPath $path -Recurse -Force -File -EA SilentlyContinue |
        ForEach-Object { $sum += $_.Length }
    return $sum
}

$freed = 0L
$removedTmp = 0
$removedInc = 0
$removedRel = 0

# --- 1) .tmp*.temp-archive ---
$cutoff = (Get-Date).AddMinutes(-[Math]::Max(0, $MinAgeMinutes))
$tmpDirs = Get-ChildItem -Path $target -Recurse -Force -Directory -Filter ".tmp*" -EA SilentlyContinue |
    Where-Object { $_.LastWriteTime -le $cutoff }
foreach ($d in $tmpDirs) {
    $size = Dir-Size $d.FullName
    Remove-Item -LiteralPath $d.FullName -Recurse -Force
    if (-not (Test-Path -LiteralPath $d.FullName)) {
        $freed += $size
        $removedTmp++
    }
}

# --- 2) incremental: keep newest s-* per unit ---
if ($PruneIncremental) {
    foreach ($prof in @("debug", "release")) {
        $incRoot = Join-Path $target "$prof\incremental"
        if (-not (Test-Path $incRoot)) { continue }
        Get-ChildItem -LiteralPath $incRoot -Directory -Force -EA SilentlyContinue | ForEach-Object {
            $sessions = @(
                Get-ChildItem -LiteralPath $_.FullName -Directory -Force -EA SilentlyContinue |
                    Where-Object { $_.Name -like "s-*" } |
                    Sort-Object LastWriteTime -Descending
            )
            if ($sessions.Count -le 1) { return }
            foreach ($old in ($sessions | Select-Object -Skip 1)) {
                $size = Dir-Size $old.FullName
                Remove-Item -LiteralPath $old.FullName -Recurse -Force
                if (-not (Test-Path -LiteralPath $old.FullName)) {
                    $freed += $size
                    $removedInc++
                }
            }
        }
    }
}

# --- 3) release intermediates (exe уже скопирован в dist) ---
if ($DropReleaseIntermediates) {
    $rel = Join-Path $target "release"
    foreach ($name in @("deps", "build", "incremental", ".fingerprint")) {
        $p = Join-Path $rel $name
        if (-not (Test-Path $p)) { continue }
        $size = Dir-Size $p
        Remove-Item -LiteralPath $p -Recurse -Force
        if (-not (Test-Path -LiteralPath $p)) {
            $freed += $size
            $removedRel++
        }
    }
}

if (-not $Quiet) {
    Write-Host (
        "clean-rust-target: tmp={0}, inc_sessions={1}, release_dirs={2}, freed~{3:N0} MB" -f
        $removedTmp, $removedInc, $removedRel, ($freed / 1MB)
    )
}
exit 0
