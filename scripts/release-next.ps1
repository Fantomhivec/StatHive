# StatHive release:
#   1) bump version in all version files
#   2) commit with message = vX.Y.Z.N
#   3) create annotated tag vX.Y.Z.N
#   4) push branch + tag -> GitHub Actions Release (on tags v*)
#
# Format: v0.0.0.1 / v0.0.0.199 / v100.2235.626.2626
# Examples:
#   .\scripts\release-next.ps1
#   .\scripts\release-next.ps1 0.0.0.5
#   .\scripts\release-next.ps1 -Yes
# ASCII-only (Windows PowerShell 5.1 + UTF-8 without BOM breaks quotes).
param(
  [string]$Version = "",
  [switch]$Yes
)

$ErrorActionPreference = "Stop"
$root = Split-Path $PSScriptRoot -Parent
Set-Location $root

function Get-CurrentVersion {
  $path = Join-Path $root "VERSION"
  if (Test-Path $path) {
    return (Get-Content $path -Raw).Trim().TrimStart('v', 'V').Trim()
  }
  $pkg = Get-Content (Join-Path $root "package.json") -Raw | ConvertFrom-Json
  return [string]$pkg.version
}

function Get-NextVersion([string]$ver) {
  $parts = @($ver.Split('.') | ForEach-Object { [int]$_ })
  if ($parts.Count -eq 0) { return "0.0.0.1" }
  if ($parts.Count -eq 3) { $parts += 0 }
  $parts[$parts.Count - 1] = $parts[$parts.Count - 1] + 1
  return ($parts -join '.')
}

function Normalize-AppVersion([string]$raw) {
  $v = $raw.Trim().TrimStart('v', 'V').Trim()
  if ($v -notmatch '^\d+(\.\d+)*$') {
    throw ("Version: digits and dots only (0.0.0.1). Got: {0}" -f $raw)
  }
  return $v
}

# --- remote / branch ---
$remote = git remote get-url origin 2>$null
if (-not $remote) {
  throw "No git remote 'origin'. Add GitHub remote first."
}
$branch = (git rev-parse --abbrev-ref HEAD).Trim()
if ($branch -eq "HEAD") {
  throw "Detached HEAD - checkout a branch (e.g. master) before release."
}

$current = Get-CurrentVersion
$defaultNext = Get-NextVersion $current

if ([string]::IsNullOrWhiteSpace($Version)) {
  Write-Host ""
  Write-Host ("Current version:  v{0}" -f $current)
  Write-Host ("Default next:     v{0}" -f $defaultNext)
  Write-Host ""
  $entered = Read-Host ("New version (Enter = {0})" -f $defaultNext)
  if ([string]::IsNullOrWhiteSpace($entered)) {
    $Version = $defaultNext
  } else {
    $Version = Normalize-AppVersion $entered
  }
} else {
  $Version = Normalize-AppVersion $Version
}

$tag = "v$Version"
$msg = "v$Version"

Write-Host ""
Write-Host "=== StatHive release ==="
Write-Host ("  Branch:   {0}" -f $branch)
Write-Host ("  Version:  {0}" -f $Version)
Write-Host ("  Commit:   {0}" -f $msg)
Write-Host ("  Tag:      {0}  (triggers GitHub Actions)" -f $tag)
Write-Host "  Files:    VERSION, package.json, package-lock.json,"
Write-Host "            Cargo.toml, Cargo.lock, tauri.conf.json"
Write-Host "  Secrets:  none required (public OAuth client is local AppData)"
Write-Host ""

if (-not $Yes) {
  $ans = Read-Host "Continue? [Y/N]"
  if ($ans -notmatch '^[YyDd]') {
    Write-Host "Cancelled."
    exit 0
  }
}

# Working tree must be clean except version files (release commit = version only)
$versionFiles = @(
  "VERSION",
  "package.json",
  "package-lock.json",
  "src-tauri/Cargo.toml",
  "src-tauri/Cargo.lock",
  "src-tauri/tauri.conf.json"
)
$status = git status --porcelain
foreach ($line in ($status -split "`n")) {
  $line = $line.TrimEnd()
  if ([string]::IsNullOrWhiteSpace($line)) { continue }
  $path = $line.Substring(2).Trim().Trim('"')
  # rename: old -> new
  if ($path -match ' -> ') { $path = ($path -split ' -> ')[-1].Trim() }
  $ok = $false
  foreach ($a in $versionFiles) {
    if ($path -eq $a -or $path -eq ($a -replace '/', '\')) { $ok = $true; break }
  }
  if (-not $ok) {
    throw (@"
Uncommitted changes outside version files: {0}

Commit or stash feature work first, then run release-next again.
Release commit should contain only version bump.
Secrets are embedded in GitHub Actions, not committed.
"@ -f $path)
  }
}

$existingTag = git tag -l $tag
if ($existingTag) {
  throw ("Tag {0} already exists locally. Choose another version." -f $tag)
}

# 1) bump versions everywhere
& (Join-Path $PSScriptRoot "set-version.ps1") $Version
if ($LASTEXITCODE -ne 0) { throw "set-version.ps1 failed" }

# 2) stage + commit with version message
git add -- VERSION package.json package-lock.json `
  src-tauri/Cargo.toml src-tauri/Cargo.lock src-tauri/tauri.conf.json

$staged = git diff --cached --name-only
if (-not $staged) {
  throw ("Nothing to commit - version files already at {0}?" -f $Version)
}

git commit -m $msg
if ($LASTEXITCODE -ne 0) { throw "git commit failed" }

# 3) annotated tag (message = version)
git tag -a $tag -m $msg
if ($LASTEXITCODE -ne 0) { throw "git tag failed" }

# 4) push branch then tag -> Actions on tags v*
Write-Host ""
Write-Host ("Pushing {0} and tag {1} to origin..." -f $branch, $tag)
git push origin ("HEAD:refs/heads/{0}" -f $branch)
if ($LASTEXITCODE -ne 0) { throw "git push branch failed" }
git push origin $tag
if ($LASTEXITCODE -ne 0) { throw "git push tag failed - Actions may not start" }

$slug = $remote -replace '.*github.com[:/]', '' -replace '\.git$', ''
Write-Host ""
Write-Host "Done."
Write-Host ("  Commit:  {0}" -f $msg)
Write-Host ("  Tag:     {0}" -f $tag)
Write-Host ("  Actions: https://github.com/{0}/actions" -f $slug)
Write-Host ("  Release: https://github.com/{0}/releases/tag/{1}" -f $slug, $tag)
