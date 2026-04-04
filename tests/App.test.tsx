import "./setupDom.ts";

import assert from "node:assert/strict";
import { afterEach, test } from "node:test";
import { useEffect } from "react";
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";

import App, { type AppOverrides } from "../src/App.tsx";
import type {
  BatterySummaryResult,
  FpingHostResult,
  NetworkInterface,
  RepairSessionStatus,
  RouteEntry,
} from "../src/api.ts";

const DEFAULT_NIC: NetworkInterface = {
  index: "11",
  ip: "192.168.88.25",
  gateway: "192.168.88.1",
  description: "Intel Test Adapter",
};

const DEFAULT_ROUTES: RouteEntry[] = [
  {
    destination: "192.168.88.0",
    netmask: "255.255.255.0",
    gateway: "0.0.0.0",
    metric: "10",
    interface_index: "11",
  },
];

const UNLOCKED_REPAIR_SESSION: RepairSessionStatus = {
  locked: false,
  connected: true,
  target_sid: null,
  requires_unlock: false,
};

afterEach(() => {
  cleanup();
  localStorage.clear();
  sessionStorage.clear();
  document.body.innerHTML = "";
});

function buildOverrides(options?: {
  executeAddRouteActionFn?: AppOverrides["executeAddRouteActionFn"];
  networkSnapshotError?: string;
  nics?: NetworkInterface[];
  repairLocked?: boolean;
  routes?: RouteEntry[];
}): Partial<AppOverrides> {
  const nics = options?.nics ?? [DEFAULT_NIC];
  const routes = options?.routes ?? DEFAULT_ROUTES;

  return {
    useAppShellStateHook: () => ({
      appVersion: "10.2.0-test",
      helpLanguage: "vi",
      persistWanLoading: false,
      persistWanOnStartup: false,
      theme: "dark",
      themeLensActive: false,
      zoomLevel: 100,
      handleToggleTheme: () => {},
      handleZoomIn: () => {},
      handleZoomOut: () => {},
      handleZoomReset: () => {},
      setHelpLanguage: () => {},
      setPersistWanOnStartup: () => {},
    }),
    useNetworkSnapshotHook: ({ setStatusMessage }) => {
      useEffect(() => {
        if (options?.networkSnapshotError) {
          setStatusMessage(`Loading network snapshot failed: ${options.networkSnapshotError}`);
          return;
        }

        setStatusMessage(`Loaded ${nics.length} NICs, ${routes.length} routes`);
      }, [setStatusMessage]);

      return {
        activeOnly: true,
        hasLoadedNicSnapshot: true,
        loading: false,
        nics: options?.networkSnapshotError ? [] : nics,
        routes: options?.networkSnapshotError ? [] : routes,
        selectedNic: options?.networkSnapshotError ? null : nics[0] ?? null,
        loadData: async () => {},
        setActiveOnly: () => {},
        setRoutes: () => {},
        setSelectedNic: () => {},
      };
    },
    useRepairModeHook: () => ({
      repairSession: options?.repairLocked
        ? {
            locked: true,
            connected: false,
            target_sid: null,
            requires_unlock: true,
          }
        : UNLOCKED_REPAIR_SESSION,
      setRepairSession: () => {},
      selectedRepairTargetSid: null,
      setSelectedRepairTargetSid: () => {},
      repairLoading: false,
      repairUnlocking: false,
      loadRepairTargets: async () => {},
      handleUnlockRepair: async () => {},
      handleLockRepair: async () => {},
    }),
    useNetworkMonitoringHook: () => ({
      isOnline: true,
      currentLatency: 12,
    }),
    usePingMonitorHook: () => ({
      pingTarget: "",
      setPingTarget: () => {},
      pingMode: "ping",
      setPingMode: () => {},
      pingRunning: false,
      handleStartPing: () => {},
      handleStopPing: () => {},
    }),
    useBloatwareManagerHook: () => ({
      open: false,
      loading: false,
      removing: false,
      items: [],
      selectedPackages: new Set<string>(),
      selectedCount: 0,
      installedCount: 0,
      progressPercent: 0,
      progressText: "",
      handleOpenModal: () => {},
      handleCloseModal: () => {},
      handleTogglePackage: () => {},
      handleSelectInstalled: () => {},
      handleSelectAll: () => {},
      handleClearSelection: () => {},
      handleRemoveSelected: () => {},
    }),
    useCacheCleanupManagerHook: () => ({
      open: false,
      cleaning: false,
      stopPending: false,
      options: [],
      selectedCaches: new Set<string>(),
      selectedCount: 0,
      progressPercent: 0,
      progressText: "",
      handleOpenModal: () => {},
      handleCloseModal: () => {},
      handleToggleCache: () => {},
      handleSelectAll: () => {},
      handleClearSelection: () => {},
      handleForceStop: () => {},
      handleStartCleanup: () => {},
    }),
    useBatterySummaryHook: () => ({
      error: "",
      loading: false,
      modal: {
        close: () => {},
        isOpen: false,
        open: () => {},
      },
      summary: null as BatterySummaryResult | null,
      handleCloseModal: () => {},
      handleOpenModal: () => {},
      loadSummary: async () => {},
    }),
    executeAddRouteActionFn: options?.executeAddRouteActionFn ?? (async () => {}),
    executeDeleteRouteActionFn: async () => {},
    executeFlushRoutesActionFn: async () => {},
    executeRepairActionImplFn: async () => {},
    executeSetInternetActionFn: async () => {},
    fpingScanFn: async () => ({
      sent: 1,
      received: 0,
      loss_percent: 100,
      min_ms: 0,
      avg_ms: 0,
      max_ms: 0,
      hosts: [] as FpingHostResult[],
    }),
    getRoutingTableFn: async () => routes,
    handleRepairCommandResultImplFn: async () => true,
    listenToEvent: async () => () => {},
    runNetworkCommandFn: async () => ({
      success: true,
      output: "Command completed",
    }),
    testTcpPortFn: async () => ({
      success: true,
      output: "Port open",
    }),
  };
}

