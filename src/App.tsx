import { memo, useState, useEffect, useCallback, useRef, useMemo } from "react";
import {
  Zap, Wifi, WifiOff, RefreshCw, Plus, Trash2, Globe, Flame,
  Activity, Send, Wrench, Monitor, Sun, Moon, OctagonAlert, Search,
  ChevronDown, ChevronUp, ArrowDownUp
} from "lucide-react";
import {
  getNetworkInterfaces, getRoutingTable, addRoute, deleteRoute,
  flushRoutes, setDefaultGateway, runNetworkCommand, pingHost,
  fpingScan,
  checkInternet,
  type NetworkInterface, type RouteEntry,
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

export default function App() {
  const APP_VERSION = "3.6.9";
  const APP_AUTHOR = "Zonzon";
  const [theme, setTheme] = useState<"dark" | "light">(() => {
    if (typeof window === "undefined") return "dark";
    return localStorage.getItem("ui-theme") === "light" ? "light" : "dark";
  });

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
  const [themeLensActive, setThemeLensActive] = useState(false);
  const [currentLatency, setCurrentLatency] = useState<number>(0);
  const [toolsOpen, setToolsOpen] = useState(true);
  const [diagnosticsOpen, setDiagnosticsOpen] = useState(true);
  const [pingOpen, setPingOpen] = useState(true);
  const [diagHost, setDiagHost] = useState("google.com");
  const [diagDnsServer, setDiagDnsServer] = useState("8.8.8.8");
  const [diagPort, setDiagPort] = useState("443");
  const [confirmOpen, setConfirmOpen] = useState(false);
  const [confirmTitle, setConfirmTitle] = useState("Confirm");
  const [confirmMessage, setConfirmMessage] = useState("");
  const [diagnosticView, setDiagnosticView] = useState<"command" | "routing">("command");
  const [routingOutput, setRoutingOutput] = useState("");

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
        <div className="flex items-center gap-3">
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

        <div className="flex items-center gap-3">
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

      {confirmOpen && (
        <div className="fixed inset-0 z-50 bg-slate-950/60 flex items-center justify-center px-4">
          <div className="w-full max-w-md rounded-xl border border-slate-600 bg-slate-900 shadow-2xl">
            <div className="flex items-center gap-2 px-4 py-3 border-b border-slate-700">
              <OctagonAlert className="w-4 h-4 text-amber-400" />
              <h3 className="text-sm font-bold text-slate-100">{confirmTitle}</h3>
            </div>
            <div className="px-4 py-4 text-sm text-slate-300">{confirmMessage}</div>
            <div className="flex justify-end gap-2 px-4 py-3 border-t border-slate-700">
              <button
                onClick={onCancelConfirm}
                className="capsule-btn px-3 py-1.5 border border-slate-600 text-slate-300 hover:bg-slate-800 transition"
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








