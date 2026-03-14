# Super Route Pro v9.0.9

Super Route Pro is a Windows desktop network toolkit built with Tauri + React + Rust.
It focuses on route management, network diagnostics, and continuous ping/fping testing in one lightweight UI.

Current stable release: `v9.0.9`

Author: Zonzon

## 1) Project Summary

This app provides a practical control panel for Windows networking tasks:

- View active network interfaces (NICs)
- View and manage IPv4 routes
- Add route, delete route, flush routes
- Set selected NIC gateway as default internet route
- Run common network fix commands
- Run diagnostics commands and show output in app
- Run continuous ping or fping-like multi-target checks
- Run tracert directly from target input
- Use a unified output console (command/routing output on top, ping/tracert output on bottom)
- Switch between light and dark mode
- Start in standard-user mode and unlock `Repair Mode` only when admin fixes are needed

## 2) Main Features

### Route and NIC Management

- NIC list with active-only filter
- Route form: destination, mask, gateway, metric
- Actions: `ADD`, `DEL`, `WAN`, `FLUSH`, `ROUTES`

### Network Fix Tools

- Flush DNS
- Renew IP
- Wi-Fi Info (`netsh wlan show interface`)
- Reset TCP/IP
- Reset Winsock
- Clear ARP cache
- Reset Firewall

### Diagnostics and Repair

- Display DNS Cache
- Reset WinHTTP Proxy
- Restart Active Adapters
- Port Connectivity Test
- NSLookup

### Ping and Tracing

- Ping mode: continuous single target
- fping mode: continuous multi-target round check
- Start/Stop controls
- Tracert command from current input

## 3) Tech Stack

- Frontend: React 19 + TypeScript + Tailwind CSS v4 + Vite
- Desktop shell: Tauri v2
- Backend: Rust
- Icons: lucide-react

## 4) Security Model

Network command execution uses a whitelist in Rust (`run_network_command`) to block arbitrary command execution.
Only allowed command prefixes can run from UI actions.

## 5) Performance Optimizations Already Applied

- Memoized UI blocks (`React.memo`) for repeated components and output console
- Ring-buffer style log storage using refs to avoid expensive string split/join on each append
- Batched log repaint scheduling with `requestAnimationFrame`
- Rust fping scan switched to bounded worker pool instead of repeated burst thread spawning

## 6) Project Structure

```text
super-route-pro/
|- src/                    # React UI
|  |- App.tsx
|  |- App.css
|  |- api.ts
|- src-tauri/              # Tauri + Rust backend
|  |- src/
|  |  |- lib.rs
|  |  |- network.rs
|  |- tauri.conf.json
|  |- Cargo.toml
|- public/
|- launch-dev.ps1
|- SETUP_GUIDE_VI.md
|- package.json
```

## 7) Prerequisites

Install these first:

- Windows 10/11 x64
- Node.js 24+
- npm 10+
- Rust toolchain (`rustup`, `cargo`)
- Microsoft Visual Studio C++ Build Tools (2022)

Notes:

- Release installers are configured with WebView2 `offlineInstaller`, so target machines do not need WebView2 preinstalled.
- The UI now starts as the logged-in standard user.
- Admin credentials are only requested when IT unlocks `Repair Mode` for privileged fixes.
- Startup rotates a stale `main-webview` profile and retries once if WebView2 reports its data directory is not writable.

## 8) Run and Validate Locally

```powershell
npm ci
npm run tauri dev
```

Run full build checks before release:

```powershell
npm run check
```

## 9) Build Release Locally

Version bump in one command (updates all 3 files together):

```powershell
npm run version:patch
# or:
# npm run version:minor
# npm run version:major
# npm run version:bump -- 6.4.0
```

One-command ship flow (bump + commit + tag + push):

```powershell
npm run release:patch
# or:
# npm run release:minor
# npm run release:major
# npm run release:ship -- 6.4.0
```

Notes:

- `release:*` runs `npm run check` by default before commit/tag.
- It requires a clean working tree by default for safe release commits.
- Dry-run preview:

```powershell
npm run release:ship -- patch -DryRun
```

Recommended one-command release build:

```powershell
npm run release:local
```

This script will:

- Run `npm ci` (unless you pass `-SkipInstall`)
- Build the Tauri `NSIS` bundle and portable `SuperRoute.exe`
- Collect all artifacts into `release-artifacts/vX.Y.Z/`
- Generate `SHA256SUMS.txt`

Optional:

```powershell
npm run release:local -- -VersionTag v9.0.9 -SkipInstall
```

## 10) Automated GitHub Release

Workflows included:

- `.github/workflows/ci.yml`
  - Runs on push/PR, validates frontend + Rust build.
- `.github/workflows/release.yml`
  - Runs on tag push (`v*`), builds installers, uploads artifacts, and publishes GitHub release assets.

Release flow:

```powershell
npm run release:patch
# (or minor/major/specific version)
```

Generated release assets:

- `Super Route Pro_<version>_x64-setup.exe` (NSIS)
- `SuperRoute.exe` (portable)
- `SHA256SUMS.txt`

## 11) Install On A New Machine

1. Download installer from GitHub Releases.
2. Run installer as Administrator.
3. Launch app as the standard user who is signed in.
4. Unlock `Repair Mode` only when you need privileged system fixes. Windows will prompt for a local admin credential once for that app session.
5. Use `SHA256SUMS.txt` to verify integrity if required.

## 12) Pre-Release Checklist

- Keep versions aligned in:
  - `package.json`
  - `src-tauri/Cargo.toml`
  - `src-tauri/tauri.conf.json`
- Run `npm run check`
- Run `npm run release:local`
- Verify at least one clean-machine install test (VM recommended)
- Confirm no generated artifacts are committed (`node_modules`, `dist`, `src-tauri/target`, `release-artifacts`)

## 13) Releases & Download

- All releases: `https://github.com/quockhanh2376/SuperRoutePro/releases`
- Latest release: `https://github.com/quockhanh2376/SuperRoutePro/releases/latest`
- Version v9.0.9: `https://github.com/quockhanh2376/SuperRoutePro/releases/tag/v9.0.9`

