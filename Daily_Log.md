# Super Route Pro Daily Log

This document is the running delivery log for Super Route Pro.
Update it after each meaningful work session so the team and NotebookLM stay aligned on current progress, decisions, blockers, and next steps.

--------------------------------------------------------------------------------

## 2026-04-01 - Released v10.1.12 Elevated Auto-Unlock + Route Service Promotion

**Done**
- Shipped automatic Repair Mode unlock for app sessions that are already running elevated, so administrator launches can execute repair-only actions without a second manual unlock step.
- Hotfixed the same `v10.1.12` line so a Microsoft or local account that belongs to `Administrators` but opens the UI with a standard UAC-filtered token now auto-prompts the broker/UAC path instead of remaining locked with no escalation attempt.
- Hotfixed the `Persist Startup Config` service install path so `sc.exe` now receives `binPath=`, `start=`, and `DisplayName=` in the argument shape Windows expects, fixing the `Invalid start= field` failure seen while enabling startup persistence.
- Promoted `SuperRouteService.exe` into a long-running Windows route service that keeps persisted WAN and custom routes aligned beyond startup rather than restoring them once at logon and exiting.
- Shared one route monitor/drift-signature layer between the background service and the in-app fallback watcher so route persistence behavior stays consistent across protection paths.
- Updated the installer hooks to stop the route service before updates and remove it on uninstall, then refreshed release-facing docs and README language to match the new persistence architecture.

**Verification**
- `npm run check`
- `npm run release:local -- -VersionTag v10.1.12 -SkipInstall`
- Built the Windows NSIS installer successfully at `D:\\SuperRoutePro\\release-artifacts\\v10.1.12\\Super Route Pro_10.1.12_x64-setup.exe`.
- Collected the portable desktop binary and checksums alongside the installer at `D:\\SuperRoutePro\\release-artifacts\\v10.1.12`.

**Next Steps**
- Validate `v10.1.12` on one real elevated admin session plus one standard-user session to confirm the new auto-unlock split behaves exactly as intended.
- NotebookLM sync is still pending on this workstation because no NotebookLM library entry or authenticated session is configured yet for Super Route Pro.

--------------------------------------------------------------------------------

## 2026-03-31 - Released v10.1.10 Route Watcher Runtime Persist Hardening

**Done**
- Implemented Option A for route persistence hardening: the desktop app now starts an in-app route watcher that polls the live IPv4 table, detects drift against `persist.json`, and re-applies the saved WAN plus custom routes while the app remains open.
- Reused one shared route-apply engine across both the login-time startup binary and the in-app watcher so persisted-route restore behavior stays aligned.
- Added frontend notifications for watcher restore/failure states, including a toast shortcut that opens the Routing Console when automatic recovery fails.
- Merged the route watcher work into `main`, fixed a release-blocking stale test reference in `package.json`, and shipped the patch release as `v10.1.10`.
- Verified that the GitHub release published the installer, portable binary, and checksums successfully.
- Fixed the release script afterward so future release commits also include `src-tauri/Cargo.lock` and no longer leave a dirty working tree after a version bump.

**Verification**
- `npm run check`
- `npm run release:ship -- patch`
- GitHub release `v10.1.10` published with 5 assets

**Next Steps**
- If product wants broader protection beyond “while app is open”, the next step is still the full Windows Service path (Option B).
- Consider adding a dedicated watcher event history panel if repeated route rewrites become a support/debugging concern.

--------------------------------------------------------------------------------

## 2026-03-31 - Speed Test Orb Polish + Route Persist Architecture Review

**Done**
- Continued Speed Test orb visual polish: orbit light effects, per-orb async animation speeds, colored head dots, enlarged orbs +15%, caption contrast improvements, dark/light mode dropdown fixes.
- Moved server selector inline with title row as a compact rounded-rectangle button.
- Fixed z-index stacking so dropdown renders above metric circles.
- All 4 metrics (Download, Upload, Ping, Stability) now start from 0.0 during active test.
- Reviewed the full route persistence architecture (`route_persist.rs`, `persist_startup.rs`, `route_service_main.rs`, `persist_commands.rs`, `win32_net.rs`).
- Identified root cause for WAN switching despite "Persist on startup": `SuperRouteService.exe` is a run-once binary that applies routes at ONLOGON then exits, leaving routes unprotected against DHCP renewal, NIC reconnect, VPN changes, and sleep/wake cycles.
- Documented three solution options: (A) background watcher thread in app, (B) proper Windows Service, (C) hybrid approach.

**Verification**
- `npm run build`
- Full backend code review of 5 core Rust files for routing system

**Next Steps**
- Decide on route persistence hardening approach (Option A recommended for v10.2).
- Implement route table change monitoring via `NotifyRouteChange2()` or polling.
- Consider proper Windows Service for v11 long-term route protection.

--------------------------------------------------------------------------------

## 2026-03-30 - Released v10.1.9 Speed Test Orb Dashboard Polish

**Done**
- Promoted the post-`v10.1.8` Speed Test UI follow-up into the shipped `v10.1.9` line after the full validation gate completed cleanly.
- Reworked both the launch card and the dedicated modal around circular orb metrics so `Download`, `Upload`, `Ping`, and `Stability` now share the same 2x2 presentation instead of the previous rectangular stat tiles.
- Added live count-up behavior inside the orb UI during active test runs, then settled the final measurements into the same display model for the end-state snapshot.
- Refined the visual treatment across both themes with stronger multi-ring borders, theme-specific glow tuning, lighter in-orb numeric weight, simplified launch-card chrome, and a wider `Server` / `Public IP` info box aligned to the orb cluster.

**Verification**
- `npm run check`
- `npm run release:local -- -VersionTag v10.1.9 -SkipInstall`
- Built the Windows NSIS installer successfully at `D:\\SuperRoutePro\\release-artifacts\\v10.1.9\\Super Route Pro_10.1.9_x64-setup.exe`.
- Collected the portable desktop binary and checksums alongside the installer at `D:\\SuperRoutePro\\release-artifacts\\v10.1.9`.

**Next Steps**
- Monitor the GitHub release flow for `v10.1.9`, then use the next thin line for any further Speed Test visual tuning rather than widening scope beyond presentation and delivery polish.

--------------------------------------------------------------------------------

## 2026-03-30 - Speed Test Launch Badges Enlarged Slightly

**Done**
- Increased the round launch-badge diameter slightly after review so the icon treatment reads more comfortably without changing the approved 2x2 metric layout.
- Kept the current light-theme polish direction intact; this pass only adjusted badge scale and spacing rather than reworking colors or structure again.
- Pushed the follow-up sizing pass to `main` in commit `a4ab77d`.

**Verification**
- `npx tsx --test tests/SpeedTestModal.test.tsx`
- `npm run build`

**Next Steps**
- If product wants one more visual pass, focus on numeric emphasis or card whiteness next instead of growing the badges further.

--------------------------------------------------------------------------------

## 2026-03-30 - Speed Test Launch Metrics Switched To Round Badges

**Done**
- Reworked the four Speed Test launch summary metrics from rectangular stat cards into round-badge rows so the latest result snapshot now matches the approved direction more closely.
- Kept the existing launch-card structure from `v10.1.8` intact: the refreshed `Download`, `Upload`, `Ping`, and `Jitter / Stability` badges still sit above the unchanged `Server` / `Public IP` row.
- Added `release-artifacts/` to `.gitignore` so local release bundles no longer appear as untracked noise during follow-up work after a ship pass.
- Pushed the UI follow-up to `main` in commit `629bb8c`.

**Verification**
- `npx tsx --test tests/SpeedTestModal.test.tsx`
- `npm run build`

**Next Steps**
- If product wants a tighter match to the lighter mockup, the next UI pass should tune badge diameter, icon stroke weight, and light-theme contrast rather than reworking the layout again.

--------------------------------------------------------------------------------

## 2026-03-30 - Pushed v10.1.8 Release Commit And Tag

**Done**
- Pushed the `v10.1.8` release commit on `main` after the full validation gate and local release build had already completed successfully.
- Pushed the annotated `v10.1.8` tag to `origin` so the GitHub release workflow can build/publish from the tagged release commit.
- Kept this note as a docs-only follow-up on `main`, matching the repo pattern where the release tag can point at the release commit immediately before the post-push logging update.

**Next Steps**
- Watch the GitHub release workflow for `v10.1.8` and confirm the installer, portable binary, and checksums publish cleanly.

--------------------------------------------------------------------------------

## 2026-03-30 - Released v10.1.8 Speed Test UI Refresh

**Done**
- Fast-forwarded local `main` to the newer `origin/main` UI head before finalizing the next release line.
- Promoted the post-`v10.1.7` frontend polish into the shipped `v10.1.8` line.
- The in-run Speed Test flow now uses greener active-state accents, a slimmer animated progress fill, and shorter stage/status copy so live measurement state reads faster.
- The modal/result layout now promotes summary metrics and `Server` / `Public IP` details into a clearer top section while keeping the target selector as a compact side panel.
- The launch card now mirrors richer result metadata, long progress messages are condensed into payload-sized status text, and the `Ping & Tracert Output` clear action now uses a dedicated themed button style.
- Updated the release-facing docs and version files to `10.1.8`, including `package.json`, `package-lock.json`, `src-tauri/Cargo.toml`, `src-tauri/Cargo.lock`, and `src-tauri/tauri.conf.json`.
- Built fresh release artifacts for `v10.1.8`, including the NSIS installer, portable binary, and checksums.

**Verification**
- `npm run check`
- `npm run release:local -- -VersionTag v10.1.8 -SkipInstall`

**Next Steps**
- Push the `10.1.8` release commit and annotated `v10.1.8` tag to `origin`.
- Watch the GitHub release workflow for the new `v10.1.8` tag and confirm installer/assets publish cleanly.

--------------------------------------------------------------------------------

## 2026-03-29 - Released v10.1.7 Thin Cleanup And Speed Test Stabilization

**Done**
- Released `v10.1.7` as a thin cleanup/hardening line on top of `v10.1.6`.
- Shipped the backend maintainability slices in:
  - `repair_actions.rs`
  - `app_bootstrap.rs`
  - `windows_paths.rs`
  - `network.rs`
- Kept Australia scoped to `Auto Australia` only because no validated city-pinned AU backends are wired into the shipped catalog yet.
- Stabilized `Auto Australia` by moving the default Cloudflare download payload away from the `16 MB` path that reproduced `HTTP 403` during local runtime testing.
- Polished the Speed Test modal by tightening the live metric-card layout, softening the live typography, shrinking the summary-card values, and increasing light-mode error contrast.
- Updated release-facing docs and version files for `10.1.7`.

**Verification**
- `npm run check`
- `npm run release:build`

**Next Steps**
- Watch the GitHub release workflow for the `v10.1.7` tag and confirm installer/assets publish cleanly.
- Revisit true city-pinned AU targets only after real backend endpoints are sourced and verified in runtime.

--------------------------------------------------------------------------------

## 2026-03-29 - Speed Test Modal Polish And AU Scope Decision

**Done**
- Tightened the live metric-card layout by reducing the desktop width of the `Live Throughput` card so the in-run dashboard feels less visually top-heavy.
- Reduced the live in-run typography by roughly 15% and removed the heavier bold treatment so the metric cards read less aggressively in both dark mode and light mode.
- Reviewed the shipped AU target catalog and confirmed the current backend only supports `Auto Australia` as a Cloudflare preferred-region profile.
- Chose to keep `Auto Australia` as the current product scope because there are still no validated city-pinned AU backends wired into `speed_test_targets.rs`.
- Kept the current final summary identity baseline (`Target / Provider / Region`) as the default post-run metadata set; revisit only if product wants more detail.

**Notes And Decisions**
- If product later wants city-pinned AU targets, the next step is sourcing and validating real backend endpoints first, then adding catalog entries only after runtime verification succeeds.
- This pass is UI/product-direction polish only; it does not change Speed Test backend behavior.

**Verification**
- `npm exec -- tsx --test tests/SpeedTestModal.test.tsx`
- `npm run build`

--------------------------------------------------------------------------------

## 2026-03-29 - Thin Network Helper Cleanup

**Done**
- Reduced repeated small helper state in `src-tauri/src/network.rs` by centralizing:
  - the DHCP timeout label builder
  - adapter enable/disable fallback copy
  - the allowed diagnostic command prefix catalog
- Kept runtime behavior unchanged; this slice only moved repeated constants/strings into shared helpers.
- Refreshed local dependencies with `npm ci` after discovering the local `tsx` binary was missing from `node_modules`, then re-ran the full repo validation gate successfully.

**Verification**
- `cargo test --manifest-path src-tauri/Cargo.toml --lib network`
- `npm run check`

**Next Steps**
- If we keep pushing `10.1.7` as a thin cleanup release, the next pass should still stay helper-only and avoid changing command/runtime behavior in `network.rs`.

--------------------------------------------------------------------------------

## 2026-03-29 - Thin Bootstrap And Windows Path Cleanup

**Done**
- Reduced repeated WebView directory creation logic in `src-tauri/src/app_bootstrap.rs` by centralizing the directory-create error formatting into one helper.
- Reduced repeated environment-path trimming logic in `src-tauri/src/windows_paths.rs` by centralizing normalized env-value handling.
- Kept behavior unchanged: this slice only removes duplication in low-risk helper code.

**Verification**
- `cargo test --manifest-path src-tauri/Cargo.toml --lib app_bootstrap`
- `cargo test --manifest-path src-tauri/Cargo.toml --lib windows_paths`

