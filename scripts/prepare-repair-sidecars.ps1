#!/usr/bin/env pwsh
[CmdletBinding()]
param()

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"
$ProgressPreference = "SilentlyContinue"

function Get-HostTargetTriple {
    $rustVersion = & rustc -vV
    $hostLine = $rustVersion | Where-Object { $_ -like "host: *" } | Select-Object -First 1
    if (-not $hostLine) {
        throw "Unable to determine rust host target triple from 'rustc -vV'."
    }

    return ($hostLine -replace '^host:\s*', '').Trim()
}

$scriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$repoRoot = Resolve-Path (Join-Path $scriptDir "..")
$tauriDir = Join-Path $repoRoot "src-tauri"
$targetTriple = if ($env:TAURI_ENV_ARCH -and $env:TAURI_ENV_PLATFORM -and $env:TAURI_ENV_FAMILY) {
    Get-HostTargetTriple
} else {
    Get-HostTargetTriple
}

$binariesDir = Join-Path $tauriDir "binaries"
New-Item -ItemType Directory -Path $binariesDir -Force | Out-Null

Write-Host "Preparing repair sidecars for target $targetTriple" -ForegroundColor Cyan

$bins = @("SuperRouteRepairBroker", "SuperRouteService")
foreach ($bin in $bins) {
    & cargo build --manifest-path (Join-Path $tauriDir "Cargo.toml") --release --target $targetTriple --bin $bin

    $sourcePath = Join-Path $tauriDir "target\$targetTriple\release\$bin.exe"
    if (-not (Test-Path $sourcePath)) {
        throw "Expected sidecar binary was not built: $sourcePath"
    }

    $destPath = Join-Path $binariesDir "$bin-$targetTriple.exe"
    Copy-Item $sourcePath -Destination $destPath -Force
}
