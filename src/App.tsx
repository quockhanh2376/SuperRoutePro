import { memo, useState, useEffect, useCallback, useRef, useMemo } from "react";
import {
  Zap, Wifi, WifiOff, RefreshCw, Plus, Trash2, Globe, Flame,
  Activity, Send, Wrench, Monitor, Sun, Moon, OctagonAlert, Search,
  ChevronDown, ChevronUp, ArrowDownUp, X
} from "lucide-react";
import {
  getNetworkInterfaces, getRoutingTable, addRoute, deleteRoute,
  flushRoutes, setDefaultGateway, runNetworkCommand, pingHost,
  fpingScan,
  checkInternet, getBloatwareCandidates, removeBloatware, clearCacheTargets, getBatteryReport,
  type NetworkInterface, type RouteEntry, type BloatwareItem, type FpingHostResult,
} from "./api";

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

const IP_SCAN_MAX_TARGETS = 512;
const IP_SCAN_BATCH_SIZE = 24;
const FALLBACK_IP_SCAN_PREFIX = 24;

type IpScanPlan = {
  targets: string[];
  subnetLabel: string;
  truncated: boolean;
  source: "route" | "fallback";
};

const parseIpv4 = (value: string): number[] | null => {
  const parts = value.trim().split(".");
  if (parts.length !== 4) return null;
  const octets = parts.map((part) => Number.parseInt(part, 10));
  if (octets.some((octet) => !Number.isFinite(octet) || octet < 0 || octet > 255)) {
    return null;
  }
  return octets;
};

const ipv4ToInt = (octets: number[]): number =>
  (
    ((octets[0] << 24) >>> 0) +
    ((octets[1] << 16) >>> 0) +
    ((octets[2] << 8) >>> 0) +
    (octets[3] >>> 0)
  ) >>> 0;

const intToIpv4 = (value: number): string =>
  `${(value >>> 24) & 255}.${(value >>> 16) & 255}.${(value >>> 8) & 255}.${value & 255}`;

const prefixToMaskInt = (prefix: number): number => {
  if (prefix <= 0) return 0;
  if (prefix >= 32) return 0xffffffff >>> 0;
  return (0xffffffff << (32 - prefix)) >>> 0;
};

const maskToPrefix = (mask: string): number | null => {
  const octets = parseIpv4(mask);
  if (!octets) return null;
  const maskInt = ipv4ToInt(octets);
  let prefix = 0;
  let zeroSeen = false;
  for (let bit = 31; bit >= 0; bit -= 1) {
    const isOne = ((maskInt >>> bit) & 1) === 1;
    if (isOne) {
      if (zeroSeen) return null;
      prefix += 1;
    } else {
      zeroSeen = true;
    }
  }
  return prefix;
};

const buildIpScanPlan = (nic: NetworkInterface, routes: RouteEntry[]): IpScanPlan | null => {
  const nicOctets = parseIpv4(nic.ip);
  if (!nicOctets) return null;

  const nicInt = ipv4ToInt(nicOctets);
  let networkInt: number | null = null;
  let prefix: number | null = null;
  let source: "route" | "fallback" = "fallback";

  const connectedRoute = routes.find((route) => {
    if (route.interface_index !== nic.index) return false;
    if (route.gateway !== "0.0.0.0") return false;
    if (route.destination === "0.0.0.0" || route.netmask === "255.255.255.255") return false;
    return parseIpv4(route.destination) !== null && parseIpv4(route.netmask) !== null;
  });

  if (connectedRoute) {
    const routePrefix = maskToPrefix(connectedRoute.netmask);
    const routeDestination = parseIpv4(connectedRoute.destination);
    if (
      routePrefix !== null &&
      routePrefix >= 16 &&
      routePrefix <= 30 &&
      routeDestination
    ) {
      const routeMaskInt = prefixToMaskInt(routePrefix);
      networkInt = ipv4ToInt(routeDestination) & routeMaskInt;
      prefix = routePrefix;
      source = "route";
    }
  }

  if (networkInt === null || prefix === null) {
    prefix = FALLBACK_IP_SCAN_PREFIX;
    networkInt = nicInt & prefixToMaskInt(prefix);
    source = "fallback";
  }

  const hostSpan = 2 ** (32 - prefix);
  const hostCapacity = Math.max(0, hostSpan - 2);
  if (hostCapacity <= 0) return null;

  const firstHost = networkInt + 1;
  const lastHost = networkInt + hostSpan - 2;
  const selfInRange = nicInt >= firstHost && nicInt <= lastHost;
  const availableTargets = Math.max(0, hostCapacity - (selfInRange ? 1 : 0));
  const scanCount = Math.min(IP_SCAN_MAX_TARGETS, availableTargets);
  const targets: string[] = [];

  for (let offset = 1; offset < hostSpan - 1 && targets.length < scanCount; offset += 1) {
    const hostInt = (networkInt + offset) >>> 0;
    if (hostInt === nicInt) continue;
    targets.push(intToIpv4(hostInt));
  }

  return {
    targets,
    subnetLabel: `${intToIpv4(networkInt)}/${prefix}`,
    truncated: availableTargets > targets.length,
    source,
  };
};

