# Sync app version everywhere.
# Source of truth: root VERSION (0.0.0.1 - any number of segments).
# Cargo/npm/tauri/Cargo.lock get first 3 numbers only (tooling semver).
# Example: .\scripts\set-version.ps1 0.0.0.1
# ASCII-only (Windows PowerShell 5.1 + UTF-8 without BOM breaks quotes).
param(
  [Parameter(Mandatory = $true)]
  [string]$Version
)

$ErrorActionPreference = "Stop"

function Normalize-AppVersion([string]$raw) {
  $v = $raw.Trim().TrimStart('v', 'V').Trim()
  if ($v -notmatch '^\d+(\.\d+)*$') {
    throw "Version must be digits and dots, e.g. 0.0.0.1 (got: $raw)"
  }
  return $v
}

function Get-CargoSemver([string]$appVer) {
  $parts = $appVer.Split('.')
  while ($parts.Count -lt 3) { $parts += '0' }
  return "$($parts[0]).$($parts[1]).$($parts[2])"
}

function Write-Utf8NoBom([string]$path, [string]$text) {
  $utf8 = New-Object System.Text.UTF8Encoding $false
  [System.IO.File]::WriteAllText($path, $text, $utf8)
}

# IMPORTANT: [regex]::Replace(..., 1) is RegexOptions, NOT count=1.
# Use instance .Replace($s, $repl, $count) to limit replacements.
function Replace-Once([string]$text, [string]$pattern, [string]$replacement) {
  $re = New-Object System.Text.RegularExpressions.Regex($pattern)
  return $re.Replace($text, $replacement, 1)
}

$Version = Normalize-AppVersion $Version
$cargoVer = Get-CargoSemver $Version

$root = Split-Path $PSScriptRoot -Parent
Set-Location $root

# 1) VERSION - full app version (UI + update check)
Write-Utf8NoBom (Join-Path $root "VERSION") ($Version + "`n")

# 2) package.json - only root "version"
$pkgPath = Join-Path $root "package.json"
$pkgRaw = [System.IO.File]::ReadAllText($pkgPath)
$pkgRaw = Replace-Once $pkgRaw '(?m)^(\s*"version"\s*:\s*")[^"]+' "`${1}$cargoVer"
Write-Utf8NoBom $pkgPath $pkgRaw

# 3) package-lock.json - ONLY root version + packages[""].version (never deps!)
$lockPath = Join-Path $root "package-lock.json"
if (Test-Path $lockPath) {
  $lockRaw = [System.IO.File]::ReadAllText($lockPath)
  # Top-level "version" right after "name"
  $lockRaw = Replace-Once $lockRaw '(?m)^(\s*"name"\s*:\s*"stathive"\s*,\s*\r?\n\s*"version"\s*:\s*")[^"]+' "`${1}$cargoVer"
  # packages[""].version
  $lockRaw = Replace-Once $lockRaw '(?s)("packages"\s*:\s*\{\s*""\s*:\s*\{[^}]*?"version"\s*:\s*")[^"]+' "`${1}$cargoVer"
  Write-Utf8NoBom $lockPath $lockRaw
}

# 4) Cargo.toml [package] version (first version= only)
$cargoPath = Join-Path $root "src-tauri\Cargo.toml"
$cargo = [System.IO.File]::ReadAllText($cargoPath)
$cargo = Replace-Once $cargo '(?m)^version\s*=\s*"[^"]+"' "version = `"$cargoVer`""
Write-Utf8NoBom $cargoPath $cargo

# 5) Cargo.lock - only local package stathive
$cargoLockPath = Join-Path $root "src-tauri\Cargo.lock"
if (Test-Path $cargoLockPath) {
  $lock = [System.IO.File]::ReadAllText($cargoLockPath)
  $lock = Replace-Once $lock '(?m)(name = "stathive"\r?\n)version = "[^"]+"' "`${1}version = `"$cargoVer`""
  Write-Utf8NoBom $cargoLockPath $lock
}

# 6) tauri.conf.json - only product version field
$tauriPath = Join-Path $root "src-tauri\tauri.conf.json"
$tauriRaw = [System.IO.File]::ReadAllText($tauriPath)
$tauriRaw = Replace-Once $tauriRaw '(?m)^(\s*"version"\s*:\s*")[^"]+' "`${1}$cargoVer"
Write-Utf8NoBom $tauriPath $tauriRaw

Write-Host "Updated:"
Write-Host "  VERSION              = $Version"
Write-Host "  package.json         = $cargoVer"
Write-Host "  package-lock.json    = $cargoVer (root only)"
Write-Host "  Cargo.toml           = $cargoVer"
Write-Host "  Cargo.lock           = $cargoVer (stathive only)"
Write-Host "  tauri.conf.json      = $cargoVer"
Write-Host "  git tag / commit     = v$Version"
