# PROJECT SUMMARY

## 1. Snapshot

- Project: **Super Route Pro**
- Audit date: **2026-02-10**
- App type: **Windows desktop app** (Tauri v2 + React + Rust)
- Current package/app version: **6.3.0**
  - `package.json` = `6.3.0`
  - `src-tauri/tauri.conf.json` = `6.3.0`
- UI label mismatch: `src/App.tsx` still shows `APP_VERSION = "5.4"`

## 2. Tech Stack

- Frontend:
  - React 19
  - TypeScript
  - Vite 7
  - Tailwind CSS v4
  - lucide-react
- Desktop shell:
  - Tauri v2
- Backend/native:
  - Rust (edition 2021)
  - serde + serde_json
  - windows-sys (Windows MessageBox dialog support)

## 3. High-Level Architecture

1. React UI in `src/App.tsx` drives all user actions.
2. Frontend calls Rust commands via Tauri `invoke` wrappers in `src/api.ts`.
3. Rust backend (`src-tauri/src/network.rs`) executes Windows commands (`route`, `netsh`, `ipconfig`, `ping`, PowerShell) and returns typed JSON results.
4. Tauri command registration is in `src-tauri/src/lib.rs`.
5. Release build embeds admin manifest (`requireAdministrator`) through `src-tauri/build.rs` + `src-tauri/super-route-pro.exe.manifest`.

## 4. Project Structure

```text
SuperrRoutePro/
|- src/
|  |- App.tsx                # Main UI + feature logic
|  |- App.css                # Styling (dark/light + modal + tables)
|  |- api.ts                 # Typed Tauri invoke wrappers
|  |- main.tsx               # React entrypoint
|  `- vite-env.d.ts
|- src-tauri/
|  |- src/
|  |  |- lib.rs              # Runtime validation + command registration
|  |  |- network.rs          # All backend command implementations
|  |  `- main.rs             # Binary entry -> super_route_pro_lib::run()
|  |- capabilities/default.json
|  |- tauri.conf.json
|  |- Cargo.toml
|  |- build.rs
|  `- super-route-pro.exe.manifest
|- public/
|- index.html
|- package.json
|- vite.config.ts
|- tsconfig.json
`- launch-dev.ps1
```

## 5. Frontend (Current Behavior)

Main file: `src/App.tsx` (single large component + memoized subcomponents).

### 5.1 Main UI areas

- Header:
  - App branding
  - Remove Apps button
  - Clear Cache button
  - Light/Dark toggle
  - Online/Offline indicator
  - Live latency indicator (ms)
- Left panel:
  - Network interface table (active-only filter + refresh)
  - Route form (destination/mask/gateway/metric)
  - Route actions: `ADD`, `DEL`, `WAN`, `FLUSH`
  - Unified output console (command/routing output + ping output)
- Right panel:
  - Network Fix Tools
  - Diagnostics & Repair
  - Ping & Tracert Monitor
- Footer:
  - Status message
  - App/version label

### 5.2 Core frontend flows

- Initial data load:
  - `getNetworkInterfaces(activeOnly)`
  - `getRoutingTable()`
- Monitors:
  - Internet check every 5s (`checkInternet`)
  - Latency check every 2s (`pingHost("8.8.8.8")`)
- Route operations:
  - Add route
  - Delete route
  - Set selected NIC as default gateway
  - Flush routes
- Network tools:
  - Flush DNS
  - Renew IP
  - Wi-Fi info
  - Clear ARP
  - Reset TCP/IP
  - Reset Winsock
  - Reset firewall
  - Battery report modal
- Diagnostics:
  - Display DNS cache
  - Reset WinHTTP proxy (with confirm)
  - Restart active adapters (with confirm)
  - Port test (`Test-NetConnection`)
  - NSLookup
  - IP scan modal
- Ping section:
  - Continuous `ping` mode (single target)
  - Continuous `fping` mode (multi-target list)
  - `tracert` from target

### 5.3 Advanced UI features

- Modal set:
  - Battery report
  - Scan IP
  - Clear cache
  - Remove apps (bloatware)
  - Generic confirm dialog
- Log buffering:
  - Ping log and command log stored in ring-buffer style arrays (`useRef`)
  - UI repaint throttled with `requestAnimationFrame`
- Theme:
  - Dark/Light theme classes
  - Theme-lens transition animation
  - Theme value is stored to `localStorage` (write only; no restore on startup)

## 6. Backend (Current Behavior)

Main backend file: `src-tauri/src/network.rs`.

### 6.1 Exposed Tauri commands

