#!/usr/bin/env pwsh
[CmdletBinding()]
param(
    [Parameter(Mandatory = $true, Position = 0)]
    [string]$Target,
    [switch]$DryRun
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

function Write-Step {
    param([string]$Message)
    Write-Host "==> $Message" -ForegroundColor Cyan
}

function Parse-Semver {
    param([string]$VersionText)

    $normalized = $VersionText.Trim()
    if ($normalized.StartsWith("v")) {
        $normalized = $normalized.Substring(1)
    }

    if ($normalized -notmatch "^\d+\.\d+\.\d+$") {
        throw "Invalid version format '$VersionText'. Use major.minor.patch (example: 6.3.1)."
    }

    $parts = $normalized.Split(".")
    return [pscustomobject]@{
        Major = [int]$parts[0]
        Minor = [int]$parts[1]
        Patch = [int]$parts[2]
        Text  = $normalized
    }
}

function Get-NextVersion {
    param(
        [string]$CurrentVersion,
        [string]$RequestedTarget
    )

    $current = Parse-Semver -VersionText $CurrentVersion
    $targetLower = $RequestedTarget.Trim().ToLowerInvariant()

    switch ($targetLower) {
        "major" { return "$($current.Major + 1).0.0" }
        "minor" { return "$($current.Major).$($current.Minor + 1).0" }
        "patch" { return "$($current.Major).$($current.Minor).$($current.Patch + 1)" }
        default { return (Parse-Semver -VersionText $RequestedTarget).Text }
    }
}

function Write-Utf8NoBomFile {
    param(
        [string]$Path,
        [string]$Content
    )

    $encoding = New-Object System.Text.UTF8Encoding($false)
    [System.IO.File]::WriteAllText($Path, $Content, $encoding)
}

$scriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$repoRoot = Resolve-Path (Join-Path $scriptDir "..")

$packagePath = Join-Path $repoRoot "package.json"
$cargoPath = Join-Path $repoRoot "src-tauri/Cargo.toml"
$tauriConfigPath = Join-Path $repoRoot "src-tauri/tauri.conf.json"

if (-not (Test-Path $packagePath)) {
    throw "Missing file: package.json"
}
if (-not (Test-Path $cargoPath)) {
    throw "Missing file: src-tauri/Cargo.toml"
}
if (-not (Test-Path $tauriConfigPath)) {
    throw "Missing file: src-tauri/tauri.conf.json"
}

$packageJsonRaw = Get-Content -Raw $packagePath
$packageJson = $packageJsonRaw | ConvertFrom-Json
$currentVersion = [string]$packageJson.version

if ([string]::IsNullOrWhiteSpace($currentVersion)) {
    throw "Unable to read current version from package.json"
}

$newVersion = Get-NextVersion -CurrentVersion $currentVersion -RequestedTarget $Target

if ($newVersion -eq $currentVersion) {
    throw "New version is the same as current version ($currentVersion)."
}

Write-Step "Bumping version $currentVersion -> $newVersion"

$packageVersionRegex = [regex]'"version"\s*:\s*"[^"]+"'
$packageUpdated = $packageVersionRegex.Replace($packageJsonRaw, "`"version`": `"$newVersion`"", 1)
if ($packageUpdated -eq $packageJsonRaw) {
    throw "Failed to update version in package.json"
}

$cargoRaw = Get-Content -Raw $cargoPath
$cargoRegex = [regex]::new('(?ms)(^\[package\][\s\S]*?^\s*version\s*=\s*")([^"]+)(")')
$cargoUpdated = $cargoRegex.Replace(
    $cargoRaw,
    [System.Text.RegularExpressions.MatchEvaluator]{
        param($match)
        "$($match.Groups[1].Value)$newVersion$($match.Groups[3].Value)"
    },
    1
)
if ($cargoUpdated -eq $cargoRaw) {
    throw "Failed to update [package].version in src-tauri/Cargo.toml"
}

$tauriRaw = Get-Content -Raw $tauriConfigPath
$tauriVersionRegex = [regex]'"version"\s*:\s*"[^"]+"'
$tauriUpdated = $tauriVersionRegex.Replace($tauriRaw, "`"version`": `"$newVersion`"", 1)
if ($tauriUpdated -eq $tauriRaw) {
    throw "Failed to update version in src-tauri/tauri.conf.json"
}

if ($DryRun) {
    Write-Step "Dry run mode: no files were changed"
} else {
    Write-Utf8NoBomFile -Path $packagePath -Content $packageUpdated
    Write-Utf8NoBomFile -Path $cargoPath -Content $cargoUpdated
    Write-Utf8NoBomFile -Path $tauriConfigPath -Content $tauriUpdated
    Write-Step "Updated package.json, src-tauri/Cargo.toml, src-tauri/tauri.conf.json"
}

Write-Host "Current : $currentVersion"
Write-Host "Next    : $newVersion"
if ($DryRun) {
    Write-Host "Mode    : dry-run"
}
