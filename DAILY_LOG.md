# Super Route Pro Daily Log

This document is the running delivery log for Super Route Pro.
Update it after each meaningful work session so the team and NotebookLM stay aligned on current progress, decisions, blockers, and next steps.

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
- NotebookLM still cannot be written directly from the current toolset, so this entry in `DAILY_LOG.md` remains the source update for notebook refresh/re-sync.

**Next Steps**
- Commit the Speed Test target-catalog foundation as its own slice before touching version files.
- Bump the app to `v10.1.4`, build a fresh release artifact, then push the branch and tag so the release remains easy to audit and roll back.

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
- NotebookLM still cannot be written directly from the current toolset, so this smoke-test note is recorded here in `DAILY_LOG.md` as the source file for notebook refresh/re-sync.

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
- NotebookLM cannot be written directly from the current toolset, so updating `DAILY_LOG.md` remains the source-of-truth path for the notebook to ingest on refresh/re-sync.

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
- Documentation/logging are in much better shape through `DAILY_LOG.md` and `CHANGELOG.md`, but the broader stale-doc cleanup noted in the roadmap has not been fully completed.

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
- Retired Beads from the tracked Super Route Pro workflow and kept `DAILY_LOG.md` as the single running narrative for NotebookLM follow-ups.
- Removed the local `.beads/` workspace and local `AGENTS.md` after migrating the remaining post-release follow-up and startup-persistence debt into this log.
- Scrubbed tracked repo references to Beads so the shipped project no longer points contributors at a retired issue-tracking layer.
- Confirmed the dedicated GitHub repo `quockhanh2376/SuperRoutePro-beads` has been deleted; `gh repo view quockhanh2376/SuperRoutePro-beads` now fails with repository not found.

**Notes And Decisions**
- Super Route Pro now treats `DAILY_LOG.md`, OpenSpec artifacts, and GitHub releases/issues as the maintained delivery trail instead of Beads/Dolt.
- Beads retirement is now complete both locally and on GitHub; there is no remaining Dolt/beads remote attached to the active SRP workflow.

**Next Steps**
- Continue tracking post-release follow-ups directly in `DAILY_LOG.md` and the existing release/docs workflow.

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