**Next Steps**
- Re-check whether any remaining bootstrap/path cleanup is still worth extracting, but keep the next pass equally thin.

--------------------------------------------------------------------------------

## 2026-03-29 - Thin Repair Actions Cleanup

**Done**
- Reduced duplicate request-validation flow in `src-tauri/src/repair_actions.rs` by centralizing the validated cleanup-target and Appx-package selection steps.
- Kept the behavior unchanged while removing repeated sanitize/selection work from:
  - profile cleanup validation + execution
  - Appx removal validation + execution
- Simplified `cleanup_plan_for_profile_root(...)` by removing an unnecessary intermediate `Vec` allocation during path flattening.

**Verification**
- `cargo test --manifest-path src-tauri/Cargo.toml --lib --test repair_protocol --test repair_targets`

**Next Steps**
- Continue the thin `10.1.7` cleanup line by reviewing remaining bootstrap/path helpers for similarly low-risk reductions.

--------------------------------------------------------------------------------

## 2026-03-29 - Daily Log Consolidation

**Done**
- Reviewed both daily log files before cleanup.
- Confirmed `Daily_Log2.md` was empty and did not contain any newer project history.
- Consolidated the running project narrative into this single canonical file: `Daily_Log.md`.
- Kept the richer, newer history already present in the main log so the latest release and planning context remain preserved.

**Next Steps**
- Continue updating only `Daily_Log.md` going forward.

--------------------------------------------------------------------------------

## 2026-03-27 - Speed Test Final Summary Metadata Clarified

**Done**
- Added an explicit `region_label` to the Speed Test target catalog and runtime result model so the frontend no longer has to infer region identity from server text.
- Updated the Speed Test modal result view to surface a dedicated `Target / Provider / Region` identity strip above the final throughput summary cards.
- Kept lower-level run metadata (`Server`, `Public IP`, `Timestamp`) in the secondary details grid instead of mixing identity and diagnostic fields together.
- Brought the frontend fallback catalog back in line with the shipped runtime catalog by restoring `Auto Australia` and attaching region labels to all fallback/demo entries.
- Extended regression coverage across the frontend demo/tests and the Rust target-catalog contract to keep the new metadata path stable.

**Notes And Decisions**
- This slice stays product-facing and intentionally does not add an Australia city selector yet; it only makes the currently shipped regional routing metadata clearer to the user.
- `region_label` is now part of the public frontend/backend contract, which is cleaner than overloading `server_label` for both identity and server-detail display.

**Verification**
- `cargo test --manifest-path src-tauri/Cargo.toml --test speed_test_targets_contract`
- `cargo check --manifest-path src-tauri/Cargo.toml`
- `tsx --test tests/SpeedTestModal.test.tsx`
- `npm run build`

**Next Steps**
- If Zon wants another product-facing pass here, the next logical slice is deciding whether `Auto Australia` remains the only AU option or expands into city-pinned targets.

--------------------------------------------------------------------------------

## 2026-03-27 - Release Finalization For v10.1.6

**Done**
- Realigned `CHANGELOG.md` so the `v10.1.6` entry now reflects the actual branch head being shipped, including the NeedToDo backend cleanup slices, `Auto Australia`, the live Speed Test metric cards, NIC/DHCP hardening, and the latest review follow-up fixes.
- Merged `feature/speed-test-modal-v1` into `main` locally as the release candidate path for `v10.1.6`, instead of tagging an outdated pre-follow-up release commit.
- Reconfirmed the release head against the same practical gates used to decide ship readiness:
  - GitHub PR CI had already passed on the current release head
  - local `npm run tauri -- build` produced the `10.1.6` NSIS installer successfully
  - full Rust tests passed again when re-run against a clean `CARGO_TARGET_DIR`

**Notes And Decisions**
- The long-lived local workspace still shows intermittent Windows target-dir linker locks during `npm run check`, but the clean-target rerun and remote CI both passed, so the release decision is based on clean-run behavior rather than stale local build state.
- `main` is being treated as the source of truth for the public `v10.1.6` tag, even though the release workflow technically allows tagging a feature branch.

**Verification**
- `npm run tauri -- build`
- `cargo test --manifest-path src-tauri/Cargo.toml` with a clean `CARGO_TARGET_DIR`
- GitHub PR CI `validate-windows-build`

**Next Steps**
- Push the merged `main` branch, tag `v10.1.6` from the merge line, and let the GitHub release workflow publish the installer, portable binary, and checksums.

--------------------------------------------------------------------------------

## 2026-03-27 - Released v10.1.6 From Main

**Done**
- Re-checked the remote release state before publish: GitHub still stopped at `v10.1.5`, so `10.1.6` had not been cut yet.
- Confirmed the `10.1.6` release head on `feature/speed-test-modal-v1` already carried the expected shipped scope:
  - startup/persist + repair wiring cleanup
  - `Auto Australia` Speed Test support
  - live Speed Test metric-card UI polish
  - NIC description stabilization and cache invalidation
  - review follow-up fixes around startup-task detection and connectivity probe versioning
- Promoted the branch cleanly onto `main` with merge commit `67b5b24358a49c94d7f2d0c95a279287d003a402`.
- Pushed annotated tag `v10.1.6` from that merge commit so the GitHub release workflow can build and publish from the same state now living on `main`.

**Notes And Decisions**
- I used a clean temporary worktree from `origin/main` for the promotion because the long-lived local worktree had a noisy `Cargo.lock` version diff from older branch state.
- The merge/tag now reflect the release-ready branch head without carrying that local worktree noise into `main`.
- `Daily_Log.md` is intentionally logged after the tag push as a docs-only follow-up, so `main` stays current even though the release tag itself points at the merge commit immediately before this note.

**Verification**
- `git push origin HEAD:main`
- `git tag -a v10.1.6 -m "v10.1.6"`
- `git push origin v10.1.6`

**Next Steps**
- Watch the GitHub `Release Windows Installer` workflow for the new `v10.1.6` tag and confirm the release assets publish cleanly.

--------------------------------------------------------------------------------

## 2026-03-27 - Review Follow-Up (Startup Task Detection + Probe Version Metadata)

**Done**
- Tightened `src-tauri/src/persist_startup.rs` so missing-task detection now inspects combined `stdout + stderr` instead of only `stderr`.
- The same startup-task detection now normalizes message casing before checking the known `schtasks` missing-task markers, so `Cannot Find` / `cannot find` no longer split behavior.
- Added small unit coverage in `persist_startup.rs` for:
  - mixed-case missing-task text from `stdout`
  - missing-task text from `stderr`
  - unrelated hard errors that must still surface as `Err`
  - combined output formatting
- Replaced the hardcoded connectivity probe user-agent in `src-tauri/src/connectivity_probe.rs` with a compile-time string derived from `env!("CARGO_PKG_VERSION")`.
- Added probe coverage to assert the user-agent always tracks the package version and retains the expected prefix/suffix.
- Re-verified that the `ProgramData` panic-safety review item remains covered by the existing drop-based guard in `src-tauri/tests/persist_config_roundtrip.rs`, so no extra patch was needed there.

**Notes And Decisions**
- This patch stays deliberately narrow: no behavior changes to startup registration itself, only the error classification path around `schtasks`.
- The task-missing heuristic is now more robust for case and output stream placement, but it is still English-string based because `schtasks` does not expose a better structured contract here.
- The connectivity probe version source is now tied directly to Cargo package metadata, so future release bumps do not need a second manual string edit.

**Verification**
- `cargo test --manifest-path src-tauri/Cargo.toml persist_startup --lib`
- `cargo test --manifest-path src-tauri/Cargo.toml connectivity_probe --lib`
- `cargo check --manifest-path src-tauri/Cargo.toml`

**Next Steps**
- Keep pushing review fixes as isolated slices like this so the branch stays easy to audit and cherry-pick.

--------------------------------------------------------------------------------

## 2026-03-27 - Review Follow-Up (Test Guard + Stage-Aware Speed Status)

**Done**
- Tightened `src-tauri/tests/persist_config_roundtrip.rs` so the `ProgramData` override now restores reliably via a small RAII guard instead of relying on the last lines of the test body.
- The same guard also cleans up the temporary test directory on drop so the roundtrip test stops leaving temp artifacts behind.
- Updated `src/SpeedTestModalView.tsx` so the live Speed Test status card is stage-aware instead of always showing `Streaming`.
- The live target/provider card now reuses the resolved display labels that already back the rest of the modal.
- Added frontend coverage in `tests/SpeedTestModal.test.tsx` for the stage-aware live status wording.

**Notes And Decisions**
- The env-var review was a real test-hygiene issue because a panic before the restore block could leak `ProgramData` into later tests in the same process.
- The Speed Test tweak is intentionally copy-only and keeps the same layout; it just stops showing a misleading status during `preflight`, `latency`, and `finalize`.
- A separate review note about PR scope is workflow-related rather than a runtime/code bug, so I did not force unrelated code churn into this patch.

**Verification**
- `cargo test --manifest-path src-tauri/Cargo.toml --test persist_config_roundtrip`
- `npm run test:node`
- `npm run build`

**Next Steps**
- Keep resolving review feedback this way: patch real code/test issues directly, and handle scope-only PR comments through branch/PR hygiene rather than mixing them into runtime files.

--------------------------------------------------------------------------------

## 2026-03-27 - NeedToDo Slice 14 (Shared Windows Path Helpers)

**Done**
- Added a small internal `src-tauri/src/windows_paths.rs` module so Windows path resolution now lives in one place instead of being split across multiple files.
- Moved the shared `%SystemRoot%` and `%ProgramData%` resolution logic into that module with the same fallback behavior as before:
  - `C:\\Windows`
  - `C:\\ProgramData`
- Added one more helper there for current-user profile root resolution so the cleanup path can derive the profile consistently from:
  - `LOCALAPPDATA`
  - or `USERPROFILE`
  - with the same `C:\\Users\\Default` fallback
- Switched `src-tauri/src/cache_cleanup.rs` to use the shared helper module instead of keeping its own local env/path resolution functions.
- Switched `src-tauri/src/route_persist.rs` to use the same shared `%ProgramData%` helper for `persist.json` location resolution.

**Notes And Decisions**
- This slice keeps the scope intentionally narrow: only Windows path resolution moved, with no command, cleanup target, or persist schema behavior changes.
- `CREATE_NO_WINDOW` was already centralized earlier, so path helpers were the cleanest remaining Windows-specific maintenance seam.
- I kept the new module internal (`mod windows_paths`) to avoid widening the public crate surface before there is a real need.

**Verification**
- `cargo check --manifest-path src-tauri/Cargo.toml`
- `cargo test --manifest-path src-tauri/Cargo.toml windows_paths::tests --lib`
- `cargo test --manifest-path src-tauri/Cargo.toml cache_cleanup::tests --lib`
- `cargo test --manifest-path src-tauri/Cargo.toml route_persist::tests --lib`

**Next Steps**
- The next thin slice can continue shaving Windows-specific duplication, or pivot back to another low-risk backend seam now that command surface and path resolution are both cleaner.

--------------------------------------------------------------------------------

## 2026-03-27 - NeedToDo Slice 13 (Remove Dead Non-Repair Command Surface)

**Done**
- Removed the unused non-repair bloatware/cache cleanup command surface from the current app runtime.
- Deleted the old frontend invoke wrappers from `src/api.ts`:
  - `removeBloatware(...)`
  - `clearCacheTargets(...)`
- Removed the matching Tauri command registration from `src-tauri/src/lib.rs`.
- Deleted the old command handlers from `src-tauri/src/network.rs`:
  - `remove_bloatware`
  - `clear_cache_targets`
- Updated `PROJECT_SUMMARY.md` so it no longer documents the dead command names and now points at the repair-mediated command flow instead.

**Notes And Decisions**
- The live app was already using only the repair variants from `App.tsx`, so this was a cleanup slice rather than a behavior change for current UI users.
- I kept `get_bloatware_candidates()` intact because the Remove Apps modal still depends on that discovery path.
- This slice intentionally removes only the dead public surface that had drifted behind the broker/repair architecture, without touching the still-active repair implementations in `repair_commands.rs`.

**Verification**
- `cargo check --manifest-path src-tauri/Cargo.toml`
- `npm run build`
- `npm run test:node`

**Next Steps**
- A good next thin slice is to centralize one more small Windows helper seam, or to continue trimming duplicated repair/runtime documentation now that the old command surface is gone.

--------------------------------------------------------------------------------

## 2026-03-27 - NeedToDo Slice 12 (Env-Driven Cleanup System Paths)

**Done**
- Landed another low-risk backend maintenance slice in `src-tauri/src/cache_cleanup.rs`.
- Replaced the remaining hardcoded system cleanup roots with small helpers that resolve from the environment first:
  - `SystemRoot` for `Windows\\Temp`, `SoftwareDistribution\\Download`, and `Prefetch`
  - `ProgramData` for the machine-level `Microsoft\\Windows\\WER` path
- Kept the runtime behavior intact:
  - target tokens did not change
  - cleanup labels did not change
  - cleanup execution still flows through the existing helper paths
- Added pure unit coverage for the new path-resolution helpers so this stays deterministic without mutating process-global environment variables during tests.

**Notes And Decisions**
- This is one of the cleaner `NeedToDo.md` quick wins because it reduces hardcoded Windows assumptions without widening any privileged behavior.
- I kept the slice inside `cache_cleanup.rs` instead of creating a new shared path module yet; the helper count is still small enough that a wider extraction would have been more churn than value.
- The environment-aware helpers now line up better with the existing `ProgramData` handling already used by `route_persist.rs`.

