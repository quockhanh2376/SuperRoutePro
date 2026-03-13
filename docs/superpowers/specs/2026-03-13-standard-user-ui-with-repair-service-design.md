# Standard-User UI With Repair Service Design

**Date:** 2026-03-13

## Goal

Allow Super Route Pro to start reliably under a standard Windows 11 user session, and only use elevated privileges for machine-repair actions through a dedicated helper/service flow used by IT.

## Problem Summary

The current app elevates the entire Tauri process when admin privileges are required. In a standard-user session, that causes two problems:

1. The UI/WebView can fail during startup because the elevated process runs under a different account/profile than the interactive standard user.
2. User-profile-sensitive actions such as cache cleanup and AppX cleanup can target the wrong profile because the elevated process inherits the admin account context.

## Approved Direction

The app will be split into three responsibilities:

- **Super Route Pro UI**: Always runs as the interactive standard user. It never relaunches itself as admin.
- **Super Route Pro Repair Service**: Installed once with the app, runs as `LocalSystem`, and executes only a fixed set of whitelisted repair actions.
- **Repair Unlock Broker**: A tiny elevated broker executable launched only when IT unlocks Repair Mode for the current app session.

Repair Mode stays active until the UI app closes.

## Chosen UX

- The UI starts for a standard user without requiring admin.
- IT chooses a target Windows user/profile from the UI.
- IT unlocks Repair Mode once per app session using a local admin account on the machine.
- After unlock, admin actions run through the service without prompting again until the app closes.
- The app does not store admin passwords or cache credentials on disk.

## Functional Scope

### Non-admin UI actions

These remain in the standard-user Tauri process:

- read NIC list
- read routing table
- ping / tracert / nslookup / connectivity tests
- battery summary/report
- subnet scan
- service health checks
- target-user selection UI

### Admin actions through the service

These move behind the service boundary:

- add / delete / flush routes
- set default gateway
- WAN persist scheduled task management
- machine-level network reset actions
- restart adapters
- target-user profile cleanup
- AppX removal for the selected target user
- provisioned package removal for the whole machine

### Explicitly disallowed

- arbitrary shell execution from the UI
- passing raw filesystem paths from UI to the service for privileged actions
- storing reusable admin credentials in config files, app storage, or the registry

## Target User Model

- The UI will let IT pick a target Windows user.
- The UI sends a stable user identity such as `SID`.
- The service resolves the actual profile path and session state.
- Profile-sensitive cleanup runs against the resolved target profile, not against `%LOCALAPPDATA%` of the UI process or the service account.
- Provisioned package removal remains machine-wide but is still invoked from the same privileged request flow.

## Security Model

- The service runs elevated, but stays locked by default.
- Unlocking Repair Mode requires explicit IT action through the broker.
- The broker uses Windows elevation/credential flow so the app itself never handles stored admin secrets.
- The service only accepts typed, whitelisted commands over a local IPC channel.
- Repair sessions are in-memory only and expire when the UI disconnects or closes.
- If the UI is not unlocked, privileged calls return `requires_unlock`.

## Technical Shape

### UI

- stays `asInvoker`
- no startup admin gate
- shows `Repair Mode: Locked/Unlocked`
- disables privileged buttons when locked
- requires target-user selection for profile-sensitive actions

### Service

- Windows service running as `LocalSystem`
- local IPC endpoint using a Windows-friendly transport such as named pipes
- request validation and audit logging
- typed command handlers for each privileged action

### Broker

- minimal executable, launched with `runas`
- receives a one-time nonce/session request from the UI
- asks Windows to elevate for local admin credentials
- tells the service to unlock the current UI session
- exits immediately after unlock

## Migration Strategy

1. Remove startup-wide elevation from the Tauri app.
2. Keep the UI running as standard user and verify the WebView startup error is gone.
3. Add service skeleton and IPC health/status path.
4. Add unlock broker and Repair Mode session handling.
5. Migrate machine-level actions first.
6. Add target-user discovery and profile resolution.
7. Migrate profile cleanup and AppX/provisioned-package actions.
8. Remove raw-string admin command execution from the current design.

## Risks To Manage

- Incorrect target-user resolution could damage the wrong profile.
- IPC without strong request validation would create a privilege-escalation surface.
- AppX operations vary more than route/netsh tasks and need stronger test coverage.
- Installer work for service/broker registration is easy to get wrong across upgrade/uninstall paths.

## Validation Expectations

At minimum, the implementation must validate:

- standard user can open the UI with no admin prompt
- IT can unlock Repair Mode once and keep it until app close
- profile cleanup targets the selected user, not the admin profile
- AppX removal affects the selected user and provisioned packages as specified
- service locks again after app exit/disconnect
- upgrade/uninstall leaves no broken service registrations
