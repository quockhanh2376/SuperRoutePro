# Super Route Pro Daily Log

This document is the running delivery log for Super Route Pro.
Update it after each meaningful work session so the team and NotebookLM stay aligned on current progress, decisions, blockers, and next steps.

--------------------------------------------------------------------------------

## 2026-03-21 — PowerShell → Native Rust Migration (v10.1.0)

**Done**
- Audited all 13+ PowerShell call sites across 5 Rust files (`network.rs`, `lib.rs`, `repair_actions.rs`, `route_service_main.rs`, `repair_targets.rs`).
- **Cache Cleanup** (`repair_actions.rs` + `network.rs`): Replaced PowerShell `Remove-Item` with native `std::fs` operations (`clean_directory_contents`, `clean_files_with_prefix`). Windows Update cache now uses `net.exe stop/start`.
- **Scheduled Tasks** (`lib.rs`): Replaced `Register-ScheduledTask` / `Unregister-ScheduledTask` with `schtasks.exe /Create` and `/Delete`.
- **NIC Enumeration** (`win32_net.rs` — NEW FILE): Created native module using `netsh interface ipv4 show interfaces` + `netsh interface ipv4 show addresses` + `getmac /fo csv`. Replaced PowerShell `Get-NetAdapter` and `Get-WmiObject` calls in `network.rs`, `lib.rs`, `route_service_main.rs`.
- **Routing Table** (`network.rs`): Replaced `Get-NetRoute` with `route print -4` text parsing.
- **Gateway Cleanup** (`network.rs`): Replaced `Remove-NetRoute` PowerShell script with `route delete 0.0.0.0`.
- **Battery Info** (`network.rs`): Replaced `Get-CimInstance Win32_Battery` PowerShell with native Win32 `GetSystemPowerStatus` + `DeviceIoControl` IOCTL (`IOCTL_BATTERY_QUERY_INFORMATION`) via `SetupDi` for detailed battery info (design capacity, full charge capacity, cycle count, chemistry, health%).
- **Restart Adapters** (`repair_actions.rs`): Replaced PowerShell `Restart-NetAdapter` with `netsh interface set interface disable/enable` using `win32_net::enumerate_adapters`.
- Removed `"powershell"` from `REQUIRED_COMMANDS` in `lib.rs`.
- Removed PowerShell commands from `allowed_prefixes` whitelist in `network.rs`.
- **Bloatware** (`Get-AppxPackage` / `Remove-AppxPackage`): Kept on PowerShell (no cmd.exe alternative) but runs with `CREATE_NO_WINDOW` — no PS window flash.
- **WAN Persist Script**: Rewrote from `.ps1` PowerShell to `.cmd` batch using `route print` + `findstr` + `route delete/add`. `schtasks` now runs `cmd.exe /c` instead of `powershell.exe -File`.
- **Test-NetConnection** (`App.tsx`): Replaced PowerShell `Test-NetConnection` with native Rust `test_tcp_port` Tauri command using `std::net::TcpStream::connect_timeout`. Frontend is now **0% PowerShell**.
- **Release Hardening Follow-Up**: Fixed the typed frontend wrapper for `test_tcp_port`, restored `interface_index` mapping in parsed `route print -4` output so route-dependent UI logic keeps working, and bundled `SuperRouteService` alongside `SuperRouteRepairBroker` for installer builds.
- **Verification**: Re-ran `npm run check` and `cargo test --manifest-path src-tauri/Cargo.toml`; both passed after the hardening fixes.
- **Release Gate Hardening**: Wired the existing Node test files and Rust test suite into `npm run check`, so `release-ship.ps1` now inherits broader verification without additional branching. Verified the expanded gate passes with frontend build, `cargo check`, Node tests, and `cargo test`.
- **NotebookLM Sync**: Logged the release-gate hardening here so NotebookLM stays aligned with the stronger `v10.1.0` verification baseline.
- **Version And Docs Alignment**: Bumped `package.json`, `src-tauri/Cargo.toml`, and `src-tauri/tauri.conf.json` to `10.1.0`, and refreshed release-facing docs to reflect the `v10.1.0` prep state without pointing release downloads at a tag that has not shipped yet.
- **Manual Verify Prep**: Wrote the focused `v10.1.0` manual verification checklist into beads issue `super-route-pro-erj.1` so QA/release can execute a single source of truth for the remaining UI pass.
- **Manual Verify Execution**: Ran the main `v10.1.0` UI verification flow against a fresh `npm run release:build` artifact. Confirmed the packaged installer now emits `Super Route Pro_10.1.0_x64-setup.exe`, startup shows `10.1.0`, routing snapshot refresh works, Repair Mode unlock/lock works, Scan IP derives `192.168.1.0/24` and supports Force Stop, Port Test passes open/closed smoke cases, Battery Info loads native values, Add/Delete Route works once unlocked with real field values entered, Clear Cache completes with warnings, and Remove Apps list loads correctly.
- **Manual Verify Findings**: Logged two follow-up bugs in beads from the execution pass: `super-route-pro-auy` for orphaned elevated `SuperRouteRepairBroker` processes after force-closing while unlocked, and `super-route-pro-nhl` for the NIC table sometimes showing `No interfaces found` during startup even while the footer already reports loaded NICs/routes.
- **Help Copy Update**: Added a dedicated Help entry for `Lock / Unlock Repair Mode` in both English and Vietnamese so users understand that Locked blocks admin fixes, Unlock opens an elevated Repair Mode session for the current app session, and Lock closes that elevated session again.
- **Repair Broker Lifecycle Fix**: Closed `super-route-pro-auy` by passing the launching app PID into `UnlockRepairSessionRequest` and teaching `SuperRouteRepairBroker` to monitor the parent process handle, so the elevated broker self-terminates if the UI is force-closed while unlocked.
- **NIC Startup Empty-State Fix**: Closed `super-route-pro-nhl` by adding a NIC-table loading placeholder and stale-load guards, so startup no longer flashes `No interfaces found` while the first NIC snapshot is still loading.
- **Persistence Tracking Update**: Logged the still-open release blocker as beads issue `super-route-pro-u3z` because the `Persist on startup OFF` path remains inconclusive and still needs direct repro before sign-off.
- **Rust Warning Cleanup**: Removed the remaining `cargo check` warning debt by deleting unused battery/NIC helper remnants, dropping an unused registry import and raw target struct, and switching `SuperRouteService` to reuse the shared `route_persist` module instead of compiling its own warning-prone copy. Re-ran `cargo check` clean, then re-ran full `npm run check` clean.
- **Persist OFF Root-Cause Fix**: Closed `super-route-pro-u3z` by moving startup-persistence save/clear operations onto the elevated repair broker path, so standard-user sessions no longer try to write `%ProgramData%\\SuperRoutePro\\persist.json` or register `SuperRouteProPersist` directly. The WAN flow now clears persisted startup state when OFF, keeps the checkbox aligned with either persisted config or the legacy WAN task, and ships with new Node + Rust coverage for the persist action contract.
- **GitHub Gate Alignment**: Updated `.github/workflows/ci.yml` and `.github/workflows/release.yml` so GitHub now runs the same `npm run check` release gate as local shipping, preventing installer publication from a weaker signal than the local baseline.
- **Release Published**: GitHub CI run `23391640153` passed with the new full gate, then tag `v10.1.0` triggered release workflow `23391750805`, which built/published `Super.Route.Pro_10.1.0_x64-setup.exe`, `SuperRoute.exe`, and `SHA256SUMS.txt` to the GitHub release page.