**Verification**
- `cargo test --manifest-path src-tauri/Cargo.toml cache_cleanup::tests --lib`
- `cargo check --manifest-path src-tauri/Cargo.toml`

**Next Steps**
- The next thin slice can keep shaving backend debt by removing more low-risk duplication, or pivot to one of the remaining `NeedToDo.md` runtime items once the current regression net is strong enough.

--------------------------------------------------------------------------------

## 2026-03-27 - NeedToDo Slice 11 (Repair Target Pure-Logic Coverage)

**Done**
- Added another low-risk backend regression slice focused on the pure helper layer in `src-tauri/src/repair_targets.rs`.
- Added unit coverage for:
  - accepting realistic SID formats used by Windows user profiles
  - rejecting malformed SID inputs
  - normalizing valid `X:\Users\<name>` profile roots while uppercasing the drive letter
  - rejecting traversal, non-user roots, UNC paths, and deeper nested paths
- Kept this slice strictly test-only:
  - no registry access behavior changed
  - no target enumeration behavior changed
  - no repair command wiring changed

**Notes And Decisions**
- `repair_targets.rs` is a good place to add pure-logic tests because it sits under privileged profile-repair flows but does not need any Windows registry mocking for these specific seams.
- Protecting `validate_target_sid()` and `normalize_profile_root()` gives the higher-level repair/profile cleanup path a safer foundation without widening the runtime surface.
- I kept the tests focused on deterministic helper logic rather than trying to fake the full registry-backed target enumeration path.

**Verification**
- `cargo check --manifest-path src-tauri/Cargo.toml`
- `cargo test --manifest-path src-tauri/Cargo.toml --lib --test nic_snapshot_resolution --test persist_config_roundtrip --test repair_broker_flow --test route_service_behavior --test speed_test_targets_contract`

**Next Steps**
- The next thin slice can either keep tightening repair/profile coverage, or return to a small runtime refactor in `repair_actions.rs` with the new guardrails in place.

--------------------------------------------------------------------------------

## 2026-03-27 - NeedToDo Slice 10 (Repair Action Validation Coverage)

**Done**
- Added a thin Rust-only regression slice around `src-tauri/src/repair_actions.rs`, which still carried important validation and planning logic without direct test coverage.
- Added unit coverage for:
  - profile cleanup request validation rejecting invalid SIDs
  - profile cleanup request validation rejecting unknown cleanup targets
  - profile cleanup plan expansion for a real target profile root
  - profile cleanup plan rejection when the resolved target user does not match the request SID
  - Appx removal request validation accepting known package names case-insensitively
  - Appx removal request validation rejecting empty/unknown package selections
- Kept this slice test-only:
  - no runtime behavior changed
  - no PowerShell command text changed
  - no broker/session wiring changed

**Notes And Decisions**
- `repair_actions.rs` is still one of the higher-risk backend modules because it combines request validation, privileged action orchestration, and user-facing error shaping.
- I chose coverage over another refactor here because the current code paths are already sensitive and the missing regression net was the bigger quality gap.
- These tests protect the inputs and plan-shaping layers without trying to mock privileged PowerShell execution.

**Verification**
- `cargo check --manifest-path src-tauri/Cargo.toml`
- `cargo test --manifest-path src-tauri/Cargo.toml --lib --test nic_snapshot_resolution --test persist_config_roundtrip --test repair_broker_flow --test route_service_behavior --test speed_test_targets_contract`

**Next Steps**
- The next thin slice can return to refactoring `repair_actions.rs` with more confidence now that its validation seams are covered, or pivot to another NeedToDo area.

--------------------------------------------------------------------------------

## 2026-03-27 - NeedToDo Slice 9 (NIC Snapshot Resolution Test Seam)

**Done**
- Added a focused integration-style regression seam for the NIC description path that has been a real source of churn in this codebase.
- Exposed a tiny test-only helper from `src-tauri/src/network_snapshot.rs` so integration tests can exercise the snapshot interface shaping without widening the production API.
- Added `src-tauri/tests/nic_snapshot_resolution.rs` to lock two behaviors together:
  - active snapshot rows surface the enriched adapter description instead of generic friendly aliases like `Ethernet 2`
  - the snapshot-derived NIC identifier still resolves cleanly through the startup route service lookup path
- Kept the slice intentionally narrow and test-first:
  - no runtime behavior changed
  - no frontend code changed
  - no new production command surface was added

**Notes And Decisions**
- This slice targets the exact seam called out in `NeedToDo.md`: protecting the network snapshot + stable NIC resolution behavior with a regression test that crosses module boundaries.
- I kept the helper test-only so production code does not gain extra surface area just to satisfy integration coverage.
- The test data intentionally mirrors the kind of enriched-vs-generic adapter naming that previously caused the UI to bounce between `Ethernet N` and richer vendor descriptions.

**Verification**
- `npm run test:node`
- `cargo check --manifest-path src-tauri/Cargo.toml`
- `cargo test --manifest-path src-tauri/Cargo.toml --lib --test nic_snapshot_resolution --test persist_config_roundtrip --test repair_broker_flow --test route_service_behavior --test speed_test_targets_contract`

**Next Steps**
- The next thin slice can stay backend-only and focus on the remaining repair orchestration hot path, or switch to a small AU speed-test validation slice if product wants that track resumed.

--------------------------------------------------------------------------------

## 2026-03-27 - NeedToDo Slice 8 (Repair Helper Dedup Follow-Up)

**Done**
- Applied another thin backend cleanup slice around `src-tauri/src/repair_actions.rs` and the shared process helper module.
- Moved the single-quoted PowerShell string escaping helper into `src-tauri/src/process_exec.rs` so repair and network code now share one escaping rule instead of carrying local copies.
- Replaced the repeated `session_status.locked` guard blocks in `src-tauri/src/repair_actions.rs` with one small local helper that returns the standard `RepairCommandResult::locked()` payload.
- Kept behavior intentionally unchanged:
  - no machine action routing changed
  - no PowerShell payload text changed
  - no repair unlock semantics changed
- Added a focused unit test in `src-tauri/src/process_exec.rs` so the shared PowerShell single-quote escaping contract stays protected.

**Notes And Decisions**
- This is deliberately a helper-hygiene slice, not a machine-action behavior slice.
- The goal was to remove small but high-churn duplication from the repair path so future fixes do not have to remember multiple local escape helpers or manually keep the locked-state payload in sync.
- `repair_actions.rs` still owns the higher-level repair orchestration; this pass only trims repeated plumbing.

**Verification**
- `npm run test:node`
- `cargo check --manifest-path src-tauri/Cargo.toml`
- `cargo test --manifest-path src-tauri/Cargo.toml --lib --test persist_config_roundtrip --test repair_broker_flow --test route_service_behavior --test speed_test_targets_contract`

**Next Steps**
- The next thin backend slice should either add a focused NIC-resolution integration seam, or continue shrinking the remaining repair orchestration hotspots.

--------------------------------------------------------------------------------

## 2026-03-27 - NeedToDo Slice 7 (Extract Connectivity Probe Module)

**Done**
- Continued the backend cleanup track with another thin slice focused on reducing the size and responsibility of `src-tauri/src/network.rs`.
- Moved the new internet connectivity probe implementation out of `src-tauri/src/network.rs` into a dedicated `src-tauri/src/connectivity_probe.rs` module.
- Kept the shipped behavior intact:
  - `check_internet()` still returns the same `Result<bool, String>` contract to the frontend
  - the sequential `Microsoft Connect Test -> Cloudflare trace` fallback model is unchanged
  - the short per-probe timeout and captive-portal-resistant response classification remain unchanged
- Moved the probe-specific matcher tests with the code so future changes to the connectivity path stay isolated from route/DHCP/cache-cleanup tests in `network.rs`.
- Added the new module to `src-tauri/src/lib.rs` so the composition root owns the probe slice explicitly instead of leaving it buried inside the larger network command module.

**Notes And Decisions**
- This slice intentionally did not widen scope into richer UI diagnostics or probe-source reporting. The goal here was module boundary cleanup only.
- Keeping probe tests beside the probe module makes the remaining `network.rs` test block easier to scan because it now focuses on command shaping, DHCP semantics, and validator behavior.
- This is a maintainability slice rather than a behavior slice, but it continues the broader NeedToDo goal of shrinking backend hotspots into more legible feature modules.

**Verification**
- `npm run test:node`
- `cargo check --manifest-path src-tauri/Cargo.toml`
- `cargo test --manifest-path src-tauri/Cargo.toml --lib --test persist_config_roundtrip --test repair_broker_flow --test route_service_behavior --test speed_test_targets_contract`

**Next Steps**
- If we keep slicing the backend hotspot down, the next good candidate is the remaining repair helper cleanup path in `repair_actions.rs`, or a focused NIC-resolution integration test slice.

--------------------------------------------------------------------------------

## 2026-03-27 - NeedToDo Slice 6 (Explicit Internet Probe Model)

**Done**
- Executed the next thin backend slice from `NeedToDo.md` by replacing the ambiguous raw TCP internet check in `src-tauri/src/network.rs`.
- Reworked `check_internet()` away from `8.8.8.8:53` and onto an explicit HTTP probe model with two small endpoints:
  - `http://www.msftconnecttest.com/connecttest.txt`
  - `https://speed.cloudflare.com/cdn-cgi/trace`
- Added explicit probe classification helpers so connectivity is judged by both HTTP status and expected response body shape:
  - Microsoft probe must return `200 OK` plus exact body `Microsoft Connect Test`
  - Cloudflare fallback must return `200 OK` plus expected trace fields (`h=speed.cloudflare.com` and `ip=...`)
- Added a dedicated reqwest client builder for the connectivity probe path with:
  - explicit per-probe timeout kept short so the sequential fallback stays close to the old online/offline polling feel
  - explicit user agent
  - redirect disabled so redirected captive-portal-like responses do not get treated as success
- Added pure unit coverage for the new response-classification helpers instead of introducing any live-network tests into the suite.
- Tightened the matcher coverage further so later refactors do not accidentally relax:
  - non-`200` status rejection
  - trimmed Microsoft payload acceptance
  - Cloudflare trace acceptance with whitespace and case variation
- Live-validated the two probe endpoints from the current dev machine before landing the change:
  - Microsoft endpoint returned `200` with `Microsoft Connect Test`
  - Cloudflare endpoint returned `200` with trace output including `h=speed.cloudflare.com` and `ip=...`

**Notes And Decisions**
- This slice stayed backend-only and intentionally did not change the frontend polling cadence in `App.tsx`.
- The per-target timeout was kept at `2s` so the sequential fallback does not noticeably slow the existing online monitor compared with the old raw socket probe.
- The goal was clarity and maintainability, not captive-portal detection. The new model is still a simple online/offline signal, but it now checks explicit HTTP semantics instead of inferring internet access from a single TCP socket.
- Keeping a Cloudflare fallback means environments that block one probe path still have a second lightweight check before the app reports offline.

**Verification**
- `npm run test:node`
- `cargo check --manifest-path src-tauri/Cargo.toml`
- `cargo test --manifest-path src-tauri/Cargo.toml --lib --test persist_config_roundtrip --test repair_broker_flow --test route_service_behavior --test speed_test_targets_contract`

**Next Steps**
- If we keep going on this track, the next thin slice could decide whether the online monitor should surface probe source/failure reason for debugging, or whether the current boolean signal is enough.

--------------------------------------------------------------------------------

## 2026-03-27 - NeedToDo Slice 5 (Cache Cleanup Service Helper Dedup)

**Done**
- Executed a very small backend-only follow-up slice on top of the shared process-helper work already on the branch.
- Centralized the remaining fire-and-forget hidden service-control calls in `src-tauri/src/cache_cleanup.rs`:
  - added a small local `run_service_control_command()` wrapper
  - routed that wrapper through `process_exec::run_hidden_output_blocking`
  - replaced the duplicated `net stop/start` `CREATE_NO_WINDOW` blocks used around `windows_update_cache` cleanup
- Kept the cleanup behavior intentionally unchanged:
  - service stop/start remains fire-and-forget
  - cleanup success still depends on filesystem deletion results, not on parsing `net` command output

**Notes And Decisions**
- This slice stayed deliberately narrow because slice 4 had already consolidated the more important shared process helpers.
- I did not widen this pass into timeout-policy or result-shaping changes for cache cleanup commands; the goal here was dedupe only.
- With this pass, the obvious hidden `net` service-control duplication is gone from the cache cleanup flow without widening review risk.

**Verification**
- `cargo check --manifest-path src-tauri/Cargo.toml`
- `cargo test --manifest-path src-tauri/Cargo.toml --lib --test persist_config_roundtrip --test repair_broker_flow --test route_service_behavior --test speed_test_targets_contract`

**Next Steps**
- Decide whether the next thin backend slice should revisit `check_internet()` probe semantics or stop the helper cleanup track here.

--------------------------------------------------------------------------------

## 2026-03-27 - NeedToDo Slice 4 (Process Helper Consolidation Follow-Up)

**Done**
- Executed another thin backend-only cleanup slice to reduce the remaining duplicated command execution wrappers.
- Consolidated `src-tauri/src/network.rs` onto the shared `src-tauri/src/process_exec.rs` process helpers:
  - removed the local copies of `run_process_blocking`
  - removed the local copies of `run_cmd_blocking`
  - removed the local copies of `run_powershell_blocking`
  - removed the local copies of the async `run_powershell`
  - switched the module to the shared timeout constants from `process_exec`
