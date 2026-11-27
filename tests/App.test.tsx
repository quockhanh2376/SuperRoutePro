import "./setupDom.ts";

import assert from "node:assert/strict";
import { afterEach, test } from "node:test";
import { useCallback, useEffect, useRef, useState } from "react";
import { act, cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";

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

test("App does not duplicate Repair Mode restore sentence in route-watcher toast", async () => {
  let routeWatcherHandler: ((event: { payload: unknown }) => void) | null = null;
  render(
    <App
      overrides={{
        ...buildOverrides(),
        listenToEvent: async (eventName, handler) => {
          if (eventName === "route-watcher://status") {
            routeWatcherHandler = handler as (event: { payload: unknown }) => void;
          }
          return () => {};
        },
      }}
    />,
  );

  await screen.findByText("Intel Test Adapter");
  await waitFor(() => {
    assert.ok(routeWatcherHandler);
  });

  act(() => {
    routeWatcherHandler!({
      payload: {
        status: "reapplied",
        title: "Routes Restored",
        message: "Restored 2 routes. Repair Mode handled the restore.",
        used_repair_host: true,
      },
    });
  });

  const toastMessages = await screen.findAllByText(/Restored 2 routes\./);
  assert.ok(toastMessages.length >= 1);
  for (const el of toastMessages) {
    const matches = (el.textContent ?? "").match(/Repair Mode handled the restore\./g) ?? [];
    assert.equal(matches.length, 1);
  }
});

test("App rejects CLI flag targets when running tracert", async () => {
  const networkCommands: string[] = [];
  render(
    <App
      overrides={{
        ...buildOverrides(),
        usePingMonitorHook: () => ({
          pingTarget: "-h",
          setPingTarget: () => {},
          pingMode: "ping",
          setPingMode: () => {},
          pingRunning: false,
          handleStartPing: () => {},
          handleStopPing: () => {},
        }),
        runNetworkCommandFn: async (command) => {
          networkCommands.push(command);
          return { success: true, output: "ok" };
        },
      }}
    />,
  );

  await screen.findByText("Intel Test Adapter");
  fireEvent.click(screen.getByRole("button", { name: "Ping & Tracert Monitor" }));
  fireEvent.click(screen.getByRole("button", { name: "Tracert" }));

  await waitFor(() => {
    assert.match(screen.getByRole("status").textContent ?? "", /Invalid tracert target/);
  });
  assert.equal(networkCommands.length, 0);
});

test("App registers route-watcher listener once and reloads NICs when toggling Active only", async () => {
  const inactiveNic: NetworkInterface = {
    index: "12",
    ip: "",
    gateway: "",
    description: "Offline Virtual Adapter",
  };
  let listenerRegistrations = 0;
  const loadCalls: Array<boolean | undefined> = [];

  render(
    <App
      overrides={{
        ...buildOverrides(),
        listenToEvent: async (eventName) => {
          if (eventName === "route-watcher://status") {
            listenerRegistrations += 1;
          }
          return () => {};
        },
        useNetworkSnapshotHook: ({ setStatusMessage }) => {
          const [activeOnly, setActiveOnlyState] = useState(true);
          const [nics, setNics] = useState<NetworkInterface[]>([DEFAULT_NIC]);
          const activeOnlyRef = useRef(true);

          const setActiveOnly = useCallback((next: boolean) => {
            activeOnlyRef.current = next;
            setActiveOnlyState(next);
          }, []);

          const loadData = useCallback(async (options?: {
            invalidateNicCache?: boolean;
            activeOnlyOverride?: boolean;
          }) => {
            const effectiveActiveOnly = options?.activeOnlyOverride ?? activeOnlyRef.current;
            loadCalls.push(effectiveActiveOnly);
            const nextNics = effectiveActiveOnly ? [DEFAULT_NIC] : [DEFAULT_NIC, inactiveNic];
            setNics(nextNics);
            setStatusMessage(`Loaded ${nextNics.length} NICs, ${DEFAULT_ROUTES.length} routes`);
          }, [setStatusMessage]);

          return {
            activeOnly,
            hasLoadedNicSnapshot: true,
            loading: false,
            nics,
            routes: DEFAULT_ROUTES,
            selectedNic: nics[0] ?? null,
            loadData,
            setActiveOnly,
            setRoutes: () => {},
            setSelectedNic: () => {},
          };
        },
      }}
    />,
  );

  await screen.findByText("Intel Test Adapter");
  assert.equal(listenerRegistrations, 1);
  assert.equal(screen.queryByText("Offline Virtual Adapter"), null);

  const activeOnlyCheckbox = screen.getByRole("checkbox", { name: "Active only" });
  fireEvent.click(activeOnlyCheckbox);

  await screen.findByText("Offline Virtual Adapter");
  assert.equal(listenerRegistrations, 1);
  assert.deepEqual(loadCalls, [true, false]);

  fireEvent.click(activeOnlyCheckbox);

  await waitFor(() => {
    assert.equal(screen.queryByText("Offline Virtual Adapter"), null);
  });
  assert.equal(listenerRegistrations, 1);
  assert.deepEqual(loadCalls, [true, false, true]);
});