**Files Changed**
| File | Change |
|------|--------|
| `src-tauri/src/win32_net.rs` | **NEW** — netsh-based NIC enumeration module |
| `src-tauri/src/network.rs` | Major — NIC, routing, battery IOCTL, cache, gateway, WAN persist, test_tcp_port |
| `src-tauri/src/repair_actions.rs` | Cache cleanup + restart adapters rewrite |
| `src-tauri/src/lib.rs` | Scheduled tasks + NIC + test_tcp_port + removed PS from REQUIRED_COMMANDS |
| `src-tauri/src/route_service_main.rs` | NIC lookup rewrite |
| `src-tauri/Cargo.toml` | Added Win32 API features for battery IOCTL + SetupDi |
| `src/App.tsx` | Bloatware UI fix + test_tcp_port migration |

**Notes & Decisions**
- `wmic` was deprecated/removed on this Windows 11 build — switched to `netsh` + `getmac` for NIC enumeration.
- Battery IOCTL uses fully manual FFI declarations (`extern "system"`) due to `windows-sys 0.59` handle type inconsistencies across modules.
- AppX bloatware operations (`Get-AppxPackage`/`Remove-AppxPackage`) remain on PowerShell as there is no native cmd.exe alternative. Runs hidden with `CREATE_NO_WINDOW`.
- The current verification machine had no installed bloatware candidates from the supported remove-app list, so the destructive uninstall path could not be executed safely in this pass.
- The root cause for the Persist-on-startup OFF blocker was that the UI called `persist_save_config` directly from the standard-user app process, which could fail silently when writing `%ProgramData%` or touching the `SuperRouteProPersist` task. The fix routes those writes through Repair Mode elevation and treats OFF as clearing persisted startup state instead of leaving a disabled sentinel file behind.