- Tightened the remaining wrapper duplication without changing behavior policy:
  - `src-tauri/src/repair_actions.rs` still keeps its own PowerShell result-shaping semantics, but now reuses the shared hidden output helper for the low-level spawn/output step
  - `src-tauri/src/route_service_main.rs` now also reuses the shared hidden output helper for `route` execution while preserving its own stdout/stderr shaping
- Left intentionally out of scope for this slice:
  - `cache_cleanup.rs` fire-and-forget `net stop/start` calls
  - any larger redesign of `network.rs` command-result shaping
  - any timeout-policy changes for repair cleanup PowerShell scripts

**Notes And Decisions**
- `repair_actions.rs` was not switched to the timeout-aware `run_powershell_blocking` helper because that would silently introduce a new timeout policy where none existed before.
- `network.rs` was safe to move onto `process_exec` because the helper implementations and timeout constants were effectively duplicated already.
- This slice stayed backend-only and touched just three files so the review stays easy to reason about.

**Verification**
- `npm run test:node`
- `npm run build`
- `cargo check --manifest-path src-tauri/Cargo.toml`
- `cargo test --manifest-path src-tauri/Cargo.toml --lib --test persist_config_roundtrip --test repair_broker_flow --test route_service_behavior --test speed_test_targets_contract`

**Next Steps**
- If we keep going on command-helper cleanup, the next low-risk seam is deciding whether `cache_cleanup.rs` service stop/start calls deserve their own small shared wrapper.
- The larger future question is whether `network.rs` should keep owning command-result shaping or whether more of that can move into a typed shared layer without obscuring behavior.

--------------------------------------------------------------------------------

## 2026-03-27 - NeedToDo Slice 3 (NIC Cache Invalidation + Hidden Command Helper Cleanup)

**Done**
- Executed a thin follow-up slice from `NeedToDo.md` focused on two concrete seams:
  - stale NIC adapter cache invalidation for manual refresh
  - duplicated hidden-command helpers outside `process_exec`
- Coordinated two short sub-agent audits before editing:
  - one reviewed the safest invalidation contract for manual refresh without hurting startup performance
  - one reviewed the smallest useful helper-centralization scope
- Added shared hidden-command helpers in `src-tauri/src/process_exec.rs`:
  - `run_hidden_output_blocking`
  - `run_hidden_stdout_blocking`
- Migrated duplicate `CREATE_NO_WINDOW + output()` helpers onto the shared process layer:
  - `src-tauri/src/win32_net.rs`
  - `src-tauri/src/persist_startup.rs`
  - `src-tauri/src/repair_commands.rs`
- Added explicit NIC cache invalidation in `src-tauri/src/win32_net.rs`:
  - new `invalidate_adapter_cache()`
  - expired cache entries now clear themselves instead of lingering invisibly
  - added unit coverage proving invalidation clears a recent snapshot
- Added a dedicated Tauri command in `src-tauri/src/network_snapshot.rs` and registered it in `src-tauri/src/lib.rs`:
  - `invalidate_network_adapter_cache`
- Wired the UI/API to use the new invalidation path selectively:
  - `src/api.ts` now exposes `invalidateNetworkAdapterCache()`
  - `src/App.tsx` `loadData()` now accepts an `invalidateNicCache` option
  - manual NIC refresh now invalidates adapter cache before pulling a fresh snapshot
  - `RenewDhcpLease` and `RestartActiveAdapters` now also request NIC cache invalidation before their post-action refresh
- Kept route-only refresh flows cache-friendly:
  - add/delete/flush route
  - set default gateway
  - `activeOnly` toggles
  - initial app load

**Notes And Decisions**
- This slice intentionally did not change the default snapshot path; `get_network_snapshot` and `get_network_interfaces` still stay cache-friendly unless the caller explicitly invalidates first.
- The helper centralization stopped at the duplicated hidden `.output()` pattern only. Timeout-aware process helpers in `network.rs` and the distinct PowerShell cleanup wrapper in `repair_actions.rs` were left alone for later slices.
- The UI invalidation path is explicit and selective so startup stays fast while manual refresh and NIC-changing repair actions can force a true adapter re-read.

**Verification**
- `npm run test:node`
- `npm run build`
- `cargo check --manifest-path src-tauri/Cargo.toml`
- `cargo test --manifest-path src-tauri/Cargo.toml --lib --test persist_config_roundtrip --test repair_broker_flow --test route_service_behavior --test speed_test_targets_contract`

**Next Steps**
- Continue later with the remaining `NeedToDo.md` backend cleanup around `win32_net` behavior and any further command-helper dedupe that does not widen review risk too far.
- Consider a later thin pass to decide whether some post-repair refresh paths beyond DHCP/adapter restart also deserve forced NIC invalidation.

--------------------------------------------------------------------------------

## 2026-03-27 - NeedToDo Slice 2 (lib.rs Split + Command/Bootstrap Wiring)

**Done**
- Executed the second thin maintenance slice from `NeedToDo.md`, focused on making `src-tauri/src/lib.rs` a real composition root without changing shipped behavior.
- Coordinated two sub-agent audits before editing:
  - one reviewed the lowest-risk extraction boundaries for `lib.rs`
  - one reviewed the smallest useful regression net so the slice stayed small and testable
- Extracted startup/runtime validation plus WebView bootstrap logic into `src-tauri/src/app_bootstrap.rs`:
  - runtime environment validation
  - startup block/error dialog path
  - main window setup
  - WebView2 data-directory recovery helpers
  - existing startup/bootstrap unit tests moved with the bootstrap code
- Extracted persist-facing Tauri commands into `src-tauri/src/persist_commands.rs`:
  - `persist_save_config`
  - `persist_load_config`
  - `persist_get_nic_stable_id`
  - `persist_get_nic_stable_ids`
- Extracted repair-facing Tauri commands into `src-tauri/src/repair_commands.rs`:
  - repair session/status commands
  - unlock/lock commands
  - all `repair_*` command wrappers
  - broker elevation launch helper
  - main-window close handler used to relock repair mode
- Reduced `src-tauri/src/lib.rs` down to module declarations, imports, the Tauri builder chain, and the `generate_handler!` registration list.

**Notes And Decisions**
- This slice intentionally did not move `tauri::generate_handler!()` or `tauri::generate_context!()` out of `lib.rs`; keeping them there avoids extra macro/type indirection while still achieving the composition-root goal.
- No new behavior was introduced in the command handlers. This was a move-only refactor for ownership and maintainability.
- The refactor was kept to four runtime files (`lib.rs` plus three new modules) to make review and rollback straightforward.

**Verification**
- `npm run test:node`
- `cargo check --manifest-path src-tauri/Cargo.toml`
- `cargo test --manifest-path src-tauri/Cargo.toml --lib --test persist_config_roundtrip --test route_service_behavior --test repair_broker_flow --test speed_test_targets_contract`

**Next Steps**
- Continue the remaining `NeedToDo.md` backend cleanup by splitting the remaining bootstrap/invoke concerns only if there is a clear ownership seam beyond this composition-root slice.
- Revisit `win32_net` cache invalidation and any remaining command-helper centralization in a later thin pass.

--------------------------------------------------------------------------------

## 2026-03-27 - NeedToDo Slice 1 (Startup Unification + Repair Hardening + AU/UI/Test Pass)

**Done**
- Read through `NeedToDo.md` again and executed the first broad delivery slice across backend, Speed Test UI, AU target groundwork, and regression coverage.
- Consolidated startup persistence onto the persisted-config path:
  - the app no longer calls the legacy WAN-only startup task APIs
  - `src-tauri/src/persist_startup.rs` now owns startup task registration plus cleanup of obsolete WAN-only artifacts
  - `src/App.tsx`, `src/api.ts`, `src/persistStartupModel.ts`, `tests/persistStartupModel.test.ts`, and `tests/persistFlow.test.ts` were updated so the checkbox now resolves from persisted config state instead of the split legacy/new task model
- Replaced generic repair shell-string usage with typed helpers in `src-tauri/src/network.rs` and wired them through `src-tauri/src/repair_actions.rs`:
  - `FlushDns`
  - `RenewDhcpLease`
  - `ClearArpCache`
  - `ResetTcpIp`
  - `ResetWinsock`
  - `ResetFirewall`
  - `ResetWinHttpProxy`
  - `RestartActiveAdapters` now routes through typed adapter enable/disable helpers instead of shell-building `netsh` commands
- Deduplicated backend catalog / result glue:
  - moved the Windows Appx allowlist into `src-tauri/src/bloatware_catalog.rs`
  - both `network.rs` and `repair_actions.rs` now use the shared catalog
  - `RepairCommandResult` in `src-tauri/src/repair_protocol.rs` now has shared constructors plus `From<network::CommandResult>` so the repair layer no longer hand-rolls the same conversion repeatedly
- Clarified repair architecture in shipped code:
  - removed the dead-end `SuperRouteRepairService` binary entry from `src-tauri/Cargo.toml`
  - deleted `src-tauri/src/repair_service_main.rs`
  - the runtime path is now explicitly broker-based (`SuperRouteRepairBroker`) plus the route replay sidecar (`SuperRouteService`)
- Added Speed Test AU groundwork in backend:
  - `src-tauri/src/speed_test_targets.rs` now exposes `Auto Australia` (`auto_au`) with preferred Cloudflare AU colos `SYD`, `MEL`, `BNE`, `PER`, `ADL`
  - `src-tauri/src/speed_test.rs` no longer hardcodes Asia-only route-fit wording for Cloudflare auto-edge labels and messages
- Integrated the frontend Speed Test redesign from the `frontend_phase2` subagent:
  - `src/SpeedTestModalView.tsx` and `src/SpeedTestModal.css` now use live metric cards during active runs instead of a progress-bar-first presentation
  - the target copy is neutralized so multi-region catalogs like `Auto Australia` fit without UI copy regressions
- Integrated the backend regression coverage from the `integration_flow_tests` subagent:
  - `src-tauri/tests/persist_config_roundtrip.rs`
  - `src-tauri/tests/route_service_behavior.rs`
  - `src-tauri/tests/repair_broker_flow.rs`
  - `src-tauri/tests/speed_test_targets_contract.rs`
  - plus small test-only seams in `src-tauri/src/route_service_main.rs` and `src-tauri/src/repair_broker_main.rs`
- Tightened one smaller backend cleanup item from the list:
  - `check_internet()` in `src-tauri/src/network.rs` no longer uses `unwrap()` on the probe socket address

**Notes And Decisions**
- This slice intentionally stopped short of the deeper `lib.rs` modular split because the runtime correctness / architectural cleanup items above were higher value and easier to verify without widening blast radius too far in one pass.
- `Auto Australia` is implemented as a Cloudflare preferred-region profile only. True city-pinned AU targets remain pending until compatible LibreSpeed-style AU backends are confirmed live.
- The route replay service (`SuperRouteService`) stays in place because it is the actual shipped startup replay worker; only the unused repair service skeleton was removed.
- A later rerun of `cargo test` hit an environmental `os error 32` lock on the `SuperRouteRepairBroker` test binary, but the full suite had already completed successfully once earlier in this same session before that lock reappeared.

**Verification**
- `npm run test:node`
- `npm run build`
- `cargo check --manifest-path src-tauri/Cargo.toml`
- `cargo test --manifest-path src-tauri/Cargo.toml`

**Next Steps**
- Continue the remaining backend maintainability work from `NeedToDo.md`, especially the deeper `lib.rs` split and any remaining command-helper centralization.
- Decide whether to rename `RepairService*` protocol types/messages toward `RepairHost*` in a later cleanup pass so naming fully matches the now-explicit broker architecture.
- If product still wants true AU city targets beyond `Auto Australia`, source and validate real AU backends before hardcoding any city-pinned endpoints.

--------------------------------------------------------------------------------

## 2026-03-27 - Released v10.1.6 With Repair Command Hardening

**Done**
- Confirmed the remote fix branch is up to date: `origin/fix-nic-active-filter` now points at `e9174b9`, which contains the post-review DHCP renew semantics patch.
- Cherry-picked the DHCP semantics hardening into the active release branch as `3354f33` so `feature/speed-test-modal-v1` includes the same repair-command fixes before cutting the next release.
- Fixed `src-tauri/src/network.rs` so DHCP renew now:
  - derives success from the actual child process exit status
  - stops before `/renew` when `/release` fails, matching the earlier `&&` behavior
  - preserves the existing 90-second end-to-end timeout budget instead of silently shrinking the operation to 30 seconds per step
  - keeps user-facing timeout text aligned with the original combined command flow
- Bumped the app from `10.1.5` to `10.1.6` across `package.json`, `package-lock.json`, `src-tauri/Cargo.toml`, `src-tauri/Cargo.lock`, and `src-tauri/tauri.conf.json`.
- Updated `CHANGELOG.md` so the `v10.1.6` release line records the repair-command hardening, runtime polish already on the branch, and the exact local artifact path used for this release build.
- Re-ran the full release gate successfully with `npm run check`.
- Built fresh local release artifacts successfully at `D:\SuperRoutePro\release-artifacts\v10.1.6`:
  - `Super Route Pro_10.1.6_x64-setup.exe`
  - `SuperRoute.exe`
  - `SHA256SUMS.txt`

**Notes And Decisions**
- `feature/speed-test-modal-v1` had already carried the earlier validator fix (`f35db38`) but not the follow-up semantics patch from the dedicated fix branch, so the release line needed one more cherry-pick before the version bump.
- The full project gate is now green on the `10.1.6` tree, including frontend build, Node tests, Rust tests, repair/session protocol tests, and installer packaging checks.
- The release artifacts are intentionally kept out of git; the repo only tracks the version/doc updates, while the built installer and portable binary live under `release-artifacts\v10.1.6`.