type CacheCleanupOption = {
  id: string;
  label: string;
  description: string;
  defaultChecked: boolean;
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

export default function App() {
  const APP_VERSION = "6.3.0";
  const APP_AUTHOR = "Zonzon";
  const [theme, setTheme] = useState<"dark" | "light">("dark");

  // State
  const [nics, setNics] = useState<NetworkInterface[]>([]);
  const [routes, setRoutes] = useState<RouteEntry[]>([]);
  const [selectedNic, setSelectedNic] = useState<NetworkInterface | null>(null);
  const [activeOnly, setActiveOnly] = useState(true);
  const [isOnline, setIsOnline] = useState<boolean | null>(null);
  const [statusMsg, setStatusMsg] = useState("System Ready");
  const [loading, setLoading] = useState(false);
  const [pingTarget, setPingTarget] = useState("1.1.1.1");
  const [pingMode, setPingMode] = useState<"ping" | "fping">("ping");
  const [pingLogVersion, setPingLogVersion] = useState(0);
  const [commandLogVersion, setCommandLogVersion] = useState(0);
  const [pingRunning, setPingRunning] = useState(false);
  const [ipScanModalOpen, setIpScanModalOpen] = useState(false);
  const [ipScanRunning, setIpScanRunning] = useState(false);
  const [ipScanStopPending, setIpScanStopPending] = useState(false);
  const [ipScanPlan, setIpScanPlan] = useState<IpScanPlan | null>(null);
  const [ipScanResults, setIpScanResults] = useState<FpingHostResult[]>([]);
  const [ipScanProgressPercent, setIpScanProgressPercent] = useState(0);
  const [ipScanProgressText, setIpScanProgressText] = useState("Ready.");
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
  const [bloatwareModalOpen, setBloatwareModalOpen] = useState(false);
  const [bloatwareLoading, setBloatwareLoading] = useState(false);
  const [bloatwareRemoving, setBloatwareRemoving] = useState(false);
  const [bloatwareItems, setBloatwareItems] = useState<BloatwareItem[]>([]);
  const [selectedBloatware, setSelectedBloatware] = useState<Set<string>>(new Set());
  const [removeProgressPercent, setRemoveProgressPercent] = useState(0);
  const [removeProgressText, setRemoveProgressText] = useState("Ready.");
  const [batteryModalOpen, setBatteryModalOpen] = useState(false);
  const [batteryLoading, setBatteryLoading] = useState(false);
  const [batteryReportHtml, setBatteryReportHtml] = useState("");
  const [batteryReportError, setBatteryReportError] = useState("");
  const [cacheModalOpen, setCacheModalOpen] = useState(false);
  const [cacheCleaning, setCacheCleaning] = useState(false);
  const [cacheStopPending, setCacheStopPending] = useState(false);
  const [selectedCaches, setSelectedCaches] = useState<Set<string>>(
    () => new Set(DEFAULT_CACHE_SELECTION)
  );
  const [cacheProgressPercent, setCacheProgressPercent] = useState(0);
  const [cacheProgressText, setCacheProgressText] = useState("Ready.");
  const selectedCacheTargets = useMemo(
    () => CACHE_CLEANUP_OPTIONS.filter((option) => selectedCaches.has(option.id)),
    [selectedCaches]
  );

  // Form state
  const [formDest, setFormDest] = useState("");
  const [formMask, setFormMask] = useState("255.255.255.0");
  const [formGw, setFormGw] = useState("");
  const [formMetric, setFormMetric] = useState("10");

  const monitorRef = useRef(true);
  const pingLoopRef = useRef<number | null>(null);
  const pingBusyRef = useRef(false);
  const pingSeqRef = useRef(0);
  const lensTimerRef = useRef<number | null>(null);
  const cacheStopRequestedRef = useRef(false);
  const ipScanStopRequestedRef = useRef(false);
  const confirmActionRef = useRef<(() => void | Promise<void>) | null>(null);
  const pingLogLinesRef = useRef<string[]>([]);
  const commandLogLinesRef = useRef<string[]>([]);
  const pingRenderRafRef = useRef<number | null>(null);
  const commandRenderRafRef = useRef<number | null>(null);
  const pingOutputRef = useRef<HTMLPreElement | null>(null);
  const commandOutputRef = useRef<HTMLPreElement | null>(null);
  const MAX_LOG_LINES = 600;
  const MAX_COMMAND_LINES = 1200;

  // ======================== DATA LOADING ========================

  const loadData = useCallback(async () => {
    setLoading(true);
    setStatusMsg("Loading data...");
    try {
      const [nicData, routeData] = await Promise.all([
        getNetworkInterfaces(activeOnly),
        getRoutingTable(),
      ]);
      setNics(nicData);
      setRoutes(routeData);
      setRoutingOutput(formatRoutingSnapshot(routeData));
      setStatusMsg(`Loaded ${nicData.length} NICs, ${routeData.length} routes`);
    } catch (err) {
      setStatusMsg(`Error: ${err}`);
    }
    setLoading(false);
  }, [activeOnly]);

  useEffect(() => {
    loadData();
  }, [loadData]);

  // Internet monitor
  useEffect(() => {
    monitorRef.current = true;
    const interval = setInterval(async () => {
      if (!monitorRef.current) return;
      try {
        const online = await checkInternet();
        setIsOnline(online);
      } catch {
        setIsOnline(false);
      }
    }, 5000);
    checkInternet().then(setIsOnline).catch(() => setIsOnline(false));
    return () => {
      monitorRef.current = false;
      clearInterval(interval);
    };
  }, []);

  // Latency monitor
  useEffect(() => {
    monitorRef.current = true;
    const interval = setInterval(async () => {
      if (!monitorRef.current) return;
      try {
        const result = await pingHost("8.8.8.8", 1);
        const ms = result.success ? result.latency_ms : 0;
        setCurrentLatency(ms);
      } catch {
        setCurrentLatency(0);
      }
    }, 2000);
    return () => {
      monitorRef.current = false;
      clearInterval(interval);
    };
  }, []);

  // ======================== ACTIONS ========================

  const handleSelectNic = useCallback((nic: NetworkInterface) => {
    setSelectedNic(nic);
    setFormGw(nic.gateway);
  }, []);

  const handleAddRoute = useCallback(async () => {
    if (!formDest || !formGw) {
      setStatusMsg("Please fill Destination and Gateway");
      return;
    }
    setStatusMsg("Adding route...");
    try {
      await addRoute(formDest, formMask, formGw, formMetric, selectedNic?.index);
      setStatusMsg("Route added successfully!");
      loadData();
    } catch (err) {
      setStatusMsg(`Error: ${err}`);
    }
  }, [formDest, formGw, formMask, formMetric, selectedNic?.index, loadData]);

  const handleDeleteRoute = useCallback(async () => {
    if (!formDest) {
      setStatusMsg("Please fill Destination IP");
      return;
    }
    setStatusMsg("Deleting route...");
    try {
      await deleteRoute(formDest, formMask);
      setStatusMsg("Route deleted!");
      loadData();
    } catch (err) {
      setStatusMsg(`Error: ${err}`);
    }
  }, [formDest, formMask, loadData]);

  const executeSetInternet = useCallback(async () => {
    if (!selectedNic || !selectedNic.gateway) {
      setStatusMsg("Select a NIC with a gateway first");
      return;
    }
    setStatusMsg("Setting default gateway...");
    try {
      await setDefaultGateway(selectedNic.gateway, selectedNic.index);
      setStatusMsg("Default gateway set!");
      loadData();
    } catch (err) {
      setStatusMsg(`Error: ${err}`);
    }
  }, [loadData, selectedNic]);

  const executeFlush = useCallback(async () => {
    setStatusMsg("Flushing routes...");
    try {
      await flushRoutes();
      setStatusMsg("All routes flushed!");
      loadData();
    } catch (err) {
      setStatusMsg(`Error: ${err}`);
    }
  }, [loadData]);

  const schedulePingLogRender = useCallback(() => {
    if (pingRenderRafRef.current !== null) return;
    pingRenderRafRef.current = window.requestAnimationFrame(() => {
      pingRenderRafRef.current = null;
      setPingLogVersion((v) => v + 1);
    });
  }, []);

  const scheduleCommandLogRender = useCallback(() => {
    if (commandRenderRafRef.current !== null) return;
    commandRenderRafRef.current = window.requestAnimationFrame(() => {
      commandRenderRafRef.current = null;
      setCommandLogVersion((v) => v + 1);
    });
  }, []);

  const clearPingOutput = useCallback(() => {
    if (!pingLogLinesRef.current.length) return;
    pingLogLinesRef.current = [];
    schedulePingLogRender();
  }, [schedulePingLogRender]);

  const clearCommandOutput = useCallback(() => {
    if (!commandLogLinesRef.current.length) return;
    commandLogLinesRef.current = [];
    scheduleCommandLogRender();
  }, [scheduleCommandLogRender]);

  const appendCommandLines = useCallback((lines: string[]) => {
    if (!lines.length) return;
    const buffer = commandLogLinesRef.current;
    buffer.push(...lines);
    if (buffer.length > MAX_COMMAND_LINES) {
      buffer.splice(0, buffer.length - MAX_COMMAND_LINES);
    }
    scheduleCommandLogRender();
  }, [scheduleCommandLogRender]);

  const appendPingLines = useCallback((lines: string[]) => {
    if (!lines.length) return;
    const buffer = pingLogLinesRef.current;
    buffer.push(...lines);
    if (buffer.length > MAX_LOG_LINES) {
      buffer.splice(0, buffer.length - MAX_LOG_LINES);
    }
    schedulePingLogRender();
  }, [schedulePingLogRender]);

  const appendPingLine = useCallback((line: string) => {
    appendPingLines([line]);
  }, [appendPingLines]);

  const appendCommandOutput = useCallback((title: string, output: string) => {
    const stamp = new Date().toLocaleTimeString("en-GB");
    const cleanOutput = output?.trim() ? output.trim() : "(No output returned)";
    const lines = [`[${stamp}] ${title}`, ...cleanOutput.split(/\r?\n/), ""];
    appendCommandLines(lines);
  }, [appendCommandLines]);

  const executeNetCmd = useCallback(async (
    cmd: string,
    title: string,
    options?: { refresh?: boolean }
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
        loadData();
      }
    } catch (err) {
      appendCommandOutput(title, `Error: ${err}`);
      setStatusMsg(`Error: ${err}`);
    }
  }, [appendCommandOutput, loadData]);

  const handleShowRoutingOutput = useCallback(async () => {
    setDiagnosticsOpen(true);
    setDiagnosticView("routing");
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
  }, []);

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

  const loadBatteryReport = useCallback(async () => {
    setBatteryLoading(true);
    setBatteryReportError("");
    try {
      const report = await getBatteryReport();
      setBatteryReportHtml(report.html);
      setStatusMsg("Battery report loaded");
    } catch (err) {
      setBatteryReportHtml("");
      setBatteryReportError(String(err));
      setStatusMsg(`Battery report error: ${err}`);
    } finally {
      setBatteryLoading(false);
    }
  }, []);

  const handleOpenBatteryModal = useCallback(() => {
    setBatteryModalOpen(true);
    void loadBatteryReport();
  }, [loadBatteryReport]);

  const handleCloseBatteryModal = useCallback(() => {
    if (batteryLoading) return;
    setBatteryModalOpen(false);
  }, [batteryLoading]);

  const handleResetWinHttpProxy = async () => {
    openConfirm(
      "Reset WinHTTP Proxy",
      "Reset WinHTTP proxy settings to direct access?",
      () => executeNetCmd("netsh winhttp reset proxy", "Reset WinHTTP Proxy", { refresh: true })
    );
  };

  const handleRestartAdapters = async () => {
    openConfirm(
      "Restart Active Adapters",
      "Restart active physical network adapters now?",
      () => executeNetCmd(
        "powershell -NoProfile -Command Get-NetAdapter -Physical ^| Where-Object {$_.Status -eq 'Up'} ^| Restart-NetAdapter -Confirm:$false",
        "Restart Active Adapters",
        { refresh: true }
      )
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
    await executeNetCmd(
      `powershell -NoProfile -Command Test-NetConnection -ComputerName ${host} -Port ${port}`,
      `Port Test ${host}:${port}`
    );
  }, [diagHost, diagPort, executeNetCmd, sanitizeHostToken]);

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
    setIpScanProgressPercent(0);
    setIpScanProgressText(`Starting scan on ${plan.subnetLabel}...`);
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
        setIpScanProgressPercent(percent);
        setIpScanProgressText(
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
  }, [ipScanRunning]);

  const handleOpenIpScanModal = useCallback(() => {
    const plan = resolveIpScanPlan();
    if (!plan) {
      setStatusMsg("Select an active NIC first to scan subnet hosts");
      return;
    }
    setIpScanPlan(plan);
    setIpScanModalOpen(true);
    setIpScanResults([]);
    setIpScanProgressPercent(0);
    if (plan.source === "fallback") {
      setIpScanProgressText(`Using fallback ${plan.subnetLabel} range from selected NIC.`);
    } else {
      setIpScanProgressText(`Ready to scan ${plan.targets.length} hosts on ${plan.subnetLabel}.`);
    }
  }, [resolveIpScanPlan]);

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
    setIpScanModalOpen(false);
  }, [ipScanRunning]);

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
    setRemoveProgressPercent(0);
    setRemoveProgressText("Ready.");
    setBloatwareModalOpen(true);
    void loadBloatwareList();
  }, [loadBloatwareList]);

  const handleCloseBloatwareModal = useCallback(() => {
    if (bloatwareRemoving) return;
    setBloatwareModalOpen(false);
  }, [bloatwareRemoving]);

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
    const packages = Array.from(selectedBloatware);
    if (!packages.length) {
      setStatusMsg("Select at least one app to remove");
      return;
    }

    setBloatwareRemoving(true);
    setDiagnosticView("command");
    setDiagnosticsOpen(true);
    setRemoveProgressPercent(0);
    setRemoveProgressText(`Starting removal... 0/${packages.length} (0%)`);
    setStatusMsg(`Removing ${packages.length} selected app(s)...`);
    let successCount = 0;
    let failedCount = 0;
    try {
      for (let index = 0; index < packages.length; index += 1) {
        const packageName = packages[index];
        const appLabel = bloatwareItems.find((item) => item.package_name === packageName)?.label ?? packageName;
        const beforePercent = Math.round((index / packages.length) * 100);
        setRemoveProgressPercent(beforePercent);
        setRemoveProgressText(`Removing ${appLabel}... ${index}/${packages.length} (${beforePercent}%)`);

        try {
          const result = await removeBloatware([packageName]);
          appendCommandOutput(`Remove Apps - ${appLabel}`, result.output);
          if (result.success) {
            successCount += 1;
          } else {
            failedCount += 1;
          }
        } catch (err) {
          failedCount += 1;
          appendCommandOutput(`Remove Apps - ${appLabel}`, `Error: ${err}`);
        }

        const processed = index + 1;
        const percent = Math.round((processed / packages.length) * 100);
        setRemoveProgressPercent(percent);
        setRemoveProgressText(`Processed ${processed}/${packages.length} (${percent}%)`);
      }

      setStatusMsg(
        failedCount === 0
          ? `Remove Apps completed (${successCount}/${packages.length})`
          : `Remove Apps completed with warnings (${failedCount} failed)`
      );
      setRemoveProgressText(`Done: ${successCount} success, ${failedCount} failed`);
      setSelectedBloatware(new Set());
      await loadBloatwareList();
    } catch (err) {
      appendCommandOutput("Remove Apps", `Error: ${err}`);
      setStatusMsg(`Remove Apps error: ${err}`);
      setRemoveProgressText("Removal aborted by error.");
    } finally {
      setBloatwareRemoving(false);
    }
  }, [appendCommandOutput, bloatwareItems, loadBloatwareList, selectedBloatware]);

  const handleOpenCacheModal = useCallback(() => {
    setSelectedCaches(new Set(DEFAULT_CACHE_SELECTION));
    setCacheProgressPercent(0);
    setCacheProgressText("Ready.");
    setCacheStopPending(false);
    cacheStopRequestedRef.current = false;
    setCacheModalOpen(true);
  }, []);

  const handleCloseCacheModal = useCallback(() => {
    if (cacheCleaning) return;
    setCacheModalOpen(false);
  }, [cacheCleaning]);

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

    setCacheCleaning(true);
    setCacheStopPending(false);
    cacheStopRequestedRef.current = false;
    setDiagnosticView("command");
    setDiagnosticsOpen(true);
    setCacheProgressPercent(0);
    setCacheProgressText(`Starting cleanup... 0/${selectedCacheTargets.length} (0%)`);
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
        setCacheProgressPercent(beforePercent);
        setCacheProgressText(
          `Cleaning ${target.label}... ${index}/${selectedCacheTargets.length} (${beforePercent}%)`
        );

        try {
          const result = await clearCacheTargets([target.id]);
          appendCommandOutput(`Clear Cache - ${target.label}`, result.output);
          if (result.success) {
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
        setCacheProgressPercent(percent);
        setCacheProgressText(
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
  }, [appendCommandOutput, selectedCacheTargets]);

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

  const commandOutputText = useMemo(
    () => commandLogLinesRef.current.join("\n"),
    [commandLogVersion]
  );
  const pingOutputText = useMemo(
    () => pingLogLinesRef.current.join("\n"),
    [pingLogVersion]
  );
  const diagnosticsOutputText = diagnosticView === "routing"
    ? (routingOutput || "Routing table output will appear here.")
    : (commandOutputText || "Command output will appear here.");
  const installedBloatwareCount = useMemo(
    () => bloatwareItems.filter((item) => item.installed).length,
    [bloatwareItems]
  );
  const ipScanReachableCount = useMemo(
    () => ipScanResults.filter((item) => item.success).length,
    [ipScanResults]
  );
  const ipScanDisplayRows = useMemo(
    () =>
      [...ipScanResults].sort((left, right) => {
        if (left.success !== right.success) {
          return left.success ? -1 : 1;
        }
        return left.target.localeCompare(right.target);
      }),
    [ipScanResults]
  );
  const ipScanScannedCount = ipScanResults.length;
  const selectedBloatwareCount = selectedBloatware.size;
  const selectedCacheCount = selectedCacheTargets.length;

  useEffect(() => {
    localStorage.setItem("ui-theme", theme);
  }, [theme]);

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
      if (pingRenderRafRef.current !== null) {
        window.cancelAnimationFrame(pingRenderRafRef.current);
      }
      if (commandRenderRafRef.current !== null) {
        window.cancelAnimationFrame(commandRenderRafRef.current);
      }
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
              SuperRoute Pro V.{APP_VERSION} | Author {APP_AUTHOR}
            </p>
          </div>
        </div>

        <div className="flex items-center gap-2">
          <button
            onClick={handleOpenBloatwareModal}
            disabled={bloatwareLoading || bloatwareRemoving}
            className="header-apps-action capsule-btn"
            title="Open app removal tools"
          >
            <Trash2 className="w-3.5 h-3.5" />
            Remove Apps
          </button>

          <button
            onClick={handleOpenCacheModal}
            disabled={cacheCleaning}
            className="header-cache-action capsule-btn"
            title="Open cache cleanup tools"
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
                  onClick={loadData}
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
                  {nics.length === 0 && (
                    <tr><td colSpan={4} className="text-center text-slate-600 py-4">No interfaces found</td></tr>
                  )}
                </tbody>
              </table>
            </div>
          </div>

          {/* Config Form */}
          <div className="p-3 border-b border-slate-700/30">
            <div className="grid grid-cols-2 gap-2 mb-2">
              <Field label="Destination" value={formDest} onChange={setFormDest} placeholder="10.0.0.0" />
              <Field label="Subnet Mask" value={formMask} onChange={setFormMask} placeholder="255.255.255.0" />
              <Field label="Gateway" value={formGw} onChange={setFormGw} placeholder="192.168.1.1" />
              <Field label="Metric" value={formMetric} onChange={setFormMetric} placeholder="10" />
            </div>
            <div className="flex flex-wrap gap-1.5">
              <ActionBtn icon={Plus} label="ADD" color="emerald" onClick={handleAddRoute} />
              <ActionBtn icon={Trash2} label="DEL" color="red" onClick={handleDeleteRoute} />
              <ActionBtn
                icon={Globe}
                label="WAN"
                color="blue"
                onClick={() => openConfirm(
                  "Set Default Gateway",
                  `Route all traffic through ${selectedNic?.description ?? "selected NIC"}?`,
                  executeSetInternet
                )}
              />
              <ActionBtn
                icon={Flame}
                label="FLUSH"
                color="orange"
                onClick={() => openConfirm(
                  "Clear All Routes",
                  "Clear ALL routes? This action is dangerous.",
                  executeFlush
                )}
              />
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
                onClick={() => executeNetCmd("ipconfig /flushdns", "Flush DNS")} tone="safe" />
              <ToolBtn icon={RefreshCw} label="Renew IP" desc="Release and renew DHCP"
                onClick={() => executeNetCmd("ipconfig /release && ipconfig /renew", "Renew IP", { refresh: true })} tone="safe" />
              <ToolBtn icon={Wifi} label="Wi-Fi Info" desc="Show WLAN interface details"
                onClick={() => executeNetCmd("netsh wlan show interface", "Wi-Fi Info")} tone="system" />
              <ToolBtn icon={Trash2} label="Clear ARP" desc="Flush ARP cache"
                onClick={() => executeNetCmd("netsh interface ip delete arpcache", "Clear ARP", { refresh: true })} tone="system" />
              <ToolBtn icon={Globe} label="Reset TCP/IP" desc="Reset network stack"
                onClick={() => executeNetCmd("netsh int ip reset", "Reset TCP/IP", { refresh: true })} tone="danger" />
              <ToolBtn icon={OctagonAlert} label="Reset Winsock" desc="Reset socket catalog"
                onClick={() => executeNetCmd("netsh winsock reset", "Reset Winsock", { refresh: true })} tone="danger" />
              <ToolBtn icon={Flame} label="Reset Firewall" desc="Reset firewall to defaults"
                onClick={() => executeNetCmd("netsh advfirewall reset", "Reset Firewall", { refresh: true })} tone="danger" />
              <ToolBtn icon={Monitor} label="Battery Info" desc="Open battery report in app"
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
                onClick={handleResetWinHttpProxy} tone="system" compact />
              <ToolBtn icon={RefreshCw} label="Restart Adapters" desc="Restart active adapters"
                onClick={handleRestartAdapters} tone="system" compact />
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
        </div>
      </div>

      {/* ====== FOOTER ====== */}
      <footer className="app-footer flex items-center justify-between px-5 py-1.5 border-t shrink-0">
        <span className="text-[0.65rem] text-slate-500">{statusMsg}</span>
        <span className="version-text text-[0.85rem] font-semibold">SuperRoute Pro V.{APP_VERSION} | Author {APP_AUTHOR}</span>
      </footer>

      {batteryModalOpen && (
        <div className="fixed inset-0 z-50 bg-slate-950/60 flex items-center justify-center px-4">
          <div className="battery-modal">
            <div className="battery-modal-header">
              <div>
                <h3 className="text-base font-bold text-slate-100">Battery Info</h3>
                <p className="text-xs text-slate-400 mt-0.5">
                  View battery report and current battery health in-app.
                </p>
              </div>
              <div className="flex items-center gap-2">
                <button
                  onClick={() => void loadBatteryReport()}
                  disabled={batteryLoading}
                  className="capsule-btn compact-pill battery-refresh-btn"
                >
                  {batteryLoading ? "Loading..." : "Refresh"}
                </button>
                <button
                  onClick={handleCloseBatteryModal}
                  disabled={batteryLoading}
                  className="battery-close-btn capsule-btn"
                  title="Close"
                >
                  <X className="w-4 h-4" />
                </button>
              </div>
            </div>
            <div className="battery-modal-body">
              {batteryLoading ? (
                <div className="battery-placeholder">Generating battery report...</div>
              ) : batteryReportError ? (
                <div className="battery-placeholder battery-placeholder-error">
                  Unable to load battery report: {batteryReportError}
                </div>
              ) : batteryReportHtml ? (
                <div className="battery-report-shell">
                  <iframe
                    title="Battery Report"
                    className="battery-report-frame"
                    srcDoc={batteryReportHtml}
                  />
                </div>
              ) : (
                <div className="battery-placeholder">No battery report available.</div>
              )}
            </div>
          </div>
        </div>
      )}

      {ipScanModalOpen && (
        <div className="fixed inset-0 z-50 bg-slate-950/60 flex items-center justify-center px-4">
          <div className="scan-ip-modal">
            <div className="scan-ip-modal-header">
              <div>
                <h3 className="text-base font-bold text-slate-100">Scan IP</h3>
                <p className="text-xs text-slate-400 mt-0.5">
                  Scan active hosts in the selected interface subnet.
                </p>
                {ipScanPlan && (
                  <p className="scan-ip-subtitle">
                    NIC {selectedNic?.index ?? "-"} | {selectedNic?.ip ?? "-"} | {ipScanPlan.subnetLabel} | {ipScanPlan.targets.length} targets
                  </p>
                )}
              </div>
              <button
                onClick={handleCloseIpScanModal}
                disabled={ipScanRunning}
                className="scan-ip-close-btn capsule-btn"
                title="Close"
              >
                <X className="w-4 h-4" />
              </button>
            </div>

            <div className="scan-ip-toolbar">
              <span>
                {ipScanScannedCount} scanned | {ipScanReachableCount} reachable
              </span>
              {ipScanPlan?.truncated && (
                <span className="scan-ip-truncated-note">
                  Target list limited to {ipScanPlan.targets.length} hosts
                </span>
              )}
            </div>

            <div className="scan-ip-table-shell">
              {ipScanDisplayRows.length === 0 ? (
                <div className="scan-ip-empty">
                  {ipScanRunning ? "Scanning hosts..." : "No scan results yet. Click Start Scan."}
                </div>
              ) : (
                <table className="scan-ip-table">
                  <thead>
                    <tr>
                      <th className="w-12">#</th>
                      <th>Host</th>
                      <th className="w-28">Status</th>
                      <th className="w-24">Latency</th>
                    </tr>
                  </thead>
                  <tbody>
                    {ipScanDisplayRows.map((host, index) => (
                      <tr key={`${host.target}-${index}`}>
                        <td className="font-mono">{index + 1}</td>
                        <td className="font-mono">{host.target}</td>
                        <td>
                          <span className={`scan-ip-status-chip ${host.success ? "scan-ip-status-up" : "scan-ip-status-down"}`}>
                            {host.success ? "Reachable" : "Timeout"}
                          </span>
                        </td>
                        <td className="font-mono">{host.success ? `${host.latency_ms} ms` : "-"}</td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              )}
            </div>

            <div className="cache-progress-panel">
              <div className="cache-progress-track">
                <div
                  className="cache-progress-fill"
                  style={{ width: `${ipScanProgressPercent}%` }}
                />
                <span className="cache-progress-value">{ipScanProgressPercent}%</span>
              </div>
              <div className="cache-progress-text">
                {ipScanRunning
                  ? ipScanProgressText
                  : ipScanProgressPercent > 0
                    ? ipScanProgressText
                    : ipScanPlan
                      ? `Ready. ${ipScanPlan.targets.length} host target(s).`
                      : "Ready."}
              </div>
            </div>

            <div className="scan-ip-modal-footer">
              <button
                onClick={handleStartIpScan}
                disabled={ipScanRunning || !ipScanPlan}
                className="capsule-btn compact-pill cache-tool-btn"
              >
                {ipScanScannedCount > 0 ? "Rescan" : "Start Scan"}
              </button>
              <div className="flex items-center gap-2">
                {ipScanRunning && (
                  <button
                    onClick={handleForceStopIpScan}
                    disabled={ipScanStopPending}
                    className="cache-force-stop-btn capsule-btn px-3 py-1.5 transition"
                  >
                    {ipScanStopPending ? "Stopping..." : "Force Stop"}
                  </button>
                )}
                <button
                  onClick={handleCloseIpScanModal}
                  disabled={ipScanRunning}
                  className="cache-footer-close-btn capsule-btn px-3 py-1.5 transition"
                >
                  Close
                </button>
              </div>
            </div>
          </div>
        </div>
      )}

      {cacheModalOpen && (
        <div className="fixed inset-0 z-50 bg-slate-950/60 flex items-center justify-center px-4">
          <div className="cache-modal">
            <div className="cache-modal-header">
              <div>
                <h3 className="text-base font-bold text-slate-100">Clear Cache</h3>
                <p className="text-xs text-slate-400 mt-0.5">
                  Select cache targets, then click Start Cleanup.
                </p>
              </div>
              <button
                onClick={handleCloseCacheModal}
                disabled={cacheCleaning}
                className="cache-close-btn capsule-btn"
                title="Close"
              >
                <X className="w-4 h-4" />
              </button>
            </div>

            <div className="cache-options-grid">
              {CACHE_CLEANUP_OPTIONS.map((option) => (
                <label key={option.id} className="cache-option-item">
                  <input
                    type="checkbox"
                    checked={selectedCaches.has(option.id)}
                    onChange={() => handleToggleCache(option.id)}
                    disabled={cacheCleaning}
                    className="w-3.5 h-3.5 rounded accent-blue-500"
                  />
                  <div className="min-w-0">
                    <div className="cache-option-title">{option.label}</div>
                    <div className="cache-option-desc">{option.description}</div>
                  </div>
                </label>
              ))}
            </div>

            <div className="cache-progress-panel">
              <div className="cache-progress-track">
                <div
                  className="cache-progress-fill"
                  style={{ width: `${cacheProgressPercent}%` }}
                />
                <span className="cache-progress-value">{cacheProgressPercent}%</span>
              </div>
              <div className="cache-progress-text">
                {cacheCleaning
                  ? cacheProgressText
                  : cacheProgressPercent > 0
                    ? cacheProgressText
                    : `Ready. ${selectedCacheCount} cache target(s) selected.`}
              </div>
            </div>

            <div className="cache-modal-footer">
              <div className="flex items-center gap-2">
                <button
                  onClick={handleSelectAllCaches}
                  disabled={cacheCleaning}
                  className="capsule-btn compact-pill cache-tool-btn"
                >
                  Select All
                </button>
                <button
                  onClick={handleClearCacheSelection}
                  disabled={cacheCleaning}
                  className="capsule-btn compact-pill cache-tool-btn"
                >
                  Clear Selection
                </button>
              </div>

              <div className="flex items-center gap-2">
                {cacheCleaning && (
                  <button
                    onClick={handleForceStopCacheCleanup}
                    disabled={cacheStopPending}
                    className="cache-force-stop-btn capsule-btn px-3 py-1.5 transition"
                  >
                    {cacheStopPending ? "Stopping..." : "Force Stop"}
                  </button>
                )}
                <button
                  onClick={handleCloseCacheModal}
                  disabled={cacheCleaning}
                  className="cache-footer-close-btn capsule-btn px-3 py-1.5 transition"
                >
                  Close
                </button>
                <button
                  onClick={() => {
                    if (selectedCacheCount === 0) {
                      setStatusMsg("Select at least one cache target");
                      return;
                    }
                    openConfirm(
                      "Start Cache Cleanup",
                      `Clean ${selectedCacheCount} selected cache target(s)?`,
                      executeClearSelectedCaches
                    );
                  }}
                  disabled={cacheCleaning || selectedCacheCount === 0}
                  className="capsule-btn px-3 py-1.5 border border-amber-400/60 bg-amber-600/90 hover:bg-amber-500 text-white transition"
                >
                  {cacheCleaning ? "Cleaning..." : `Start Cleanup (${selectedCacheCount})`}
                </button>
              </div>
            </div>
          </div>
        </div>
      )}

      {bloatwareModalOpen && (
        <div className="fixed inset-0 z-50 bg-slate-950/60 flex items-center justify-center px-4">
          <div className="bloatware-modal">
            <div className="bloatware-modal-header">
              <div>
                <h3 className="text-base font-bold text-slate-100">Remove Apps</h3>
                <p className="text-xs text-slate-400 mt-0.5">
                  Select built-in Windows apps, then remove selected packages.
                </p>
              </div>
              <button
                onClick={handleCloseBloatwareModal}
                disabled={bloatwareRemoving}
                className="bloatware-close-btn capsule-btn"
                title="Close"
              >
                <X className="w-4 h-4" />
              </button>
            </div>

            <div className="bloatware-toolbar">
              <div className="flex items-center gap-2">
                <button
                  onClick={handleSelectAllBloatware}
                  disabled={bloatwareLoading || bloatwareRemoving}
                  className="capsule-btn compact-pill bloatware-tool-btn"
                >
                  Select All
                </button>
                <button
                  onClick={handleSelectInstalledBloatware}
                  disabled={bloatwareLoading || bloatwareRemoving}
                  className="capsule-btn compact-pill bloatware-tool-btn"
                >
                  Select Installed
                </button>
                <button
                  onClick={handleClearBloatwareSelection}
                  disabled={bloatwareLoading || bloatwareRemoving}
                  className="capsule-btn compact-pill bloatware-tool-btn"
                >
                  Clear Selection
                </button>
              </div>
              <span className="text-[0.72rem] text-slate-400">
                {selectedBloatwareCount} selected | {installedBloatwareCount} installed
              </span>
            </div>

            <div className="bloatware-table-shell">
              {bloatwareLoading ? (
                <div className="bloatware-empty">Loading bloatware catalog...</div>
              ) : bloatwareItems.length === 0 ? (
                <div className="bloatware-empty">No bloatware candidates available.</div>
              ) : (
                <table className="bloatware-table">
                  <thead>
                    <tr>
                      <th className="w-14">Pick</th>
                      <th className="w-48">Application</th>
                      <th>Package Name</th>
                      <th className="w-28">Status</th>
                    </tr>
                  </thead>
                  <tbody>
                    {bloatwareItems.map((item) => (
                      <tr key={item.package_name} className={!item.installed ? "bloatware-row-disabled" : ""}>
                        <td>
                          <input
                            type="checkbox"
                            checked={selectedBloatware.has(item.package_name)}
                            onChange={() => handleToggleBloatware(item.package_name)}
                            disabled={bloatwareRemoving}
                            className="w-3.5 h-3.5 rounded accent-blue-500"
                          />
                        </td>
                        <td className="font-semibold">{item.label}</td>
                        <td className="font-mono text-[0.7rem] text-slate-300">{item.package_name}</td>
                        <td>
                          <span className={`bloatware-status-chip ${item.installed ? "bloatware-status-installed" : "bloatware-status-missing"}`}>
                            {item.installed ? "Installed" : "Not installed"}
                          </span>
                        </td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              )}
            </div>

            <div className="remove-progress-panel">
              <div className="remove-progress-track">
                <div
                  className="remove-progress-fill"
                  style={{ width: `${removeProgressPercent}%` }}
                />
                <span className="remove-progress-value">{removeProgressPercent}%</span>
              </div>
              <div className="remove-progress-text">
                {bloatwareRemoving
                  ? removeProgressText
                  : removeProgressPercent > 0
                    ? removeProgressText
                    : `Ready. ${selectedBloatwareCount} app(s) selected.`}
              </div>
            </div>

            <div className="bloatware-modal-footer">
              <button
                onClick={handleCloseBloatwareModal}
                disabled={bloatwareRemoving}
                className="bloatware-footer-close-btn capsule-btn px-3 py-1.5 transition"
              >
                Close
              </button>
              <button
                onClick={() => {
                  if (selectedBloatwareCount === 0) {
                    setStatusMsg("Select at least one app to remove");
                    return;
                  }
                  openConfirm(
                    "Remove Selected Apps",
                    `Remove ${selectedBloatwareCount} selected app(s)? This operation may require Administrator privileges.`,
                    executeRemoveSelectedBloatware
                  );
                }}
                disabled={bloatwareRemoving || selectedBloatwareCount === 0 || bloatwareLoading}
                className="capsule-btn px-3 py-1.5 border border-rose-400/60 bg-rose-600/85 hover:bg-rose-500 text-white transition"
              >
                {bloatwareRemoving ? "Removing..." : `Remove Selected (${selectedBloatwareCount})`}
              </button>
            </div>
          </div>
        </div>
      )}

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
      <div className={`theme-lens ${themeLensActive ? "theme-lens-active" : ""}`} />
    </div>
  );
}

// ======================== SUBCOMPONENTS ========================

const OutputConsole = memo(function OutputConsole({
  diagnosticView,
  routesCount,
  diagnosticsOutputText,
  pingOutputText,
  commandOutputRef,
  pingOutputRef,
  onShowCommand,
  onShowRouting,
  onClearCommand,
  onClearPing,
}: {
  diagnosticView: "command" | "routing";
  routesCount: number;
  diagnosticsOutputText: string;
  pingOutputText: string;
  commandOutputRef: React.RefObject<HTMLPreElement | null>;
  pingOutputRef: React.RefObject<HTMLPreElement | null>;
  onShowCommand: () => void;
  onShowRouting: () => void;
  onClearCommand: () => void;
  onClearPing: () => void;
}) {
  return (
    <div className="flex flex-col flex-1 p-3 overflow-hidden">
      <div className="flex items-center gap-2 mb-2">
        <Activity className="w-4 h-4 text-blue-400" />
        <span className="text-xs font-bold text-slate-400 uppercase tracking-wider">Output Console</span>
        <span className="text-[0.62rem] text-slate-600 ml-auto">
          {diagnosticView === "routing" ? `${routesCount} routes snapshot` : "Command + Ping live logs"}
        </span>
      </div>

      <div className="grid grid-rows-[1fr_1fr] gap-2 flex-1 min-h-0">
        <div className="min-h-0 flex flex-col">
          <div className="flex items-center justify-between mb-1 gap-2">
            <span className="text-[0.72rem] text-slate-400 uppercase tracking-wider font-semibold">
              {diagnosticView === "routing" ? "Routing Table Output" : "Command Output"}
            </span>
            <div className="flex items-center gap-1.5">
              <button
                onClick={onShowCommand}
                className={`capsule-btn compact-pill console-chip console-chip-command ${
                  diagnosticView === "command" ? "console-chip-command-active" : ""
                }`}
              >
                Command
              </button>
              <button
                onClick={onShowRouting}
                className={`capsule-btn compact-pill console-chip console-chip-routing ${
                  diagnosticView === "routing" ? "console-chip-routing-active" : ""
                }`}
              >
                Routing
              </button>
              <button
                onClick={diagnosticView === "routing" ? onShowRouting : onClearCommand}
                className="capsule-btn compact-pill console-chip console-chip-refresh"
              >
                {diagnosticView === "routing" ? "Refresh" : "Clear"}
              </button>
            </div>
          </div>
          <pre
            ref={commandOutputRef}
            className="text-[0.76rem] font-mono bg-[#0c1220] border border-slate-700/50 rounded-xl p-3 flex-1 min-h-0 overflow-auto text-slate-300 whitespace-pre-wrap"
          >
            {diagnosticsOutputText}
          </pre>
        </div>

        <div className="min-h-0 flex flex-col">
          <div className="flex items-center justify-between mb-1 gap-2">
            <span className="text-[0.72rem] text-slate-400 uppercase tracking-wider font-semibold">
              Ping & Tracert Output
            </span>
            <button
              onClick={onClearPing}
              className="capsule-btn compact-pill bg-slate-700/60 hover:bg-slate-600/60 text-slate-200 border-slate-600 transition"
            >
              Clear
            </button>
          </div>
          <pre
            ref={pingOutputRef}
            className="text-[0.8rem] font-mono bg-[#0c1220] border border-slate-700/50 rounded-xl p-3 flex-1 min-h-0 overflow-auto text-slate-300 whitespace-pre-wrap"
          >
            {pingOutputText || "Ping log is ready. Click Start to run continuous ping."}
          </pre>
        </div>
      </div>
    </div>
  );
});

const Field = memo(function Field({ label, value, onChange, placeholder }: {
  label: string; value: string; onChange: (v: string) => void; placeholder?: string;
}) {
  return (
    <div>
      <label className="text-[0.6rem] text-slate-500 uppercase tracking-wider font-bold">{label}</label>
      <input
        type="text"
        value={value}
        onChange={(e) => onChange(e.target.value)}
        placeholder={placeholder}
        className="w-full mt-0.5 px-2.5 py-1.5 text-xs font-mono bg-[#0c1220] border border-slate-700/50 rounded-md focus:border-blue-500/50 focus:outline-none text-slate-200 placeholder:text-slate-700"
      />
    </div>
  );
});

const ActionBtn = memo(function ActionBtn({ icon: Icon, label, color, onClick }: {
  icon: React.ElementType; label: string; color: string; onClick: () => void;
}) {
  const colors: Record<string, string> = {
    emerald: "bg-emerald-600/80 hover:bg-emerald-500 border-emerald-700/50",
    red: "bg-red-600/80 hover:bg-red-500 border-red-700/50",
    blue: "bg-blue-600/80 hover:bg-blue-500 border-blue-700/50",
    orange: "bg-orange-600/80 hover:bg-orange-500 border-orange-700/50",
    slate: "bg-slate-700/80 hover:bg-slate-600 border-slate-600/70",
  };
  return (
    <button
      onClick={onClick}
      className={`capsule-btn min-w-[72px] px-2.5 flex items-center justify-center gap-1.5 py-1.5 text-[0.76rem] font-bold text-white border transition ${colors[color] || colors.blue}`}
    >
      <Icon className="w-3.5 h-3.5" /> {label}
    </button>
  );
});

const Section = memo(function Section({ icon: Icon, title, open, onToggle, children }: {
  icon: React.ElementType; title: string; open: boolean; onToggle: () => void; children: React.ReactNode;
}) {
  return (
    <div className="bg-[#1e293b]/50 border border-slate-700/30 rounded-xl overflow-hidden">
      <button
        onClick={onToggle}
        className="capsule-btn-soft flex items-center justify-between w-full px-4 py-3 hover:bg-slate-700/20 transition"
      >
        <div className="flex items-center gap-2">
          <Icon className="w-4 h-4 text-blue-400" />
          <span className="text-sm font-bold text-slate-300">{title}</span>
        </div>
        {open ? <ChevronUp className="w-4 h-4 text-slate-500" /> : <ChevronDown className="w-4 h-4 text-slate-500" />}
      </button>
      {open && <div className="px-4 pb-4">{children}</div>}
    </div>
  );
});

const ToolBtn = memo(function ToolBtn({ icon: Icon, label, desc, onClick, tone, compact }: {
  icon: React.ElementType; label: string; desc: string; onClick: () => void; tone?: "safe" | "system" | "danger"; compact?: boolean;
}) {
  const toneClass = tone ?? "safe";
  return (
    <button
      onClick={onClick}
      className={`tool-card tool-card-${toneClass} ${compact ? "tool-card-compact" : ""}`}
    >
      <span className="tool-icon-shell">
        <Icon className="w-3.5 h-3.5" />
      </span>
      <div className="min-w-0">
        <div className="tool-title">{label}</div>
        <div className="tool-desc">{desc}</div>
      </div>
    </button>
  );
});








