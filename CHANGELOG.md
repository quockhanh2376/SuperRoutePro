# Changelog

## v10.0.8 (2026-03-21)

### ✨ New Features
- **Route Persistence Service** — `SuperRouteService.exe` runs at login via Task Scheduler
  - Auto re-applies WAN default gateway + custom routes after system restart
  - NIC identification by description + MAC address (survives InterfaceIndex changes)
  - Retry loop (60s timeout) for NIC that hasn't initialized yet
  - Balloon tip notification if NIC not found
  - Toggle "Persist on startup" in WAN section — no manual config needed
  - Run-once and exit — zero background resource usage

### 🐛 Bug Fixes
- **Fix PowerShell window flash on startup** — replaced 3 process-spawning startup checks with native Rust Win32 API
  - `detect_windows_build_number` → native registry read
  - `command_exists` → PATH env var lookup
  - `has_webview2_runtime` → native `RegQueryValueExW`

### 📦 Internal
- Added `windows-sys` features: `Win32_System_Registry`, `Win32_Foundation`
- New files: `route_persist.rs`, `route_service_main.rs`
- 5 new unit tests for config serialization
- Updated sidecar build script to include SuperRouteService