**Next Steps**
- Push the `10.1.6` release commit on `feature/speed-test-modal-v1` so the remote branch matches the local artifact build.
- If this should become the outward-facing release, create and push the `v10.1.6` tag after final human smoke acceptance.

--------------------------------------------------------------------------------

## 2026-03-26 - Tauri Dev Runner Cargo PATH Fallback

**Done**
- Fixed `npm run tauri -- dev` / `npm run tauri -- info` on Windows shells that do not have Rust in `PATH`.
- Updated `scripts/run-tauri.mjs` so the Tauri launcher now detects `C:\Users\<user>\.cargo\bin\cargo.exe` and prepends that folder to the spawned Tauri process environment automatically.
- Added coverage in `tests/run-tauri.test.mjs` for:
  - local cargo-bin resolution from the user profile
  - PATH prepending without mutating the original env
  - the existing normalized cwd / local CLI invocation contract
- Verified the fix with:
  - `npm run test:node`
  - `npm run tauri -- info`

**Notes And Decisions**
- The root problem was not Tauri itself; the Node wrapper launched correctly, but the spawned Tauri CLI inherited a shell environment where `cargo` was missing from `PATH`, so `cargo metadata` failed immediately.
- This fix is scoped to the Tauri runner path, so standard PowerShell sessions still behave normally outside the repo.

**Next Steps**
- If we want the same resilience for other Rust npm scripts later, we can wrap `check:rust` / `test:rust` behind the same PATH-bootstrap strategy.

--------------------------------------------------------------------------------

## 2026-03-26 - NIC Card Name Stabilization

**Done**
- Fixed the NIC table regression where device names could bounce between generic friendly aliases like `Ethernet 2` / `Ethernet 3` and the richer adapter names such as `Broadcom NetXtreme Gigabit Ethernet` or `Realtek PCIe GbE Family Controller`.
- Frontend snapshot handling in `src/App.tsx` now stabilizes incoming NIC rows against the previously enriched list instead of blindly replacing the table with a fresh generic snapshot.
- Added description-preference helpers in `src/nicDescriptionModel.ts` so the app prefers richer adapter descriptions over generic aliases and keeps the selected NIC aligned with the stabilized NIC list.
- Backend snapshot reads in `src-tauri/src/network_snapshot.rs` now prefer the fresh enriched adapter cache from `src-tauri/src/win32_net.rs` when available, so refreshes stop regressing to friendly aliases after the first stable-ID enrichment pass.
- Added Node coverage in `tests/nicDescriptionModel.test.ts` for the new rules that reject generic replacements once a richer NIC description is known.

**Notes And Decisions**
- The root cause was not just the async enrich step itself; the bigger problem was that each fresh snapshot replaced the current NIC list with `enumerate_adapters_basic()` output before the later enrich pass restored the richer descriptions.
- The fix keeps startup responsiveness while eliminating the repeated flip-flop on refresh and reload within the running session.

**Next Steps**
- If we still want to remove the very first-session friendly-name flash completely on cold launch, the next step would be a dedicated backend snapshot API that returns stable descriptions in one call without a second frontend enrich phase.

--------------------------------------------------------------------------------

## 2026-03-26 - Optimisation Program Slice (Baseline + Backend Split + UI/Test Follow-Up)

**Done**
- Captured a fresh local baseline for the current Windows machine before continuing the optimization work:
  - `route print -4` ~= `421.96 ms`
  - `netsh interface ipv4 show interfaces` ~= `93.28 ms`
  - `netsh interface ipv4 show addresses` ~= `105.17 ms`
  - `getmac /fo csv /v /nh` ~= `985.11 ms`
- Logged the current React shell pressure points and followed through on two more frontend decomposition slices already pushed on `feature/speed-test-modal-v1`:
  - `ed115d6` `refactor: split app chrome and log hooks`
  - `88b1975` `refactor: split app modal UI`
- Reduced the frontend root further:
  - `src/App.tsx` now sits at `2601` lines instead of staying near the old `~3k` line hot path.
  - Large battery and IP-scan modal JSX moved out into `src/components/BatteryModal.tsx` and `src/components/IpScanModal.tsx`.
- Continued the Rust modularization program:
  - added `src-tauri/src/process_exec.rs` for shared hidden-process execution and timeout helpers
  - added `src-tauri/src/ping.rs` and switched ping/fping commands to the extracted module
  - added `src-tauri/src/battery.rs` and moved battery-report / battery-summary commands out of `network.rs`
  - added `src-tauri/src/cache_cleanup.rs` and switched repair/profile cleanup logic onto the shared cleanup path
  - added `src-tauri/src/network_snapshot.rs` and moved NIC snapshot / route-table read logic out of `network.rs`
- Reduced the backend hotspot materially:
  - `src-tauri/src/network.rs` is now `915` lines
  - snapshot/routing read responsibilities now live in `src-tauri/src/network_snapshot.rs` (`331` lines)
- Optimized the NIC enrich path in `src-tauri/src/win32_net.rs`:
  - cached the recent basic adapter snapshot
  - reused cached `netsh` enumeration and layered `getmac` metadata only when stable-description enrichment is requested
  - added tests covering `getmac` metadata parsing and cache-based adapter enrichment
- Added a new persist flow contract test slice:
  - `386a589` `test: add persist flow contract coverage`
  - new `tests/persistFlow.test.ts` covers NIC enrichment -> persist config shaping and startup-state resolution
- Fixed the native ICMP path in the new ping module so the Windows handle is closed once via `IcmpCloseHandle`, not double-closed.
- Verified the integrated tree with:
  - `C:\Users\ADMVN\.cargo\bin\cargo.exe check --manifest-path src-tauri/Cargo.toml`
  - `npm run test:node`
  - `npm run build`

**Notes And Decisions**
- The `getmac` pass remains the expensive part of NIC enrichment on this machine, so the optimization keeps startup on `enumerate_adapters_basic()` and reuses cached basic adapter data for later stable-ID enrichment instead of re-running the full `netsh + netsh + getmac` chain.
- The ping/IP-scan redesign is now split into a dedicated backend module, but the frontend orchestration flow still has room for a deeper hook split if we want to keep shrinking `App.tsx`.
- `network.rs` is no longer the single home for snapshot read, battery, ping/fping, and cleanup helpers; route mutation, diagnostics, and bloatware remain there for now.
- NotebookLM could not be updated directly from this session because local NotebookLM MCP health is currently `authenticated=false` and the library is empty. `Daily_Log.md` remains the source-of-truth file for the next notebook sync.

**Next Steps**
- Continue the `network.rs` split by extracting route mutation / diagnostics / bloatware catalog if we want the file to stop acting as the remaining Rust façade.
- Consider a deeper frontend hook split for ping/IP-scan orchestration after the modal/UI extraction settles.
- When NotebookLM auth/library is restored locally, sync this `Daily_Log.md` entry into the notebook source set.

--------------------------------------------------------------------------------

## 2026-03-26 - Frontend App.tsx Decomposition Slice 2

**Done**
- Split the large `App.tsx` modal blocks into focused frontend components:
  - `src/components/BatteryModal.tsx`
  - `src/components/IpScanModal.tsx`
- Moved battery formatting helpers into `src/batteryUtils.ts` so the battery summary status text and modal rendering share the same logic.
- Kept the existing `App.tsx` behavior intact while reducing its render responsibility and removing the bulky battery/IP scan JSX from the composition root.
- Verified the frontend slice with `npm run test:node` and `npm run build`.

**Notes And Decisions**
- The extraction stayed on the frontend only, per scope, and did not touch Rust, package metadata, or test files.
- IP scan rendering now owns its own row sorting and counters inside the modal component, while `App.tsx` keeps only the orchestration state and handlers.
- NotebookLM still consumes this `Daily_Log.md` file as the source-of-truth for session sync.

**Next Steps**
- Continue decomposing `App.tsx` if more low-risk UI clusters remain.
- Fold any remaining shared presentation helpers into small reusable frontend modules only when there is a clear duplicate use case.

--------------------------------------------------------------------------------

## 2026-03-26 - Released v10.1.5 With Real Regional Speed Test Targets

**Done**
- Bumped the app from `10.1.4` to `10.1.5` across `package.json`, `package-lock.json`, `src-tauri/Cargo.toml`, `src-tauri/Cargo.lock`, and `src-tauri/tauri.conf.json`.
- Ran the full release gate on the `10.1.5` tree with `npm run check`.
- Built a fresh Windows NSIS installer successfully at `E:\\srprel-1015\\release\\bundle\\nsis\\Super Route Pro_10.1.5_x64-setup.exe`.
- Updated `CHANGELOG.md` so the `v10.1.5` release line records the shipped regional Speed Test catalog, target-aware backend engine, and verification path used for the release build.

**Notes And Decisions**
- `v10.1.5` is the first release where the Speed Test selector is backed by real non-Cloudflare regional endpoints instead of only the `Auto Asia` catalog foundation.
- The release still keeps the scope intentionally curated: only the regional backends that were live-probed successfully from the current environment made it into the shipped catalog.
- NotebookLM still cannot be written directly from the current toolset, so this `Daily_Log.md` update remains the source file for notebook refresh/re-sync.

**Next Steps**
- Monitor the `v10.1.5` tag workflow and verify the GitHub release assets publish cleanly.
- If product wants to move even closer to `speedtest.net`, the next slice should add manual server selection inside each region rather than expanding the fixed regional catalog blindly.

--------------------------------------------------------------------------------

## 2026-03-26 - Real Regional Speed Test Backends Added And Smoke Checked

**Done**
- Extracted the Speed Test target catalog into `src-tauri/src/speed_test_targets.rs` and split the backend model into:
  - `CloudflareAutoEdge` for `Auto Asia`
  - `LibreSpeedRegional` for the fixed regional targets
- Added three real regional targets to the catalog:
  - `JP/KR` -> `Tokyo, Japan (A573)`
  - `US West` -> `Los Angeles, United States (Clouvider)`
  - `EU` -> `London, England (Clouvider)`
- Updated the native Speed Test engine so each backend kind now uses the correct request semantics:
  - Cloudflare stays on `__down?bytes=...`, `__up?bytes=...`, and Cloudflare trace parsing
  - regional LibreSpeed targets use `empty.php`, `garbage.php?ckSize=...`, raw-body uploads, and JSON `getIP.php`
- Added target-aware payload sizing so long-haul regional tests do not inherit the old `24 MB` Cloudflare default.
- Updated the modal selector/copy to surface the real regional catalog and stop implying the feature was only a future foundation.
- Verified the slice with:
  - `npm run test:node`
  - `npm run build`
  - `cargo check --manifest-path src-tauri/Cargo.toml --target-dir E:\\srp-speedtest-v1015-check`
  - `cargo test --manifest-path src-tauri/Cargo.toml speed_test --target-dir E:\\srp-speedtest-v1015`

**Notes And Decisions**
- London replaced the earlier Prague-style EU candidate because the London backend responded more reliably from the current Southeast Asia route during live probes.
- `JP/KR` is intentionally labeled as a regional bucket backed by Tokyo today; no Korea-pinned backend was promoted into the catalog because the current probe set did not validate one cleanly enough.
- Native desktop smoke used a current local binary built from the branch and confirmed:
  - the Speed Test card is visible in desktop runtime,
  - the modal opens with the new regional selector/copy,
  - a live regional run completed and flowed back into the runtime card/status message.
- NotebookLM still cannot be written directly from the current toolset, so this entry in `Daily_Log.md` remains the source update for notebook refresh/re-sync.

**Next Steps**
- Ship the regional-target slice as `v10.1.5`.
- If we need deeper per-run transparency, expose the regional `provider/server` metadata more prominently in the modal/card after the initial regional release settles.

--------------------------------------------------------------------------------

## 2026-03-26 - Released v10.1.4 With NIC Name Enrichment And Speed Test Catalog Foundation

**Done**
- Bumped the app from `10.1.3` to `10.1.4` across `package.json`, `package-lock.json`, `src-tauri/Cargo.toml`, `src-tauri/Cargo.lock`, and `src-tauri/tauri.conf.json`.
- Cut a fresh release build that includes both the NIC display-name recovery and the new Speed Test target catalog foundation.
- Produced the Windows NSIS installer successfully at `E:\\srprel-1014\\release\\bundle\\nsis\\Super Route Pro_10.1.4_x64-setup.exe`.
- Updated `CHANGELOG.md` so the `v10.1.4` release line explicitly documents the shipped NIC and Speed Test changes plus the verification route used for this build.

**Notes And Decisions**
- The NIC fix stays on the low-risk path: startup still uses the fast snapshot flow, while richer adapter names are restored immediately after load through the stable-ID enrichment pass.
- The Speed Test release scope remains intentionally narrow: `Auto Asia` is the only real catalog entry in `v10.1.4`, and the UI keeps the selector disabled until more than one real target exists.
- Rust/release verification had to move to `E:\\` target directories because `D:\\` ran out of space during local build work; the repo also had an untracked `.cargo-target-speedtest-catalog` temp directory removed to recover local disk space before the final release run.
- NotebookLM still cannot be written directly from the current toolset, so this entry in `Daily_Log.md` is the release-side source update for notebook refresh/re-sync.

**Next Steps**
- Push the release commit and tag so `v10.1.4` is available on the remote branch/release flow.
- If product wants Speed Test to move closer to `speedtest.net`, the next slice should add real region/server targets to the catalog instead of only expanding the selector UI.

