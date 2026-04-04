import { useState, useEffect, useCallback, useMemo, useRef } from "react";
import { listen } from "@tauri-apps/api/event";
import {
  Zap, Wifi, WifiOff, RefreshCw, Plus, Minus, Trash2, Globe, Flame,
  Activity, Send, Wrench, Monitor, Sun, Moon, OctagonAlert, Search,
  ArrowDownUp, X, CircleHelp
} from "lucide-react";
import {
  getRoutingTable,
  runNetworkCommand,
  testTcpPort,
  fpingScan,
  type FpingHostResult,
  type NetworkInterface,
  type RepairMachineAction,
} from "./api";
import {
  APP_AUTHOR,
  IP_SCAN_BATCH_SIZE,
  ROUTE_WATCHER_STATUS_EVENT,
  ZOOM_MAX,
  ZOOM_MIN,
} from "./constants/app";
import { formatRoutingSnapshot } from "./constants/routeTable";
import {
  formatActionResultMessage,
  formatErrorMessage,
  formatOutputError,
  getFirstValidationError,
  isAdminElevationError,
} from "./errorUtils";
import {
  getProfileSensitiveActionHint,
  isMachineRepairEnabled,
  isProfileSensitiveActionEnabled,
} from "./repairModeModel";
import { getNicTableMessage } from "./nicTableModel";
import { validateRouteForm } from "./networkValidation";
import { SpeedTestModal } from "./SpeedTestModal";
import { BatteryModal } from "./components/BatteryModal";
import { BloatwareModal } from "./components/BloatwareModal";
import { CacheModal } from "./components/CacheModal";
import { ConfirmDialog } from "./components/ConfirmDialog";
import { DonateModal } from "./components/DonateModal";
import { HelpModal } from "./components/HelpModal";
import { ActionBtn, Field, OutputConsole, Section, ToolBtn } from "./components/AppChrome";
import { IpScanModal } from "./components/IpScanModal";
import { useAutoScroll } from "./hooks/useAutoScroll";
import { useAppShellState } from "./hooks/useAppShellState";
import { useBatterySummary } from "./hooks/useBatterySummary";
import { useBloatwareManager } from "./hooks/useBloatwareManager";
import { useCacheCleanupManager } from "./hooks/useCacheCleanupManager";
import { useConfirmDialog } from "./hooks/useConfirmDialog";
import { buildIpScanPlan, type IpScanPlan } from "./hooks/ipScanPlan";
import { useNetworkMonitoring } from "./hooks/useNetworkMonitoring";
import { useNetworkSnapshot } from "./hooks/useNetworkSnapshot";
import { usePingMonitor } from "./hooks/usePingMonitor";
import { useProgressTracker } from "./hooks/useProgressTracker";
import { useRepairMode } from "./hooks/useRepairMode";
import { useBufferedLog } from "./hooks/useBufferedLog";
import { useModal } from "./hooks/useModal";
import {
  executeRepairAction as executeRepairActionImpl,
  handleRepairCommandResult as handleRepairCommandResultImpl,
} from "./repairActions";
import {
  executeAddRouteAction,
  executeDeleteRouteAction,
  executeFlushRoutesAction,
  executeSetInternetAction,
} from "./routeActions";

type RouteWatcherStatusEventPayload = {
  status: "reapplied" | "failed";
  title: string;
  message: string;
  detail: string | null;
  used_repair_host: boolean;
};

type RouteWatcherToast = {
  tone: "success" | "warning";
  title: string;
  message: string;
  actionLabel?: string;
};

type PanelState = {
  toolsOpen: boolean;
  diagnosticsOpen: boolean;
  pingOpen: boolean;
};

type DiagnosticsInputsState = {
  host: string;
  dnsServer: string;
  port: string;
};

type RouteFormState = {
  dest: string;
  mask: string;
  gw: string;
  metric: string;
};

type RouteFormTouchedState = Record<keyof RouteFormState, boolean>;

const INITIAL_ROUTE_FORM_TOUCHED: RouteFormTouchedState = {
  dest: false,
  mask: false,
  gw: false,
  metric: false,
};

export type AppOverrides = {
  executeAddRouteActionFn: typeof executeAddRouteAction;
  executeDeleteRouteActionFn: typeof executeDeleteRouteAction;
  executeFlushRoutesActionFn: typeof executeFlushRoutesAction;
  executeRepairActionImplFn: typeof executeRepairActionImpl;
  executeSetInternetActionFn: typeof executeSetInternetAction;
  fpingScanFn: typeof fpingScan;
  getRoutingTableFn: typeof getRoutingTable;
  handleRepairCommandResultImplFn: typeof handleRepairCommandResultImpl;
  listenToEvent: typeof listen;
  runNetworkCommandFn: typeof runNetworkCommand;
  testTcpPortFn: typeof testTcpPort;
  useAppShellStateHook: typeof useAppShellState;
  useBatterySummaryHook: typeof useBatterySummary;
  useBloatwareManagerHook: typeof useBloatwareManager;
  useCacheCleanupManagerHook: typeof useCacheCleanupManager;
  useNetworkMonitoringHook: typeof useNetworkMonitoring;
  useNetworkSnapshotHook: typeof useNetworkSnapshot;
  usePingMonitorHook: typeof usePingMonitor;
  useRepairModeHook: typeof useRepairMode;
};

type AppProps = {
  overrides?: Partial<AppOverrides>;
};

