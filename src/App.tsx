import { useState, useEffect, useCallback, useRef, useMemo } from "react";
import { getVersion } from "@tauri-apps/api/app";
import { listen } from "@tauri-apps/api/event";
import {
  Zap, Wifi, WifiOff, RefreshCw, Plus, Minus, Trash2, Globe, Flame,
  Activity, Send, Wrench, Monitor, Sun, Moon, OctagonAlert, Search,
  ArrowDownUp, X, CircleHelp
} from "lucide-react";
import {
  getNetworkSnapshot, getRoutingTable,
  runNetworkCommand, pingHost, testTcpPort,
  fpingScan, checkInternet,
  getBloatwareCandidates, repairRemoveBloatware, repairClearCacheTargets, getBatterySummary,
  autoUnlockRepairMode, getRepairSessionStatus, listRepairTargets, unlockRepairMode, lockRepairMode,
  repairAddRoute, repairDeleteRoute, repairFlushRoutes, repairSetDefaultGateway,
  repairSavePersistConfig, repairClearPersistConfig,
  runRepairMachineAction, persistLoadConfig, persistGetNicStableIds, invalidateNetworkAdapterCache,
  type NetworkInterface, type RouteEntry, type BloatwareItem, type FpingHostResult,
  type PersistConfig,
  type BatterySummaryResult, type RepairMachineAction, type RepairSessionStatus,
} from "./api";
import {
  getProfileSensitiveActionHint,
  isMachineRepairEnabled,
  isProfileSensitiveActionEnabled,
} from "./repairModeModel";
import {
  mergeNicDescriptions,
  stabilizeNicSnapshotDescriptions,
  syncSelectedNicToList,
} from "./nicDescriptionModel";
import { getNicTableMessage } from "./nicTableModel";
import { buildPersistCustomRoutes, getPersistRouteInterfaceIndexes } from "./persistRouteModel";
import { getPersistStartupWriteMode, resolvePersistStartupEnabled } from "./persistStartupModel";
import { SpeedTestModal } from "./SpeedTestModal";
import { BatteryModal } from "./components/BatteryModal";
import { BloatwareModal } from "./components/BloatwareModal";
import { CacheModal, type CacheCleanupOption } from "./components/CacheModal";
import { DonateModal } from "./components/DonateModal";
import { HelpModal } from "./components/HelpModal";
import { ActionBtn, Field, OutputConsole, Section, ToolBtn } from "./components/AppChrome";
import { IpScanModal } from "./components/IpScanModal";
import { getBatteryWearLevel } from "./batteryUtils";
import { buildIpScanPlan, type IpScanPlan } from "./hooks/ipScanPlan";
import { useProgressTracker } from "./hooks/useProgressTracker";
import { useBufferedLog } from "./hooks/useBufferedLog";
import { useModal } from "./hooks/useModal";

const ROUTE_TABLE_COLUMNS: Array<{ key: keyof RouteEntry; label: string; width: number }> = [
  { key: "destination", label: "Destination", width: 18 },
  { key: "netmask", label: "Netmask", width: 18 },
  { key: "gateway", label: "Gateway", width: 18 },
  { key: "metric", label: "Met", width: 6 },
  { key: "interface_index", label: "IF", width: 6 },
];

const formatRouteCell = (value: string, width: number) => {
  if (value.length <= width) {
    return value.padEnd(width, " ");
  }
  if (width <= 3) {
    return value.slice(0, width);
  }
  return `${value.slice(0, width - 3)}...`;
};

const formatRoutingSnapshot = (routeData: RouteEntry[]) => {
  const stamp = new Date().toLocaleTimeString("en-GB");
  if (!routeData.length) {
    return `[${stamp}] Routing table snapshot\nNo routes found.`;
  }

  const header = ROUTE_TABLE_COLUMNS
    .map((column) => formatRouteCell(column.label, column.width))
    .join(" ");
  const divider = ROUTE_TABLE_COLUMNS
    .map((column) => "-".repeat(column.width))
    .join(" ");
  const rows = routeData.map((route) =>
    ROUTE_TABLE_COLUMNS
      .map((column) => formatRouteCell(String(route[column.key] ?? ""), column.width))
      .join(" ")
  );

  return [
    `[${stamp}] Routing table snapshot (${routeData.length} routes)`,
    header,
    divider,
    ...rows,
  ].join("\n");
};

const IP_SCAN_BATCH_SIZE = 24;
const ROUTE_WATCHER_STATUS_EVENT = "route-watcher://status";

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

const CACHE_CLEANUP_OPTIONS: CacheCleanupOption[] = [
  {
    id: "user_temp",
    label: "User Temp",
    description: "Clear %LOCALAPPDATA%\\Temp",
    defaultChecked: true,
  },
  {
    id: "windows_temp",
    label: "Windows Temp",
    description: "Clear Windows temporary files",
    defaultChecked: true,
  },
  {
    id: "windows_update_cache",
    label: "Windows Update Cache",
    description: "Clear SoftwareDistribution download cache",
    defaultChecked: true,
  },
  {
    id: "prefetch",
    label: "Prefetch",
    description: "Clear prefetch cache files",
    defaultChecked: false,
  },
  {
    id: "explorer_cache",
    label: "Explorer Cache",
    description: "Clear icon and thumbnail cache",
    defaultChecked: true,
  },
  {
    id: "edge_cache",
    label: "Microsoft Edge Cache",
    description: "Clear Edge browser cache",
    defaultChecked: false,
  },
  {
    id: "chrome_cache",
    label: "Google Chrome Cache",
    description: "Clear Chrome browser cache",
    defaultChecked: false,
  },
  {
    id: "firefox_cache",
    label: "Mozilla Firefox Cache",
    description: "Clear Firefox browser cache",
    defaultChecked: false,
  },
  {
    id: "inet_cache",
    label: "INetCache",
    description: "Clear legacy internet cache",
    defaultChecked: true,
  },
  {
    id: "web_cache",
    label: "WebCache",
    description: "Clear Windows WebCache store",
    defaultChecked: false,
  },
  {
    id: "crash_dumps",
    label: "Crash Dumps",
    description: "Clear local crash dump files",
    defaultChecked: true,
  },
  {
    id: "wer_reports",
    label: "Windows Error Reporting (WER)",
    description: "Clear WER reports and queue",
    defaultChecked: true,
  },
  {
    id: "d3d_shader_cache",
    label: "DirectX Shader Cache",
    description: "Clear D3DSCache",
    defaultChecked: true,
  },
];

const DEFAULT_CACHE_SELECTION = new Set(
  CACHE_CLEANUP_OPTIONS.filter((option) => option.defaultChecked).map((option) => option.id)
);

type HelpLanguage = "en" | "vi";