--------------------------------------------------------------------------------

## 2026-03-26 - Speed Test Target Catalog Foundation And NIC Name Enrichment Ready For v10.1.4

**Done**
- Confirmed the NIC display-name regression is fixed in the current tree without reopening the slow startup path: the app still loads through `get_network_snapshot()` first, then enriches the visible NIC descriptions asynchronously through `persistGetNicStableIds(interfaceIndexes)` matched by interface index.
- Added the first Speed Test target-catalog slice across backend and frontend:
  - backend now exposes `list_speed_test_targets()` and accepts `target_id` in `run_speed_test(...)`,
  - result metadata now includes `target_id` and `target_label`,
  - the modal now shows a `Target Profile` selector and target metadata block.
- Kept the initial catalog intentionally honest and low-risk: it currently exposes a single real target profile, `Auto Asia`, instead of pretending country-pinned endpoints already exist.
- Re-ran `npm run test:node`, `npm run build`, `cargo check`, and `cargo test --manifest-path src-tauri/Cargo.toml speed_test` successfully for this slice. Rust verification used fresh target directories on `E:\\` because `D:\\` was full and an old local `.cargo-target-speedtest-catalog` directory had to be removed first.

**Notes And Decisions**
- `persistGetNicStableIds(interfaceIndexes)` is sufficient for the NIC enrichment pass because it resolves the requested interface indexes against the full enriched adapter enumeration and returns the richer adapter `description` plus MAC-backed stable ID data.
- The Speed Test UI is now structurally ready for multi-target growth, but product behavior is still single-target in this build. That keeps the release truthful while reducing the amount of rework when real region-pinned targets are added later.
- The target selector is disabled when only one catalog entry exists, so this slice does not imply user-selectable countries before the backend supports them.
- NotebookLM still cannot be written directly from the current toolset, so this entry in `Daily_Log.md` remains the source update for notebook refresh/re-sync.

**Next Steps**
- Commit the Speed Test target-catalog foundation as its own slice before touching version files.
- Bump the app to `v10.1.4`, build a fresh release artifact, then push the branch and tag so the release remains easy to audit and roll back.

--------------------------------------------------------------------------------

## 2026-03-26 - Speed Test Target Catalog Foundation And Native Modal Smoke

**Done**
- Added a real target catalog foundation for Speed Test instead of keeping the feature hard-wired to one implicit backend target:
  - backend now exposes `list_speed_test_targets()` and accepts optional `target_id` in `run_speed_test`,
  - result payload now includes `target_id` and `target_label`,
  - frontend API, browser demo result, modal state, and component tests were updated to use the new contract.
- Shipped the first UI shell for multi-target work in the modal:
  - added `Target` metadata beside `Provider` and `Server`,
  - added a target selector surface in the modal with the initial `Auto Asia` catalog entry,
  - kept the selector honest by exposing only one real option instead of fake country choices on top of Cloudflare auto-edge.
- Verified the slice with:
  - `cargo check --manifest-path src-tauri/Cargo.toml --target-dir .cargo-target-speedtest-catalog`
  - `cargo test --manifest-path src-tauri/Cargo.toml speed_test --target-dir .cargo-target-speedtest-catalog`
  - `npm run test:node`
  - `npm run build`
- Completed a native desktop smoke pass for the new selector shell by:
  - launching the native `SuperRoute.exe` against the local dev server at `http://localhost:1420`,
  - confirming the desktop app loaded normally,
  - confirming the Speed Test modal opens and renders the new `AUTO ASIA` target chip plus target dropdown in native runtime.

**Notes And Decisions**
- This slice is intentionally a catalog/contract slice, not a fake country-picker slice. The product still only supports the real `Auto Asia` target until backend endpoints exist for region-pinned tests.
- Earlier native smoke work already confirmed the finalized runtime provider/server labels for the Cloudflare Asia auto-edge policy; this smoke pass focused on the new target selector shell and native modal plumbing.
- NotebookLM still cannot be written directly from the current toolset, so this `Daily_Log.md` entry remains the source file for notebook refresh/re-sync.

**Next Steps**
- Add real region-pinned targets only when the backend has concrete endpoints or provider strategy for them.
- When those targets exist, expand the modal selector from `Auto Asia` to a curated list such as `Japan/Korea`, `US West`, and `Europe`.

--------------------------------------------------------------------------------

## 2026-03-26 - NIC Description Regression Fixed After Fast Startup Optimization

**Done**
- Re-checked the `Ethernet X` / `Ethernet Y` naming issue and confirmed it was a regression introduced by the fast startup NIC snapshot path, not expected product behavior.
- Kept the startup optimization in place, but restored richer NIC names after first paint by asynchronously enriching loaded NIC rows with `persistGetNicStableIds(interfaceIndexes)`.
- Added a dedicated frontend merge model in `src/nicDescriptionModel.ts` plus targeted Node tests in `tests/nicDescriptionModel.test.ts`.
- Verified the regression fix with `npm run test:node` and `npm run build`.
- Confirmed in native desktop runtime that the NIC table again shows richer adapter names such as `Broadcom NetXtreme Gigabit Ethernet` and `Realtek PCIe GbE Family Controller` instead of generic aliases like `Ethernet 4`.

**Notes And Decisions**
- This fix preserves the startup performance win from the basic snapshot path instead of reverting to the slower `getmac`-heavy load path.
- The enrichment step is keyed by interface index so the UI can recover vendor/model naming without expanding backend surface area or delaying the first render.
- NotebookLM still cannot be written directly from the current toolset, so this `Daily_Log.md` entry remains the source file for notebook refresh/re-sync.

**Next Steps**
- If the brief alias-first render is still noticeable on slower machines, add a small UI hint or skeleton state for description enrichment.
- Keep the richer-description merge local to the frontend until another feature actually requires those full names during the initial backend snapshot.

--------------------------------------------------------------------------------

## 2026-03-26 - Speed Test Native Desktop Smoke Test After Asia Auto-Edge Policy

**Done**
- Ran a real native desktop smoke test against a freshly rebuilt binary from the current branch at `D:\\srprel-speed-smoke\\release\\SuperRoute.exe` instead of relying only on unit tests.
- Confirmed the updated modal/runtime strings now appear as intended in the native app:
  - progress copy showed `Speed test finished via Cloudflare (Asia auto-edge).`
  - provider metadata showed `Cloudflare (Asia auto-edge)`
  - server metadata showed `Asia Preferred (SIN edge)`
- Confirmed the full native run completed successfully in the desktop app with real measurements visible in the modal, including download/upload/ping/jitter/public IP/timestamp.

**Notes And Decisions**
- The previously shipped `v10.1.3` release artifact still showed the older wording because it was built before this Speed Test policy patch; that behavior is expected and not a regression in the new code.
- For runtime verification of the new wording, the correct artifact was the freshly rebuilt native binary from the current branch, not the already-tagged `v10.1.3` installer.
- NotebookLM still cannot be written directly from the current toolset, so this smoke-test note is recorded here in `Daily_Log.md` as the source file for notebook refresh/re-sync.

**Next Steps**
- If we want this native smoke-tested policy to ship outside the branch, cut a new release/build from the updated commit rather than reusing the older `v10.1.3` installer.
- If product direction moves closer to `speedtest.net`, the next feature slice should be multi-target or server-selectable testing rather than more relabeling on top of Cloudflare auto-edge.

--------------------------------------------------------------------------------

## 2026-03-26 - Speed Test Policy Finalized As Cloudflare Asia Auto-Edge

**Done**
- Re-audited the Speed Test feature across backend, frontend, tests, and branch history to confirm the only major open product decision was the final target policy rather than missing plumbing.
- Finalized the backend target policy in `src-tauri/src/speed_test.rs` as an explicit Cloudflare auto-edge strategy for Asia-oriented usage instead of continuing to imply a hard-pinned Asia server.
- Split the metadata more honestly without changing the UI contract:
  - `provider` now surfaces the policy as `Cloudflare (Asia auto-edge)`,
  - `server_label` now distinguishes preferred Asia edges (`Asia Preferred (SIN edge)`), non-Asia fallback edges (`Global Fallback (LAX edge, outside Asia preference)`), or `Cloudflare auto edge` when trace metadata is unavailable.
- Kept the existing Cloudflare download/upload/trace endpoints and measurement flow intact so the change stays backend-only and low-risk for the modal/UI.
- Updated the focused tests to match the finalized policy wording, including the Rust speed-test unit coverage and the modal render assertion for provider/server metadata.
- Re-ran `npm run test:node` and `cargo test --manifest-path src-tauri/Cargo.toml speed_test` successfully after the policy change.

**Notes And Decisions**
- This change intentionally does **not** pretend to pin a specific Asia city or POP because the current provider path still uses Cloudflare's auto-routed public speed-test endpoints.
- The product decision is now explicit: with the current backend, the supported behavior is `Asia auto-edge`, not `manual Asia server selection`.
- If we later need a truly hard-pinned Asia test server, that will likely require a different provider strategy or a more opinionated multi-target backend rather than another label-only tweak.
- NotebookLM cannot be written directly from the current toolset, so updating `Daily_Log.md` remains the source-of-truth path for the notebook to ingest on refresh/re-sync.

**Next Steps**
- Do one native desktop smoke test and confirm the modal now reads `Cloudflare (Asia auto-edge)` plus the resolved policy-aware edge label in the metadata block.
- If the product later demands a true region-pinned test, treat that as a separate feature slice instead of extending the current Cloudflare auto-edge policy piecemeal.

--------------------------------------------------------------------------------

## 2026-03-26 - Optimization Roadmap Status Summary For NotebookLM

**Done**
- Closed the release-safe quick wins called out by both `Optimise.md` and `Lộ trình Tối ưu hóa và Tái cấu trúc Super Route Pro`:
  - restored the saved theme from `localStorage` before first paint,
  - moved `check_internet()` work onto `spawn_blocking`,
  - consolidated the duplicated Windows `CREATE_NO_WINDOW` constant into `src-tauri/src/win32_consts.rs`.
- Implemented the main low-risk startup/runtime optimizations from the roadmap:
  - parallelized startup persistence-status and repair-context awaits,
  - reused cached route data for the first diagnostics routing view open,
  - batched persisted stable-NIC lookups instead of re-enumerating adapters per route,
  - optimized startup route replay to enumerate adapters once and reuse lookup maps,
  - switched `RestartActiveAdapters` to the lighter `enumerate_adapters_basic()` path.
- Landed the broader shipping fixes that materially improved the app even though they were larger than the roadmap's smallest quick wins:
  - fixed the single-WAN + per-NIC persisted-route behavior so only the chosen WAN keeps the default route while NIC2/NIC3 keep their own specific routes,
  - improved NIC startup performance by removing `getmac` from the startup critical path and introducing the `get_network_snapshot()` fast path,
  - shipped the Speed Test modal/native backend path plus extra tests and release packaging follow-through.
- Verified the shipped state through `v10.1.3` with `npm run check` plus a successful NSIS release build.

**Partially Done / Needs Follow-Up**
- Test coverage improved meaningfully, but only in targeted areas:
  - added Rust unit tests for speed test label/fallback logic,
  - added frontend/model tests for persisted-route shaping and Speed Test modal rendering.
- Startup persistence has been improved and the new per-route-NIC replay path is in place, but the roadmap's bigger goal of one clean source of truth is not finished yet because the legacy `SuperRoutePro-PersistWAN` path still exists beside the newer `SuperRouteProPersist` service flow.
- Backend deduplication has started only at the small/shared-constant level; the larger duplicate cleanup across cache/bloatware/file helpers is still outstanding.
- Documentation/logging are in much better shape through `Daily_Log.md` and `CHANGELOG.md`, but the broader stale-doc cleanup noted in the roadmap has not been fully completed.

**Not Done Yet**
- Break up the `App.tsx` god component into smaller hooks/components/modules.
- Fully consolidate startup persistence into one end-state and retire the remaining legacy WAN scheduled-task/script path.
- Finish deeper backend deduplication for repeated repair/network helper logic instead of only sharing constants and some route lookup work.
- Add broader E2E/integration coverage for the high-value user flows called out by the roadmap.
- Revisit deeper process polling / blocking-command cleanup beyond the already completed `check_internet()` fix.

**Notes And Decisions**
- NotebookLM ad-hoc retrieval for the two roadmap documents was noisy on 2026-03-26, so this status summary is based on the validated implementation work already logged in this file across 2026-03-25 sessions rather than on a fresh raw export from NotebookLM.
- The roadmap documents were still useful as prioritization guides, but `v10.1.3` intentionally focused on low-risk, release-safe items plus the WAN/NIC/Speed Test fixes that were already in flight.

**Next Steps**
- If continuing the roadmap after `v10.1.3`, the highest-value next slice is startup-persistence consolidation.
- After that, the clean architectural follow-up is splitting `App.tsx`, then tackling the remaining backend deduplication and broader E2E coverage.

--------------------------------------------------------------------------------

## 2026-03-25 - v10.1.3 Release Finalization

**Done**
- Bumped the outward-facing app version from `10.1.2` to `10.1.3` across `package.json`, `package-lock.json`, `src-tauri/Cargo.toml`, `src-tauri/Cargo.lock`, and `src-tauri/tauri.conf.json`.
- Added a curated `v10.1.3` release summary to `CHANGELOG.md` so the shipped notes reflect the Speed Test work, the single-WAN persistence fix, and the startup optimization pass.
- Re-ran the full release gate successfully with `npm run check` while pointing Cargo at `D:\\srpcheck-1013` to avoid the known Windows file-lock flake on shared target directories.
- Produced a fresh NSIS installer for `v10.1.3` at `D:\\srprel-1013b\\release\\bundle\\nsis\\Super Route Pro_10.1.3_x64-setup.exe` using a clean dedicated release target directory.

