#!/usr/bin/env pwsh
[CmdletBinding()]
param(
    [Parameter(ValueFromRemainingArguments = $true)]
    [string[]]$CargoArgs
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"
$ProgressPreference = "SilentlyContinue"

function Write-Step {
    param([string]$Message)
    Write-Host "`n==> $Message" -ForegroundColor Cyan
}

function Get-CleanCargoTargetRoot {
    if ($env:CARGO_TARGET_DIR) {
        return $null
    }

    $isWindows = $env:OS -eq "Windows_NT"
    if ($isWindows -and (Test-Path "D:\")) {
        return "D:\srptgt-test"
    }

    return Join-Path ([System.IO.Path]::GetTempPath()) "srptgt-test"
}

function Remove-StaleCargoTargets {
    param([string]$TargetRoot)

    if (-not (Test-Path -LiteralPath $TargetRoot)) {
        return
    }

    $cutoff = (Get-Date).AddDays(-3)
    Get-ChildItem -LiteralPath $TargetRoot -Directory -ErrorAction SilentlyContinue |
        Where-Object { $_.LastWriteTime -lt $cutoff } |
        ForEach-Object {
            Remove-Item -LiteralPath $_.FullName -Recurse -Force -ErrorAction SilentlyContinue
        }
}

$scriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$repoRoot = Resolve-Path (Join-Path $scriptDir "..")
Set-Location $repoRoot

$targetRoot = Get-CleanCargoTargetRoot
$usingEphemeralTarget = $null -ne $targetRoot

if ($usingEphemeralTarget) {
    New-Item -ItemType Directory -Path $targetRoot -Force | Out-Null
    Remove-StaleCargoTargets -TargetRoot $targetRoot

    $runId = "{0}-{1}" -f (Get-Date -Format "yyyyMMdd-HHmmss"), $PID
    $env:CARGO_TARGET_DIR = Join-Path $targetRoot $runId
    New-Item -ItemType Directory -Path $env:CARGO_TARGET_DIR -Force | Out-Null

    Write-Step "Using clean Cargo target dir $($env:CARGO_TARGET_DIR)"
}

$cargoArguments = @("test", "--manifest-path", "src-tauri/Cargo.toml")
if ($CargoArgs) {
    $cargoArguments += $CargoArgs
}

& cargo @cargoArguments
$exitCode = $LASTEXITCODE

if ($usingEphemeralTarget -and (Test-Path -LiteralPath $env:CARGO_TARGET_DIR)) {
    Remove-Item -LiteralPath $env:CARGO_TARGET_DIR -Recurse -Force -ErrorAction SilentlyContinue
}

exit $exitCode