test("App loads and renders the current NIC snapshot", async () => {
  render(<App overrides={buildOverrides()} />);

  await screen.findByText("Intel Test Adapter");
  assert.match(screen.getByRole("status").textContent ?? "", /Loaded 1 NICs, 1 routes/);
});

test("App shows inline validation and blocks invalid route submissions", async () => {
  const addRouteCalls: number[] = [];
  render(
    <App
      overrides={buildOverrides({
        executeAddRouteActionFn: async () => {
          addRouteCalls.push(1);
        },
      })}
    />,
  );

  await screen.findByText("Intel Test Adapter");

  const destinationField = screen.getByLabelText("Destination");
  const gatewayField = screen.getByLabelText("Gateway");
  fireEvent.change(destinationField, { target: { value: "10.0.0.42" } });
  fireEvent.change(gatewayField, { target: { value: "192.168.88.1" } });
  fireEvent.blur(destinationField);

  await screen.findByText("Destination must be the network address for the provided subnet mask.");
  fireEvent.click(screen.getByRole("button", { name: "ADD" }));

  assert.equal(addRouteCalls.length, 0);
  assert.match(
    screen.getByRole("status").textContent ?? "",
    /Destination must be the network address/,
  );
});

test("App lets the operator select a NIC row and auto-fill the gateway field", async () => {
  render(<App overrides={buildOverrides()} />);

  await screen.findByText("Intel Test Adapter");
  fireEvent.click(screen.getByRole("button", { name: /11 192\.168\.88\.25 192\.168\.88\.1 Intel Test Adapter/i }));

  assert.equal(
    (screen.getByLabelText("Gateway") as HTMLInputElement).value,
    "192.168.88.1",
  );
});

test("App forwards valid route submissions to the add-route action", async () => {
  const addRouteCalls: number[] = [];
  render(
    <App
      overrides={buildOverrides({
        executeAddRouteActionFn: async () => {
          addRouteCalls.push(1);
        },
      })}
    />,
  );

  await screen.findByText("Intel Test Adapter");
  fireEvent.change(screen.getByLabelText("Destination"), { target: { value: "10.0.0.0" } });
  fireEvent.change(screen.getByLabelText("Gateway"), { target: { value: "192.168.88.1" } });
  fireEvent.click(screen.getByRole("button", { name: "ADD" }));

  await waitFor(() => {
    assert.equal(addRouteCalls.length, 1);
  });
});

test("App surfaces snapshot loading failures with a consistent status message", async () => {
  render(
    <App
      overrides={buildOverrides({
        networkSnapshotError: "offline",
      })}
    />,
  );

  await waitFor(() => {
    assert.match(
      screen.getByRole("status").textContent ?? "",
      /Loading network snapshot failed: offline/,
    );
  });
});

test("App opens the help modal from the footer", async () => {
  render(<App overrides={buildOverrides()} />);

  await screen.findByText("Intel Test Adapter");
  fireEvent.click(screen.getByRole("button", { name: /Help/i }));

  await screen.findByRole("dialog", { name: "Application Help" });
});

test("App disables machine repair actions while Repair Mode is locked", async () => {
  render(
    <App
      overrides={buildOverrides({
        repairLocked: true,
      })}
    />,
  );

  await screen.findByText("Intel Test Adapter");
  assert.equal(screen.getByRole("button", { name: "ADD" }).hasAttribute("disabled"), true);
  assert.equal(screen.getByRole("button", { name: "WAN" }).hasAttribute("disabled"), true);
});
