# Standard-User UI With Repair Service Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make Super Route Pro start under a standard Windows user while moving all privileged repair actions behind a one-time-unlock repair service session used by IT.

**Architecture:** Keep the Tauri UI in the interactive user context and add a Windows service plus a tiny elevation broker. The UI talks to the service over typed IPC requests, the broker unlocks the service session once per app instance, and profile-sensitive actions are executed against a selected target user resolved by the service.

**Tech Stack:** Tauri v2, Rust, Windows service APIs or service wrapper crate, named pipes or equivalent local IPC, React 19 + TypeScript, NSIS/WiX installer config, PowerShell/Windows networking tools.

---

## File Structure

- Modify: `E:\SuperrRoutePro\src-tauri\src\lib.rs`
  - Remove startup-wide elevation, keep UI startup safe for standard users, and wire new local commands for service/session status.
- Modify: `E:\SuperrRoutePro\src-tauri\src\network.rs`
  - Split read-only commands from privileged actions and phase out raw-string admin execution.
- Modify: `E:\SuperrRoutePro\src\App.tsx`
  - Add Repair Mode UX, target-user picker, lock/unlock state, and route admin-only actions through the new local commands.
- Modify: `E:\SuperrRoutePro\src\api.ts`
  - Replace direct privileged invocations with typed service-backed API calls.
- Modify: `E:\SuperrRoutePro\src-tauri\build.rs`
  - Stop implying release builds embed an admin manifest for the UI process.
- Modify: `E:\SuperrRoutePro\src-tauri\super-route-pro.exe.manifest`
  - Keep the UI explicitly `asInvoker`.
- Modify: `E:\SuperrRoutePro\src-tauri\tauri.conf.json`
  - Keep the window/UI config aligned with the new startup flow and installer behavior.
- Modify: `E:\SuperrRoutePro\src-tauri\installer-hooks.nsh`
  - Install, upgrade, stop, start, and uninstall the repair service/broker correctly.
- Create: `E:\SuperrRoutePro\src-tauri\src\repair_protocol.rs`
  - Shared request/response/job/session/target-user types used by UI-side Rust code, broker, and service.
- Create: `E:\SuperrRoutePro\src-tauri\src\repair_ipc.rs`
  - Named-pipe server/client helpers and message framing.
- Create: `E:\SuperrRoutePro\src-tauri\src\repair_targets.rs`
  - Enumerate Windows profiles, resolve SID -> profile path, identify interactive user state.
- Create: `E:\SuperrRoutePro\src-tauri\src\repair_session.rs`
  - In-memory Repair Mode session lifecycle keyed to the current app instance/connection.
- Create: `E:\SuperrRoutePro\src-tauri\src\repair_actions.rs`
  - Typed privileged handlers for route fixes, network resets, cleanup, and AppX operations.
- Create: `E:\SuperrRoutePro\src-tauri\src\repair_service_main.rs`
  - Windows service entry point that hosts IPC, session management, and privileged actions.
- Create: `E:\SuperrRoutePro\src-tauri\src\repair_broker_main.rs`
  - Tiny broker that receives a nonce/session request, elevates via Windows, and unlocks the current app session.
- Create: `E:\SuperrRoutePro\src-tauri\tests\repair_protocol.rs`
  - Unit tests for protocol serialization, session state, and request validation.
- Create: `E:\SuperrRoutePro\src-tauri\tests\repair_targets.rs`
  - Unit tests for profile resolution helpers and path-safety checks.
- Create: `E:\SuperrRoutePro\src-tauri\tests\repair_session.rs`
  - Unit tests for lock/unlock/close/disconnect behavior.
- Create: `E:\SuperrRoutePro\docs\superpowers\specs\2026-03-13-standard-user-ui-with-repair-service-design.md`
  - Approved design baseline.

## Chunk 1: Make The UI Safe For Standard-User Startup

### Task 1: Add failing tests for the no-admin startup contract

**Files:**
- Modify: `E:\SuperrRoutePro\src-tauri\src\lib.rs`
- Test: `E:\SuperrRoutePro\src-tauri\src\lib.rs`

- [ ] **Step 1: Write the failing test**