**Next Steps**
- Complete the optional startup-task/logoff-reboot persistence verification now that the Persist-on-startup OFF path is fixed.
- Consider consolidating the legacy `SuperRoutePro-PersistWAN` task and the newer `SuperRouteProPersist` service flow after `v10.1.0` so startup persistence has a single mechanism/end-state.
- Expand automated coverage further for the migrated native-Rust paths beyond the current route parser and Node smoke tests.
- Run a post-release reboot/logon verification pass against the shipped `v10.1.0` installer so the startup persistence flow is exercised once on a real reboot boundary.

--------------------------------------------------------------------------------

## 2026-03-14 - Release v9.0.9 (Output Console Polish)

**Done**
- Rebalanced the unified output console so the `Command Output` and `Ping & Tracert Output` panes fit better for daily use.
- Increased the visible height of the `Ping & Tracert Output` panel to make live ping logs easier to read.
- Tightened the overall console width balance after the previous expansion so the app layout feels more proportional.
- Improved the light-mode `Routing` chip colors and active state contrast for clearer tab switching.
- Updated release docs in `README.md` and bumped app versions to `9.0.9`.

**Next Steps**
- Run End-to-End testing of the standard user UI with local admin credentials.

## 2026-03-14 - Release v9.0.6 (Header Control Visual Sync)

**Done**
- Removed the inline `Status: LOCKED/UNLOCKED` text under the Repair Mode button to declutter the header.
- Restyled the Lock/Unlock button so Locked and Unlocked states are differentiated directly by button color.
- Moved the zoom controls into the primary header and updated their visual style to match the main action buttons.
- Removed the divider between the zoom and Repair Mode controls for a cleaner unified header group.
- Improved the light-mode `Command` chip so its blue color and label contrast are easier to read.
- Updated `DAILY_LOG.md` so NotebookLM can stay aligned with the current release work.

**Next Steps**
- Run End-to-End testing of the standard user UI with local admin credentials.

## 2026-03-14 - Release v9.0.5 (Responsive UI Zoom Controls)

**Done**
- Modified base `font-size` using `clamp()` for responsive auto-scaling on smaller screens like 14-inch laptops.
- Added zoom control buttons (`−` / `+`) to the footer, allowing users to fine-tune UI scaling from 75% to 120%.
- Saved user zoom preference persistently via `localStorage`.
- Bumped app versions to `9.0.5` across frontend and backend.
- Pushed release `9.0.5` to GitHub.

## 2026-03-14 - Release v9.0.4 (UI Layout refinements)

**Done**
- Moved the Lock/Unlock button into the primary header of the app to save vertical space.
- Removed the secondary top bar to ensure the app fits better on 14-inch laptop screens without being cut off at the bottom.
- Bumped app versions to `9.0.4` across the frontend and backend.
- Released version `9.0.4` on GitHub.

## 2026-03-14 - Release v9.0.3 (Repair Mode UI Simplification)

**Done**
- Simplified the Repair Mode UI for standard users.
- Removed the Target User selection dropdown, instead auto-selecting the active user profile in the background.
- Merged the "Lock Repair Mode" and "Unlock Repair Mode" buttons into a single toggle button.
- Verified that clicking the Unlock toggle properly calls the backend and displays the native Windows UAC prompt for Administrative credentials, removing the need for a custom password modal.
- Bumped app versions to `9.0.3` across the frontend and backend.
- Applied UI hotfix to ensure all buttons work while Unlocked by removing target SID validation, and repositioned the Unlocked status badge under the toggle button.
- Moved the Lock/Unlock button into the primary header and removed the secondary top bar, saving vertical space to properly fit 14-inch laptop screens.

**Next Steps**
- End-to-End testing of the standard user UI with local admin credentials.

## 2026-03-14 - Initialize Daily Log

**Goals**
- Set up a dedicated NotebookLM for Super Route Pro
- Establish a standalone daily log to track development

**Done**
- Created `DAILY_LOG.md` to track progress and decisions for Super Route Pro.

**Notes And Decisions**
- The project just completed the "Repair Service Migration" for standard user UI access.
- Future network tools and updates will be logged here.

**Next Steps**
- Add remaining project documentation to NotebookLM.
- Prepare for End-to-End testing of the standard user UI with local admin credentials.
