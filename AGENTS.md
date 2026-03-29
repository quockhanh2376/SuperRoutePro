# AGENTS.md

Repo-specific guidance for coding agents working in `E:\SuperrRoutePro`.

## Overview
- Windows-first desktop network toolkit built with Tauri v2, React 19, TypeScript, and Rust.
- Canonical version files:
  - `package.json`
  - `src-tauri/Cargo.toml`
  - `src-tauri/tauri.conf.json`
- Recent project decisions and releases:
  - `Daily_Log.md`
  - `CHANGELOG.md`
- `README.md` is helpful but can lag behind active branches.

## Rules Files
- No prior `AGENTS.md` existed in the repo root.
- No `.cursor/rules/` directory was found.
- No `.cursorrules` file was found.
- No `.github/copilot-instructions.md` file was found.

## Environment
- Primary platform: Windows 10/11.
- CI runner: `windows-2022`.
- CI Node version: `24`.
- CI Rust toolchain: `stable`.
- Prefer the repo npm wrapper for Tauri commands because it bootstraps Cargo PATH on Windows.

## Core Commands
Run all commands from the repo root.

Install and dev:
```powershell
npm ci
npm run dev
npm run tauri -- dev
```

Build and validation:
```powershell
npm run build
npm run check:rust
npm run test:node
npm run test:rust
npm run check
```

Equivalent Rust commands:
```powershell
cargo check --manifest-path src-tauri/Cargo.toml
cargo test --manifest-path src-tauri/Cargo.toml
```

Release/build commands:
```powershell
npm run release:build
npm run tauri -- build
npm run release:local
npm run release:local -- -VersionTag v10.1.5 -SkipInstall
```

Versioning and release ship flow:
```powershell
npm run version:patch
npm run version:minor
npm run version:major
npm run version:bump -- 10.1.6
npm run release:patch
npm run release:minor
npm run release:major
npm run release:ship -- 10.1.6
```

## Running A Single Test
There is no universal single-test wrapper; use the runner that matches the file.

Node / TS model tests:
```powershell
node --test tests/run-tauri.test.mjs
node --test --experimental-strip-types tests/repairModeModel.test.ts
node --test --experimental-strip-types tests/persistFlow.test.ts
```

TSX component test:
```powershell
tsx --test tests/SpeedTestModal.test.tsx
```

Rust integration tests and filters:
```powershell
cargo test --manifest-path src-tauri/Cargo.toml --test repair_protocol
cargo test --manifest-path src-tauri/Cargo.toml --test repair_session
cargo test --manifest-path src-tauri/Cargo.toml --test installer_packaging
cargo test --manifest-path src-tauri/Cargo.toml repair_protocol_types_serialize_expected_status_fields
cargo test --manifest-path src-tauri/Cargo.toml speed_test
```

## Linting / Formatting Reality
- There is no dedicated `lint` script in `package.json`.
- TypeScript correctness is enforced mainly by `npm run build` with `strict: true`.
- Rust correctness is enforced by `cargo check` and `cargo test`.
- Do not introduce ESLint/Prettier/rustfmt churn unless explicitly asked.
- If you format, keep changes scoped to files you touched.

## TypeScript / React Style
- Use 2-space indentation.
- Use double quotes and semicolons.
- Keep trailing commas in multiline objects, arrays, params, and imports.
- Prefer `type` import modifiers where appropriate.
- Prefer `interface` for exported API/data contracts, especially in `src/api.ts`.
- Prefer `type` aliases for local props, unions, and internal UI state.
- Use `PascalCase` for React components and exported types.
- Use `camelCase` for variables, functions, hooks, and handlers.
- Use `UPPER_SNAKE_CASE` for module-level constants.
- Preserve intentional `snake_case` field names across the Tauri boundary.
- Reuse existing models/helpers/components before adding more inline logic to `src/App.tsx`.

## CSS / UI Style
- The UI uses Tailwind utility classes plus repo-local CSS files.
- Match the existing utility-first style rather than adding a new styling system.
- Extend existing light/dark mode rules instead of creating one-off inline styles.
- Preserve the current dense Windows desktop layout unless the task explicitly changes UX.

## Rust Style
- Use standard Rust formatting and 4-space indentation.
- Use `snake_case` for modules, functions, and variables.
- Use `PascalCase` for structs and enums.
- Use `SCREAMING_SNAKE_CASE` for constants.
- Match surrounding import grouping instead of reordering unrelated imports.
- Return `Result<_, String>` at Tauri command and cross-module boundaries where that is the repo pattern.
- Gate Windows-only logic with `#[cfg(target_os = "windows")]` and provide explicit fallback errors where needed.
- Use `spawn_blocking` for blocking OS/process work inside async Tauri commands.
- Reuse shared modules like `process_exec`, `network_snapshot`, `win32_net`, `repair_protocol`, and `persist_startup` before adding new process or IPC helpers.

## Error Handling
- Surface clear, actionable user-facing error strings.
- Do not silently swallow process, filesystem, or IPC failures.
- Preserve `requires_unlock` behavior for privileged Repair Mode flows.
- Reserve `unwrap` and `expect` for tests or true invariants.

## Testing Conventions
- Frontend tests use the Node test runner plus `assert/strict`.
- TS model tests use `--experimental-strip-types`.
- TSX component tests use `tsx --test`.
- Rust integration tests live in `src-tauri/tests/`.
- For focused changes, run the narrowest relevant test first, then broader checks if the change crosses boundaries.

## Security / Platform Constraints
- This is a security-sensitive Windows app; do not add arbitrary shell execution from the UI.
- Keep privileged actions behind typed commands and existing whitelists.
- Be careful with target SID handling, profile cleanup, AppX removal, and persisted route startup logic.
- Do not casually change contracts shared between `src/api.ts` and `src-tauri/src/repair_protocol.rs`.
- Route persistence (`SuperRouteService`) and Repair Mode service/broker flows are related but distinct.

## Files Worth Reading Before Large Changes
- `src/App.tsx`
- `src/api.ts`
- `src/SpeedTestModal.tsx`
- `src-tauri/src/lib.rs`
- `src-tauri/src/network.rs`
- `src-tauri/src/repair_actions.rs`
- `src-tauri/src/repair_protocol.rs`
- `src-tauri/src/speed_test.rs`
- `scripts/release-ship.ps1`
- `Daily_Log.md`
- `CHANGELOG.md`

## Release Hygiene
- Keep these aligned for release work:
  - `package.json`
  - `package-lock.json`
  - `src-tauri/Cargo.toml`
  - `src-tauri/Cargo.lock`
  - `src-tauri/tauri.conf.json`
- For release-facing changes, also check whether `CHANGELOG.md`, `Daily_Log.md`, and `README.md` need updates.
- `scripts/release-ship.ps1` expects a clean tracked worktree unless `-AllowDirty` is used.

## Practical Advice
- Make the smallest correct change first.
- Follow the surrounding file's style more than generic preferences.
- Treat untracked artifact folders as non-source unless the user explicitly asks you to review or commit them.
