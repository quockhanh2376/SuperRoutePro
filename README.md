# Super Route Pro v3.6.9

Super Route Pro is a Windows desktop network toolkit built with Tauri + React + Rust.
It focuses on route management, network diagnostics, and continuous ping/fping testing in one lightweight UI.

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

- Node.js 20+
- npm 10+
- Rust toolchain (`rustup`, `cargo`)
- WebView2 Runtime (Windows 11 usually already has it)
- Microsoft C++ build tools (for Tauri/Rust native build on Windows)

## 8) Run and Test

Install dependencies:

```powershell
npm install
```

Run desktop app in dev mode:

```powershell
npm run tauri dev
```

Build frontend only:

```powershell
npm run build
```

Validate Rust backend:

```powershell
cd src-tauri
cargo check
```

## 9) Build Release

Build installers/executables via Tauri:

```powershell
npm run tauri build
```

Typical outputs:

- Installer bundles:
  - `src-tauri/target/release/bundle/nsis/`
  - `src-tauri/target/release/bundle/msi/`
- Release exe (bin name from Cargo):
  - `src-tauri/target/release/SuperRoute.exe`

## 10) Push Entire Project to GitHub

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

## 11) Pre-Push Checklist

- Run `npm run build`
- Run `cargo check` in `src-tauri`
- Confirm no secrets/API keys are committed
- Confirm large generated files are ignored (`node_modules`, `dist`, `src-tauri/target`)

## 12) Notes

- Some network commands require Administrator privileges on Windows.
- For stable behavior in diagnostics tools, run the app as Administrator when needed.