Add a unit test that reads `tauri.conf.json` and `super-route-pro.exe.manifest`, then asserts the UI manifest stays `asInvoker` and no release-only config implies whole-app admin startup.

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test startup_contract --manifest-path src-tauri/Cargo.toml`
Expected: FAIL because the current startup path still forces admin behavior in `lib.rs`.

- [ ] **Step 3: Write minimal implementation**

Refactor `validate_runtime_environment` in `lib.rs` so standard-user startup is allowed and startup checks focus on OS/runtime prerequisites, not admin presence.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test startup_contract --manifest-path src-tauri/Cargo.toml`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/lib.rs src-tauri/tauri.conf.json src-tauri/super-route-pro.exe.manifest src-tauri/build.rs
git commit -m "refactor: allow standard-user ui startup"
```

### Task 2: Expose UI-visible service/session status from Rust

**Files:**
- Create: `E:\SuperrRoutePro\src-tauri\src\repair_protocol.rs`
- Create: `E:\SuperrRoutePro\src-tauri\src\repair_ipc.rs`
- Modify: `E:\SuperrRoutePro\src-tauri\src\lib.rs`
- Test: `E:\SuperrRoutePro\src-tauri\tests\repair_protocol.rs`

- [ ] **Step 1: Write the failing test**

Write tests that expect a serializable `RepairSessionStatus` and `RepairServiceHealth` contract with fields such as `locked`, `connected`, `target_sid`, and `requires_unlock`.

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test repair_protocol --manifest-path src-tauri/Cargo.toml`
Expected: FAIL because the types and plumbing do not exist yet.

- [ ] **Step 3: Write minimal implementation**

Add the shared protocol types plus placeholder IPC client methods that return predictable `service unavailable` state until the service is built.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test repair_protocol --manifest-path src-tauri/Cargo.toml`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/repair_protocol.rs src-tauri/src/repair_ipc.rs src-tauri/src/lib.rs src-tauri/tests/repair_protocol.rs
git commit -m "feat: add repair protocol and status scaffolding"
```

## Chunk 2: Build The Repair Service Skeleton

### Task 3: Add repair-session state with disconnect-based locking

**Files:**
- Create: `E:\SuperrRoutePro\src-tauri\src\repair_session.rs`
- Test: `E:\SuperrRoutePro\src-tauri\tests\repair_session.rs`

- [ ] **Step 1: Write the failing test**

Write tests for:
- locked by default
- unlock with app-instance id
- lock again on explicit close
- lock again on disconnect

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test repair_session --manifest-path src-tauri/Cargo.toml`
Expected: FAIL because no session manager exists.

- [ ] **Step 3: Write minimal implementation**

Create an in-memory session manager keyed by app instance id / connection id with methods like `unlock`, `lock`, `status`, and `on_disconnect`.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test repair_session --manifest-path src-tauri/Cargo.toml`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/repair_session.rs src-tauri/tests/repair_session.rs
git commit -m "feat: add repair session manager"
```

### Task 4: Stand up the Windows service entry point and health IPC

**Files:**
- Create: `E:\SuperrRoutePro\src-tauri\src\repair_service_main.rs`
- Modify: `E:\SuperrRoutePro\src-tauri\Cargo.toml`
- Modify: `E:\SuperrRoutePro\src-tauri\src\repair_ipc.rs`
- Test: `E:\SuperrRoutePro\src-tauri\tests\repair_protocol.rs`

- [ ] **Step 1: Write the failing test**

Add a client-side test for the IPC framing and a service-health request/response round-trip helper.

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test service_health --manifest-path src-tauri/Cargo.toml`
Expected: FAIL because there is no service main and no IPC server implementation.

- [ ] **Step 3: Write minimal implementation**

Add a new binary target for the service, stand up a named-pipe listener, and implement `GetServiceHealth` plus `GetRepairSessionStatus`.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test service_health --manifest-path src-tauri/Cargo.toml`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add src-tauri/Cargo.toml src-tauri/src/repair_ipc.rs src-tauri/src/repair_service_main.rs src-tauri/tests/repair_protocol.rs
git commit -m "feat: add repair service health endpoint"
```

## Chunk 3: Add Unlock Broker And Target-User Resolution

### Task 5: Add target-user/profile discovery

**Files:**
- Create: `E:\SuperrRoutePro\src-tauri\src\repair_targets.rs`
- Modify: `E:\SuperrRoutePro\src-tauri\src\lib.rs`
- Test: `E:\SuperrRoutePro\src-tauri\tests\repair_targets.rs`

- [ ] **Step 1: Write the failing test**

