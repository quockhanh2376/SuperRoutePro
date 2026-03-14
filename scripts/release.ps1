#!/usr/bin/env pwsh
[CmdletBinding()]
param(
    [string]$VersionTag = "",
    [switch]$SkipInstall
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"
$ProgressPreference = "SilentlyContinue"

function Write-Step {
    param([string]$Message)
    Write-Host "`n==> $Message" -ForegroundColor Cyan
}

$scriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$repoRoot = Resolve-Path (Join-Path $scriptDir "..")
Set-Location $repoRoot

Write-Step "Checking required tools"
$requiredTools = @("node", "npm", "rustc", "cargo")
foreach ($tool in $requiredTools) {
    if (-not (Get-Command $tool -ErrorAction SilentlyContinue)) {
        throw "Missing required tool: $tool"
    }
}

$package = Get-Content (Join-Path $repoRoot "package.json") -Raw | ConvertFrom-Json
$packageVersion = [string]$package.version
$tauriConfig = Get-Content (Join-Path $repoRoot "src-tauri/tauri.conf.json") -Raw | ConvertFrom-Json
$tauriVersion = [string]$tauriConfig.version
$cargoToml = Get-Content (Join-Path $repoRoot "src-tauri/Cargo.toml") -Raw
$cargoVersionMatch = [regex]::Match($cargoToml, '(?ms)^\[package\].*?^\s*version\s*=\s*"([^"]+)"')

if (-not $cargoVersionMatch.Success) {
    throw "Unable to read package version from src-tauri/Cargo.toml"
}

$cargoVersion = $cargoVersionMatch.Groups[1].Value

if ($packageVersion -ne $tauriVersion -or $packageVersion -ne $cargoVersion) {
    throw "Version mismatch detected. package.json=$packageVersion, tauri.conf.json=$tauriVersion, Cargo.toml=$cargoVersion"
}

if ([string]::IsNullOrWhiteSpace($VersionTag)) {
    $VersionTag = "v$packageVersion"
} elseif (-not $VersionTag.StartsWith("v")) {
    $VersionTag = "v$VersionTag"
}

$normalizedVersion = $VersionTag.Substring(1)
if ($normalizedVersion -ne $packageVersion) {
    Write-Host "Warning: package.json version is $packageVersion but release tag is $VersionTag" -ForegroundColor Yellow
}

$releaseDir = Join-Path $repoRoot "release-artifacts\$VersionTag"
New-Item -ItemType Directory -Path $releaseDir -Force | Out-Null

if (-not $SkipInstall) {
    Write-Step "Installing Node dependencies"
    npm ci
}

Write-Step "Building Tauri release bundles (NSIS)"
npm run tauri build

$nsisInstaller = Get-ChildItem (Join-Path $repoRoot "src-tauri/target/release/bundle/nsis/*-setup.exe") |
    Sort-Object LastWriteTime -Descending |
    Select-Object -First 1
$portableExe = Get-Item (Join-Path $repoRoot "src-tauri/target/release/SuperRoute.exe") -ErrorAction SilentlyContinue

if (-not $nsisInstaller) {
    throw "NSIS installer was not found under src-tauri/target/release/bundle/nsis."
}
if (-not $portableExe) {
    throw "Portable executable was not found at src-tauri/target/release/SuperRoute.exe."
}

Write-Step "Collecting release artifacts in $releaseDir"
$copiedNsis = Copy-Item $nsisInstaller.FullName -Destination $releaseDir -Force -PassThru
$copiedExe = Copy-Item $portableExe.FullName -Destination $releaseDir -Force -PassThru

$checksumFile = Join-Path $releaseDir "SHA256SUMS.txt"
$checksums = @($copiedNsis, $copiedExe) | ForEach-Object {
    $hash = (Get-FileHash -Path $_.FullName -Algorithm SHA256).Hash.ToLowerInvariant()
    "$hash  $($_.Name)"
}
Set-Content -Path $checksumFile -Value $checksums -Encoding ascii

Write-Step "Release artifacts are ready"
Write-Host "Version Tag : $VersionTag" -ForegroundColor Green
Write-Host "Output Dir  : $releaseDir" -ForegroundColor Green
Write-Host "Artifacts   :" -ForegroundColor Green
Write-Host "  - $($copiedNsis.Name)"
Write-Host "  - $($copiedExe.Name)"
Write-Host "  - $(Split-Path -Leaf $checksumFile)"