**Notes And Decisions**
- The NotebookLM optimization roadmap was used as release guidance only; the larger restructuring items remain intentionally deferred until after this patch release.
- The existing GitHub release workflow is tag-driven, so pushing `v10.1.3` remains the final publication step after this release commit is created.

**Next Steps**
- Commit the release metadata update on the working branch, push the branch, and push the `v10.1.3` tag to trigger the GitHub release workflow.

--------------------------------------------------------------------------------

## 2026-03-25 - NotebookLM Roadmap Cross-Check And Release Prep

**Done**
- Read the NotebookLM document titled `Lộ trình Tối ưu hóa và Tái cấu trúc Super Route Pro` as a second optimization reference before cutting the next outward-facing release.
- Confirmed that the two highest-value release-safe roadmap items had already been applied earlier in the day: restoring theme from `localStorage` before first paint and moving `check_internet()` work onto `spawn_blocking`.
- Applied the remaining roadmap quick win that was still clearly relevant for this branch: consolidated the duplicated Windows `CREATE_NO_WINDOW` constant into a shared `src-tauri/src/win32_consts.rs` module and updated the app/backend/service code paths to use it instead of redefining the same value in multiple files.
- Re-ran `npm run build`, `npm run test:node`, `cargo check --manifest-path src-tauri/Cargo.toml`, and `cargo test --manifest-path src-tauri/Cargo.toml` successfully after the constant consolidation. Rust verification again used fresh alternate target directories to avoid the known Windows file-lock flake on shared target folders.

**Notes And Decisions**
- The NotebookLM roadmap remains directionally useful for prioritization, but only the low-risk release-facing items were pulled into this cut; the larger structural work (`App.tsx` breakup, startup-persistence consolidation, backend deduplication beyond constants) remains a follow-up track rather than something to squeeze into the release candidate.
- Consolidating `CREATE_NO_WINDOW` was intentionally kept mechanical so the change improves maintainability without altering runtime behavior.

**Next Steps**
- Bump the product version to `10.1.3`, produce a fresh NSIS installer from the current branch, and push the release tag once final verification completes.

--------------------------------------------------------------------------------

## 2026-03-25 - Optimise.md Review And Safe Optimization Sweep

**Done**
- Read `Optimise.md` from NotebookLM and compared its recommendations against the current branch state instead of treating the document as blindly current truth.
- Implemented the low-risk/high-value frontend optimizations that were still relevant:
  - restored theme from `localStorage` during startup,
  - parallelized startup persistence-status and repair-context awaits that were previously serialized,
  - reused cached `routes` for the first diagnostics routing view open instead of immediately re-fetching,
  - batched stable NIC identifier resolution when saving WAN persistence so the app no longer re-enumerates adapters once per route.
- Implemented the low-risk/high-value backend optimizations that were still relevant:
  - wrapped `check_internet()` in `spawn_blocking`,
  - optimized startup route replay to enumerate adapters once and reuse lookup data for all persisted per-NIC routes,
  - switched `RestartActiveAdapters` to `enumerate_adapters_basic()` because it only needs `friendly_name` and `oper_status_up`.
- Re-ran `npm run build`, `npm run test:node`, `cargo check --manifest-path src-tauri/Cargo.toml`, and `cargo test --manifest-path src-tauri/Cargo.toml` successfully after the optimization sweep. Rust verification again used a fresh alternate target directory to avoid the known Windows file-lock flake.

**Notes And Decisions**
- The biggest remaining items from `Optimise.md` are broader refactors, not quick wins: the `App.tsx` god-component split, consolidating the duplicate startup-persistence mechanisms into one source of truth, and removing the remaining legacy code paths once the new route persistence flow is fully validated.
- The NotebookLM document was directionally useful, but some recommendations had to be re-checked against the current branch because several baseline items were already partially addressed by earlier sessions.

**Next Steps**
- Validate the new startup route persistence flow once after a real reboot/logon so the optimization sweep and the earlier WAN-routing fix are verified together.
- If we want to keep following `Optimise.md`, the next major slice should be startup-persistence consolidation rather than another scatter of micro-optimizations.

--------------------------------------------------------------------------------

## 2026-03-25 - Single-WAN Enforcement And Per-NIC Persisted Routes

**Done**
- Reviewed the WAN apply flow and confirmed the direct `Set WAN` action already removed competing default routes at runtime, but the startup persistence flow did not reliably preserve the same single-WAN behavior.
- Confirmed the old persisted route model only stored custom routes against the selected WAN NIC and was also saving `On-link` connected routes, which meant NIC2/NIC3 route-specific entries were not represented correctly for startup replay.
- Added `src/persistRouteModel.ts` plus focused Node tests so the UI now persists only real custom routes (skipping default/on-link entries) and attaches a stable NIC identifier to each persisted route.
- Updated `src/App.tsx` to resolve stable NIC identifiers for every interface that owns a persistable route before writing `PersistConfig`, so startup replay can keep NIC2/NIC3 routes on their own interfaces instead of forcing them onto the WAN NIC.
- Extended `src-tauri/src/route_persist.rs` so `CustomRoute` can optionally carry its own NIC identity while remaining backward-compatible with older `persist.json` files.
- Updated `src-tauri/src/route_service_main.rs` so startup replay now clears competing default routes before applying WAN and resolves each persisted custom route against its own NIC when present.
- Tightened the legacy `persist-wan.cmd` generator and the live `set_default_gateway_blocking()` path so both now repeat default-route cleanup instead of assuming one delete call is enough.
- Re-ran `npm run build`, `npm run test:node`, `cargo check --manifest-path src-tauri/Cargo.toml`, and `cargo test --manifest-path src-tauri/Cargo.toml` successfully after the WAN-routing fix. The Rust test suite again used a fresh alternate target directory to avoid the existing Windows file-lock flake.
- Produced a fresh NSIS installer from the updated WAN-routing code at `D:\\srprel-0513f3a5\\release\\bundle\\nsis\\Super Route Pro_10.1.2_x64-setup.exe`.

**Notes And Decisions**
- The local machine's current `persist.json` showed the exact old failure mode: it only contained `On-link` routes for the selected WAN NIC and no per-NIC route ownership for the other interfaces.
- The new `CustomRoute.nic` field is optional on disk, so existing configs still load; routes without that field continue to fall back to the main persisted NIC for compatibility.

**Next Steps**
- Re-apply WAN once from the UI on the target machine so a fresh `persist.json` is written with the new per-route NIC metadata.
- Do one restart/logon smoke test and confirm only the chosen NIC keeps the default route while NIC2/NIC3 keep only their intended specific routes.

--------------------------------------------------------------------------------

## 2026-03-25 - NIC Startup Performance Investigation And Fix

**Done**
- Traced the slow startup NIC load to backend adapter discovery rather than the `activeOnly` filter itself.
- Measured the local command costs to isolate the hotspot: `netsh interface ipv4 show interfaces` around `~78ms`, `netsh interface ipv4 show addresses` around `~87ms`, `route print -4` around `~371ms`, and `getmac /fo csv /v /nh` around `~997ms`.
- Added the combined startup snapshot command path (`get_network_snapshot`) and kept `src/App.tsx` loading through `getNetworkSnapshot()` so NICs plus routes are still fetched together for the main view.
- Refined the startup/query backend path to use a new lightweight adapter enumeration mode that skips the expensive `getmac` enrichment for `get_network_interfaces`, `get_network_snapshot`, and `get_routing_table`, while preserving the full enriched enumeration for flows that actually need MAC addresses or richer adapter descriptions.
- Tightened the snapshot path further by running lightweight adapter enumeration and `route print -4` concurrently inside `get_network_snapshot()` and `get_routing_table()` instead of serializing those two independent tasks.
- Kept the fast path low-risk by still using the same `netsh` interface names and IPv4/gateway parsing, which were enough on this machine to preserve blacklist detection for VMware/OpenVPN/loopback-style adapters.
- Re-ran `npm run build`, `npm run test:node`, `cargo check --manifest-path src-tauri/Cargo.toml`, and `cargo test --manifest-path src-tauri/Cargo.toml` successfully after the startup optimization. The Rust suite again used a short alternate target directory (`D:\\srptgt_parallel`) to avoid the existing default-target file-lock flake on this machine.
- Fixed `scripts/prepare-repair-sidecars.ps1` so release builds honor `CARGO_TARGET_DIR` instead of assuming `src-tauri/target`, which was blocking release packaging when using a fresh target directory to avoid Windows linker locks.
- Produced a fresh NSIS installer successfully at `D:\\srprel-b53d8d11\\release\\bundle\\nsis\\Super Route Pro_10.1.2_x64-setup.exe`.

**Notes And Decisions**
- The old startup path had two separate issues: duplicate adapter work before the snapshot refactor, then the remaining `getmac` enrichment cost that still dominated adapter discovery.
- Skipping `getmac` on the startup-facing commands trades richer adapter model descriptions for much faster first paint; the UI now leans on the `netsh` connection names (`Wi-Fi`, `Ethernet 2`, `VMware Network Adapter VMnet1`, etc.) during startup.
- The snapshot path only becomes a real latency win once `route print -4` stops sitting behind adapter enumeration; keeping those tasks concurrent preserves the simpler one-call frontend flow without paying the old serial wait.
- Development builds may still feel a little worse than production because React `StrictMode` in `src/main.tsx` can double-run mount effects locally.

**Next Steps**
- Run a manual cold-start check in the desktop app to confirm the NIC table now populates materially faster on the affected machine.
- If richer adapter model names are still desired on first paint, consider an asynchronous post-load enrichment pass or a short-lived cache instead of putting `getmac` back on the critical startup path.
- If this build is accepted as the next handoff, decide whether to keep using `10.1.2` for internal validation or bump the app version before the next outward-facing release.

--------------------------------------------------------------------------------

## 2026-03-25 - Speed Test Asia-Preferred Edge Labeling

**Done**
- Reworked the native Speed Test preflight in `src-tauri/src/speed_test.rs` so it now reads Cloudflare trace metadata up front and reuses that response for both public-IP reporting and server-label resolution.
- Replaced the vague `Cloudflare Auto` result label with an Asia-preferred label that surfaces the resolved Cloudflare edge when trace metadata is available, for example `Asia Preferred (SIN edge)`.
- Kept the change backend-only so the existing modal/UI contract stays intact; `src/SpeedTestModal.tsx` continues to display the backend-provided `provider` and `server_label` without adding a new selector surface.
- Added Rust unit coverage for trace metadata parsing and the new server-label resolution path in `src-tauri/src/speed_test.rs`.
- Added the explicit fallback Rust test for `resolve_speed_test_server_label(None)` and a lightweight component render test for the extracted Speed Test modal dialog so the new `server_label` display is now covered on both the backend and modal-render paths.
- Bootstrapped the local Windows verify machine with `rustup`/`cargo` so the native Tauri/Rust path could be validated locally instead of staying code-review only.
- Ran `cargo check --manifest-path src-tauri/Cargo.toml` and `cargo test --manifest-path src-tauri/Cargo.toml` successfully after the toolchain bootstrap; the new Speed Test tests and the broader existing Rust suite all passed.
- Refreshed `src-tauri/Cargo.lock` so the branch now records the native dependency graph it actually builds with, including the Speed Test backend additions that were already declared in `Cargo.toml`.

**Notes And Decisions**
- Cloudflare's public speed test flow still auto-selects the serving edge; there is no supported manual city picker in the current implementation, so the honest short-term product move is to surface the resolved edge POP instead of pretending the branch has a hard-pinned Asia city target.
- The fallback label remains `Asia Preferred (Cloudflare edge auto)` when trace metadata is unavailable, so the UI still communicates intent even if the POP code cannot be resolved during preflight.
- The local blocker was environmental rather than repo-level: the machine originally lacked `cargo` on `PATH`, so the verify pass first had to restore a working Rust toolchain under `%USERPROFILE%\.cargo\bin` before the native checks could run.
- The machine still shows an intermittent Windows file-lock problem when `cargo test` writes into the default `src-tauri/target` tree, so the refreshed Rust suite was revalidated successfully from a short alternate target directory (`D:\\srptgt`) to avoid stale debug-artifact locks without changing project behavior.

**Next Steps**
- Perform a manual Windows smoke pass to confirm the modal now shows the resolved edge label and that public-IP display still behaves correctly on real desktop runs.
- Decide whether the branch should keep the now-synced `src-tauri/Cargo.lock` diff alongside the Speed Test work when it is pushed/reviewed, since the native dependency graph is no longer theoretical after the successful verify pass.

--------------------------------------------------------------------------------

## 2026-03-24 - Speed Test Feature Branch Progress (Modal + Demo Mode + Runtime Hardening)