Write tests for safe SID validation, profile-path normalization, and rejecting non-profile-root paths.

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test repair_targets --manifest-path src-tauri/Cargo.toml`
Expected: FAIL because target-user helpers do not exist.

- [ ] **Step 3: Write minimal implementation**

Add helpers that enumerate user profiles, resolve SID to profile path, and expose typed target-user records to the UI.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test repair_targets --manifest-path src-tauri/Cargo.toml`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/repair_targets.rs src-tauri/src/lib.rs src-tauri/tests/repair_targets.rs
git commit -m "feat: add target-user discovery"
```

### Task 6: Add the elevation broker and unlock handshake

**Files:**
- Create: `E:\SuperrRoutePro\src-tauri\src\repair_broker_main.rs`
- Modify: `E:\SuperrRoutePro\src-tauri\Cargo.toml`
- Modify: `E:\SuperrRoutePro\src-tauri\src\repair_protocol.rs`
- Modify: `E:\SuperrRoutePro\src-tauri\src\repair_service_main.rs`
- Modify: `E:\SuperrRoutePro\src-tauri\src\lib.rs`
- Test: `E:\SuperrRoutePro\src-tauri\tests\repair_session.rs`

- [ ] **Step 1: Write the failing test**

Write a session test that expects an unlock request with a nonce to transition the session from `locked` to `unlocked`.

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test unlock_nonce --manifest-path src-tauri/Cargo.toml`
Expected: FAIL because the unlock handshake does not exist yet.

- [ ] **Step 3: Write minimal implementation**

Create the broker binary, define the nonce-bearing unlock message, and wire `unlock_repair_mode()` from the UI-side Rust code to launch the broker and complete the service handshake.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test unlock_nonce --manifest-path src-tauri/Cargo.toml`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add src-tauri/Cargo.toml src-tauri/src/repair_broker_main.rs src-tauri/src/repair_protocol.rs src-tauri/src/repair_service_main.rs src-tauri/src/lib.rs src-tauri/tests/repair_session.rs
git commit -m "feat: add repair unlock broker"
```

## Chunk 4: Move Machine-Level Repair Actions Behind The Service

### Task 7: Replace raw-string admin execution with typed service actions

**Files:**
- Create: `E:\SuperrRoutePro\src-tauri\src\repair_actions.rs`
- Modify: `E:\SuperrRoutePro\src-tauri\src\network.rs`
- Modify: `E:\SuperrRoutePro\src-tauri\src\repair_service_main.rs`
- Modify: `E:\SuperrRoutePro\src\api.ts`
- Modify: `E:\SuperrRoutePro\src\App.tsx`
- Test: `E:\SuperrRoutePro\src-tauri\tests\repair_protocol.rs`

- [ ] **Step 1: Write the failing test**

Write protocol tests for typed actions such as `ResetWinsock`, `ResetFirewall`, `AddRoute`, and `SetDefaultGateway`.

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test typed_actions --manifest-path src-tauri/Cargo.toml`
Expected: FAIL because admin actions are still raw-string and local.

- [ ] **Step 3: Write minimal implementation**

Move machine-level admin actions into `repair_actions.rs`, add service handlers for them, and update the frontend API so admin buttons call the service-backed Rust commands instead of raw command strings.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test typed_actions --manifest-path src-tauri/Cargo.toml`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/repair_actions.rs src-tauri/src/network.rs src-tauri/src/repair_service_main.rs src/api.ts src/App.tsx src-tauri/tests/repair_protocol.rs
git commit -m "refactor: move machine-level repairs behind service"
```

### Task 8: Add Repair Mode UX to the frontend

**Files:**
- Modify: `E:\SuperrRoutePro\src\App.tsx`
- Modify: `E:\SuperrRoutePro\src\App.css`
- Modify: `E:\SuperrRoutePro\src\api.ts`
- Test: `E:\SuperrRoutePro\src\App.tsx`

- [ ] **Step 1: Write the failing test**

Add frontend tests or minimal component-level checks for:
- locked badge shown by default
- unlock button visible
- privileged buttons disabled when locked
- target-user-required message for profile-sensitive actions

- [ ] **Step 2: Run test to verify it fails**

Run: `npm test -- App`
Expected: FAIL because the new Repair Mode UX does not exist yet.

- [ ] **Step 3: Write minimal implementation**

Add `Repair Mode: Locked/Unlocked`, target-user picker, unlock/lock controls, and disable-state wiring for privileged actions.

- [ ] **Step 4: Run test to verify it passes**

Run: `npm test -- App`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add src/App.tsx src/App.css src/api.ts
git commit -m "feat: add repair mode ui"
```

## Chunk 5: Move Profile Cleanup And AppX Workflows Behind The Service

### Task 9: Move cleanup actions to target-user-aware service handlers

**Files:**
- Modify: `E:\SuperrRoutePro\src-tauri\src\repair_actions.rs`
- Modify: `E:\SuperrRoutePro\src-tauri\src\network.rs`
- Modify: `E:\SuperrRoutePro\src\App.tsx`
- Modify: `E:\SuperrRoutePro\src\api.ts`
- Test: `E:\SuperrRoutePro\src-tauri\tests\repair_targets.rs`