export default function App() {
  const APP_AUTHOR = "Zonzon";
  const [appVersion, setAppVersion] = useState("dev");
  const [theme, setTheme] = useState<"dark" | "light">(() => {
    const saved = localStorage.getItem("ui-theme");
    return saved === "light" || saved === "dark" ? saved : "dark";
  });
  const [persistWanOnStartup, setPersistWanOnStartup] = useState(false);
  const [persistWanLoading, setPersistWanLoading] = useState(true);

  const ZOOM_MIN = 75;
  const ZOOM_MAX = 120;
  const ZOOM_STEP = 5;
  const ZOOM_DEFAULT = 95;
  const [zoomLevel, setZoomLevel] = useState<number>(() => {
    const saved = localStorage.getItem("app-zoom-level");
    if (saved) {
      const parsed = Number.parseInt(saved, 10);
      if (Number.isFinite(parsed) && parsed >= ZOOM_MIN && parsed <= ZOOM_MAX) return parsed;
    }
    return ZOOM_DEFAULT;
  });

  // State
  const [nics, setNics] = useState<NetworkInterface[]>([]);
  const [routes, setRoutes] = useState<RouteEntry[]>([]);
  const [selectedNic, setSelectedNic] = useState<NetworkInterface | null>(null);
  const [activeOnly, setActiveOnly] = useState(true);
  const [isOnline, setIsOnline] = useState<boolean | null>(null);
  const [statusMsg, setStatusMsg] = useState("System Ready");
  const [routeWatcherToast, setRouteWatcherToast] = useState<RouteWatcherToast | null>(null);
  const [repairSession, setRepairSession] = useState<RepairSessionStatus>({
    locked: true,
    connected: false,
    target_sid: null,
    requires_unlock: true,
  });
  const [selectedRepairTargetSid, setSelectedRepairTargetSid] = useState<string | null>(null);
  const [repairLoading, setRepairLoading] = useState(true);
  const [repairUnlocking, setRepairUnlocking] = useState(false);
  const [loading, setLoading] = useState(true);
  const [hasLoadedNicSnapshot, setHasLoadedNicSnapshot] = useState(false);
  const [pingTarget, setPingTarget] = useState("1.1.1.1");
  const [pingMode, setPingMode] = useState<"ping" | "fping">("ping");
  const [pingRunning, setPingRunning] = useState(false);
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
  const [themeLensActive, setThemeLensActive] = useState(false);
  const [currentLatency, setCurrentLatency] = useState<number>(0);
  const [toolsOpen, setToolsOpen] = useState(false);
  const [diagnosticsOpen, setDiagnosticsOpen] = useState(false);
  const [pingOpen, setPingOpen] = useState(false);
  const [diagHost, setDiagHost] = useState("google.com");
  const [diagDnsServer, setDiagDnsServer] = useState("8.8.8.8");
  const [diagPort, setDiagPort] = useState("443");
  const [confirmOpen, setConfirmOpen] = useState(false);
  const [confirmTitle, setConfirmTitle] = useState("Confirm");
  const [confirmMessage, setConfirmMessage] = useState("");
  const [diagnosticView, setDiagnosticView] = useState<"command" | "routing">("command");
  const [routingOutput, setRoutingOutput] = useState("");
  const bloatwareModal = useModal();
  const [bloatwareLoading, setBloatwareLoading] = useState(false);
  const [bloatwareRemoving, setBloatwareRemoving] = useState(false);
  const [bloatwareItems, setBloatwareItems] = useState<BloatwareItem[]>([]);
  const [selectedBloatware, setSelectedBloatware] = useState<Set<string>>(new Set());
  const {
    percent: removeProgressPercent,
    text: removeProgressText,
    update: updateRemoveProgress,
    setMessage: setRemoveProgressText,
    reset: resetRemoveProgress,
  } = useProgressTracker();
  const [batteryLoading, setBatteryLoading] = useState(false);
  const [batterySummary, setBatterySummary] = useState<BatterySummaryResult | null>(null);
  const [batterySummaryError, setBatterySummaryError] = useState("");
  const batteryModal = useModal();
  const donateModal = useModal();
  const helpModal = useModal();
  const [helpLanguage, setHelpLanguage] = useState<HelpLanguage>("vi");
  const cacheModal = useModal();
  const [cacheCleaning, setCacheCleaning] = useState(false);
  const [cacheStopPending, setCacheStopPending] = useState(false);
  const [selectedCaches, setSelectedCaches] = useState<Set<string>>(
    () => new Set(DEFAULT_CACHE_SELECTION)
  );
  const {
    percent: cacheProgressPercent,
    text: cacheProgressText,
    update: updateCacheProgress,
    setMessage: setCacheProgressText,
    reset: resetCacheProgress,
  } = useProgressTracker();
  const selectedCacheTargets = useMemo(
    () => CACHE_CLEANUP_OPTIONS.filter((option) => selectedCaches.has(option.id)),
    [selectedCaches]
  );
  const repairAppInstanceId = useMemo(
    () => globalThis.crypto?.randomUUID?.() ?? `srp-ui-${Date.now()}`,
    []
  );
  const repairConnectionId = useMemo(
    () => globalThis.crypto?.randomUUID?.() ?? `srp-conn-${Date.now()}`,
    []
  );

  useEffect(() => {
    let active = true;

    const loadAppVersion = async () => {
      try {
        const version = await getVersion();
        if (active) {
          setAppVersion(version);
        }
      } catch {
        if (active) {
          setAppVersion("dev");
        }
      }
    };

    void loadAppVersion();

    return () => {
      active = false;
    };
  }, []);

  useEffect(() => {
    const savedLanguage = localStorage.getItem("help-language");
    if (savedLanguage === "en" || savedLanguage === "vi") {
      setHelpLanguage(savedLanguage);
    }
  }, []);

  useEffect(() => {
    let active = true;
    const savedPreference = localStorage.getItem("wan-persist-on-startup");
    const localPreference =
      savedPreference === "true" ? true : savedPreference === "false" ? false : null;

    const loadPersistStatus = async () => {
      try {
        const persistedConfigResult = await persistLoadConfig();
        const persistedConfigEnabled =
          persistedConfigResult ? persistedConfigResult.enabled : null;

        if (active) {
          setPersistWanOnStartup(
            resolvePersistStartupEnabled({
              localPreference,
              persistedConfigEnabled,
            }),
          );
        }
      } catch {
        if (active) {
          setPersistWanOnStartup(
            resolvePersistStartupEnabled({
              localPreference,
              persistedConfigEnabled: null,
            }),
          );
        }
      } finally {
        if (active) {
          setPersistWanLoading(false);
        }
      }
    };

    void loadPersistStatus();
    return () => {
      active = false;
    };
  }, []);

  useEffect(() => {
    const htmlEl = document.documentElement;
    if (zoomLevel === ZOOM_DEFAULT) {
      htmlEl.removeAttribute("data-zoom");
      htmlEl.style.removeProperty("--zoom-level");
    } else {
      htmlEl.setAttribute("data-zoom", String(zoomLevel));
      htmlEl.style.setProperty("--zoom-level", `${zoomLevel}%`);
    }
    localStorage.setItem("app-zoom-level", String(zoomLevel));
  }, [zoomLevel]);

  const handleZoomIn = useCallback(() => {
    setZoomLevel((prev) => Math.min(ZOOM_MAX, prev + ZOOM_STEP));
  }, []);

  const handleZoomOut = useCallback(() => {
    setZoomLevel((prev) => Math.max(ZOOM_MIN, prev - ZOOM_STEP));
  }, []);

  const handleZoomReset = useCallback(() => {
    setZoomLevel(ZOOM_DEFAULT);
  }, []);

  const loadRepairTargets = useCallback(async () => {
    try {
      const targets = await listRepairTargets();
      if (targets.length > 0) {
        const activeTarget = targets.find((t) => t.is_loaded) || targets[0];
        setSelectedRepairTargetSid(activeTarget.sid);
        console.debug("Auto-selected target user:", activeTarget.account_name, activeTarget.sid);
      }
    } catch (err) {
      console.warn("Could not load repair targets:", err);
    }
  }, []);

  const refreshRepairContext = useCallback(async () => {
    setRepairLoading(true);
    try {
      let autoUnlockFailure: string | null = null;
      const [sessionResult, targetsResult] = await Promise.allSettled([
        getRepairSessionStatus(),
        listRepairTargets(),
      ]);
      let nextRepairSession =
        sessionResult.status === "fulfilled" ? sessionResult.value : null;
      if (sessionResult.status === "fulfilled") {
        setRepairSession(sessionResult.value);
      }
      if (targetsResult.status === "fulfilled" && targetsResult.value.length > 0) {
        const targets = targetsResult.value;
        const activeTarget = targets.find((t) => t.is_loaded) || targets[0];
        setSelectedRepairTargetSid(activeTarget.sid);
        console.debug("Auto-selected target user:", activeTarget.account_name, activeTarget.sid);
      }

      if (nextRepairSession?.locked) {
        try {
          const autoUnlocked = await autoUnlockRepairMode(
            repairAppInstanceId,
            repairConnectionId,
          );
          nextRepairSession = autoUnlocked;
          setRepairSession(autoUnlocked);
          if (!autoUnlocked.locked) {
            setStatusMsg("Repair Mode unlocked automatically for this app session.");
          }
        } catch (autoUnlockErr) {
          autoUnlockFailure =
            autoUnlockErr instanceof Error
              ? autoUnlockErr.message
              : String(autoUnlockErr);
          console.warn("Auto-unlock repair mode skipped:", autoUnlockErr);
        }
      }

      const sessionFailure =
        sessionResult.status === "rejected" ? sessionResult.reason : null;
      const targetsFailure =
        targetsResult.status === "rejected" ? targetsResult.reason : null;
      const failure = sessionFailure ?? targetsFailure;
      if (failure) {
        setStatusMsg(`Repair context error: ${failure}`);
      } else if (autoUnlockFailure) {
        setStatusMsg(`Repair Mode stayed locked: ${autoUnlockFailure}`);
      } else if (nextRepairSession?.locked) {
        setStatusMsg("Repair Mode is locked.");
      }
    } catch (err) {
      setStatusMsg(`Repair context error: ${err}`);
    } finally {
      setRepairLoading(false);
    }
  }, [repairAppInstanceId, repairConnectionId]);

  useEffect(() => {
    void refreshRepairContext();
  }, [refreshRepairContext]);

  // Form state
  const [formDest, setFormDest] = useState("");
  const [formMask, setFormMask] = useState("255.255.255.0");
  const [formGw, setFormGw] = useState("");
  const [formMetric, setFormMetric] = useState("10");

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

  const pingLoopRef = useRef<number | null>(null);
  const pingBusyRef = useRef(false);
  const pingSeqRef = useRef(0);
  const lensTimerRef = useRef<number | null>(null);
  const cacheStopRequestedRef = useRef(false);
  const ipScanStopRequestedRef = useRef(false);
  const latestLoadRequestRef = useRef(0);
  const confirmActionRef = useRef<(() => void | Promise<void>) | null>(null);
  const pingOutputRef = useRef<HTMLPreElement | null>(null);
  const commandOutputRef = useRef<HTMLPreElement | null>(null);
  const latestNicsRef = useRef<NetworkInterface[]>([]);
  const routeWatcherToastTimerRef = useRef<number | null>(null);

  useEffect(() => {
    latestNicsRef.current = nics;
  }, [nics]);

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

  // ======================== DATA LOADING ========================

  const loadData = useCallback(async (options?: { invalidateNicCache?: boolean }) => {
    const requestId = latestLoadRequestRef.current + 1;
    latestLoadRequestRef.current = requestId;
    setLoading(true);
    setStatusMsg("Loading data...");
    try {
      if (options?.invalidateNicCache) {
        await invalidateNetworkAdapterCache();
        if (requestId !== latestLoadRequestRef.current) {
          return;
        }
      }
      const snapshot = await getNetworkSnapshot(activeOnly);
      if (requestId !== latestLoadRequestRef.current) {
        return;
      }
      const stabilizedInterfaces = stabilizeNicSnapshotDescriptions(
        latestNicsRef.current,
        snapshot.interfaces,
      );
      latestNicsRef.current = stabilizedInterfaces;
      setNics(stabilizedInterfaces);
      setSelectedNic((current) => syncSelectedNicToList(current, stabilizedInterfaces));
      setRoutes(snapshot.routes);
      setRoutingOutput(formatRoutingSnapshot(snapshot.routes));
      setHasLoadedNicSnapshot(true);
      setStatusMsg(`Loaded ${stabilizedInterfaces.length} NICs, ${snapshot.routes.length} routes`);

      const interfaceIndexes = stabilizedInterfaces.map((nic) => nic.index);
      if (interfaceIndexes.length > 0) {
        void (async () => {
          try {
            const stableIds = await persistGetNicStableIds(interfaceIndexes);
            if (requestId !== latestLoadRequestRef.current) {
              return;
            }
            const descriptionEntries = interfaceIndexes.map((interfaceIndex, index) => ({
              interfaceIndex,
              description: stableIds[index]?.description ?? "",
            }));
            setNics((current) => {
              const enriched = mergeNicDescriptions(current, descriptionEntries);
              latestNicsRef.current = enriched;
              return enriched;
            });
            setSelectedNic((current) => {
              if (!current) {
                return current;
              }
              const [enrichedCurrent] = mergeNicDescriptions([current], descriptionEntries);
              return enrichedCurrent;
            });
          } catch (enrichErr) {
            console.warn("Failed to enrich NIC descriptions:", enrichErr);
          }
        })();
      }
    } catch (err) {
      if (requestId !== latestLoadRequestRef.current) {
        return;
      }
      setHasLoadedNicSnapshot(true);
      setStatusMsg(`Error: ${err}`);
    } finally {
      if (requestId === latestLoadRequestRef.current) {
        setLoading(false);
      }
    }
  }, [activeOnly]);

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
    let active = true;
    let cleanup = () => {};

    void listen<RouteWatcherStatusEventPayload>(ROUTE_WATCHER_STATUS_EVENT, ({ payload }) => {
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
  }, [loadData, pushRouteWatcherToast]);

  // Internet monitor with adaptive polling and cancellation-safe updates.
  useEffect(() => {
    let stopped = false;
    let timerId: number | null = null;
    let inFlight = false;
    let successStreak = 0;
    let failureStreak = 0;

    const computeDelay = (online: boolean): number => {
      if (!online) {
        return Math.min(12000, 2500 + failureStreak * 1200);
      }
      if (successStreak >= 6) return 15000;
      if (successStreak >= 3) return 9000;
      return 5000;
    };

    const scheduleNext = (delayMs: number) => {
      if (stopped) return;
      timerId = window.setTimeout(() => {
        void tick();
      }, delayMs);
    };

    const tick = async () => {
      if (stopped || inFlight) return;
      inFlight = true;
      let online = false;
      try {
        online = await checkInternet();
        if (stopped) return;
        setIsOnline(online);
      } catch {
        if (stopped) return;
        online = false;
        setIsOnline(false);
      } finally {
        if (online) {
          successStreak += 1;
          failureStreak = 0;
        } else {
          failureStreak += 1;
          successStreak = 0;
        }
        inFlight = false;
        scheduleNext(computeDelay(online));
      }
    };

    void tick();
    return () => {
      stopped = true;
      if (timerId !== null) {
        window.clearTimeout(timerId);
      }
    };
  }, []);

  // Latency monitor with adaptive polling and cancellation-safe updates.
  useEffect(() => {
    let stopped = false;
    let timerId: number | null = null;
    let inFlight = false;
    let failureStreak = 0;

    const computeDelay = (success: boolean, latencyMs: number): number => {
      if (!success) {
        return Math.min(7000, 1800 + failureStreak * 700);
      }
      if (latencyMs <= 40) return 5000;
      if (latencyMs <= 90) return 3500;
      if (latencyMs <= 180) return 2500;
      return 1800;
    };

    const scheduleNext = (delayMs: number) => {
      if (stopped) return;
      timerId = window.setTimeout(() => {
        void tick();
      }, delayMs);
    };

    const tick = async () => {
      if (stopped || inFlight) return;
      inFlight = true;
      let success = false;
      let latency = 0;
      try {
        const result = await pingHost("8.8.8.8", 1);
        if (stopped) return;
        success = result.success;
        latency = success ? result.latency_ms : 0;
        setCurrentLatency(latency);
      } catch {
        if (stopped) return;
        setCurrentLatency(0);
      } finally {
        failureStreak = success ? 0 : failureStreak + 1;
        inFlight = false;
        scheduleNext(computeDelay(success, latency));
      }
    };

    void tick();
    return () => {
      stopped = true;
      if (timerId !== null) {
        window.clearTimeout(timerId);
      }
    };
  }, []);

  // ======================== ACTIONS ========================

  const handleSelectNic = useCallback((nic: NetworkInterface) => {
    setSelectedNic(nic);
    setFormGw(nic.gateway);
  }, []);

  const handleUnlockRepair = useCallback(async () => {
    setRepairUnlocking(true);
    setStatusMsg("Unlocking Repair Mode...");
    try {
      const status = await unlockRepairMode(repairAppInstanceId, repairConnectionId);
      setRepairSession(status);
      setStatusMsg("Repair Mode unlocked for this app session.");
    } catch (err) {
      setStatusMsg(`Repair unlock error: ${err}`);
    } finally {
      setRepairUnlocking(false);
    }
  }, [repairAppInstanceId, repairConnectionId]);

  const handleLockRepair = useCallback(async () => {
    try {
      const status = await lockRepairMode();
      setRepairSession(status);
      setStatusMsg("Repair Mode locked.");
    } catch (err) {
      setStatusMsg(`Repair lock error: ${err}`);
    }
  }, []);

  const handleAddRoute = useCallback(async () => {
    if (!formDest || !formGw) {
      setStatusMsg("Please fill Destination and Gateway");
      return;
    }
    setStatusMsg("Adding route...");
    try {
      const result = await repairAddRoute(
        formDest,
        formMask,
        formGw,
        formMetric,
        selectedNic?.index
      );
      await handleRepairCommandResult("Add Route", result, {
        appendOutput: true,
        refresh: true,
        successMessage: "Route added successfully!",
        failureMessage: "Add Route - Failed",
      });
    } catch (err) {
      setStatusMsg(`Error: ${err}`);
    }
  }, [formDest, formGw, formMask, formMetric, handleRepairCommandResult, selectedNic?.index]);

  const handleDeleteRoute = useCallback(async () => {
    if (!formDest) {
      setStatusMsg("Please fill Destination IP");
      return;
    }
    setStatusMsg("Deleting route...");
    try {
      const result = await repairDeleteRoute(formDest, formMask);
      await handleRepairCommandResult("Delete Route", result, {
        appendOutput: true,
        refresh: true,
        successMessage: "Route deleted!",
        failureMessage: "Delete Route - Failed",
      });
    } catch (err) {
      setStatusMsg(`Error: ${err}`);
    }
  }, [formDest, formMask, handleRepairCommandResult]);

  const executeSetInternet = useCallback(async () => {
    if (!selectedNic || !selectedNic.gateway) {
      setStatusMsg("Select a NIC with a gateway first");
      return;
    }
    setStatusMsg("Setting default gateway...");
    try {
      const gatewayResult = await repairSetDefaultGateway(selectedNic.gateway, selectedNic.index);
      const gatewayApplied = await handleRepairCommandResult("Set Default Gateway", gatewayResult, {
        appendOutput: true,
        successMessage: "Default gateway set.",
        failureMessage: "Set Default Gateway - Failed",
      });
      if (!gatewayApplied) {
        return;
      }

      const persistWriteMode = getPersistStartupWriteMode(persistWanOnStartup);
      if (persistWriteMode === "save") {
        try {
          const persistRouteInterfaceIndexes = getPersistRouteInterfaceIndexes(routes);
          const stableIdIndexes = Array.from(
            new Set([selectedNic.index, ...persistRouteInterfaceIndexes]),
          );
          const stableIds = await persistGetNicStableIds(stableIdIndexes);
          const nicId = stableIds[0];
          const routeNicEntries = new Map(
            [
              [selectedNic.index, nicId] as const,
              ...stableIdIndexes.slice(1).map((interfaceIndex, index) => [
                interfaceIndex,
                stableIds[index + 1],
              ] as const),
            ],
          );
          const config: PersistConfig = {
            schema_version: 1,
            enabled: true,
            nic: nicId,
            wan: { gateway: selectedNic.gateway, metric: "1" },
            custom_routes: buildPersistCustomRoutes(
              routes,
              routeNicEntries,
            ),
            updated_at: new Date().toISOString(),
          };
          const persistConfigResult = await repairSavePersistConfig(config);
          await handleRepairCommandResult("Persist Startup Config", persistConfigResult, {
            appendOutput: true,
            successMessage: "Default gateway set. Persist on startup enabled.",
            failureMessage: "Persist Startup Config - Failed",
          });
        } catch (persistErr) {
          console.warn("Failed to save persist config:", persistErr);
        }
      } else {
        try {
          const persistConfigResult = await repairClearPersistConfig();
          await handleRepairCommandResult("Persist Startup Config", persistConfigResult, {
            appendOutput: true,
            successMessage: "Default gateway set. Persist on startup disabled.",
            failureMessage: "Persist Startup Config - Failed",
          });
        } catch (persistErr) {
          console.warn("Failed to disable persist config:", persistErr);
        }
      }
    } catch (err) {
      setStatusMsg(`Error: ${err}`);
    }
  }, [handleRepairCommandResult, persistWanOnStartup, routes, selectedNic]);

  const executeFlush = useCallback(async () => {
    setStatusMsg("Flushing routes...");
    try {
      const result = await repairFlushRoutes();
      await handleRepairCommandResult("Flush Routes", result, {
        appendOutput: true,
        refresh: true,
        successMessage: "All routes flushed!",
        failureMessage: "Flush Routes - Failed",
      });
    } catch (err) {
      setStatusMsg(`Error: ${err}`);
    }
  }, [handleRepairCommandResult]);

  const appendCommandOutput = useCallback((title: string, output: string) => {
    const stamp = new Date().toLocaleTimeString("en-GB");
    const cleanOutput = output?.trim() ? output.trim() : "(No output returned)";
    const lines = [`[${stamp}] ${title}`, ...cleanOutput.split(/\r?\n/), ""];
    appendCommandLines(lines);
  }, [appendCommandLines]);

  async function handleRepairCommandResult(
    title: string,
    result: { success: boolean; output: string; requires_unlock: boolean },
    options?: {
      refresh?: boolean;
      invalidateNicCache?: boolean;
      appendOutput?: boolean;
      successMessage?: string;
      failureMessage?: string;
    },
  ) {
    if (options?.appendOutput !== false) {
      appendCommandOutput(title, result.output);
    }

    if (result.requires_unlock) {
      setStatusMsg("Unlock Repair Mode first to run admin fixes.");
      const status = await getRepairSessionStatus();
      setRepairSession(status);
      return false;
    }

    setStatusMsg(
      result.success
        ? (options?.successMessage ?? `${title} - Success!`)
        : (options?.failureMessage ?? `${title} - Failed`)
    );

    if (result.success && options?.refresh) {
      await loadData({ invalidateNicCache: options.invalidateNicCache });
    }
    return result.success;
  }

  async function executeRepairAction(
    action: RepairMachineAction,
    title: string,
    options?: { refresh?: boolean; invalidateNicCache?: boolean }
  ) {
    setDiagnosticView("command");
    setStatusMsg(`Running ${title}...`);
    try {
      const result = await runRepairMachineAction(action);
      await handleRepairCommandResult(title, result, {
        appendOutput: true,
        refresh: options?.refresh,
        invalidateNicCache: options?.invalidateNicCache,
      });
    } catch (err) {
      appendCommandOutput(title, `Error: ${err}`);
      setStatusMsg(`Error: ${err}`);
    }
  }

  const executeNetCmd = useCallback(async (
    cmd: string,
    title: string,
    options?: { refresh?: boolean; invalidateNicCache?: boolean }
  ) => {
    setDiagnosticView("command");
    setStatusMsg(`Running ${title}...`);
    try {
      const result = await runNetworkCommand(cmd);
      appendCommandOutput(title, result.output);
      const elevationRequired =
        /requires elevation|run as administrator|os error 740/i.test(result.output || "");
      if (elevationRequired) {
        setStatusMsg(`${title} requires Administrator privileges`);
      } else {
        setStatusMsg(result.success ? `${title} - Success!` : `${title} - Failed`);
      }
      if (options?.refresh) {
        void loadData({ invalidateNicCache: options?.invalidateNicCache });
      }
    } catch (err) {
      appendCommandOutput(title, `Error: ${err}`);
      setStatusMsg(`Error: ${err}`);
    }
  }, [appendCommandOutput, loadData]);

  const handleShowRoutingOutput = useCallback(async () => {
    setDiagnosticsOpen(true);
    setDiagnosticView("routing");
    if (routes.length > 0) {
      setRoutingOutput(formatRoutingSnapshot(routes));
      setStatusMsg(`Routing table snapshot loaded (${routes.length} cached routes)`);
      return;
    }

    setStatusMsg("Loading routing table snapshot...");
    try {
      const routeData = await getRoutingTable();
      setRoutes(routeData);
      setRoutingOutput(formatRoutingSnapshot(routeData));
      setStatusMsg(`Routing table snapshot loaded (${routeData.length} routes)`);
    } catch (err) {
      const errorText = `Error: ${err}`;
      setRoutingOutput(`Failed to load routing table snapshot.\n${errorText}`);
      setStatusMsg(errorText);
    }
  }, [routes]);

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

  const loadBatterySummary = useCallback(async () => {
    setBatteryLoading(true);
    setBatterySummaryError("");
    try {
      const summary = await getBatterySummary();
      setBatterySummary(summary);
      if (summary.present) {
        const wearLabel = getBatteryWearLevel(summary.wear_percent);
        setStatusMsg(`Battery summary loaded (${wearLabel})`);
      } else {
        setStatusMsg("Battery summary loaded (no battery detected)");
      }
    } catch (err) {
      setBatterySummary(null);
      setBatterySummaryError(String(err));
      setStatusMsg(`Battery summary error: ${err}`);
    } finally {
      setBatteryLoading(false);
    }
  }, []);

  const handleOpenBatteryModal = useCallback(() => {
    batteryModal.open();
    void loadBatterySummary();
  }, [batteryModal, loadBatterySummary]);

  const handleCloseBatteryModal = useCallback(() => {
    if (batteryLoading) return;
    batteryModal.close();
  }, [batteryModal, batteryLoading]);

  const handleResetWinHttpProxy = async () => {
    openConfirm(
      "Reset WinHTTP Proxy",
      "Reset WinHTTP proxy settings to direct access?",
      () => executeRepairAction("ResetWinHttpProxy", "Reset WinHTTP Proxy", { refresh: true })
    );
  };

  const handleRestartAdapters = async () => {
    openConfirm(
      "Restart Active Adapters",
      "Restart active physical network adapters now?",
      () => executeRepairAction("RestartActiveAdapters", "Restart Active Adapters", {
        refresh: true,
        invalidateNicCache: true,
      })
    );
  };

  const handleNslookupTest = useCallback(async () => {
    const host = sanitizeHostToken(diagHost) || "google.com";
    const dns = sanitizeDnsToken(diagDnsServer) || "8.8.8.8";
    setDiagHost(host);
    setDiagDnsServer(dns);
    await executeNetCmd(`nslookup ${host} ${dns}`, `NSLookup ${host}`);
  }, [diagDnsServer, diagHost, executeNetCmd, sanitizeDnsToken, sanitizeHostToken]);

  const handlePortConnectivityTest = useCallback(async () => {
    const host = sanitizeHostToken(diagHost) || "google.com";
    const portNum = Number.parseInt(diagPort, 10);
    const port = Number.isFinite(portNum) && portNum >= 1 && portNum <= 65535 ? portNum : 443;
    setDiagHost(host);
    setDiagPort(String(port));
    setDiagnosticView("command");
    setStatusMsg(`Testing port ${host}:${port}...`);
    try {
      const result = await testTcpPort(host, port);
      appendCommandOutput(`Port Test ${host}:${port}`, result.output);
      setStatusMsg(result.success ? `Port ${port} open on ${host}` : `Port ${port} closed on ${host}`);
    } catch (err: unknown) {
      const msg = err instanceof Error ? err.message : String(err);
      appendCommandOutput(`Port Test ${host}:${port}`, `Error: ${msg}`);
      setStatusMsg(`Port test failed: ${msg}`);
    }
  }, [diagHost, diagPort, appendCommandOutput, sanitizeHostToken]);

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
        const result = await fpingScan(batchTargets, 700);
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
    } catch (err) {
      setStatusMsg(`Scan IP error: ${err}`);
      setIpScanProgressText(`Scan failed: ${err}`);
    } finally {
      setIpScanRunning(false);
      setIpScanStopPending(false);
      ipScanStopRequestedRef.current = false;
    }
  }, [ipScanRunning, updateIpScanProgress]);

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
      const result = await runNetworkCommand(`tracert -d ${target}`);
      appendPingLines(result.output.trim().split(/\r?\n/));
      setStatusMsg(result.success ? `Tracert ${target} done` : `Tracert ${target} failed`);
    } catch (err) {
      appendPingLine(`Tracert error: ${err}`);
      setStatusMsg(`Tracert error: ${err}`);
    }
  }, [appendPingLine, appendPingLines, pingTarget]);

  const loadBloatwareList = useCallback(async () => {
    setBloatwareLoading(true);
    try {
      const items = await getBloatwareCandidates();
      setBloatwareItems(items);
      setSelectedBloatware((previous) => {
        if (previous.size === 0) return previous;
        const available = new Set(items.map((item) => item.package_name));
        const next = new Set<string>();
        previous.forEach((name) => {
          if (available.has(name)) {
            next.add(name);
          }
        });
        return next;
      });
    } catch (err) {
      setStatusMsg(`Bloatware list error: ${err}`);
    } finally {
      setBloatwareLoading(false);
    }
  }, []);

  const handleOpenBloatwareModal = useCallback(() => {
    resetRemoveProgress();
    bloatwareModal.open();
    void loadBloatwareList();
    void loadRepairTargets();
  }, [bloatwareModal, loadBloatwareList, loadRepairTargets, resetRemoveProgress]);

  const handleCloseBloatwareModal = useCallback(() => {
    if (bloatwareRemoving) return;
    bloatwareModal.close();
  }, [bloatwareModal, bloatwareRemoving]);

  const handleToggleBloatware = useCallback((packageName: string) => {
    setSelectedBloatware((previous) => {
      const next = new Set(previous);
      if (next.has(packageName)) {
        next.delete(packageName);
      } else {
        next.add(packageName);
      }
      return next;
    });
  }, []);

  const handleSelectInstalledBloatware = useCallback(() => {
    const next = new Set<string>();
    for (const item of bloatwareItems) {
      if (item.installed) {
        next.add(item.package_name);
      }
    }
    setSelectedBloatware(next);
  }, [bloatwareItems]);

  const handleSelectAllBloatware = useCallback(() => {
    setSelectedBloatware(new Set(bloatwareItems.map((item) => item.package_name)));
  }, [bloatwareItems]);

  const handleClearBloatwareSelection = useCallback(() => {
    setSelectedBloatware(new Set());
  }, []);

  const executeRemoveSelectedBloatware = useCallback(async () => {
    console.log("[BLOATWARE] executeRemoveSelectedBloatware CALLED");
    const packages = Array.from(selectedBloatware);
    if (!packages.length) {
      console.log("[BLOATWARE] No packages selected, returning");
      setStatusMsg("Select at least one app to remove");
      return;
    }

    // Resolve target SID — use state value or auto-load repair targets
    let targetSid = selectedRepairTargetSid;
    console.log("[BLOATWARE] Initial targetSid from state:", targetSid);
    if (!targetSid) {
      console.log("[BLOATWARE] No targetSid, auto-loading repair targets...");
      try {
        const targets = await listRepairTargets();
        console.log("[BLOATWARE] Loaded repair targets:", targets.length, targets);
        if (targets.length > 0) {
          const activeTarget = targets.find((t) => t.is_loaded) || targets[0];
          targetSid = activeTarget.sid;
          setSelectedRepairTargetSid(targetSid);
          console.log("[BLOATWARE] Auto-selected targetSid:", targetSid);
        } else {
          console.log("[BLOATWARE] No repair targets found!");
          setRemoveProgressText("Error: No repair target found. Unlock Repair Mode first.");
          return;
        }
      } catch (err) {
        console.error("[BLOATWARE] Failed to load repair targets:", err);
        setRemoveProgressText("Error: Could not load repair targets. Unlock Repair Mode first.");
        return;
      }
    }

    console.log("[BLOATWARE] Starting removal loop for", packages.length, "packages with SID:", targetSid);
    setBloatwareRemoving(true);
    setDiagnosticView("command");
    setDiagnosticsOpen(true);
    updateRemoveProgress(0, `Starting removal... 0/${packages.length} (0%)`);
    setStatusMsg(`Removing ${packages.length} selected app(s)...`);
    let successCount = 0;
    let failedCount = 0;
    try {
      for (let index = 0; index < packages.length; index += 1) {
        const packageName = packages[index];
        const appLabel = bloatwareItems.find((item) => item.package_name === packageName)?.label ?? packageName;
        const beforePercent = Math.round((index / packages.length) * 100);
        updateRemoveProgress(
          beforePercent,
          `Removing ${appLabel}... ${index}/${packages.length} (${beforePercent}%)`
        );

        try {
          console.log(`[BLOATWARE] Calling repairRemoveBloatware for ${packageName} with SID ${targetSid}`);
          const result = await repairRemoveBloatware(
            targetSid,
            [packageName],
            true
          );
          console.log(`[BLOATWARE] Result for ${packageName}:`, result);
          appendCommandOutput(`Remove Apps - ${appLabel}`, result.output);
          if (result.requires_unlock) {
            failedCount += 1;
            console.log("[BLOATWARE] Requires unlock! Breaking loop.");
            setStatusMsg("Unlock Repair Mode first to remove apps");
            setRemoveProgressText("Error: Repair Mode is locked. Unlock first, then retry.");
            const status = await getRepairSessionStatus();
            setRepairSession(status);
            break;
          } else if (result.success) {
            successCount += 1;
          } else {
            failedCount += 1;
            console.log(`[BLOATWARE] Failed for ${packageName}: ${result.output}`);
          }
        } catch (err) {
          failedCount += 1;
          console.error(`[BLOATWARE] Exception for ${packageName}:`, err);
          appendCommandOutput(`Remove Apps - ${appLabel}`, `Error: ${err}`);
        }

        const processed = index + 1;
        const percent = Math.round((processed / packages.length) * 100);
        updateRemoveProgress(percent, `Processed ${processed}/${packages.length} (${percent}%)`);
      }

      console.log(`[BLOATWARE] Loop done. Success: ${successCount}, Failed: ${failedCount}`);
      setStatusMsg(
        failedCount === 0
          ? `Remove Apps completed (${successCount}/${packages.length})`
          : `Remove Apps completed with warnings (${failedCount} failed)`
      );
      setRemoveProgressText(`Done: ${successCount} success, ${failedCount} failed`);
      setSelectedBloatware(new Set());
      await loadBloatwareList();
    } catch (err) {
      console.error("[BLOATWARE] Outer error:", err);
      appendCommandOutput("Remove Apps", `Error: ${err}`);
      setStatusMsg(`Remove Apps error: ${err}`);
      setRemoveProgressText("Removal aborted by error.");
    } finally {
      setBloatwareRemoving(false);
    }
  }, [
    appendCommandOutput,
    bloatwareItems,
    loadBloatwareList,
    selectedBloatware,
    selectedRepairTargetSid,
    updateRemoveProgress,
  ]);

  const handleOpenCacheModal = useCallback(() => {
    setSelectedCaches(new Set(DEFAULT_CACHE_SELECTION));
    resetCacheProgress();
    setCacheStopPending(false);
    cacheStopRequestedRef.current = false;
    cacheModal.open();
  }, [cacheModal, resetCacheProgress]);

  const handleCloseCacheModal = useCallback(() => {
    if (cacheCleaning) return;
    cacheModal.close();
  }, [cacheModal, cacheCleaning]);

  const handleToggleCache = useCallback((cacheId: string) => {
    setSelectedCaches((previous) => {
      const next = new Set(previous);
      if (next.has(cacheId)) {
        next.delete(cacheId);
      } else {
        next.add(cacheId);
      }
      return next;
    });
  }, []);

  const handleSelectAllCaches = useCallback(() => {
    setSelectedCaches(new Set(CACHE_CLEANUP_OPTIONS.map((option) => option.id)));
  }, []);

  const handleClearCacheSelection = useCallback(() => {
    setSelectedCaches(new Set());
  }, []);

  const handleForceStopCacheCleanup = useCallback(() => {
    if (!cacheCleaning || cacheStopPending) return;
    cacheStopRequestedRef.current = true;
    setCacheStopPending(true);
    setStatusMsg("Force stop requested. Waiting for current task to finish...");
    setCacheProgressText("Stopping... waiting for current task to finish.");
  }, [cacheCleaning, cacheStopPending]);

  const executeClearSelectedCaches = useCallback(async () => {
    if (!selectedCacheTargets.length) {
      setStatusMsg("Select at least one cache target");
      return;
    }
    // Resolve target SID — use state value or auto-load repair targets
    let cacheTargetSid = selectedRepairTargetSid;
    if (!cacheTargetSid) {
      try {
        const targets = await listRepairTargets();
        if (targets.length > 0) {
          const activeTarget = targets.find((t) => t.is_loaded) || targets[0];
          cacheTargetSid = activeTarget.sid;
          setSelectedRepairTargetSid(cacheTargetSid);
        } else {
          setCacheProgressText("Error: No repair target found. Unlock Repair Mode first.");
          return;
        }
      } catch {
        setCacheProgressText("Error: Could not load repair targets. Unlock Repair Mode first.");
        return;
      }
    }

    setCacheCleaning(true);
    setCacheStopPending(false);
    cacheStopRequestedRef.current = false;
    setDiagnosticView("command");
    setDiagnosticsOpen(true);
    updateCacheProgress(0, `Starting cleanup... 0/${selectedCacheTargets.length} (0%)`);
    setStatusMsg(`Cleaning ${selectedCacheTargets.length} cache target(s)...`);
    let successCount = 0;
    let failedCount = 0;
    let processedCount = 0;
    try {
      for (let index = 0; index < selectedCacheTargets.length; index += 1) {
        if (cacheStopRequestedRef.current) {
          break;
        }

        const target = selectedCacheTargets[index];
        const beforePercent = Math.round((index / selectedCacheTargets.length) * 100);
        updateCacheProgress(
          beforePercent,
          `Cleaning ${target.label}... ${index}/${selectedCacheTargets.length} (${beforePercent}%)`
        );

        try {
          const result = await repairClearCacheTargets(cacheTargetSid, [target.id]);
          appendCommandOutput(`Clear Cache - ${target.label}`, result.output);
          if (result.requires_unlock) {
            failedCount += 1;
            setStatusMsg("Unlock Repair Mode first to clean profile caches");
            const status = await getRepairSessionStatus();
            setRepairSession(status);
            break;
          } else if (result.success) {
            successCount += 1;
          } else {
            failedCount += 1;
          }
        } catch (err) {
          failedCount += 1;
          appendCommandOutput(`Clear Cache - ${target.label}`, `Error: ${err}`);
        }

        processedCount = index + 1;
        const percent = Math.round((processedCount / selectedCacheTargets.length) * 100);
        updateCacheProgress(
          percent,
          `Processed ${processedCount}/${selectedCacheTargets.length} (${percent}%)`
        );

        if (cacheStopRequestedRef.current) {
          break;
        }
      }

      const stoppedEarly = cacheStopRequestedRef.current && processedCount < selectedCacheTargets.length;
      if (stoppedEarly) {
        setStatusMsg(`Cleanup stopped by user (${processedCount}/${selectedCacheTargets.length})`);
        setCacheProgressText(
          `Stopped: processed ${processedCount}/${selectedCacheTargets.length}, success ${successCount}, failed ${failedCount}`
        );
      } else {
        setStatusMsg(
          failedCount === 0
            ? `Clear Cache completed (${successCount}/${selectedCacheTargets.length})`
            : `Clear Cache completed with warnings (${failedCount} failed)`
        );
        setCacheProgressText(
          `Done: ${successCount} success, ${failedCount} failed`
        );
      }
    } catch (err) {
      appendCommandOutput("Clear Cache", `Error: ${err}`);
      setStatusMsg(`Clear Cache error: ${err}`);
      setCacheProgressText("Cleanup aborted by error.");
    } finally {
      setCacheCleaning(false);
      setCacheStopPending(false);
      cacheStopRequestedRef.current = false;
    }
  }, [appendCommandOutput, selectedCacheTargets, selectedRepairTargetSid, updateCacheProgress]);

  const openConfirm = (
    title: string,
    message: string,
    action: () => void | Promise<void>
  ) => {
    confirmActionRef.current = action;
    setConfirmTitle(title);
    setConfirmMessage(message);
    setConfirmOpen(true);
  };

  const onConfirm = () => {
    const action = confirmActionRef.current;
    confirmActionRef.current = null;
    setConfirmOpen(false);
    if (!action) return;
    Promise.resolve(action()).catch((err) => setStatusMsg(`Error: ${err}`));
  };

  const onCancelConfirm = () => {
    confirmActionRef.current = null;
    setConfirmOpen(false);
  };

  const handleStartPing = useCallback(() => {
    const target = pingTarget.trim() || "1.1.1.1";
    setPingTarget(target);
    pingSeqRef.current = 0;
    const label = pingMode === "fping" ? "fping-like" : "ping";
    appendPingLine(`--- Start ${label} continuous check to ${target} ---`);
    setStatusMsg(`${label} ${target} continuously...`);
    setPingRunning(true);
  }, [appendPingLine, pingMode, pingTarget]);

  const handleStopPing = useCallback(() => {
    const target = pingTarget.trim() || "1.1.1.1";
    setPingRunning(false);
    appendPingLine(`--- Stopped continuous ping to ${target} ---`);
    setStatusMsg("Ping stopped");
  }, [appendPingLine, pingTarget]);

  useEffect(() => {
    if (!pingRunning) {
      if (pingLoopRef.current) {
        window.clearInterval(pingLoopRef.current);
        pingLoopRef.current = null;
      }
      return;
    }

    const target = pingTarget.trim() || "1.1.1.1";
    const parsedTargets = target
      .split(/[\s,;]+/)
      .map((t) => t.trim())
      .filter(Boolean);
    const fpingTargets = parsedTargets.length > 0 ? parsedTargets : ["1.1.1.1"];
    const runOnce = async () => {
      if (pingBusyRef.current) return;
      pingBusyRef.current = true;
      try {
        if (pingMode === "fping") {
          const result = await fpingScan(fpingTargets, 1200);
          const stamp = new Date().toLocaleTimeString("en-GB");
          pingSeqRef.current += 1;
          const lines: string[] = [
            `[${stamp}] fping-like round=${pingSeqRef.current} sent=${result.sent} recv=${result.received} loss=${result.loss_percent.toFixed(0)}% min/avg/max=${result.min_ms}/${result.avg_ms}/${result.max_ms}ms`,
          ];
          for (const host of result.hosts) {
            if (host.success) {
              lines.push(`  [UP] ${host.target} ${host.latency_ms} ms`);
            } else {
              lines.push(`  [DOWN] ${host.target} timeout`);
            }
          }
          appendPingLines(lines);
        } else {
          const result = await pingHost(target, 1);
          const stamp = new Date().toLocaleTimeString("en-GB");
          pingSeqRef.current += 1;
          if (result.success) {
            appendPingLine(`[${stamp}] Reply from ${target}: bytes=32 time=${result.latency_ms}ms TTL=52`);
          } else {
            appendPingLine(`[${stamp}] Request timed out (${target})`);
          }
        }
      } catch (err) {
        appendPingLine(`[${new Date().toLocaleTimeString("en-GB")}] Ping error: ${err}`);
      } finally {
        pingBusyRef.current = false;
      }
    };

    void runOnce();
    pingLoopRef.current = window.setInterval(() => {
      void runOnce();
    }, pingMode === "fping" ? 450 : 1000);

    return () => {
      if (pingLoopRef.current) {
        window.clearInterval(pingLoopRef.current);
        pingLoopRef.current = null;
      }
    };
  }, [pingRunning, pingTarget, pingMode]);

  useEffect(() => {
    if (pingOutputRef.current) {
      pingOutputRef.current.scrollTop = pingOutputRef.current.scrollHeight;
    }
  }, [pingLogVersion]);

  useEffect(() => {
    if (commandOutputRef.current) {
      commandOutputRef.current.scrollTop = commandOutputRef.current.scrollHeight;
    }
  }, [commandLogVersion]);

  // ======================== RENDER ========================

  const diagnosticsOutputText = diagnosticView === "routing"
    ? (routingOutput || "Routing table output will appear here.")
    : (commandOutputText || "Command output will appear here.");
  const installedBloatwareCount = useMemo(
    () => bloatwareItems.filter((item) => item.installed).length,
    [bloatwareItems]
  );
  const selectedBloatwareCount = selectedBloatware.size;
  const selectedCacheCount = selectedCacheTargets.length;
  const machineRepairEnabled = isMachineRepairEnabled({ locked: repairSession.locked });
  const profileSensitiveActionEnabled = isProfileSensitiveActionEnabled({
    locked: repairSession.locked,
    selectedTargetSid: selectedRepairTargetSid,
  });
  const profileSensitiveActionHint = getProfileSensitiveActionHint({
    locked: repairSession.locked,
    selectedTargetSid: selectedRepairTargetSid,
  });

  const handleRemoveSelectedBloatware = useCallback(() => {
    if (selectedBloatwareCount === 0) {
      setStatusMsg("Select at least one app to remove");
      return;
    }
    openConfirm(
      "Remove Selected Apps",
      `Remove ${selectedBloatwareCount} selected app(s)? This operation may require Administrator privileges.`,
      executeRemoveSelectedBloatware
    );
  }, [executeRemoveSelectedBloatware, selectedBloatwareCount]);

  const handleStartCacheCleanup = useCallback(() => {
    if (selectedCacheCount === 0) {
      setStatusMsg("Select at least one cache target");
      return;
    }
    openConfirm(
      "Start Cache Cleanup",
      `Clean ${selectedCacheCount} selected cache target(s)?`,
      executeClearSelectedCaches
    );
  }, [executeClearSelectedCaches, selectedCacheCount]);

  useEffect(() => {
    localStorage.setItem("ui-theme", theme);
  }, [theme]);

  useEffect(() => {
    localStorage.setItem("wan-persist-on-startup", persistWanOnStartup ? "true" : "false");
  }, [persistWanOnStartup]);

  useEffect(() => {
    localStorage.setItem("help-language", helpLanguage);
  }, [helpLanguage]);

  const handleToggleTheme = () => {
    if (lensTimerRef.current) {
      window.clearTimeout(lensTimerRef.current);
    }
    setThemeLensActive(true);
    setTheme((t) => (t === "dark" ? "light" : "dark"));
    lensTimerRef.current = window.setTimeout(() => {
      setThemeLensActive(false);
    }, 650);
  };

  useEffect(() => {
    return () => {
      if (lensTimerRef.current) window.clearTimeout(lensTimerRef.current);
    };
  }, []);

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
            <span
              onClick={handleZoomReset}
              className="zoom-label-header"
              title="Reset zoom to 100%"
            >
              {zoomLevel}%
            </span>
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
            onClick={handleOpenBloatwareModal}
            disabled={!profileSensitiveActionEnabled || bloatwareLoading || bloatwareRemoving}
            className="header-apps-action capsule-btn"
            title={profileSensitiveActionHint || "Open app removal tools"}
          >
            <Trash2 className="w-3.5 h-3.5" />
            Remove Apps
          </button>

          <button
            onClick={handleOpenCacheModal}
            disabled={!profileSensitiveActionEnabled || cacheCleaning}
            className="header-cache-action capsule-btn"
            title={profileSensitiveActionHint || "Open cache cleanup tools"}
          >
            <Flame className="w-3.5 h-3.5" />
            Clear Cache
          </button>

          <button
            onClick={handleToggleTheme}
            className="theme-toggle capsule-btn flex items-center gap-1.5 px-2.5 py-1.5 text-xs font-semibold transition"
            title="Toggle light/dark mode"
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
                    <th className="w-12">ID</th>
                    <th className="w-28">IPv4</th>
                    <th className="w-28">Gateway</th>
                    <th>Device</th>
                  </tr>
                </thead>
                <tbody>
                  {nics.map((nic) => (
                    <tr
                      key={nic.index}
                      onClick={() => handleSelectNic(nic)}
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
              <Field label="Destination" value={formDest} onChange={setFormDest} placeholder="10.0.0.0" />
              <Field label="Subnet Mask" value={formMask} onChange={setFormMask} placeholder="255.255.255.0" />
              <Field label="Gateway" value={formGw} onChange={setFormGw} placeholder="192.168.1.1" />
              <Field label="Metric" value={formMetric} onChange={setFormMetric} placeholder="10" />
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
            open={toolsOpen}
            onToggle={() => setToolsOpen(!toolsOpen)}
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
                onClick={handleOpenBatteryModal} tone="system" />
            </div>
          </Section>

          <Section
            icon={Monitor}
            title="Diagnostics & Repair"
            open={diagnosticsOpen}
            onToggle={() => setDiagnosticsOpen(!diagnosticsOpen)}
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
                  value={diagHost}
                  onChange={(e) => setDiagHost(e.target.value)}
                  placeholder="Domain or IP (e.g. google.com)"
                  className="diag-input"
                />
                <input
                  type="text"
                  value={diagPort}
                  onChange={(e) => setDiagPort(e.target.value)}
                  placeholder="Port"
                  className="diag-input diag-port"
                />
                <button
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
                  value={diagDnsServer}
                  onChange={(e) => setDiagDnsServer(e.target.value)}
                  placeholder="DNS server (e.g. 8.8.8.8)"
                  className="diag-input"
                />
                <button
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
            open={pingOpen}
            onToggle={() => setPingOpen(!pingOpen)}
          >
            <div className="segmented-control mb-2">
              <button
                onClick={() => setPingMode("ping")}
                className={`segment-btn ${pingMode === "ping" ? "segment-btn-active" : ""}`}
              >
                Ping
              </button>
              <button
                onClick={() => setPingMode("fping")}
                className={`segment-btn ${pingMode === "fping" ? "segment-btn-active" : ""}`}
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
              />
              <button
                onClick={handleStartPing}
                disabled={pingRunning}
                className="ping-cmd-btn ping-cmd-start"
              >
                <Send className="w-4 h-4" /> Start
              </button>
              <button
                onClick={handleStopPing}
                disabled={!pingRunning}
                className="ping-cmd-btn ping-cmd-stop"
              >
                <OctagonAlert className="w-4 h-4" />
                Stop
              </button>
              <button
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
          <span className="text-[0.65rem] text-slate-500">{statusMsg}</span>
          <button
            onClick={donateModal.open}
            className="donate-footer-btn capsule-btn"
            title="Donate to the author Zozon"
          >
            Donate
          </button>
          <button
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
        open={batteryModal.isOpen}
        loading={batteryLoading}
        summary={batterySummary}
        error={batterySummaryError}
        onRefresh={loadBatterySummary}
        onClose={handleCloseBatteryModal}
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
        open={cacheModal.isOpen}
        cleaning={cacheCleaning}
        stopPending={cacheStopPending}
        options={CACHE_CLEANUP_OPTIONS}
        selectedCaches={selectedCaches}
        selectedCount={selectedCacheCount}
        progressPercent={cacheProgressPercent}
        progressText={cacheProgressText}
        onToggleCache={handleToggleCache}
        onSelectAll={handleSelectAllCaches}
        onClearSelection={handleClearCacheSelection}
        onForceStop={handleForceStopCacheCleanup}
        onStartCleanup={handleStartCacheCleanup}
        onClose={handleCloseCacheModal}
      />

      <BloatwareModal
        open={bloatwareModal.isOpen}
        loading={bloatwareLoading}
        removing={bloatwareRemoving}
        items={bloatwareItems}
        selectedPackages={selectedBloatware}
        selectedCount={selectedBloatwareCount}
        installedCount={installedBloatwareCount}
        progressPercent={removeProgressPercent}
        progressText={removeProgressText}
        onTogglePackage={handleToggleBloatware}
        onSelectAll={handleSelectAllBloatware}
        onSelectInstalled={handleSelectInstalledBloatware}
        onClearSelection={handleClearBloatwareSelection}
        onRemoveSelected={handleRemoveSelectedBloatware}
        onClose={handleCloseBloatwareModal}
      />

      {confirmOpen && (
        <div className="fixed inset-0 z-50 bg-slate-950/60 flex items-center justify-center px-4">
          <div className="w-full max-w-md rounded-xl border border-slate-600 bg-slate-900 shadow-2xl">
            <div className="flex items-center gap-2 px-4 py-3 border-b border-slate-700">
              <OctagonAlert className="w-4 h-4 text-amber-400" />
              <h3 className="text-sm font-bold text-slate-100">{confirmTitle}</h3>
            </div>
            <div className="confirm-dialog-body px-4 py-4 text-sm">{confirmMessage}</div>
            <div className="flex justify-end gap-2 px-4 py-3 border-t border-slate-700">
              <button
                onClick={onCancelConfirm}
                className="capsule-btn px-3 py-1.5 min-w-[84px] border border-slate-500 bg-slate-700/70 text-white font-semibold hover:bg-slate-600 transition"
              >
                Cancel
              </button>
              <button
                onClick={onConfirm}
                className="capsule-btn px-3 py-1.5 bg-blue-600 text-white hover:bg-blue-500 transition"
              >
                Confirm
              </button>
            </div>
          </div>
        </div>
      )}

      {routeWatcherToast && (
        <div className="fixed bottom-14 right-4 z-40 w-full max-w-sm px-4 sm:px-0 pointer-events-none">
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
                    onClick={handleOpenRouteWatcherToastAction}
                    className="mt-2 inline-flex rounded-lg border border-amber-300/35 bg-amber-400/10 px-2.5 py-1 text-[0.7rem] font-semibold text-amber-100 transition hover:bg-amber-300/15"
                  >
                    {routeWatcherToast.actionLabel}
                  </button>
                )}
              </div>
              <button
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

