# Changelog

## v10.1.6 (2026-03-27)

### Highlights
- Executed the first `NeedToDo` delivery slices across backend, frontend, and tests: startup persistence now flows through one main path, repair actions use typed command helpers instead of ad-hoc shell strings, and shared Windows process/path helpers were centralized to reduce future maintenance risk.
- Expanded Speed Test substantially for the `10.1.6` line: added `Auto Australia`, moved the live run UI from generic progress bars to metric cards, and tightened the light/dark theme treatment so live status, metric labels, and hint text stay readable in both modes.
- Hardened networking and repair reliability: quoted adapter names like `Ethernet (Corp)` remain valid while shell chaining stays blocked, DHCP renew preserves the old `ipconfig /release && ipconfig /renew` semantics with real exit-code reporting and a 90-second end-to-end timeout budget, and NIC display names now stay stable across refreshes with targeted cache invalidation after adapter-affecting actions.
- Split the Tauri composition root and supporting backend seams for maintainability: `lib.rs` is now a thinner command/builder shell, internet probing moved into a dedicated module, dead non-repair command surfaces were removed, and startup task detection now treats mixed-case `schtasks` "missing task" output correctly across `stdout` and `stderr`.
- Extended regression coverage around the shipped seams, including persist round-trips, route-service behavior, repair broker flow, speed-test target contracts, NIC snapshot resolution, repair validation, startup task detection, connectivity probe versioning, and the refreshed Speed Test modal stages.

### Verification
- GitHub CI `validate-windows-build` passed for the current `10.1.6` release head before tagging.
- Re-ran `cargo test --manifest-path src-tauri/Cargo.toml` against a clean `CARGO_TARGET_DIR` to eliminate the local Windows target-dir lock noise seen in the long-lived workspace.
- Rebuilt the Windows NSIS installer successfully at `D:\\SuperRoutePro\\release-artifacts\\v10.1.6\\Super Route Pro_10.1.6_x64-setup.exe`.
- Collected the portable desktop binary and checksums alongside the installer at `D:\\SuperRoutePro\\release-artifacts\\v10.1.6`.

## v10.1.5 (2026-03-26)

### Highlights
- Added real Speed Test regional targets on top of the existing `Auto Asia` baseline: `JP/KR` now uses a fixed Tokyo backend, `US West` uses a fixed Los Angeles backend, and `EU` uses a fixed London backend.
- Split the backend measurement engine by provider semantics so Cloudflare auto-edge and regional LibreSpeed targets use the correct latency, download, upload, and public-IP lookup flows instead of sharing one fake target model.
- Tuned payload sizing per region to keep long-haul runs stable, then updated the modal copy and selector to reflect the real regional catalog now available in the desktop app.

### Verification
- Re-ran the full release gate on `10.1.5` with `npm run check`.
- Rebuilt the Windows NSIS installer successfully at `E:\\srprel-1015\\release\\bundle\\nsis\\Super Route Pro_10.1.5_x64-setup.exe`.
- Live-probed the Tokyo, Los Angeles, and London regional endpoints before shipping and completed a native desktop smoke pass that showed the new regional selector in the modal and a completed live regional run in runtime.

## v10.1.4 (2026-03-26)

### Highlights
- Restored richer NIC device naming in the main table without undoing the startup optimization: the app still paints from the fast network snapshot first, then asynchronously enriches adapter descriptions by interface index.
- Added the first Speed Test target-catalog foundation across backend and frontend, including `list_speed_test_targets`, target-aware result metadata, and a `Target Profile` section in the modal.
- Kept the shipped behavior honest by exposing one real catalog profile, `Auto Asia`, on top of the existing Cloudflare Asia auto-edge flow instead of faking country-pinned endpoints that do not exist yet.

### Verification
- Re-ran frontend/model verification for the release line with `npm run test:node` and `npm run build`.
- Re-ran Rust verification for the Speed Test slice with `cargo check` and `cargo test --manifest-path src-tauri/Cargo.toml speed_test` using fresh `E:\\` target directories because the working `D:\\` drive had hit a space ceiling.
- Built a fresh Windows NSIS installer successfully at `E:\\srprel-1014\\release\\bundle\\nsis\\Super Route Pro_10.1.4_x64-setup.exe`.

## v10.1.3 (2026-03-25)

### Highlights
- Added the Speed Test modal flow with native Tauri execution, browser demo fallback, clearer error formatting, and Cloudflare edge labeling for the selected test target.
- Fixed persisted WAN replay so only the chosen NIC owns the default route while NIC2/NIC3 keep their own specific routes after startup.
- Reduced startup delay when loading NICs and routes by removing `getmac` from the critical path and reusing faster snapshot/enumeration flows.

### Optimization And Reliability
- Restored the saved UI theme before first paint instead of always starting in the default theme.
- Moved `check_internet()` off the async runtime hot path with `spawn_blocking`.
- Batched persisted stable-NIC lookups, reused adapter enumeration during route replay, and consolidated Windows `CREATE_NO_WINDOW` usage through a shared module.

### Verification
- Added Rust regression coverage for speed test label resolution and fallback behavior.
- Added focused frontend/model tests for persisted-route shaping and Speed Test modal rendering.
- Re-ran the full release gate and produced fresh Windows release artifacts for this version.

## v10.1.1 (2026-03-23)

### Bug Fixes
- Tightened the active-only NIC filter so adapters must be up and have a real IPv4 address before they appear in the main NIC table.
- Rejected empty, `0.0.0.0`, and APIPA/link-local `169.254.x.x` values in the shared IPv4 validator used by NIC discovery.
- Hardened the network command whitelist so otherwise allowed diagnostics commands cannot append shell chaining, pipes, redirection, or grouped command syntax.

### Verification
- Added focused Rust regression coverage for the NIC active-only filter and whitelist hardening paths.
- Re-ran the full `npm run check` release gate before shipping.

## v10.1.0 (2026-03-22)

### ✨ Highlights
- Migrated the remaining practical PowerShell-dependent network flows to native Rust/cmd equivalents for NIC discovery, route parsing, cache cleanup, scheduled tasks, and TCP port testing.
- Hardened route parsing so IPv4 routes are mapped back to NIC interface indexes, preserving route-dependent UI behavior after the native migration.
- Updated installer packaging to stage both `SuperRouteRepairBroker` and `SuperRouteService`.
- Expanded `npm run check` so the release baseline now includes frontend build, Rust compile, Node tests, and Rust tests.

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