- [ ] **Step 1: Write the failing test**

Write tests that expect cleanup request builders to reject missing `target_sid` and to resolve profile-sensitive paths from the service side only.

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test target_cleanup --manifest-path src-tauri/Cargo.toml`
Expected: FAIL because cleanup is still bound to process-local environment paths.

- [ ] **Step 3: Write minimal implementation**

Move cleanup recipes into service-backed handlers that use resolved target profile roots instead of `%LOCALAPPDATA%` from the calling process.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test target_cleanup --manifest-path src-tauri/Cargo.toml`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/repair_actions.rs src-tauri/src/network.rs src/App.tsx src/api.ts src-tauri/tests/repair_targets.rs
git commit -m "refactor: move profile cleanup behind service"
```

### Task 10: Move AppX and provisioned-package removal behind typed service requests

**Files:**
- Modify: `E:\SuperrRoutePro\src-tauri\src\repair_actions.rs`
- Modify: `E:\SuperrRoutePro\src-tauri\src\network.rs`
- Modify: `E:\SuperrRoutePro\src\App.tsx`
- Modify: `E:\SuperrRoutePro\src\api.ts`
- Test: `E:\SuperrRoutePro\src-tauri\tests\repair_protocol.rs`

- [ ] **Step 1: Write the failing test**

Write protocol and validation tests for:
- target-user-specific AppX removal requests
- `remove_provisioned = true`
- package whitelist validation

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test appx_removal --manifest-path src-tauri/Cargo.toml`
Expected: FAIL because AppX work is still tied to the current process model.

- [ ] **Step 3: Write minimal implementation**

Move AppX/provisioned-package logic into service handlers that require an unlocked repair session and a resolved target user.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test appx_removal --manifest-path src-tauri/Cargo.toml`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/repair_actions.rs src-tauri/src/network.rs src/App.tsx src/api.ts src-tauri/tests/repair_protocol.rs
git commit -m "refactor: move appx cleanup behind service"
```

## Chunk 6: Installer, Upgrade, And End-To-End Verification

### Task 11: Install and upgrade the service/broker correctly

**Files:**
- Modify: `E:\SuperrRoutePro\src-tauri\installer-hooks.nsh`
- Modify: `E:\SuperrRoutePro\src-tauri\tauri.conf.json`
- Test: manual install/upgrade checklist in repo notes

- [ ] **Step 1: Write the failing test**

Add a checklist-driven regression note in the plan implementation branch describing the exact installer expectations:
- fresh install registers the service
- upgrade stops old service and replaces binaries
- uninstall removes the service cleanly

- [ ] **Step 2: Run test to verify it fails**

Run: manual local installer test against a VM or throwaway machine
Expected: one or more failures before installer hooks are updated

- [ ] **Step 3: Write minimal implementation**

Update NSIS hooks and bundle configuration so install/upgrade/uninstall manage the service and broker binaries explicitly.

- [ ] **Step 4: Run test to verify it passes**

Run: manual installer test again on the same VM or throwaway machine
Expected: install/upgrade/uninstall all succeed cleanly

- [ ] **Step 5: Commit**

```bash
git add src-tauri/installer-hooks.nsh src-tauri/tauri.conf.json
git commit -m "feat: register repair service in installer"
```

### Task 12: Run the final verification matrix

**Files:**
- Modify: `E:\SuperrRoutePro\README.md`
- Modify: `E:\SuperrRoutePro\SETUP_GUIDE_VI.md`
- Modify: `E:\SuperrRoutePro\IMPLEMENTATION_SUMMARY_VI.md`

- [ ] **Step 1: Write the failing test**

Create a final checklist that includes:
- standard user opens app with no admin prompt
- IT unlocks Repair Mode once
- privileged buttons work while unlocked
- target-user cleanup hits the selected profile
- AppX/provisioned-package operations match the selected scope
- closing the app re-locks the session

- [ ] **Step 2: Run test to verify it fails**

Run: manual end-to-end matrix on Win11
Expected: identify any remaining unlock/target/installer gaps

- [ ] **Step 3: Write minimal implementation**

Fix the remaining documented gaps, then update README/setup docs to explain the new Repair Mode model for IT.

- [ ] **Step 4: Run test to verify it passes**

Run:
- `npm run check`
- `cargo test --manifest-path src-tauri/Cargo.toml`
- local Win11 standard-user test matrix
Expected: all automated checks pass and the manual matrix is green

- [ ] **Step 5: Commit**

```bash
git add README.md SETUP_GUIDE_VI.md IMPLEMENTATION_SUMMARY_VI.md
git commit -m "docs: update repair service workflow"
```
