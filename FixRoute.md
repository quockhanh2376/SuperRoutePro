# FixRoute - Option A

## Goal

Keep the persisted WAN and custom routes aligned while the app is open.
This closes the gap where `SuperRouteService.exe` restores the routes once at logon and then exits.

## Root Cause

- Startup persistence is currently a run-once path.
- Windows can rewrite the active route table later because of DHCP renew, NIC reconnect, VPN changes, or sleep/wake.
- When that happens, the saved `persist.json` still exists, but no running process checks for drift.

## Chosen Option

Option A: run an in-app watcher while Super Route Pro is open.

## Design

1. Create a shared route-apply module so startup restore and in-app restore use the same logic.
2. Start a background watcher when the Tauri app starts.
3. Poll the active IPv4 route table on a fixed interval.
4. Compare the live table against `persist.json`.
5. If drift is seen twice in a row, re-apply the saved config.
6. Try direct apply first.
7. If direct apply fails because the app is not elevated, fall back to the existing elevated repair host when Repair Mode is already unlocked.
8. Stop the watcher when the app closes.

## Scope

- Protect the default route and saved custom routes while the app is open.
- Avoid re-applying if the current route table already matches the persisted config.
- Reuse existing repair IPC instead of inventing a second elevated channel.

## Caveat

If the app is not elevated and Repair Mode is locked, the watcher can detect drift but cannot silently fix it without an elevated helper. The implementation still tries the direct path first and then the repair host path.

## Implementation Checklist

- [x] Add shared persist apply module.
- [x] Add watcher module with drift detection and debounce.
- [x] Start watcher during app startup.
- [x] Stop watcher on window close.
- [x] Add elevated repair-host fallback path for runtime re-apply.
- [x] Keep startup binary on the same shared apply logic.
- [x] Add unit coverage for drift detection.

## Verification

- Build the Rust/Tauri app successfully.
- Confirm the watcher starts with the app and stops on shutdown.
- Confirm route drift detection stays quiet when live routes already match the saved config.
- Confirm drift triggers a restore attempt.