# AGENTS.md - Developer Guide for SuperRoutePro

This guide is for AI coding agents and developers working on SuperRoutePro, a network routing manager built with React, TypeScript, and Tauri (Rust).

---

## NotebookLM Integration

SuperRoutePro uses [NotebookLM MCP server](https://github.com/PleasePrompto/notebooklm-mcp) to provide zero-hallucination documentation access via Google's NotebookLM.

### Setup
1. The MCP server is configured in `opencode.json` and runs via `npx -y notebooklm-mcp@latest`
2. On first use, authenticate by saying: `"Log me in to NotebookLM"`
3. A Chrome window will open for Google authentication (one-time setup)
4. Create or refresh the project notebook using `docs/notebooklm-project-setup.md`
5. `Daily_Log.md` remains the primary source-of-truth file for notebook refreshes after each meaningful work session

### Usage
When working on this project, you can access NotebookLM knowledge by:
- Adding `use notebooklm` to prompts when you need project-specific documentation
- The agent will automatically query NotebookLM for context before making changes
- NotebookLM provides citation-backed answers with zero hallucinations
- If the local NotebookLM library is still empty, finish the setup checklist in `docs/notebooklm-project-setup.md` first

**Example:**
```
Research the latest optimization work in NotebookLM before refactoring App.tsx. use notebooklm
```

### Benefits
- **Zero hallucinations**: NotebookLM refuses to answer if info isn't in the docs
- **Always current**: No training cutoff, reflects latest project state
- **Multi-source synthesis**: Connects information across all synced documentation
- **Citation-backed**: Every answer includes source references

---

## Build, Test, and Development Commands

### Frontend Development
```bash
npm run dev                    # Start Vite dev server (port 1420)
npm run build                  # TypeScript compile + Vite build
npm run preview                # Preview production build
npm run check:frontend         # Run frontend build check
```

### Rust/Tauri Development
```bash
npm run tauri dev              # Start Tauri app in dev mode
npm run tauri build            # Build production Tauri app
npm run check:rust             # Check Rust code (cargo check)
npm run release:build          # Production Tauri build
```

### Testing

**Frontend Tests (Node.js):**
```bash
npm run test:node              # Run all Node.js tests
node --test tests/nicDescriptionModel.test.ts              # Run single test file
node --test --experimental-strip-types tests/[name].test.ts # Run specific test
```

**Rust Tests:**
```bash
npm run test:rust              # Run all Rust tests via PowerShell script
cargo test --manifest-path src-tauri/Cargo.toml            # Run Rust tests directly
cargo test --manifest-path src-tauri/Cargo.toml [test_name] # Run specific test
```

**Run All Checks:**
```bash
npm run check                  # Frontend + Rust checks + all tests
```

### Version Management
```bash
npm run version:patch          # Bump patch version (10.1.12 → 10.1.13)
npm run version:minor          # Bump minor version (10.1.12 → 10.2.0)
npm run version:major          # Bump major version (10.1.12 → 11.0.0)
```

### Release
```bash
npm run release:patch          # Build and ship patch release
npm run release:minor          # Build and ship minor release
npm run release:major          # Build and ship major release
```

---

## Project Structure

```
SuperRoutePro/
├── src/                       # Frontend TypeScript/React code
│   ├── components/            # React UI components (BatteryModal, AppChrome, etc.)
│   ├── hooks/                 # Custom React hooks (useBufferedLog, ipScanPlan)
│   ├── *Model.ts              # Business logic modules (pure functions)
│   ├── *Utils.ts              # Utility functions (batteryUtils)
│   ├── api.ts                 # Centralized API/Tauri interface + types
│   ├── App.tsx                # Main application component
│   └── main.tsx               # React entry point
├── src-tauri/                 # Rust backend code
│   ├── src/                   # Rust source files
│   │   ├── main.rs            # Main Tauri app binary (SuperRoute)
│   │   ├── repair_broker_main.rs  # Repair broker binary
│   │   ├── route_service_main.rs  # Route service binary
│   │   └── bin/speed_test_probe.rs # Speed test probe binary
│   ├── tests/                 # Rust integration tests
│   ├── Cargo.toml             # Rust dependencies
│   └── tauri.conf.json        # Tauri configuration
├── tests/                     # Frontend Node.js tests
├── public/                    # Static assets
├── scripts/                   # Build and release scripts (PowerShell)
└── package.json               # npm scripts and dependencies
```

---

## Code Style Guidelines

### TypeScript/React

#### File Naming
- **Components**: PascalCase + `.tsx` extension (`BatteryModal.tsx`, `AppChrome.tsx`)
- **Hooks**: camelCase starting with "use" + `.ts` (`useBufferedLog.ts`)
- **Models**: camelCase + "Model" suffix + `.ts` (`nicDescriptionModel.ts`)
- **Utils**: camelCase + "Utils" suffix + `.ts` (`batteryUtils.ts`)
- **Tests**: Source file name + `.test.ts` (`nicDescriptionModel.test.ts`)

#### Import Order
```typescript
// 1. External libraries first
import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Zap } from "lucide-react";

// 2. Internal API/types (use `type` keyword for type-only imports)
import { getNetworkSnapshot, type NetworkInterface } from "./api";

// 3. Internal modules/utilities
import { mergeNicDescriptions } from "./nicDescriptionModel";

// 4. Components
import { BatteryModal } from "./components/BatteryModal";

// 5. Styles (if any)
import "./App.css";
```

#### Naming Conventions
- **Functions**: camelCase, verb-first (`getNetworkSnapshot`, `buildIpScanPlan`)
- **Event handlers**: `handle` prefix (`handleSelectNic`, `handleAddRoute`)
- **Action executors**: `execute` prefix (`executeSetInternet`, `executeFlush`)
- **Boolean functions**: `is/has` prefix (`isGenericNicDescription`, `isPersistableCustomRoute`)
- **Components**: PascalCase (`BatteryModal`, `OutputConsole`)
- **Constants**: SCREAMING_SNAKE_CASE (`IP_SCAN_BATCH_SIZE`, `DONATE_QR_IMAGE_PATH`)
- **Config objects**: PascalCase (`ROUTE_TABLE_COLUMNS`, `CACHE_CLEANUP_OPTIONS`)
- **Variables**: camelCase (`selectedNic`, `ipScanRunning`)
- **Refs**: camelCase + `Ref` suffix (`pingLoopRef`, `commandOutputRef`)

#### Formatting
- **Indentation**: 2 spaces
- **Quotes**: Double quotes for strings
- **Semicolons**: Always use semicolons
- **Trailing commas**: Use in multi-line arrays/objects
- **Prefer**: `const` over `let`, never use `var`
- **Functions**: Arrow functions for callbacks, named functions for exports

#### TypeScript Types
- **Use `interface`** for API data structures and object shapes
- **Use `type`** for unions, primitives, utility types
- **Explicit return types** on exported functions
- **Type-only imports**: Use `import { type Foo }` syntax
- **Strict typing**: `strict: true`, handle null/undefined with `| null` unions

#### React Patterns
- **Only functional components** with hooks (no class components)
- **Memoization**: Use `memo()` for expensive components
- **Callbacks**: Use `useCallback()` for stable function references
- **Computed values**: Use `useMemo()` for expensive computations
- **Props**: Define explicit TypeScript types for all props
- **Named exports** for reusable components, **default export** for main App

#### Error Handling
```typescript
// Try-catch with type-safe error handling
try {
  const result = await someApiCall();
  setData(result);
} catch (err: unknown) {
  const msg = err instanceof Error ? err.message : String(err);
  setStatusMsg(`Error: ${msg}`);
}

// Validation before action
if (!formDest || !formGw) {
  setStatusMsg("Please fill Destination and Gateway");
  return;
}

// Always cleanup in finally
try {
  // operation
} catch (err) {
  // handle
} finally {
  setLoading(false);
}
```

#### Async Patterns
```typescript
// Cancellation-safe useEffect
useEffect(() => {
  let active = true;
  const load = async () => {
    const result = await api();
    if (active) setState(result); // Only update if not unmounted
  };
  void load(); // Use void for fire-and-forget
  return () => { active = false };
}, []);
```

### Rust

#### File Organization
- Binaries defined in `Cargo.toml` with `[[bin]]` sections
- Library exports via `[lib]` with `crate-type = ["staticlib", "cdylib", "rlib"]`
- Integration tests in `src-tauri/tests/` directory

#### Naming Conventions
- **Files**: snake_case (e.g., `route_service_main.rs`, `repair_broker_flow.rs`)
- **Functions**: snake_case (e.g., `build_nic_index_lookup`)
- **Types/Structs**: PascalCase (e.g., `NativeNic`, `CustomRoute`)
- **Test functions**: snake_case with `#[test]` attribute

#### Testing
- Use Rust's built-in test framework with `#[test]` attribute
- Integration tests in `src-tauri/tests/`
- Test naming: descriptive snake_case (e.g., `route_service_lookup_prefers_description_then_mac_and_friendly_name`)

---

## Common Patterns

### State Management
```typescript
// Local state only (no Redux/MobX)
const [loading, setLoading] = useState(false);
const [data, setData] = useState<DataType[]>([]);

// Refs for mutable values and DOM references
const timerRef = useRef<number | null>(null);
const outputRef = useRef<HTMLPreElement | null>(null);
```

### Conditional Rendering
```typescript
// Early returns for guards
if (!open) return null;

// Ternary for simple conditions
{loading ? "Loading..." : "Ready"}

// Logical AND for conditional rendering
{plan?.truncated && <span>Limited</span>}
```

### Null Handling
```typescript
// Nullish coalescing
const value = localPreference ?? defaultValue;

// Optional chaining
const count = summary?.cycle_count ?? "--";
```

---

## Testing Guidelines

### Node.js Tests (Frontend)
- Use Node.js native test runner (`node:test`)
- Assertions via `node:assert/strict`
- Test names: descriptive, present tense, behavior-focused
- Deep equality for objects: `assert.deepEqual(actual, expected)`

### Rust Tests (Backend)
- Use `#[test]` attribute for test functions
- Run via `cargo test` or `npm run test:rust`
- Integration tests in `src-tauri/tests/`

---

## Important Notes

- **Platform**: Windows-only (uses `windows-sys` and `windows-service` crates)
- **Architecture**: Tauri app with React frontend + Rust backend
- **Port**: Dev server runs on port 1420, HMR on 1421
- **TypeScript**: Strict mode enabled with comprehensive linting
- **No ESLint/Prettier**: Style enforced manually through conventions
- **Binaries**: 4 Rust binaries (SuperRoute, SuperRouteRepairBroker, SuperRouteService, speed_test_probe)

---

## Key Dependencies

**Frontend:**
- React 19, Vite 7, TypeScript 5.8
- Tauri 2 (`@tauri-apps/api`, `@tauri-apps/plugin-opener`)
- Tailwind CSS 4, Lucide React (icons)

**Backend:**
- Tauri 2, Serde (JSON serialization)
- Reqwest (HTTP client), Chrono (time), Futures
- Windows-sys (Windows API bindings)
- Windows-service (Windows service management)

---

## Scripts Location

Build and release automation scripts are in `scripts/` directory (PowerShell):
- `prepare-repair-sidecars.ps1`
- `run-rust-test.ps1`
- `bump-version.ps1`
- `release-ship.ps1`
- `release.ps1`
- `run-tauri.mjs`

---

*This guide was generated for AI coding agents working on SuperRoutePro. Keep it updated as conventions evolve.*