**Done**
- Built the first dedicated Speed Test feature slice on `feature/speed-test-modal-v1`, including a standalone modal/card flow in the lower-right panel area instead of reusing the existing Ping/Tracert console space.
- Added the native Rust `run_speed_test` command in `src-tauri/src/speed_test.rs`, wired through `src-tauri/src/lib.rs`, `src/api.ts`, and `src/App.tsx`, with the agreed `event + return` contract (`speed-test://progress` + final result payload).
- Added a browser-safe preview path via `src/speedTestDemo.ts`, so the Speed Test modal can be demonstrated in plain Vite/browser mode even when the Tauri desktop runtime or local Rust toolchain is unavailable.
- Restored and polished the modal preview UX in `src/SpeedTestModal.tsx` and `src/SpeedTestModal.css`, including clearer browser-demo copy (`Preview`, `Start Demo`, `Replay Demo`) and explicit in-modal demo-state messaging.
- Hardened native runtime error handling in `src-tauri/src/speed_test.rs` by mapping `reqwest` failures into more user-readable per-stage errors and rejecting the false-success case where the download stage returns `0` payload bytes.
- Added frontend-side error formatting in `src/speedTestError.ts` so timeouts, latency-probe failures, and test-server reachability issues surface as clearer UI messages inside the modal.
- Refactored target/provider selection in `src-tauri/src/speed_test.rs` into an isolated `SpeedTestTarget` resolver so the current Cloudflare-backed implementation can later be swapped to an Asia-specific server strategy without reworking the full measurement flow.
- Hardened latency sampling so the native flow no longer aborts the whole test on a single failed probe; it now tolerates partial probe failures and only fails if too few stable latency samples are collected.
- Added/updated focused Node-side test coverage for the browser demo and speed-test error formatter, and re-ran `npm run test:node` plus `npm run build` successfully after each meaningful Speed Test slice.

**Files Changed**
| File | Change |
|------|--------|
| `src-tauri/src/speed_test.rs` | **NEW** native speed test engine + later hardening for transport errors, target selection, and latency tolerance |
| `src-tauri/src/lib.rs` | Registered `run_speed_test` command |
| `src-tauri/Cargo.toml` | Added native HTTP/streaming/time dependencies required by speed test |
| `src/App.tsx` | Mounted the Speed Test launch card/modal into the right-side panel area |
| `src/api.ts` | Added `SpeedTestProgress`, `SpeedTestResult`, and `runSpeedTest()` wrapper |
| `src/SpeedTestModal.tsx` | **NEW** dedicated Speed Test modal UI and browser-preview/native-runtime switching |
| `src/SpeedTestModal.css` | **NEW** Speed Test modal/card styling |
| `src/speedTestDemo.ts` | **NEW** browser preview/demo-mode flow for non-Tauri runtime |
| `src/speedTestError.ts` | **NEW** user-facing error formatter for speed test failures |
| `tests/speedTestDemo.test.ts` | **NEW** demo-mode flow tests |
| `tests/speedTestError.test.ts` | **NEW** speed-test error mapping tests |
| `package.json` | Added new Speed Test-focused Node tests into `test:node` |

**Notes And Decisions**
- Kept the first real native backend on the existing Cloudflare endpoints so the feature can run end-to-end now, but isolated target selection because the likely next product decision is still an Asia-oriented server strategy.
- The browser demo mode is intentional product/dev tooling, not a fallback release path: it exists so UI review and flow demos can continue even on machines that cannot run `tauri dev`.
- Local frontend verification stayed green (`npm run test:node`, `npm run build`) throughout the branch work.
- Native Rust compilation is still not fully verified on this machine because `cargo` is not available on `PATH`; the native side has therefore been validated by code review, contract wiring, focused pure tests, and frontend integration rather than local `cargo check`.

**Next Steps**
- Decide and implement the final Asia-oriented `SpeedTestTarget` policy inside the isolated resolver instead of leaving the branch on generic `Cloudflare Auto`.
- Run native `cargo check` / `cargo test` for the Speed Test backend on a machine with a working Rust toolchain.
- Perform manual desktop smoke validation of the real Speed Test modal on Windows: progress events, download/upload completion, public-IP display, timeout messaging, and latency-probe tolerance.

--------------------------------------------------------------------------------

## 2026-03-24 - v10.1.2 Release Triggered (Virtual/Tunnel NIC Filter Follow-Up)

**Done**
- Investigated the remaining `Active only` NIC false-positive report on the affected Windows machine where `Tailscale Tunnel` and `vEthernet (Default Switch)` were still appearing alongside the real Ethernet NIC.
- Confirmed the issue was not a raw OS misread: Windows can legitimately report multiple connected interfaces at once, but the UI-facing NIC filter was still too permissive for virtual/tunnel adapter naming variants.
- Tightened the NIC table filter in `src-tauri/src/network.rs` so the virtual-adapter blacklist now checks both `description` and `friendly_name`, and now covers the observed naming families (`tailscale`, `hyper-v`, `vEthernet`, `default switch`, `wsl`, `wireguard`) on top of the existing virtual/VPN tokens.
- Added focused Rust regression coverage for the new blacklist behavior so the follow-up filter blocks `Tailscale Tunnel` and `vEthernet (Default Switch)` rows from the main NIC table path.
- Landed the fix on top of the already released `v10.1.1` baseline, then pushed `main` and tag `v10.1.2` so the GitHub release workflow could publish the new patch build.

**Notes And Decisions**
- Kept the change scoped to the UI-facing NIC list builder only; raw adapter enumeration, route persistence, and repair/persistence internals were left untouched.
- The product rule remains: the app should still show multiple NICs if they are truly meaningful routing interfaces. This patch only suppresses the known virtual/tunnel false positives that were still leaking through `Active only`.
- Local release gating was skipped for this patch because the shell used for the release step did not have the Rust toolchain available on `PATH`; the patch was still pushed/tagged so QA can validate directly on the affected real machine.

**Next Steps**
- Run the shipped `v10.1.2` build on the affected machine and confirm the NIC table now hides `Tailscale` / `vEthernet (Default Switch)` while preserving the actual connected Ethernet NIC.
- If any extra NIC still leaks through, capture the exact adapter name as shown both in app and `ncpa.cpl`, then refine the blacklist with another narrow follow-up instead of broadening the persistence/routing layer.

--------------------------------------------------------------------------------

## 2026-03-23 - v10.1.1 Release Published

**Done**
- Merged the NIC active-filter hotfix onto `main`, then added a follow-up whitelist hardening patch that blocks shell chaining and other `cmd.exe` metacharacters after otherwise allowed diagnostic prefixes.
- Bumped release metadata to `10.1.1` across `package.json`, `src-tauri/Cargo.toml`, `src-tauri/Cargo.lock`, `src-tauri/tauri.conf.json`, and the top-level release-facing docs/changelog entries.
- Re-ran `npm run check` successfully on the release candidate and completed a local `npm run release:build`, which produced `Super Route Pro_10.1.1_x64-setup.exe`.
- Pushed `main` and tag `v10.1.1`, then confirmed GitHub Actions release workflow `23446094024` published `Super.Route.Pro_10.1.1_x64-setup.exe`, `SuperRoute.exe`, and `SHA256SUMS.txt`.

**Notes And Decisions**
- Kept the release as a patch bump because the shipped changes are hotfix-level behavior/security hardening on top of the already published `v10.1.0` baseline.

**Next Steps**
- Run the shipped `v10.1.1` installer on a real Windows machine with the target NIC mix to verify the active-only filter and diagnostic command flows end-to-end.

--------------------------------------------------------------------------------

## 2026-03-23 - Network Command Whitelist Hardening

**Done**
- Hardened `run_network_command` so an allowed prefix is no longer enough on its own; commands are now rejected if they contain `cmd.exe` shell metacharacters such as chaining, pipes, redirection, or grouped command syntax.
- Added focused Rust regression tests covering both the expected `tracert` / `nslookup` allow cases and rejected shell-chaining cases after an otherwise valid prefix.
- Re-ran `npm run check` successfully after the whitelist hardening change.

**Notes And Decisions**
- Kept the change narrow by validating the existing string command path before `cmd /C` executes, instead of broadening this slice into a full command-parser rewrite.

**Next Steps**
- If we want to go further later, the next security step would be replacing the remaining `cmd /C` path with explicit program/argument dispatch for each allowed diagnostic command.

--------------------------------------------------------------------------------

## 2026-03-23 - NIC Active Filter Hotfix (v10.1.0)

**Done**
- Tightened the active-only NIC list so it now requires both `oper_status_up` and at least one real IPv4 address before showing an adapter in the main table.
- Added a shared IPv4 validator in `src-tauri/src/win32_net.rs` that rejects empty values, `0.0.0.0`, APIPA/link-local `169.254.x.x`, and non-IPv4 strings.
- Extracted the NIC list builder in `src-tauri/src/network.rs` so the active-filter behavior is covered by direct Rust unit tests instead of only manual UI verification.
- Added regression coverage for the IPv4 validator and the active-only NIC builder, then re-ran `cargo test --manifest-path src-tauri/Cargo.toml --lib` successfully.

**Notes And Decisions**
- Kept the patch narrow to the active-only filter path and left the non-active NIC listing behavior unchanged outside of preferring a real IPv4 when one is present.

**Next Steps**
- Push `fix-nic-active-filter` for review and, if needed, follow with a Windows `npm run tauri dev` smoke pass on a machine that has the expected NIC mix (virtual NIC + Ethernet + Wi-Fi).

--------------------------------------------------------------------------------

## 2026-03-22 - Post-Release Workflow Cleanup

**Done**
- Retired Beads from the tracked Super Route Pro workflow and kept `Daily_Log.md` as the single running narrative for NotebookLM follow-ups.
- Removed the local `.beads/` workspace and local `AGENTS.md` after migrating the remaining post-release follow-up and startup-persistence debt into this log.
- Scrubbed tracked repo references to Beads so the shipped project no longer points contributors at a retired issue-tracking layer.
- Confirmed the dedicated GitHub repo `quockhanh2376/SuperRoutePro-beads` has been deleted; `gh repo view quockhanh2376/SuperRoutePro-beads` now fails with repository not found.

**Notes And Decisions**
- Super Route Pro now treats `Daily_Log.md`, OpenSpec artifacts, and GitHub releases/issues as the maintained delivery trail instead of Beads/Dolt.
- Beads retirement is now complete both locally and on GitHub; there is no remaining Dolt/beads remote attached to the active SRP workflow.

**Next Steps**
- Continue tracking post-release follow-ups directly in `Daily_Log.md` and the existing release/docs workflow.

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
- **Manual Verify Prep**: Wrote the focused `v10.1.0` manual verification checklist so QA/release can execute a single source of truth for the remaining UI pass.
- **Manual Verify Execution**: Ran the main `v10.1.0` UI verification flow against a fresh `npm run release:build` artifact. Confirmed the packaged installer now emits `Super Route Pro_10.1.0_x64-setup.exe`, startup shows `10.1.0`, routing snapshot refresh works, Repair Mode unlock/lock works, Scan IP derives `192.168.1.0/24` and supports Force Stop, Port Test passes open/closed smoke cases, Battery Info loads native values, Add/Delete Route works once unlocked with real field values entered, Clear Cache completes with warnings, and Remove Apps list loads correctly.
- **Manual Verify Findings**: The execution pass surfaced two concrete follow-ups: orphaned elevated `SuperRouteRepairBroker` processes after force-closing while unlocked, and the NIC table sometimes showing `No interfaces found` during startup even while the footer already reports loaded NICs/routes.
- **Help Copy Update**: Added a dedicated Help entry for `Lock / Unlock Repair Mode` in both English and Vietnamese so users understand that Locked blocks admin fixes, Unlock opens an elevated Repair Mode session for the current app session, and Lock closes that elevated session again.
- **Repair Broker Lifecycle Fix**: Fixed the orphaned broker-process bug by passing the launching app PID into `UnlockRepairSessionRequest` and teaching `SuperRouteRepairBroker` to monitor the parent process handle, so the elevated broker self-terminates if the UI is force-closed while unlocked.
- **NIC Startup Empty-State Fix**: Fixed the startup NIC empty-state bug by adding a NIC-table loading placeholder and stale-load guards, so startup no longer flashes `No interfaces found` while the first NIC snapshot is still loading.
- **Persistence Tracking Update**: Flagged the `Persist on startup OFF` path as the last release blocker because it remained inconclusive and still needed direct repro before sign-off.
- **Rust Warning Cleanup**: Removed the remaining `cargo check` warning debt by deleting unused battery/NIC helper remnants, dropping an unused registry import and raw target struct, and switching `SuperRouteService` to reuse the shared `route_persist` module instead of compiling its own warning-prone copy. Re-ran `cargo check` clean, then re-ran full `npm run check` clean.
- **Persist OFF Root-Cause Fix**: Fixed the `Persist on startup OFF` blocker by moving startup-persistence save/clear operations onto the elevated repair broker path, so standard-user sessions no longer try to write `%ProgramData%\\SuperRoutePro\\persist.json` or register `SuperRouteProPersist` directly. The WAN flow now clears persisted startup state when OFF, keeps the checkbox aligned with either persisted config or the legacy WAN task, and ships with new Node + Rust coverage for the persist action contract.
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
- Post-release follow-up: run one real reboot/logon verification pass against the shipped `v10.1.0` installer so `SuperRouteProPersist` is exercised across a true reboot boundary.
- Technical debt: consolidate the legacy `SuperRoutePro-PersistWAN` task and the newer `SuperRouteProPersist` service flow into one startup-persistence mechanism/end-state.
- Expand automated coverage further for the migrated native-Rust paths beyond the current route parser and Node smoke tests.

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
- Updated `Daily_Log.md` so NotebookLM can stay aligned with the current release work.

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
- Created `Daily_Log.md` to track progress and decisions for Super Route Pro.

**Notes And Decisions**
- The project just completed the "Repair Service Migration" for standard user UI access.
- Future network tools and updates will be logged here.

**Next Steps**
- Add remaining project documentation to NotebookLM.
- Prepare for End-to-End testing of the standard user UI with local admin credentials.
