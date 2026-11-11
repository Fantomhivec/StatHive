# Auto-clean src-tauri/target: rustc leftovers and bloated debug cache.
# Does not touch sources. Keeps third-party debug deps for fast cargo check.
# ASCII-only (Windows PowerShell 5.1).
param(
    # Skip .tmp* newer than N minutes (parallel linker).
    [int]$MinAgeMinutes = 5,
    # Keep newest s-* session per incremental unit; also prune obsolete units.
    [switch]$PruneIncremental = $true,
    # After release pack: drop release/{deps,build,.fingerprint,incremental}.
    [switch]$DropReleaseIntermediates,
    # Remove artifacts from renamed crates (default: universal_rust).
    [switch]$DropRenamedLeftovers = $true,
    # Comma-separated old crate name stems (no lib- prefix).
    [string]$RenamedStems = "universal_rust,universal_rust_lib",
    # Keep newest CGU .o cohort for workspace crates; drop older hashes.
    [switch]$PruneStaleCgu = $true,
    # Workspace crate stems for CGU prune / hard drop (lib + bin).
    [string]$WorkspaceStems = "stathive,stathive_lib",
    # Hard reset: wipe all workspace + renamed artifacts under debug (deps keep third-party).
    [switch]$DropDebugWorkspaceArtifacts,
    # Cargo target root (default: src-tauri/target).
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

function Remove-PathFreeing([string]$path, [ref]$freedRef, [ref]$countRef) {
    if (-not (Test-Path -LiteralPath $path)) { return }
    $item = Get-Item -LiteralPath $path -Force -EA SilentlyContinue
    if (-not $item) { return }
    $size = if ($item.PSIsContainer) { Dir-Size $path } else { [int64]$item.Length }
    Remove-Item -LiteralPath $path -Recurse -Force
    if (-not (Test-Path -LiteralPath $path)) {
        $freedRef.Value += $size
        $countRef.Value++
    }
}

function Split-StemList([string]$raw) {
    @(
        $raw -split ',' |
            ForEach-Object { $_.Trim() } |
            Where-Object { $_ -ne '' }
    )
}

function Test-NameMatchesStem([string]$name, [string]$stem) {
    # Match crate, lib-prefixed, and rename leftovers like libuniversal_rust_lib.*.
    foreach ($c in @($stem, "lib$stem")) {
        if ($name -eq $c) { return $true }
        if ($name.StartsWith("$c.")) { return $true }
        if ($name.StartsWith("$c-")) { return $true }
        if ($name.StartsWith("${c}_")) { return $true }
    }
    return $false
}

function Get-IncrementalUnitStem([string]$dirName) {
    $idx = $dirName.LastIndexOf('-')
    if ($idx -le 0) { return $dirName }
    return $dirName.Substring(0, $idx)
}

$freed = 0L
$removedTmp = 0
$removedIncSessions = 0
$removedIncUnits = 0
$removedRenamed = 0
$removedCgu = 0
$removedRel = 0
$removedWs = 0

$renamedList = @(Split-StemList $RenamedStems)
$workspaceList = @(Split-StemList $WorkspaceStems)
# Hard drop also wipes renamed leftovers.
$dropStems = @($workspaceList + $renamedList | Select-Object -Unique)

# --- 1) .tmp* ---
$cutoff = (Get-Date).AddMinutes(-[Math]::Max(0, $MinAgeMinutes))
$tmpDirs = Get-ChildItem -Path $target -Recurse -Force -Directory -Filter ".tmp*" -EA SilentlyContinue |
    Where-Object { $_.LastWriteTime -le $cutoff }
foreach ($d in $tmpDirs) {
    Remove-PathFreeing $d.FullName ([ref]$freed) ([ref]$removedTmp)
}

# --- 2a) incremental: keep newest s-* per unit ---
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
                Remove-PathFreeing $old.FullName ([ref]$freed) ([ref]$removedIncSessions)
            }
        }
    }
}

# --- 2b) incremental: keep newest unit dir per crate stem ---
if ($PruneIncremental) {
    foreach ($prof in @("debug", "release")) {
        $incRoot = Join-Path $target "$prof\incremental"
        if (-not (Test-Path $incRoot)) { continue }
        $units = @(Get-ChildItem -LiteralPath $incRoot -Directory -Force -EA SilentlyContinue)
        $byStem = $units | Group-Object { Get-IncrementalUnitStem $_.Name }
        foreach ($g in $byStem) {
            if ($g.Count -le 1) { continue }
            $ordered = @($g.Group | Sort-Object LastWriteTime -Descending)
            foreach ($old in ($ordered | Select-Object -Skip 1)) {
                Remove-PathFreeing $old.FullName ([ref]$freed) ([ref]$removedIncUnits)
            }
        }
    }
}

