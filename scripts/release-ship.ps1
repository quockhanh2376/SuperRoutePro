#!/usr/bin/env pwsh
[CmdletBinding()]
param(
    [Parameter(Mandatory = $true, Position = 0)]
    [string]$Target,
    [string]$Remote = "origin",
    [switch]$SkipCheck,
    [switch]$AllowDirty,
    [switch]$DryRun
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

function Write-Step {
    param([string]$Message)
    Write-Host "`n==> $Message" -ForegroundColor Cyan
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

function Invoke-Checked {
    param(
        [string]$Command,
        [string[]]$CommandArgs = @()
    )

    & $Command @CommandArgs
    if ($LASTEXITCODE -ne 0) {
        $joinedArgs = ($CommandArgs -join " ")
        throw "Command failed: $Command $joinedArgs"
    }
}

$scriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$repoRoot = Resolve-Path (Join-Path $scriptDir "..")
Set-Location $repoRoot

if (-not (Get-Command git -ErrorAction SilentlyContinue)) {
    throw "git is required but was not found in PATH."
}
if (-not (Get-Command npm -ErrorAction SilentlyContinue)) {
    throw "npm is required but was not found in PATH."
}

$packagePath = Join-Path $repoRoot "package.json"
$bumpScriptPath = Join-Path $repoRoot "scripts/bump-version.ps1"

if (-not (Test-Path $packagePath)) {
    throw "Missing file: package.json"
}
if (-not (Test-Path $bumpScriptPath)) {
    throw "Missing file: scripts/bump-version.ps1"
}

$currentVersion = [string]((Get-Content -Raw $packagePath | ConvertFrom-Json).version)
if ([string]::IsNullOrWhiteSpace($currentVersion)) {
    throw "Unable to read current version from package.json"
}

$newVersion = Get-NextVersion -CurrentVersion $currentVersion -RequestedTarget $Target
$tagName = "v$newVersion"

$currentBranch = (git rev-parse --abbrev-ref HEAD | Out-String).Trim()
if ($LASTEXITCODE -ne 0) {
    throw "Unable to detect current git branch."
}
if ($currentBranch -eq "HEAD") {
    throw "Detached HEAD detected. Checkout a branch before shipping release."
}

git remote get-url $Remote *> $null
if ($LASTEXITCODE -ne 0) {
    throw "Git remote '$Remote' does not exist."
}

$localTagExists = (git tag --list $tagName | Out-String).Trim()
if ($localTagExists) {
    throw "Tag '$tagName' already exists locally."
}

$remoteTagLookup = (git ls-remote --tags $Remote "refs/tags/$tagName" | Out-String).Trim()
if ($LASTEXITCODE -ne 0) {
    throw "Unable to query tags from remote '$Remote'."
}
if ($remoteTagLookup) {
    throw "Tag '$tagName' already exists on remote '$Remote'."
}

if (-not $DryRun -and -not $AllowDirty) {
    $workingTreeState = (git status --porcelain | Out-String).Trim()
    if ($workingTreeState) {
        throw "Working tree is not clean. Commit/stash changes first, or pass -AllowDirty explicitly."
    }
}

Write-Step "Release plan"
Write-Host "Current version : $currentVersion"
Write-Host "Target          : $Target"
Write-Host "Next version    : $newVersion"
Write-Host "Tag             : $tagName"
Write-Host "Branch          : $currentBranch"
Write-Host "Remote          : $Remote"
Write-Host "Run checks      : $(-not $SkipCheck)"
Write-Host "Dry run         : $DryRun"

if ($DryRun) {
    Write-Step "Dry-run: validating bump command"
    Invoke-Checked "powershell" @(
        "-ExecutionPolicy", "Bypass",
        "-File", $bumpScriptPath,
        $Target,
        "-DryRun"
    )

    Write-Step "Dry-run: commands that would run"
    Write-Host "powershell -ExecutionPolicy Bypass -File scripts/bump-version.ps1 $Target"
    if (-not $SkipCheck) {
        Write-Host "npm run check"
    }
    Write-Host "git add package.json src-tauri/Cargo.toml src-tauri/tauri.conf.json"
    Write-Host "git commit -m `"chore(release): $tagName`" -- package.json src-tauri/Cargo.toml src-tauri/tauri.conf.json"
    Write-Host "git tag -a $tagName -m `"Release $tagName`""
    Write-Host "git push $Remote $currentBranch"
    Write-Host "git push $Remote $tagName"
    return
}

Write-Step "Bumping version files"
Invoke-Checked "powershell" @(
    "-ExecutionPolicy", "Bypass",
    "-File", $bumpScriptPath,
    $Target
)

$actualVersion = [string]((Get-Content -Raw $packagePath | ConvertFrom-Json).version)
if ($actualVersion -ne $newVersion) {
    throw "Version mismatch after bump. Expected $newVersion but found $actualVersion."
}

if (-not $SkipCheck) {
    Write-Step "Running project checks"
    Invoke-Checked "npm" @("run", "check")
}

$versionFiles = @(
    "package.json",
    "src-tauri/Cargo.toml",
    "src-tauri/tauri.conf.json"
)

Write-Step "Creating release commit"
Invoke-Checked "git" @("add", "--", $versionFiles[0], $versionFiles[1], $versionFiles[2])
Invoke-Checked "git" @(
    "commit",
    "-m", "chore(release): $tagName",
    "--",
    $versionFiles[0],
    $versionFiles[1],
    $versionFiles[2]
)

Write-Step "Creating git tag"
Invoke-Checked "git" @("tag", "-a", $tagName, "-m", "Release $tagName")

Write-Step "Pushing commit and tag"
Invoke-Checked "git" @("push", $Remote, $currentBranch)
Invoke-Checked "git" @("push", $Remote, $tagName)

Write-Step "Release trigger completed"
Write-Host "Pushed $tagName to $Remote."
Write-Host "GitHub Actions release workflow should start automatically."
