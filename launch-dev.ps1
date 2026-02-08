#!/usr/bin/env pwsh
# Super Route Pro - Quick Start Script

$ProjectPath = Split-Path -Parent $MyInvocation.MyCommand.Path
Write-Host "🚀 Super Route Pro - Quick Start" -ForegroundColor Cyan
Write-Host "📁 Project: $ProjectPath" -ForegroundColor Gray
Write-Host ""

# Refresh PATH
Write-Host "🔄 Refreshing PATH..." -ForegroundColor Yellow
$env:Path = [System.Environment]::GetEnvironmentVariable("Path","Machine") + ";" + [System.Environment]::GetEnvironmentVariable("Path","User")

# Verify tools
Write-Host "✅ Checking prerequisites..." -ForegroundColor Yellow
$tools = @{
    "Node.js" = "node"
    "npm" = "npm"
    "Rust" = "rustc"
    "Cargo" = "cargo"
}

foreach ($tool in $tools.GetEnumerator()) {
    try {
        $version = & $tool.Value --version 2>&1 | Select-Object -First 1
        Write-Host "  ✓ $($tool.Key): $version" -ForegroundColor Green
    } catch {
        Write-Host "  ✗ $($tool.Key): NOT FOUND" -ForegroundColor Red
        Write-Host "     Install from: https://nodejs.org or https://rustup.rs" -ForegroundColor Yellow
        Exit 1
    }
}

Write-Host ""
Write-Host "📦 Installing dependencies..." -ForegroundColor Yellow
Set-Location $ProjectPath
npm install --silent

Write-Host ""
Write-Host "🎨 Building frontend..." -ForegroundColor Yellow
npm run build --silent

Write-Host ""
Write-Host "✨ Starting development server..." -ForegroundColor Green
Write-Host "   Frontend: http://localhost:1420" -ForegroundColor Gray
Write-Host "   DevTools: Ctrl+Shift+I" -ForegroundColor Gray
Write-Host ""

npm run tauri dev