# --- 3) renamed leftovers (files + incremental dirs) ---
if ($DropRenamedLeftovers -and $renamedList.Count -gt 0 -and -not $DropDebugWorkspaceArtifacts) {
    foreach ($prof in @("debug", "release")) {
        $profDir = Join-Path $target $prof
        if (-not (Test-Path $profDir)) { continue }
        $depsDir = Join-Path $profDir "deps"
        $scanDirs = @($profDir)
        if (Test-Path $depsDir) { $scanDirs += $depsDir }
        foreach ($dir in $scanDirs) {
            Get-ChildItem -LiteralPath $dir -Force -File -EA SilentlyContinue | ForEach-Object {
                foreach ($stem in $renamedList) {
                    if (Test-NameMatchesStem $_.Name $stem) {
                        Remove-PathFreeing $_.FullName ([ref]$freed) ([ref]$removedRenamed)
                        break
                    }
                }
            }
        }
        $incRoot = Join-Path $profDir "incremental"
        if (Test-Path $incRoot) {
            Get-ChildItem -LiteralPath $incRoot -Directory -Force -EA SilentlyContinue | ForEach-Object {
                foreach ($stem in $renamedList) {
                    if (Test-NameMatchesStem $_.Name $stem) {
                        Remove-PathFreeing $_.FullName ([ref]$freed) ([ref]$removedRenamed)
                        break
                    }
                }
            }
        }
    }
}

# --- 4) stale workspace CGU .o (keep newest hash cohort) ---
if ($PruneStaleCgu -and -not $DropDebugWorkspaceArtifacts) {
    $cguStems = @($workspaceList | Select-Object -Unique)
    foreach ($prof in @("debug")) {
        $depsDir = Join-Path $target "$prof\deps"
        if (-not (Test-Path $depsDir)) { continue }
        foreach ($stem in $cguStems) {
            $files = @(
                Get-ChildItem -LiteralPath $depsDir -Force -File -EA SilentlyContinue |
                    Where-Object {
                        $_.Name -like "$stem.*.rcgu.o" -or $_.Name -like "$stem.*.o"
                    }
            )
            if ($files.Count -eq 0) { continue }
            # Group by CGU metadata hash (second dotted segment).
            $groups = @{}
            foreach ($f in $files) {
                $parts = $f.Name.Split('.')
                if ($parts.Count -lt 3) { continue }
                $hash = $parts[1]
                if (-not $groups.ContainsKey($hash)) {
                    $groups[$hash] = @{
                        Files = New-Object System.Collections.Generic.List[object]
                        Latest = $f.LastWriteTime
                    }
                }
                $groups[$hash].Files.Add($f) | Out-Null
                if ($f.LastWriteTime -gt $groups[$hash].Latest) {
                    $groups[$hash].Latest = $f.LastWriteTime
                }
            }
            if ($groups.Count -le 1) { continue }
            $keepHash = (
                $groups.GetEnumerator() |
                    Sort-Object { $_.Value.Latest } -Descending |
                    Select-Object -First 1
            ).Key
            foreach ($entry in $groups.GetEnumerator()) {
                if ($entry.Key -eq $keepHash) { continue }
                foreach ($f in $entry.Value.Files) {
                    Remove-PathFreeing $f.FullName ([ref]$freed) ([ref]$removedCgu)
                }
            }
        }
    }
}

# --- 5) hard drop workspace + renamed under debug ---
if ($DropDebugWorkspaceArtifacts) {
    $profDir = Join-Path $target "debug"
    if (Test-Path $profDir) {
        $depsDir = Join-Path $profDir "deps"
        $scanDirs = @($profDir)
        if (Test-Path $depsDir) { $scanDirs += $depsDir }
        foreach ($dir in $scanDirs) {
            Get-ChildItem -LiteralPath $dir -Force -File -EA SilentlyContinue | ForEach-Object {
                foreach ($stem in $dropStems) {
                    if (Test-NameMatchesStem $_.Name $stem) {
                        Remove-PathFreeing $_.FullName ([ref]$freed) ([ref]$removedWs)
                        break
                    }
                }
            }
        }
        $incRoot = Join-Path $profDir "incremental"
        if (Test-Path $incRoot) {
            Get-ChildItem -LiteralPath $incRoot -Directory -Force -EA SilentlyContinue | ForEach-Object {
                foreach ($stem in $dropStems) {
                    if (Test-NameMatchesStem $_.Name $stem) {
                        Remove-PathFreeing $_.FullName ([ref]$freed) ([ref]$removedWs)
                        break
                    }
                }
            }
        }
    }
}

# --- 6) release intermediates (exe already copied to dist) ---
if ($DropReleaseIntermediates) {
    $rel = Join-Path $target "release"
    foreach ($name in @("deps", "build", "incremental", ".fingerprint")) {
        $p = Join-Path $rel $name
        if (-not (Test-Path $p)) { continue }
        Remove-PathFreeing $p ([ref]$freed) ([ref]$removedRel)
    }
}

if (-not $Quiet) {
    Write-Host (
        "clean-rust-target: tmp={0}, inc_sess={1}, inc_units={2}, renamed={3}, cgu={4}, ws={5}, release_dirs={6}, freed~{7:N0} MB" -f
        $removedTmp, $removedIncSessions, $removedIncUnits, $removedRenamed,
        $removedCgu, $removedWs, $removedRel, ($freed / 1MB)
    )
}
exit 0
