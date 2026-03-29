# NeedToDo

Restored on 2026-03-27 from the latest agreed roadmap and current repo state.

## Backend Cleanup And Maintainability

### Priority Order Agreed With Zon

#### Do Next

- [ ] Re-check whether any remaining `repair_actions` helpers can be reduced further without changing behavior.
- [ ] Re-check whether there is any leftover bootstrap or Windows-only path logic that still belongs in a shared helper.
- [ ] Keep an eye on Windows linker/file-lock noise during full local Rust test runs; current evidence points to environment noise, not runtime bugs.

#### Phase Later

- [ ] Evaluate true city-pinned Australia targets if product wants explicit Sydney / Melbourne / Brisbane / Perth selection instead of only `Auto Australia`.
- [ ] Validate which public or owned backends are reliable enough for AU city pinning and compatible with the current engine.
- [ ] Decide whether AU support should remain auto-region only or expand into a city selector in the modal.
- [ ] Decide whether the current live metric cards need another visual polish pass for spacing, hierarchy, or motion.
- [ ] Decide whether the final run summary should expose more per-target metadata, especially when regional routing is involved.

### Completed

- [x] Reduce duplicated request-validation / selection flow inside `repair_actions.rs` so cleanup/Appx execution paths reuse validated target/package selections instead of re-sanitizing after validation.
- [x] Unify startup persistence onto the current persisted-startup path and remove the legacy split startup behavior.
- [x] Replace generic DHCP renew shell chaining with typed execution that preserves success semantics and timeout behavior.
- [x] Harden network command validation so quoted adapter names with parentheses are allowed, while dangerous shell chaining remains blocked.
- [x] Remove dead non-repair command surface that duplicated repair-only flows.
- [x] Deduplicate shared cleanup/process helpers and centralize common Windows path helpers.
- [x] Split Tauri bootstrap and command wiring so `src-tauri/src/lib.rs` is thinner and easier to maintain.
- [x] Add NIC cache invalidation hooks so manual refresh and selected repair flows do not leave stale adapter metadata behind.
- [x] Extract connectivity probing into a dedicated module and clean up repair-action helper duplication.
- [x] Add integration and regression coverage for persist config, route service behavior, repair broker flow, speed test target contracts, NIC snapshot seams, repair-action validation, startup task detection, and related helper seams.
- [x] Route `npm run test:rust` through a clean-target runner on Windows so local Rust verification avoids long-lived target-dir file-lock noise.
- [x] Expose final Speed Test identity metadata clearly in the UI with dedicated Target / Provider / Region summary cards.

### Still Worth Reviewing

- [ ] Re-check whether any remaining `repair_actions` helpers can be reduced further without changing behavior.
- [ ] Re-check whether there is any leftover bootstrap or Windows-only path logic that still belongs in a shared helper.
- [ ] Keep an eye on Windows linker/file-lock noise during full local Rust test runs; current evidence points to environment noise, not runtime bugs.

## Speed Test Australia

### Completed

- [x] Add `Auto Australia` support in the speed test target model and backend flow.
- [x] Generalize region/target labeling so Australia can be surfaced cleanly in the runtime UI and backend.

### Pending

- [ ] Evaluate true city-pinned Australia targets if product wants explicit Sydney / Melbourne / Brisbane / Perth selection instead of only `Auto Australia`.
- [ ] Validate which public or owned backends are reliable enough for AU city pinning and compatible with the current engine.
- [ ] Decide whether AU support should remain auto-region only or expand into a city selector in the modal.

## Speed Test UI Direction

### Completed

- [x] Replace the single progress-bar-centric live view with a metric-card layout that works for both light mode and dark mode.
- [x] Improve light-mode contrast for the live speed test metrics and repair-related hint text.
- [x] Replace hardcoded live status copy with stage-aware status text.

### Pending

- [ ] Decide whether the current live metric cards need another visual polish pass for spacing, hierarchy, or motion.

## Notes

- This file is restored after the local untracked copy was removed during workspace cleanup.
- The content reflects the latest agreed roadmap plus the current implementation status in the repo, not an untouched historical snapshot.
- The agreed execution order is: thin backend cleanup first, local test-hygiene second, AU product decision third, UI polish after the AU direction is settled.