export default function App({ overrides }: AppProps = {}) {
  const executeAddRouteActionFn = overrides?.executeAddRouteActionFn ?? executeAddRouteAction;
  const executeDeleteRouteActionFn = overrides?.executeDeleteRouteActionFn ?? executeDeleteRouteAction;
  const executeFlushRoutesActionFn = overrides?.executeFlushRoutesActionFn ?? executeFlushRoutesAction;
  const executeRepairActionImplFn = overrides?.executeRepairActionImplFn ?? executeRepairActionImpl;
  const executeSetInternetActionFn = overrides?.executeSetInternetActionFn ?? executeSetInternetAction;
  const fpingScanFn = overrides?.fpingScanFn ?? fpingScan;
  const getRoutingTableFn = overrides?.getRoutingTableFn ?? getRoutingTable;
  const handleRepairCommandResultImplFn =
    overrides?.handleRepairCommandResultImplFn ?? handleRepairCommandResultImpl;
  const listenToEvent = overrides?.listenToEvent ?? listen;
  const runNetworkCommandFn = overrides?.runNetworkCommandFn ?? runNetworkCommand;
  const testTcpPortFn = overrides?.testTcpPortFn ?? testTcpPort;
  const useAppShellStateHook = overrides?.useAppShellStateHook ?? useAppShellState;
  const useBatterySummaryHook = overrides?.useBatterySummaryHook ?? useBatterySummary;
  const useBloatwareManagerHook = overrides?.useBloatwareManagerHook ?? useBloatwareManager;
  const useCacheCleanupManagerHook = overrides?.useCacheCleanupManagerHook ?? useCacheCleanupManager;
  const useNetworkMonitoringHook = overrides?.useNetworkMonitoringHook ?? useNetworkMonitoring;
  const useNetworkSnapshotHook = overrides?.useNetworkSnapshotHook ?? useNetworkSnapshot;
  const usePingMonitorHook = overrides?.usePingMonitorHook ?? usePingMonitor;
  const useRepairModeHook = overrides?.useRepairModeHook ?? useRepairMode;

  const {
    appVersion,
    helpLanguage,
    persistWanLoading,
    persistWanOnStartup,
    theme,
    themeLensActive,
    zoomLevel,
    handleToggleTheme,
    handleZoomIn,
    handleZoomOut,
    handleZoomReset,
    setHelpLanguage,
    setPersistWanOnStartup,
  } = useAppShellStateHook();
  const [statusMsg, setStatusMsg] = useState("System Ready");
  const [routeWatcherToast, setRouteWatcherToast] = useState<RouteWatcherToast | null>(null);
  const {
    activeOnly,
    hasLoadedNicSnapshot,
    loading,
    nics,
    routes,
    selectedNic,
    loadData,
    setActiveOnly,
    setRoutes,
    setSelectedNic,
  } = useNetworkSnapshotHook({
    setStatusMessage: setStatusMsg,
  });
  const ipScanModal = useModal();
  const [ipScanRunning, setIpScanRunning] = useState(false);
  const [ipScanStopPending, setIpScanStopPending] = useState(false);
  const [ipScanPlan, setIpScanPlan] = useState<IpScanPlan | null>(null);
  const [ipScanResults, setIpScanResults] = useState<FpingHostResult[]>([]);
  const {
    percent: ipScanProgressPercent,
    text: ipScanProgressText,
    update: updateIpScanProgress,
    setMessage: setIpScanProgressText,
  } = useProgressTracker();
  const [panels, setPanels] = useState<PanelState>({
    toolsOpen: false,
    diagnosticsOpen: false,
    pingOpen: false,
  });
  const [diagnosticsInputs, setDiagnosticsInputs] = useState<DiagnosticsInputsState>({
    host: "google.com",
    dnsServer: "8.8.8.8",
    port: "443",
  });
  const [diagnosticView, setDiagnosticView] = useState<"command" | "routing">("command");
  const [routingOutput, setRoutingOutput] = useState("");
  const batterySummary = useBatterySummaryHook({
    setStatusMessage: setStatusMsg,
  });
  const donateModal = useModal();
  const helpModal = useModal();
  const {
    repairSession,
    setRepairSession,
    selectedRepairTargetSid,
    setSelectedRepairTargetSid,
    repairLoading,
    repairUnlocking,
    loadRepairTargets,
    handleUnlockRepair,
    handleLockRepair,
  } = useRepairModeHook({
    setStatusMessage: setStatusMsg,
  });
  const {
    confirmOpen,
    confirmTitle,
    confirmMessage,
    openConfirm,
    onConfirm,
    onCancelConfirm,
  } = useConfirmDialog({
    onErrorMessage: setStatusMsg,
  });
  const [routeForm, setRouteForm] = useState<RouteFormState>({
    dest: "",
    mask: "255.255.255.0",
    gw: "",
    metric: "10",
  });
  const [routeFormTouched, setRouteFormTouched] = useState<RouteFormTouchedState>(
    INITIAL_ROUTE_FORM_TOUCHED,
  );

  const {
    version: pingLogVersion,
    text: pingOutputText,
    appendLines: appendPingLines,
    appendLine: appendPingLine,
    clear: clearPingOutput,
  } = useBufferedLog(600);
  const {
    version: commandLogVersion,
    text: commandOutputText,
    appendLines: appendCommandLines,
    clear: clearCommandOutput,
  } = useBufferedLog(1200);
  const {
    pingTarget,
    setPingTarget,
    pingMode,
    setPingMode,
    pingRunning,
    handleStartPing,
    handleStopPing,
  } = usePingMonitorHook({
    appendLine: appendPingLine,
    appendLines: appendPingLines,
    setStatusMessage: setStatusMsg,
  });
  const { isOnline, currentLatency } = useNetworkMonitoringHook();

  const ipScanStopRequestedRef = useRef(false);
  const pingOutputRef = useRef<HTMLPreElement | null>(null);
  const commandOutputRef = useRef<HTMLPreElement | null>(null);
  const routeWatcherToastTimerRef = useRef<number | null>(null);
  useAutoScroll(pingOutputRef, pingLogVersion);
  useAutoScroll(commandOutputRef, commandLogVersion);

  const routeFormValidation = useMemo(
    () => validateRouteForm(routeForm),
    [routeForm],
  );
  const visibleRouteFormErrors = useMemo(() => ({
    dest: routeFormTouched.dest ? routeFormValidation.dest : undefined,
    mask: routeFormTouched.mask ? routeFormValidation.mask : undefined,
    gw: routeFormTouched.gw ? routeFormValidation.gw : undefined,
    metric: routeFormTouched.metric ? routeFormValidation.metric : undefined,
  }), [routeFormTouched, routeFormValidation]);

  const setRouteFormField = useCallback(<K extends keyof RouteFormState,>(field: K, value: RouteFormState[K]) => {
    setRouteForm((current) => ({
      ...current,
      [field]: value,
    }));
  }, []);

  const markRouteFieldTouched = useCallback((field: keyof RouteFormState) => {
    setRouteFormTouched((current) => ({
      ...current,
      [field]: true,
    }));
  }, []);

  const markAllRouteFieldsTouched = useCallback(() => {
    setRouteFormTouched({
      dest: true,
      mask: true,
      gw: true,
      metric: true,
    });
  }, []);

  const setDiagnosticsField = useCallback(<K extends keyof DiagnosticsInputsState,>(
    field: K,
    value: DiagnosticsInputsState[K],
  ) => {
    setDiagnosticsInputs((current) => ({
      ...current,
      [field]: value,
    }));
  }, []);

  const handleRouteDestChange = useCallback((value: string) => {
    setRouteFormField("dest", value);
  }, [setRouteFormField]);

  const handleRouteMaskChange = useCallback((value: string) => {
    setRouteFormField("mask", value);
  }, [setRouteFormField]);

  const handleRouteGatewayChange = useCallback((value: string) => {
    setRouteFormField("gw", value);
  }, [setRouteFormField]);

  const handleRouteMetricChange = useCallback((value: string) => {
    setRouteFormField("metric", value);
  }, [setRouteFormField]);

  const handleDiagHostChange = useCallback((value: string) => {
    setDiagnosticsField("host", value);
  }, [setDiagnosticsField]);

  const handleDiagDnsServerChange = useCallback((value: string) => {
    setDiagnosticsField("dnsServer", value);
  }, [setDiagnosticsField]);

  const handleDiagPortChange = useCallback((value: string) => {
    setDiagnosticsField("port", value);
  }, [setDiagnosticsField]);

  const handleToggleToolsPanel = useCallback(() => {
    setPanels((current) => ({
      ...current,
      toolsOpen: !current.toolsOpen,
    }));
  }, []);

  const handleToggleDiagnosticsPanel = useCallback(() => {
    setPanels((current) => ({
      ...current,
      diagnosticsOpen: !current.diagnosticsOpen,
    }));
  }, []);

  const handleTogglePingPanel = useCallback(() => {
    setPanels((current) => ({
      ...current,
      pingOpen: !current.pingOpen,
    }));
  }, []);

  const openCommandDiagnostics = useCallback(() => {
    setPanels((current) => ({
      ...current,
      diagnosticsOpen: true,
    }));
    setDiagnosticView("command");
  }, []);

  const pushRouteWatcherToast = useCallback((payload: RouteWatcherStatusEventPayload) => {
    setRouteWatcherToast({
      tone: payload.status === "reapplied" ? "success" : "warning",
      title: payload.title,
      message: payload.used_repair_host
        ? `${payload.message} Repair Mode handled the restore.`
        : payload.message,
      actionLabel: payload.status === "failed" ? "Open Routing Console" : undefined,
    });

    if (routeWatcherToastTimerRef.current !== null) {
      window.clearTimeout(routeWatcherToastTimerRef.current);
    }

    routeWatcherToastTimerRef.current = window.setTimeout(() => {
      setRouteWatcherToast(null);
      routeWatcherToastTimerRef.current = null;
    }, 6500);
  }, []);

  useEffect(() => {
    return () => {
      if (routeWatcherToastTimerRef.current !== null) {
        window.clearTimeout(routeWatcherToastTimerRef.current);
      }
    };
  }, []);

  const nicTableMessage = getNicTableMessage({
    nicCount: nics.length,
    loading,
    hasLoadedOnce: hasLoadedNicSnapshot,
    activeOnly,
  });

  useEffect(() => {
    void loadData();
  }, [loadData]);

  useEffect(() => {
    setRoutingOutput(formatRoutingSnapshot(routes));
  }, [routes]);

  useEffect(() => {
    let active = true;
    let cleanup = () => {};

    void listenToEvent<RouteWatcherStatusEventPayload>(ROUTE_WATCHER_STATUS_EVENT, ({ payload }) => {
      if (!active) {
        return;
      }

      setStatusMsg(payload.message);
      pushRouteWatcherToast(payload);

      if (payload.status === "reapplied") {
        void loadData({ invalidateNicCache: true });
      }
    }).then((unlisten) => {
      if (!active) {
        unlisten();
        return;
      }
      cleanup = unlisten;
    });

    return () => {
      active = false;
      cleanup();
    };
  }, [listenToEvent, loadData, pushRouteWatcherToast]);

  // ======================== ACTIONS ========================

  const appendCommandOutput = useCallback((title: string, output: string) => {
    const stamp = new Date().toLocaleTimeString("en-GB");
    const cleanOutput = output?.trim() ? output.trim() : "(No output returned)";
    const lines = [`[${stamp}] ${title}`, ...cleanOutput.split(/\r?\n/), ""];
    appendCommandLines(lines);
  }, [appendCommandLines]);

  const handleRepairCommandResult = useCallback(async (
    title: string,
    result: { success: boolean; output: string; requires_unlock: boolean },
    options?: {
      refresh?: boolean;
      invalidateNicCache?: boolean;
      appendOutput?: boolean;
      successMessage?: string;
      failureMessage?: string;
    },
  ) => {
    return handleRepairCommandResultImplFn({
      appendCommandOutput,
      setStatusMessage: setStatusMsg,
      setRepairSession,
      loadData,
    }, title, result, options);
  }, [appendCommandOutput, handleRepairCommandResultImplFn, loadData, setRepairSession]);

  const executeRepairAction = useCallback(async (
    action: RepairMachineAction,
    title: string,
    options?: { refresh?: boolean; invalidateNicCache?: boolean },
  ) => {
    return executeRepairActionImplFn({
      appendCommandOutput,
      setStatusMessage: setStatusMsg,
      setRepairSession,
      loadData,
      setDiagnosticView,
    }, action, title, options);
  }, [appendCommandOutput, executeRepairActionImplFn, loadData, setRepairSession]);

  const bloatwareManager = useBloatwareManagerHook({
    setStatusMessage: setStatusMsg,
    appendCommandOutput,
    openCommandDiagnostics,
    selectedRepairTargetSid,
    setSelectedRepairTargetSid,
    setRepairSession,
    loadRepairTargets,
    openConfirm,
  });

  const cacheCleanupManager = useCacheCleanupManagerHook({
    setStatusMessage: setStatusMsg,
    appendCommandOutput,
    openCommandDiagnostics,
    selectedRepairTargetSid,
    setSelectedRepairTargetSid,
    setRepairSession,
    loadRepairTargets,
    openConfirm,
  });

  const handleSelectNic = useCallback((nic: NetworkInterface) => {
    setSelectedNic(nic);
    setRouteForm((current) => ({
      ...current,
      gw: nic.gateway,
    }));
  }, [setSelectedNic]);

  const handleAddRoute = useCallback(async () => {
    markAllRouteFieldsTouched();
    const validationMessage = getFirstValidationError(routeFormValidation);
    if (validationMessage) {
      setStatusMsg(validationMessage);
      return;
    }

    await executeAddRouteActionFn({
      formDest: routeForm.dest,
      formMask: routeForm.mask,
      formGw: routeForm.gw,
      formMetric: routeForm.metric,
      selectedNicIndex: selectedNic?.index,
      setStatusMessage: setStatusMsg,
      handleRepairCommandResult,
    });
  }, [
    executeAddRouteActionFn,
    handleRepairCommandResult,
    markAllRouteFieldsTouched,
    routeForm,
    routeFormValidation,
    selectedNic?.index,
  ]);

  const handleDeleteRoute = useCallback(async () => {
    markRouteFieldTouched("dest");
    markRouteFieldTouched("mask");
    await executeDeleteRouteActionFn({
      formDest: routeForm.dest,
      formMask: routeForm.mask,
      setStatusMessage: setStatusMsg,
      handleRepairCommandResult,
    });
  }, [
    executeDeleteRouteActionFn,
    handleRepairCommandResult,
    markRouteFieldTouched,
    routeForm.dest,
    routeForm.mask,
  ]);

  const executeSetInternet = useCallback(async () => {
    await executeSetInternetActionFn({
      selectedNic,
      persistWanOnStartup,
      routes,
      setStatusMessage: setStatusMsg,
      handleRepairCommandResult,
    });
  }, [
    executeSetInternetActionFn,
    handleRepairCommandResult,
    persistWanOnStartup,
    routes,
    selectedNic,
  ]);

  const executeFlush = useCallback(async () => {
    await executeFlushRoutesActionFn({
      setStatusMessage: setStatusMsg,
      handleRepairCommandResult,
    });
  }, [executeFlushRoutesActionFn, handleRepairCommandResult]);

  const executeNetCmd = useCallback(async (
    cmd: string,
    title: string,
    options?: { refresh?: boolean; invalidateNicCache?: boolean }
  ) => {
    setDiagnosticView("command");
    setStatusMsg(`Running ${title}...`);
    try {
      const result = await runNetworkCommandFn(cmd);
      appendCommandOutput(title, result.output);
      if (isAdminElevationError(result.output)) {
        setStatusMsg(`${title} requires Administrator privileges.`);
      } else {
        setStatusMsg(formatActionResultMessage(title, result.success));
      }
      if (options?.refresh) {
        void loadData({ invalidateNicCache: options?.invalidateNicCache });
      }
    } catch (error: unknown) {
      appendCommandOutput(title, formatOutputError(error));
      setStatusMsg(formatErrorMessage(`${title} failed`, error));
    }
  }, [appendCommandOutput, loadData, runNetworkCommandFn]);

  const handleShowRoutingOutput = useCallback(async () => {
    setPanels((current) => ({
      ...current,
      diagnosticsOpen: true,
    }));
    setDiagnosticView("routing");
    if (routes.length > 0) {
      setRoutingOutput(formatRoutingSnapshot(routes));
      setStatusMsg(`Routing table snapshot loaded (${routes.length} cached routes)`);
      return;
    }

    setStatusMsg("Loading routing table snapshot...");
    try {
      const routeData = await getRoutingTableFn();
      setRoutes(routeData);
      setRoutingOutput(formatRoutingSnapshot(routeData));
      setStatusMsg(`Routing table snapshot loaded (${routeData.length} routes)`);
    } catch (error: unknown) {
      const errorText = formatErrorMessage("Routing table snapshot failed", error);
      setRoutingOutput(`Failed to load routing table snapshot.\n${errorText}`);
      setStatusMsg(errorText);
    }
  }, [getRoutingTableFn, routes, setRoutes]);

  const handleOpenRouteWatcherToastAction = useCallback(() => {
    if (routeWatcherToastTimerRef.current !== null) {
      window.clearTimeout(routeWatcherToastTimerRef.current);
      routeWatcherToastTimerRef.current = null;
    }
    setRouteWatcherToast(null);
    void handleShowRoutingOutput();
  }, [handleShowRoutingOutput]);

  const handleShowCommandOutput = useCallback(() => {
    setDiagnosticView("command");
  }, []);

  const sanitizeHostToken = useCallback((value: string) =>
    value.trim().replace(/[^a-zA-Z0-9.-]/g, ""), []);

  const sanitizeDnsToken = useCallback((value: string) =>
    value.trim().replace(/[^a-zA-Z0-9:.-]/g, ""), []);

  const handleDisplayDnsCache = useCallback(async () => {
    await executeNetCmd("ipconfig /displaydns", "Display DNS Cache");
  }, [executeNetCmd]);

  const handleResetWinHttpProxy = useCallback(() => {
    openConfirm(
      "Reset WinHTTP Proxy",
      "Reset WinHTTP proxy settings to direct access?",
      () => executeRepairAction("ResetWinHttpProxy", "Reset WinHTTP Proxy", { refresh: true }),
    );
  }, [executeRepairAction, openConfirm]);

  const handleRestartAdapters = useCallback(() => {
    openConfirm(
      "Restart Active Adapters",
      "Restart active physical network adapters now?",
      () => executeRepairAction("RestartActiveAdapters", "Restart Active Adapters", {
        refresh: true,
        invalidateNicCache: true,
      }),
    );
  }, [executeRepairAction, openConfirm]);

  const handleNslookupTest = useCallback(async () => {
    const host = sanitizeHostToken(diagnosticsInputs.host) || "google.com";
    const dns = sanitizeDnsToken(diagnosticsInputs.dnsServer) || "8.8.8.8";
    setDiagnosticsInputs((current) => ({
      ...current,
      host,
      dnsServer: dns,
    }));
    await executeNetCmd(`nslookup ${host} ${dns}`, `NSLookup ${host}`);
  }, [diagnosticsInputs.dnsServer, diagnosticsInputs.host, executeNetCmd, sanitizeDnsToken, sanitizeHostToken]);

  const handlePortConnectivityTest = useCallback(async () => {
    const host = sanitizeHostToken(diagnosticsInputs.host) || "google.com";
    const portNum = Number.parseInt(diagnosticsInputs.port, 10);
    const port = Number.isFinite(portNum) && portNum >= 1 && portNum <= 65535 ? portNum : 443;
    setDiagnosticsInputs((current) => ({
      ...current,
      host,
      port: String(port),
    }));
    setDiagnosticView("command");
    setStatusMsg(`Testing port ${host}:${port}...`);
    try {
      const result = await testTcpPortFn(host, port);
      appendCommandOutput(`Port Test ${host}:${port}`, result.output);
      setStatusMsg(result.success ? `Port ${port} open on ${host}` : `Port ${port} closed on ${host}`);
    } catch (error: unknown) {
      appendCommandOutput(`Port Test ${host}:${port}`, formatOutputError(error));
      setStatusMsg(formatErrorMessage("Port test failed", error));
    }
  }, [
    appendCommandOutput,
    diagnosticsInputs.host,
    diagnosticsInputs.port,
    sanitizeHostToken,
    testTcpPortFn,
  ]);

  const resolveIpScanPlan = useCallback((): IpScanPlan | null => {
    if (!selectedNic) return null;
    const plan = buildIpScanPlan(selectedNic, routes);
    if (!plan || plan.targets.length === 0) return null;
    return plan;
  }, [routes, selectedNic]);

  const runIpScan = useCallback(async (plan: IpScanPlan) => {
    if (ipScanRunning) return;
    setIpScanRunning(true);
    setIpScanStopPending(false);
    ipScanStopRequestedRef.current = false;
    setIpScanResults([]);
    updateIpScanProgress(0, `Starting scan on ${plan.subnetLabel}...`);
    setStatusMsg(`Scan IP started on ${plan.subnetLabel}`);

    const totalTargets = plan.targets.length;
    let processed = 0;
    let reachable = 0;
    const collected: FpingHostResult[] = [];

    try {
      for (let offset = 0; offset < totalTargets; offset += IP_SCAN_BATCH_SIZE) {
        if (ipScanStopRequestedRef.current) {
          break;
        }

        const batchTargets = plan.targets.slice(offset, offset + IP_SCAN_BATCH_SIZE);
        const result = await fpingScanFn(batchTargets, 700);
        collected.push(...result.hosts);
        processed += batchTargets.length;
        reachable += result.received;
        setIpScanResults([...collected]);

        const percent = Math.round((processed / totalTargets) * 100);
        updateIpScanProgress(
          percent,
          `Scanning ${processed}/${totalTargets} hosts... Reachable ${reachable}`
        );
      }

      const stoppedEarly = ipScanStopRequestedRef.current && processed < totalTargets;
      if (stoppedEarly) {
        setStatusMsg(`Scan IP stopped (${processed}/${totalTargets})`);
        setIpScanProgressText(`Stopped: scanned ${processed}/${totalTargets}, reachable ${reachable}`);
      } else {
        setStatusMsg(`Scan IP done: ${reachable}/${totalTargets} reachable`);
        setIpScanProgressText(`Done: scanned ${totalTargets} hosts, reachable ${reachable}`);
      }
    } catch (error: unknown) {
      setStatusMsg(formatErrorMessage("Scan IP failed", error));
      setIpScanProgressText(`Scan failed: ${formatErrorMessage("Error", error)}`);
    } finally {
      setIpScanRunning(false);
      setIpScanStopPending(false);
      ipScanStopRequestedRef.current = false;
    }
  }, [fpingScanFn, ipScanRunning, updateIpScanProgress]);

  const handleOpenIpScanModal = useCallback(() => {
    const plan = resolveIpScanPlan();
    if (!plan) {
      setStatusMsg("Select an active NIC first to scan subnet hosts");
      return;
    }
    setIpScanPlan(plan);
    ipScanModal.open();
    setIpScanResults([]);
    if (plan.source === "fallback") {
      updateIpScanProgress(0, `Using fallback ${plan.subnetLabel} range from selected NIC.`);
    } else {
      updateIpScanProgress(0, `Ready to scan ${plan.targets.length} hosts on ${plan.subnetLabel}.`);
    }
  }, [ipScanModal, resolveIpScanPlan, updateIpScanProgress]);

  const handleStartIpScan = useCallback(() => {
    if (ipScanRunning) return;
    const plan = resolveIpScanPlan();
    if (!plan) {
      setStatusMsg("Select an active NIC first to scan subnet hosts");
      setIpScanProgressText("Cannot build scan plan from current selection.");
      return;
    }
    setIpScanPlan(plan);
    void runIpScan(plan);
  }, [ipScanRunning, resolveIpScanPlan, runIpScan]);

  const handleForceStopIpScan = useCallback(() => {
    if (!ipScanRunning || ipScanStopPending) return;
    ipScanStopRequestedRef.current = true;
    setIpScanStopPending(true);
    setStatusMsg("Force stop requested for IP scan...");
    setIpScanProgressText("Stopping scan... waiting for current batch.");
  }, [ipScanRunning, ipScanStopPending]);

  const handleCloseIpScanModal = useCallback(() => {
    if (ipScanRunning) return;
    ipScanModal.close();
  }, [ipScanModal, ipScanRunning]);

  const handleTracertFromTarget = useCallback(async () => {
    const target = pingTarget
      .trim()
      .split(/[\s,;]+/)
      .map((t) => t.trim())
      .find(Boolean) || "8.8.8.8";
    appendPingLine(`--- Tracert ${target} ---`);
    setStatusMsg(`Running tracert ${target}...`);
    try {
      const result = await runNetworkCommandFn(`tracert -d ${target}`);
      appendPingLines(result.output.trim().split(/\r?\n/));
      setStatusMsg(result.success ? `Tracert ${target} done` : `Tracert ${target} failed`);
    } catch (error: unknown) {
      appendPingLine(formatErrorMessage("Tracert failed", error));
      setStatusMsg(formatErrorMessage("Tracert failed", error));
    }
  }, [appendPingLine, appendPingLines, pingTarget, runNetworkCommandFn]);

  // ======================== RENDER ========================

  const diagnosticsOutputText = useMemo(() => (
    diagnosticView === "routing"
      ? (routingOutput || "Routing table output will appear here.")
      : (commandOutputText || "Command output will appear here.")
  ), [commandOutputText, diagnosticView, routingOutput]);
  const machineRepairEnabled = useMemo(
    () => isMachineRepairEnabled({ locked: repairSession.locked }),
    [repairSession.locked],
  );
  const profileSensitiveActionEnabled = useMemo(() => isProfileSensitiveActionEnabled({
    locked: repairSession.locked,
    selectedTargetSid: selectedRepairTargetSid,
  }), [repairSession.locked, selectedRepairTargetSid]);
  const profileSensitiveActionHint = useMemo(() => getProfileSensitiveActionHint({
    locked: repairSession.locked,
    selectedTargetSid: selectedRepairTargetSid,
  }), [repairSession.locked, selectedRepairTargetSid]);

  return (
    <div className={`app-shell ${theme === "light" ? "theme-light" : "theme-dark"} h-screen flex flex-col font-['Segoe_UI',system-ui,sans-serif] overflow-hidden select-none`}>
      {/* ====== HEADER ====== */}
      <header className="app-header flex items-center justify-between px-5 py-3 border-b shrink-0">
        <div className="flex items-center gap-3 min-w-0">
          <div className="brand-logo">
            <Zap className="w-6 h-6" />
          </div>
          <div>
            <h1 className="title-text text-lg font-bold tracking-tight">SUPER ROUTE PRO</h1>
            <p className="version-text text-[0.8rem] font-semibold -mt-0.5">
              SuperRoute Pro V.{appVersion} | Author {APP_AUTHOR}
            </p>
          </div>
        </div>

        <div className="flex items-center gap-2">
          <div className="zoom-control-header" title="Adjust interface zoom">
            <button
              type="button"
              onClick={handleZoomOut}
              disabled={zoomLevel <= ZOOM_MIN}
              className="zoom-btn-header"
              title="Zoom out"
            >
              <Minus className="w-3.5 h-3.5" />
            </button>
            <button
              type="button"
              onClick={handleZoomReset}
              className="zoom-label-header"
              title="Reset zoom to 100%"
            >
              {zoomLevel}%
            </button>
            <button
              type="button"
              onClick={handleZoomIn}
              disabled={zoomLevel >= ZOOM_MAX}
              className="zoom-btn-header"
              title="Zoom in"
            >
              <Plus className="w-3.5 h-3.5" />
            </button>
          </div>

          <button
            type="button"
            onClick={repairSession.locked ? handleUnlockRepair : handleLockRepair}
            disabled={repairUnlocking || repairLoading}
            className={`header-lock-action capsule-btn ${
              repairSession.locked ? "header-lock-action-locked" : "header-lock-action-unlocked"
            }`}
            title={repairSession.locked ? "Unlock Repair Mode" : "Lock Repair Mode"}
          >
            {repairSession.locked
              ? (repairUnlocking ? "Unlocking..." : "Unlock")
              : "Lock"}
          </button>

          <button
            type="button"
            onClick={bloatwareManager.handleOpenModal}
            disabled={!profileSensitiveActionEnabled || bloatwareManager.loading || bloatwareManager.removing}
            className="header-apps-action capsule-btn"
            title={profileSensitiveActionHint || "Open app removal tools"}
          >
            <Trash2 className="w-3.5 h-3.5" />
            Remove Apps
          </button>

          <button
            type="button"
            onClick={cacheCleanupManager.handleOpenModal}
            disabled={!profileSensitiveActionEnabled || cacheCleanupManager.cleaning}
            className="header-cache-action capsule-btn"
            title={profileSensitiveActionHint || "Open cache cleanup tools"}
          >
            <Flame className="w-3.5 h-3.5" />
            Clear Cache
          </button>

          <button
            type="button"
            onClick={handleToggleTheme}
            className="theme-toggle capsule-btn flex items-center gap-1.5 px-2.5 py-1.5 text-xs font-semibold transition"
            title="Toggle light/dark mode"
            aria-pressed={theme === "light"}
          >
            {theme === "dark" ? <Sun className="w-3.5 h-3.5" /> : <Moon className="w-3.5 h-3.5" />}
            {theme === "dark" ? "Light" : "Dark"}
          </button>

          <div className={`online-pill ${
            isOnline === null ? "online-pill-checking" :
            isOnline ? "online-pill-on" : "online-pill-off"
          }`}>
            {isOnline ? <Wifi className="w-3.5 h-3.5" /> : <WifiOff className="w-3.5 h-3.5" />}
            <span className="pulse-dot">.</span>
            {isOnline === null ? "Checking..." : isOnline ? "ONLINE" : "OFFLINE"}
          </div>

          <div className="ms-pill">
            {currentLatency > 0 ? `${currentLatency} ms` : "-- ms"}
          </div>
        </div>
      </header>

      {profileSensitiveActionHint && (
        <div className="px-5 pt-2 shrink-0">
          <p className="profile-sensitive-action-hint text-[0.72rem] text-amber-300">
            {profileSensitiveActionHint}
          </p>
        </div>
      )}



      {/* ====== MAIN CONTENT ====== */}
      <div className="content-grid flex-1 overflow-hidden">
        {/* --- LEFT PANEL --- */}
        <div className="left-panel flex flex-col border-r overflow-hidden">
          {/* NIC List */}
          <div className="p-3 border-b border-slate-700/30">
            <div className="flex items-center justify-between mb-2">
              <div className="flex items-center gap-2">
                <Monitor className="w-4 h-4 text-blue-400" />
                <span className="text-xs font-bold text-slate-400 uppercase tracking-wider">Network Interfaces</span>
              </div>
              <div className="flex items-center gap-2">
                <label className="flex items-center gap-1.5 cursor-pointer">
                  <input
                    type="checkbox"
                    checked={activeOnly}
                    onChange={(e) => setActiveOnly(e.target.checked)}
                    className="w-3 h-3 rounded accent-blue-500"
                  />
                  <span className="text-[0.65rem] text-slate-500">Active only</span>
                </label>
                <button
                  type="button"
                  onClick={() => {
                    void loadData({ invalidateNicCache: true });
                  }}
                  disabled={loading}
                  className="capsule-btn p-1.5 hover:bg-slate-700/50 text-slate-400 hover:text-white transition disabled:opacity-50"
                  title="Refresh"
                >
                  <RefreshCw className={`w-3.5 h-3.5 ${loading ? "animate-spin" : ""}`} />
                </button>
              </div>
            </div>
            <div className="overflow-auto max-h-[140px] rounded-lg border border-slate-700/50 bg-[#0c1220]">
              <table className="data-table">
                <thead>
                  <tr>
                    <th scope="col" className="w-12">ID</th>
                    <th scope="col" className="w-28">IPv4</th>
                    <th scope="col" className="w-28">Gateway</th>
                    <th scope="col">Device</th>
                  </tr>
                </thead>
                <tbody>
                  {nics.map((nic) => (
                    <tr
                      key={nic.index}
                      onClick={() => handleSelectNic(nic)}
                      onKeyDown={(event) => {
                        if (event.key === "Enter" || event.key === " ") {
                          event.preventDefault();
                          handleSelectNic(nic);
                        }
                      }}
                      role="button"
                      tabIndex={0}
                      aria-pressed={selectedNic?.index === nic.index}
                      className={selectedNic?.index === nic.index ? "selected" : ""}
                    >
                      <td className="font-mono text-blue-300">{nic.index}</td>
                      <td className="font-mono">{nic.ip}</td>
                      <td className="font-mono text-slate-400">{nic.gateway || "-"}</td>
                      <td className="truncate max-w-[150px]" title={nic.description}>{nic.description}</td>
                    </tr>
                  ))}
                  {nicTableMessage && (
                    <tr><td colSpan={4} className="text-center text-slate-600 py-4">{nicTableMessage}</td></tr>
                  )}
                </tbody>
              </table>
            </div>
          </div>

          {/* Config Form */}
          <div className="p-3 border-b border-slate-700/30">
            <div className="route-form-grid mb-2">
              <Field
                fieldId="route-destination"
                label="Destination"
                value={routeForm.dest}
                onChange={handleRouteDestChange}
                onBlur={() => markRouteFieldTouched("dest")}
                error={visibleRouteFormErrors.dest}
                placeholder="10.0.0.0"
              />
              <Field
                fieldId="route-mask"
                label="Subnet Mask"
                value={routeForm.mask}
                onChange={handleRouteMaskChange}
                onBlur={() => markRouteFieldTouched("mask")}
                error={visibleRouteFormErrors.mask}
                placeholder="255.255.255.0"
              />
              <Field
                fieldId="route-gateway"
                label="Gateway"
                value={routeForm.gw}
                onChange={handleRouteGatewayChange}
                onBlur={() => markRouteFieldTouched("gw")}
                error={visibleRouteFormErrors.gw}
                placeholder="192.168.1.1"
              />
              <Field
                fieldId="route-metric"
                label="Metric"
                value={routeForm.metric}
                onChange={handleRouteMetricChange}
                onBlur={() => markRouteFieldTouched("metric")}
                error={visibleRouteFormErrors.metric}
                placeholder="10"
                inputMode="numeric"
              />
            </div>
            <div className="flex flex-wrap gap-1.5">
              <ActionBtn
                icon={Plus}
                label="ADD"
                color="emerald"
                onClick={handleAddRoute}
                disabled={!machineRepairEnabled}
                compact
              />
              <ActionBtn
                icon={Trash2}
                label="DEL"
                color="red"
                onClick={handleDeleteRoute}
                disabled={!machineRepairEnabled}
                compact
              />
              <ActionBtn
                icon={Globe}
                label="WAN"
                color="blue"
                disabled={!machineRepairEnabled}
                compact
                onClick={() => openConfirm(
                  "Set Default Gateway",
                  `Route all traffic through ${selectedNic?.description ?? "selected NIC"}?\nPersist on startup: ${persistWanOnStartup ? "ON" : "OFF"}.`,
                  executeSetInternet
                )}
              />
              <ActionBtn
                icon={Flame}
                label="FLUSH"
                color="orange"
                disabled={!machineRepairEnabled}
                compact
                onClick={() => openConfirm(
                  "Clear All Routes",
                  "Clear ALL routes? This action is dangerous.",
                  executeFlush
                )}
              />
            </div>
            <div className="wan-persist-row">
              <label className="wan-persist-option">
                <input
                  type="checkbox"
                  checked={persistWanOnStartup}
                  onChange={(event) => setPersistWanOnStartup(event.target.checked)}
                  disabled={persistWanLoading}
                  className="w-3.5 h-3.5 rounded accent-blue-500"
                />
                <span>Persist on startup</span>
              </label>
              <span className="wan-persist-hint">
                Save or clear one unified startup replay config when you click WAN
              </span>
            </div>
          </div>

          {/* Unified Output Console */}
          <OutputConsole
            diagnosticView={diagnosticView}
            routesCount={routes.length}
            diagnosticsOutputText={diagnosticsOutputText}
            pingOutputText={pingOutputText}
            commandOutputRef={commandOutputRef}
            pingOutputRef={pingOutputRef}
            onShowCommand={handleShowCommandOutput}
            onShowRouting={handleShowRoutingOutput}
            onClearCommand={clearCommandOutput}
            onClearPing={clearPingOutput}
          />
        </div>

        {/* --- RIGHT PANEL --- */}
        <div className="right-panel overflow-y-auto p-3 space-y-2.5">
          {/* Network Fix Tools */}
          <Section
            icon={Wrench}
            title="Network Fix Tools"
            open={panels.toolsOpen}
            onToggle={handleToggleToolsPanel}
          >
            <div className="tool-grid">
              <ToolBtn icon={Zap} label="Flush DNS" desc="Clear resolver cache"
                onClick={() => executeRepairAction("FlushDns", "Flush DNS")} tone="safe" disabled={!machineRepairEnabled} />
              <ToolBtn icon={RefreshCw} label="Renew IP" desc="Release and renew DHCP"
                onClick={() => executeRepairAction("RenewDhcpLease", "Renew IP", { refresh: true, invalidateNicCache: true })} tone="safe" disabled={!machineRepairEnabled} />
              <ToolBtn icon={Wifi} label="Wi-Fi Info" desc="Show WLAN interface details"
                onClick={() => executeNetCmd("netsh wlan show interface", "Wi-Fi Info")} tone="system" />
              <ToolBtn icon={Trash2} label="Clear ARP" desc="Flush ARP cache"
                onClick={() => executeRepairAction("ClearArpCache", "Clear ARP", { refresh: true })} tone="system" disabled={!machineRepairEnabled} />
              <ToolBtn icon={Globe} label="Reset TCP/IP" desc="Reset network stack"
                onClick={() => executeRepairAction("ResetTcpIp", "Reset TCP/IP", { refresh: true })} tone="danger" disabled={!machineRepairEnabled} />
              <ToolBtn icon={OctagonAlert} label="Reset Winsock" desc="Reset socket catalog"
                onClick={() => executeRepairAction("ResetWinsock", "Reset Winsock", { refresh: true })} tone="danger" disabled={!machineRepairEnabled} />
              <ToolBtn icon={Flame} label="Reset Firewall" desc="Reset firewall to defaults"
                onClick={() => executeRepairAction("ResetFirewall", "Reset Firewall", { refresh: true })} tone="danger" disabled={!machineRepairEnabled} />
              <ToolBtn icon={Monitor} label="Battery Info" desc="View battery wear and lifetime summary"
                onClick={batterySummary.handleOpenModal} tone="system" />
            </div>
          </Section>

          <Section
            icon={Monitor}
            title="Diagnostics & Repair"
            open={panels.diagnosticsOpen}
            onToggle={handleToggleDiagnosticsPanel}
          >
            <div className="tool-grid mb-2">
              <ToolBtn icon={Monitor} label="Display DNS Cache" desc="Inspect current resolver cache"
                onClick={handleDisplayDnsCache} tone="safe" compact />
              <ToolBtn icon={Wrench} label="Reset WinHTTP Proxy" desc="Clear system proxy settings"
                onClick={handleResetWinHttpProxy} tone="system" compact disabled={!machineRepairEnabled} />
              <ToolBtn icon={RefreshCw} label="Restart Adapters" desc="Restart active adapters"
                onClick={handleRestartAdapters} tone="system" compact disabled={!machineRepairEnabled} />
              <ToolBtn icon={Search} label="Scan IP" desc="Scan active subnet hosts"
                onClick={handleOpenIpScanModal} tone="safe" compact />
            </div>

            <div className="diag-group">
              <div className="diag-inline">
                <input
                  type="text"
                  value={diagnosticsInputs.host}
                  onChange={(e) => handleDiagHostChange(e.target.value)}
                  placeholder="Domain or IP (e.g. google.com)"
                  className="diag-input"
                  aria-label="Diagnostic host"
                />
                <input
                  type="text"
                  value={diagnosticsInputs.port}
                  onChange={(e) => handleDiagPortChange(e.target.value)}
                  placeholder="Port"
                  className="diag-input diag-port"
                  inputMode="numeric"
                  aria-label="Diagnostic port"
                />
                <button
                  type="button"
                  onClick={handlePortConnectivityTest}
                  className="diag-action-btn"
                >
                  <Activity className="w-3.5 h-3.5" />
                  Port Test
                </button>
              </div>
              <div className="diag-inline diag-inline-dns">
                <input
                  type="text"
                  value={diagnosticsInputs.dnsServer}
                  onChange={(e) => handleDiagDnsServerChange(e.target.value)}
                  placeholder="DNS server (e.g. 8.8.8.8)"
                  className="diag-input"
                  aria-label="Diagnostic DNS server"
                />
                <button
                  type="button"
                  onClick={handleNslookupTest}
                  className="diag-action-btn diag-action-btn-alt"
                >
                  <Search className="w-3.5 h-3.5" />
                  NSLookup
                </button>
              </div>
            </div>
          </Section>

          {/* Ping & Monitor */}
          <Section
            icon={Activity}
            title="Ping & Tracert Monitor"
            open={panels.pingOpen}
            onToggle={handleTogglePingPanel}
          >
            <div className="segmented-control mb-2">
              <button
                type="button"
                onClick={() => setPingMode("ping")}
                className={`segment-btn ${pingMode === "ping" ? "segment-btn-active" : ""}`}
                aria-pressed={pingMode === "ping"}
              >
                Ping
              </button>
              <button
                type="button"
                onClick={() => setPingMode("fping")}
                className={`segment-btn ${pingMode === "fping" ? "segment-btn-active" : ""}`}
                aria-pressed={pingMode === "fping"}
              >
                fping
              </button>
            </div>
            <div className="ping-action-row mb-3">
              <input
                type="text"
                placeholder={pingMode === "fping" ? "8.8.8.8 1.1.1.1 192.168.1.1" : "google.com or 8.8.8.8"}
                value={pingTarget}
                onChange={(e) => setPingTarget(e.target.value)}
                onKeyDown={(e) => e.key === "Enter" && handleStartPing()}
                className="diag-input"
                aria-label="Ping target"
              />
              <button
                type="button"
                onClick={handleStartPing}
                disabled={pingRunning}
                className="ping-cmd-btn ping-cmd-start"
              >
                <Send className="w-4 h-4" /> Start
              </button>
              <button
                type="button"
                onClick={handleStopPing}
                disabled={!pingRunning}
                className="ping-cmd-btn ping-cmd-stop"
              >
                <OctagonAlert className="w-4 h-4" />
                Stop
              </button>
              <button
                type="button"
                onClick={handleTracertFromTarget}
                className="ping-cmd-btn ping-cmd-trace"
              >
                <ArrowDownUp className="w-4 h-4" />
                Tracert
              </button>
            </div>

            <div className="text-[0.66rem] text-slate-500">
              Ping and tracert logs are shown in the left Output Console.
            </div>
          </Section>

          <SpeedTestModal onStatusChange={setStatusMsg} />
        </div>
      </div>

      {/* ====== FOOTER ====== */}
      <footer className="app-footer flex items-center justify-between px-5 py-1.5 border-t shrink-0">
        <div className="app-footer-left">
          <span className="text-[0.65rem] text-slate-500" role="status" aria-live="polite">
            {statusMsg}
          </span>
          <button
            type="button"
            onClick={donateModal.open}
            className="donate-footer-btn capsule-btn"
            title="Donate to the author Zozon"
          >
            Donate
          </button>
          <button
            type="button"
            onClick={helpModal.open}
            className="help-footer-btn capsule-btn"
            title="Open help"
          >
            <CircleHelp className="w-3.5 h-3.5" />
            Help
          </button>


        </div>
        <span className="version-text text-[0.85rem] font-semibold">SuperRoute Pro V.{appVersion} | Author {APP_AUTHOR}</span>
      </footer>

      <DonateModal
        open={donateModal.isOpen}
        onClose={donateModal.close}
      />

      <HelpModal
        open={helpModal.isOpen}
        language={helpLanguage}
        onLanguageChange={setHelpLanguage}
        onClose={helpModal.close}
      />

      <BatteryModal
        open={batterySummary.modal.isOpen}
        loading={batterySummary.loading}
        summary={batterySummary.summary}
        error={batterySummary.error}
        onRefresh={() => {
          void batterySummary.loadSummary();
        }}
        onClose={batterySummary.handleCloseModal}
      />

      <IpScanModal
        open={ipScanModal.isOpen}
        selectedNic={selectedNic}
        plan={ipScanPlan}
        running={ipScanRunning}
        stopPending={ipScanStopPending}
        results={ipScanResults}
        progressPercent={ipScanProgressPercent}
        progressText={ipScanProgressText}
        onStart={handleStartIpScan}
        onForceStop={handleForceStopIpScan}
        onClose={handleCloseIpScanModal}
      />

      <CacheModal
        open={cacheCleanupManager.open}
        cleaning={cacheCleanupManager.cleaning}
        stopPending={cacheCleanupManager.stopPending}
        options={cacheCleanupManager.options}
        selectedCaches={cacheCleanupManager.selectedCaches}
        selectedCount={cacheCleanupManager.selectedCount}
        progressPercent={cacheCleanupManager.progressPercent}
        progressText={cacheCleanupManager.progressText}
        onToggleCache={cacheCleanupManager.handleToggleCache}
        onSelectAll={cacheCleanupManager.handleSelectAll}
        onClearSelection={cacheCleanupManager.handleClearSelection}
        onForceStop={cacheCleanupManager.handleForceStop}
        onStartCleanup={cacheCleanupManager.handleStartCleanup}
        onClose={cacheCleanupManager.handleCloseModal}
      />

      <BloatwareModal
        open={bloatwareManager.open}
        loading={bloatwareManager.loading}
        removing={bloatwareManager.removing}
        items={bloatwareManager.items}
        selectedPackages={bloatwareManager.selectedPackages}
        selectedCount={bloatwareManager.selectedCount}
        installedCount={bloatwareManager.installedCount}
        progressPercent={bloatwareManager.progressPercent}
        progressText={bloatwareManager.progressText}
        onTogglePackage={bloatwareManager.handleTogglePackage}
        onSelectAll={bloatwareManager.handleSelectAll}
        onSelectInstalled={bloatwareManager.handleSelectInstalled}
        onClearSelection={bloatwareManager.handleClearSelection}
        onRemoveSelected={bloatwareManager.handleRemoveSelected}
        onClose={bloatwareManager.handleCloseModal}
      />

      <ConfirmDialog
        open={confirmOpen}
        title={confirmTitle}
        message={confirmMessage}
        onConfirm={onConfirm}
        onCancel={onCancelConfirm}
      />

      {routeWatcherToast && (
        <div
          className="fixed bottom-14 right-4 z-40 w-full max-w-sm px-4 sm:px-0 pointer-events-none"
          aria-live="polite"
        >
          <div
            className={`pointer-events-auto rounded-2xl border shadow-2xl backdrop-blur px-4 py-3 ${routeWatcherToast.tone === "success"
              ? "border-emerald-400/40 bg-slate-950/92"
              : "border-amber-400/45 bg-slate-950/94"
            }`}
          >
            <div className="flex items-start gap-3">
              {routeWatcherToast.tone === "success" ? (
                <RefreshCw className="mt-0.5 h-4 w-4 text-emerald-300" />
              ) : (
                <OctagonAlert className="mt-0.5 h-4 w-4 text-amber-300" />
              )}
              <div className="min-w-0 flex-1">
                <div className="text-[0.75rem] font-semibold text-slate-100">
                  {routeWatcherToast.title}
                </div>
                <div className="mt-1 text-[0.72rem] leading-5 text-slate-300">
                  {routeWatcherToast.message}
                </div>
                {routeWatcherToast.actionLabel && (
                  <button
                    type="button"
                    onClick={handleOpenRouteWatcherToastAction}
                    className="mt-2 inline-flex rounded-lg border border-amber-300/35 bg-amber-400/10 px-2.5 py-1 text-[0.7rem] font-semibold text-amber-100 transition hover:bg-amber-300/15"
                  >
                    {routeWatcherToast.actionLabel}
                  </button>
                )}
              </div>
              <button
                type="button"
                onClick={() => {
                  if (routeWatcherToastTimerRef.current !== null) {
                    window.clearTimeout(routeWatcherToastTimerRef.current);
                    routeWatcherToastTimerRef.current = null;
                  }
                  setRouteWatcherToast(null);
                }}
                className="rounded-md p-1 text-slate-400 transition hover:bg-white/5 hover:text-slate-200"
                aria-label="Dismiss route watcher notification"
              >
                <X className="h-3.5 w-3.5" />
              </button>
            </div>
          </div>
        </div>
      )}

      <div className={`theme-lens ${themeLensActive ? "theme-lens-active" : ""}`} />
    </div>
  );
}

