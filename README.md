# Super Route Pro v10.1.6

Super Route Pro is a Windows desktop network toolkit built with Tauri + React + Rust.
It focuses on route management, network diagnostics, and continuous ping/fping testing in one lightweight UI.

Current stable release: `v10.1.6`

Author: Zonzon

## 1) Project Summary

This app provides a practical control panel for Windows networking tasks:

- View active network interfaces (NICs)
- View and manage IPv4 routes
- Add route, delete route, flush routes
- Set selected NIC gateway as default internet route
- **Route Persistence Service** — auto re-apply WAN + custom routes on system restart
- Run common network fix commands
- Run diagnostics commands and show output in app
- Run continuous ping or fping-like multi-target checks
- Run the dedicated native Speed Test modal with `Auto Asia`, `Auto Australia`, and fixed regional targets
- Run tracert directly from target input
- Use a unified output console (command/routing output on top, ping/tracert output on bottom)
- Broker-based Repair Mode unlock for privileged actions
- Bloatware manager — bulk remove Windows apps
- Cache cleaner — clear browser/system caches
- Battery report generation
- Switch between light and dark mode

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

### Route Persistence Service

- Background service (`SuperRouteService.exe`) runs at login via Task Scheduler
- Identifies NIC by description + MAC address (survives InterfaceIndex changes)
- Re-applies WAN default gateway + custom routes automatically
- Balloon tip notification if NIC not found
- Toggle "Persist on startup" in the WAN section
- Run-once and exit — no background resource usage

### Speed Test

- Native Tauri-backed speed test flow with progress events
- `Auto Asia` and `Auto Australia` profiles plus fixed regional targets like `JP/KR`, `US West`, and `EU`
- Final summary shows dedicated `Target / Provider / Region` identity metadata

## 3) Tech Stack

- Frontend: React 19 + TypeScript + Vite
- Desktop shell: Tauri v2
- Backend: Rust (Win32 API, Registry, Shell_NotifyIcon)
- Icons: lucide-react

## 4) Security Model

Network command execution uses a whitelist in Rust (`run_network_command`) to block arbitrary command execution.
Only allowed command prefixes can run from UI actions.
Privileged machine actions flow through Repair Mode and the elevated repair broker rather than direct arbitrary shell access from the UI.

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
|  |- repairModeModel.ts
|  |- SpeedTestModal.tsx
|  |- SpeedTestModalView.tsx
|- src-tauri/              # Tauri + Rust backend
|  |- src/
|  |  |- lib.rs                 # Tauri builder/composition root
|  |  |- app_bootstrap.rs       # startup/runtime validation and main window setup
|  |  |- network.rs             # network operations + diagnostics helpers
|  |  |- repair_commands.rs     # repair-facing Tauri command wrappers
|  |  |- route_persist.rs       # persist config (JSON read/write)
|  |  |- route_service_main.rs  # SuperRouteService binary
|  |  |- speed_test*.rs         # native speed test engine + target catalog
|  |- binaries/            # Compiled sidecar EXEs
|  |- tauri.conf.json
|  |- Cargo.toml
|- scripts/
|  |- prepare-repair-sidecars.ps1
|- public/
|- package.json
```

## 7) Prerequisites

Install these first:

- Windows 10/11 x64
- Node.js 24+
- npm 10+
- Rust toolchain (`rustup`, `cargo`)
- WebView2 Runtime (Windows 11 usually already has it)
- Microsoft Visual Studio C++ Build Tools (2022)

## 8) Run and Test

Install dependencies:

```powershell
npm ci
```

Run desktop app in dev mode:

```powershell
npm run tauri -- dev
```

Build frontend only:

```powershell
npm run build
```

Validate Rust backend:

```powershell
npm run check:rust
```

Validate the release baseline:

```powershell
npm run check
```

## 9) Build Release

Build installers/executables via Tauri:

```powershell
npm run tauri -- build
```

Collect local release artifacts in one folder:

```powershell
npm run release:local -- -VersionTag v10.1.6 -SkipInstall
```

Typical outputs:

- Installer bundles:
  - `src-tauri/target/release/bundle/nsis/`
- Release exe (bin name from Cargo):
  - `src-tauri/target/release/SuperRoute.exe`

## 10) Release And Versioning

Version bump commands:

```powershell
npm run version:patch
npm run version:minor
npm run version:major
npm run version:bump -- 10.1.6
```

Tag-and-push release flow:

```powershell
npm run release:ship -- 10.1.6
```

## 11) Push Entire Project to GitHub

If this folder is not a git repo yet:

```powershell
cd E:\super-route-pro
git init
git add .
git commit -m "chore: initial import Super Route Pro v3.6.9"
git branch -M main
git remote add origin https://github.com/<your-user>/<your-repo>.git
git push -u origin main
```

If remote repo already exists and has commits:

```powershell
git remote add origin https://github.com/<your-user>/<your-repo>.git
# if origin already exists, run: git remote set-url origin <url>
git fetch origin
git pull --rebase origin main
git push -u origin main
```

## 12) Pre-Push Checklist

- Run `npm run check`
- Confirm no secrets/API keys are committed
- Confirm large generated files are ignored (`node_modules`, `dist`, `src-tauri/target`)

## 13) Notes

- Some network commands require Administrator privileges on Windows.
- The app is designed to run as a standard user first; unlock Repair Mode only when privileged fixes are needed.

## 14) Releases & Download

- All releases: `https://github.com/quockhanh2376/SuperRoutePro/releases`
- Latest release: `https://github.com/quockhanh2376/SuperRoutePro/releases/latest`
- Current working version on `main`: `v10.1.6`
- Latest published release: `v10.1.6` — `https://github.com/quockhanh2376/SuperRoutePro/releases/tag/v10.1.6`