- `get_network_interfaces(active_only)`
- `get_routing_table()`
- `add_route(destination, mask, gateway, metric, interface_index?)`
- `delete_route(destination, mask)`
- `flush_routes()`
- `set_default_gateway(gateway, interface_index)`
- `run_network_command(command)`
- `ping_host(target, count?)`
- `fping_scan(targets, timeout_ms?)`
- `check_internet()`
- `get_bloatware_candidates()`
- `remove_bloatware(packages)`
- `clear_cache_targets(targets)`
- `get_battery_report()`

### 6.2 Command execution model

- Uses hidden process creation (`CREATE_NO_WINDOW`) to avoid extra console windows.
- Uses:
  - PowerShell for system data and cleanup scripts.
  - Native commands (`route`, `ipconfig`, `netsh`, `ping`, `powercfg`, etc.).
- `run_network_command` enforces prefix whitelist to block arbitrary command execution.

### 6.3 Feature blocks in backend

- NIC discovery:
  - Reads WMI adapter config
  - Filters many virtual/irrelevant adapters by blacklist keywords
- Routing table:
  - Reads IPv4 routes via `Get-NetRoute`
  - Converts CIDR prefix to netmask string
- Route management:
  - Persistent route add (`route -p add`)
  - Delete and flush support
  - Default route switch logic (`0.0.0.0/0`)
- Ping:
  - Single ping parser (extract latency)
  - fping-like parallel scan with bounded worker pool
- Remove apps:
  - Predefined candidate list
  - Detect installed/provisioned packages
  - Remove selected packages with summary output
- Cache cleanup:
  - Predefined cleanup targets (Windows + browser + crash/wer/shader cache)
  - Per-target PowerShell recipe execution with summary
- Battery report:
  - Generates HTML via `powercfg /batteryreport`
  - Reads report file and returns HTML string for iframe preview
- Internet check:
  - TCP connect timeout to `8.8.8.8:53`

## 7. Runtime Environment Validation

Before app startup (`src-tauri/src/lib.rs`), app blocks launch if any check fails:

- OS: Windows only
- Minimum build: 10240 (Windows 10+ baseline)
- Admin privileges required
- WebView2 runtime present (registry checks)
- Required commands present: `route`, `netsh`, `ipconfig`, `ping`, `powershell`

If failed, app shows Windows MessageBox error and exits.

## 8. Build, Dev, Distribution

### 8.1 Frontend + Tauri config

- `vite.config.ts`:
  - fixed port `1420` (strict)
  - ignores `src-tauri` watch
- `src-tauri/tauri.conf.json`:
  - dev URL: `http://localhost:1420`
  - bundle targets: `nsis`, `msi`
  - NSIS install mode: `perMachine`
  - WebView2 install mode: `downloadBootstrapper` silent

### 8.2 NPM scripts

- `npm run dev` -> Vite dev server
- `npm run build` -> `tsc && vite build`
- `npm run preview` -> Vite preview
- `npm run tauri` -> Tauri CLI

### 8.3 Rust/Tauri build notes

- Release build embeds admin manifest in `build.rs`.
- Dev build does not force embed same manifest step.

## 9. Data/State Model (Frontend)

Important state groups in `App.tsx`:

- Network data:
  - `nics`, `routes`, `selectedNic`, route form fields
- Status/monitoring:
  - `isOnline`, `currentLatency`, `statusMsg`, loading flags
- Diagnostics output:
  - command log buffer + routing snapshot + ping log buffer
- Feature modals:
  - battery / scan IP / cache cleanup / remove apps / confirm
- Progress:
  - IP scan progress
  - cache cleanup progress
  - remove apps progress

## 10. Current Differences and Doc Risks

- Version label mismatch:
  - UI shows `5.4` but package + tauri config are `6.3.0`.
- Old docs in repo mention files/features not in current code snapshot (example: older notes reference `LatencyChart.tsx`, but current source set has no such file).
- Theme persistence is partial:
  - app writes theme to localStorage but does not read on startup.

## 11. Validation Snapshot (Audit Run)

Executed on 2026-02-10:

- `npm run build` -> success
  - built assets in `dist/`
- `cargo check` in `src-tauri` -> success

No automated test suite files were detected in current repository (`src` / `src-tauri`).

## 12. Suggested README Update Outline

Use this order to keep README consistent with real code:

1. Product scope and OS constraints
2. Runtime requirements + startup environment validation
3. Actual feature list (NIC/routes/tools/diagnostics/ping/fping/cache/apps/battery)
4. Security model (`run_network_command` whitelist + fixed target lists)
5. Real project structure
6. Dev/build/release commands
7. Known limitations/mismatches (version label, theme restore behavior)

