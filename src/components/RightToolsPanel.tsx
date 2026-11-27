import {
  Activity,
  ArrowDownUp,
  Flame,
  Globe,
  Monitor,
  OctagonAlert,
  RefreshCw,
  Search,
  Send,
  Trash2,
  Wifi,
  Wrench,
  Zap,
} from "lucide-react";

import type { RepairMachineAction } from "../api";
import { SpeedTestModal } from "../SpeedTestModal";
import { Section, ToolBtn } from "./AppChrome";

interface RightToolsPanelProps {
  toolsOpen: boolean;
  diagnosticsOpen: boolean;
  pingOpen: boolean;
  machineRepairEnabled: boolean;
  diagnosticsHost: string;
  diagnosticsPort: string;
  diagnosticsDnsServer: string;
  pingMode: "ping" | "fping";
  pingTarget: string;
  pingRunning: boolean;
  onToggleToolsPanel: () => void;
  onToggleDiagnosticsPanel: () => void;
  onTogglePingPanel: () => void;
  onExecuteRepairAction: (
    action: RepairMachineAction,
    title: string,
    options?: { refresh?: boolean; invalidateNicCache?: boolean },
  ) => void;
  onExecuteNetCmd: (cmd: string, title: string) => void;
  onOpenBatteryModal: () => void;
  onDisplayDnsCache: () => void;
  onResetWinHttpProxy: () => void;
  onRestartAdapters: () => void;
  onOpenIpScanModal: () => void;
  onDiagHostChange: (value: string) => void;
  onDiagPortChange: (value: string) => void;
  onDiagDnsServerChange: (value: string) => void;
  onPortConnectivityTest: () => void;
  onNslookupTest: () => void;
  onPingModeChange: (mode: "ping" | "fping") => void;
  onPingTargetChange: (value: string) => void;
  onStartPing: () => void;
  onStopPing: () => void;
  onTracertFromTarget: () => void;
  onStatusChange: (message: string) => void;
}

export function RightToolsPanel({
  toolsOpen,
  diagnosticsOpen,
  pingOpen,
  machineRepairEnabled,
  diagnosticsHost,
  diagnosticsPort,
  diagnosticsDnsServer,
  pingMode,
  pingTarget,
  pingRunning,
  onToggleToolsPanel,
  onToggleDiagnosticsPanel,
  onTogglePingPanel,
  onExecuteRepairAction,
  onExecuteNetCmd,
  onOpenBatteryModal,
  onDisplayDnsCache,
  onResetWinHttpProxy,
  onRestartAdapters,
  onOpenIpScanModal,
  onDiagHostChange,
  onDiagPortChange,
  onDiagDnsServerChange,
  onPortConnectivityTest,
  onNslookupTest,
  onPingModeChange,
  onPingTargetChange,
  onStartPing,
  onStopPing,
  onTracertFromTarget,
  onStatusChange,
}: RightToolsPanelProps) {
  return (
    <div className="right-panel overflow-y-auto p-3 space-y-2.5">
      {/* Network Fix Tools */}
      <Section
        icon={Wrench}
        title="Network Fix Tools"
        open={toolsOpen}
        onToggle={onToggleToolsPanel}
      >
        <div className="tool-grid">
          <ToolBtn
            icon={Zap}
            label="Flush DNS"
            desc="Clear resolver cache"
            onClick={() => onExecuteRepairAction("FlushDns", "Flush DNS")}
            tone="safe"
            disabled={!machineRepairEnabled}
          />
          <ToolBtn
            icon={RefreshCw}
            label="Renew IP"
            desc="Release and renew DHCP"
            onClick={() => onExecuteRepairAction("RenewDhcpLease", "Renew IP", { refresh: true, invalidateNicCache: true })}
            tone="safe"
            disabled={!machineRepairEnabled}
          />
          <ToolBtn
            icon={Wifi}
            label="Wi-Fi Info"
            desc="Show WLAN interface details"
            onClick={() => onExecuteNetCmd("netsh wlan show interface", "Wi-Fi Info")}
            tone="system"
          />
          <ToolBtn
            icon={Trash2}
            label="Clear ARP"
            desc="Flush ARP cache"
            onClick={() => onExecuteRepairAction("ClearArpCache", "Clear ARP", { refresh: true })}
            tone="system"
            disabled={!machineRepairEnabled}
          />
          <ToolBtn
            icon={Globe}
            label="Reset TCP/IP"
            desc="Reset network stack"
            onClick={() => onExecuteRepairAction("ResetTcpIp", "Reset TCP/IP", { refresh: true })}
            tone="danger"
            disabled={!machineRepairEnabled}
          />
          <ToolBtn
            icon={OctagonAlert}
            label="Reset Winsock"
            desc="Reset socket catalog"
            onClick={() => onExecuteRepairAction("ResetWinsock", "Reset Winsock", { refresh: true })}
            tone="danger"
            disabled={!machineRepairEnabled}
          />
          <ToolBtn
            icon={Flame}
            label="Reset Firewall"
            desc="Reset firewall to defaults"
            onClick={() => onExecuteRepairAction("ResetFirewall", "Reset Firewall", { refresh: true })}
            tone="danger"
            disabled={!machineRepairEnabled}
          />
          <ToolBtn
            icon={Monitor}
            label="Battery Info"
            desc="View battery wear and lifetime summary"
            onClick={onOpenBatteryModal}
            tone="system"
          />
        </div>
      </Section>

      <Section
        icon={Monitor}
        title="Diagnostics & Repair"
        open={diagnosticsOpen}
        onToggle={onToggleDiagnosticsPanel}
      >
        <div className="tool-grid mb-2">
          <ToolBtn
            icon={Monitor}
            label="Display DNS Cache"
            desc="Inspect current resolver cache"
            onClick={onDisplayDnsCache}
            tone="safe"
            compact
          />
          <ToolBtn
            icon={Wrench}
            label="Reset WinHTTP Proxy"
            desc="Clear system proxy settings"
            onClick={onResetWinHttpProxy}
            tone="system"
            compact
            disabled={!machineRepairEnabled}
          />
          <ToolBtn
            icon={RefreshCw}
            label="Restart Adapters"
            desc="Restart active adapters"
            onClick={onRestartAdapters}
            tone="system"
            compact
            disabled={!machineRepairEnabled}
          />
          <ToolBtn
            icon={Search}
            label="Scan IP"
            desc="Scan active subnet hosts"
            onClick={onOpenIpScanModal}
            tone="safe"
            compact
          />
        </div>

        <div className="diag-group">
          <div className="diag-inline">
            <input
              type="text"
              value={diagnosticsHost}
              onChange={(e) => onDiagHostChange(e.target.value)}
              placeholder="Domain or IP (e.g. google.com)"
              className="diag-input"
              aria-label="Diagnostic host"
            />
            <input
              type="text"
              value={diagnosticsPort}
              onChange={(e) => onDiagPortChange(e.target.value)}
              placeholder="Port"
              className="diag-input diag-port"
              inputMode="numeric"
              aria-label="Diagnostic port"
            />
            <button
              type="button"
              onClick={onPortConnectivityTest}
              className="diag-action-btn"
            >
              <Activity className="w-3.5 h-3.5" />
              Port Test
            </button>
          </div>
          <div className="diag-inline diag-inline-dns">
            <input
              type="text"
              value={diagnosticsDnsServer}
              onChange={(e) => onDiagDnsServerChange(e.target.value)}
              placeholder="DNS server (e.g. 8.8.8.8)"
              className="diag-input"
              aria-label="Diagnostic DNS server"
            />
            <button
              type="button"
              onClick={onNslookupTest}
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
        onToggle={onTogglePingPanel}
      >
        <div className="segmented-control mb-2">
          <button
            type="button"
            onClick={() => onPingModeChange("ping")}
            className={`segment-btn ${pingMode === "ping" ? "segment-btn-active" : ""}`}
            aria-pressed={pingMode === "ping"}
          >
            Ping
          </button>
          <button
            type="button"
            onClick={() => onPingModeChange("fping")}
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
            onChange={(e) => onPingTargetChange(e.target.value)}
            onKeyDown={(e) => e.key === "Enter" && onStartPing()}
            className="diag-input"
            aria-label="Ping target"
          />
          <button
            type="button"
            onClick={onStartPing}
            disabled={pingRunning}
            className="ping-cmd-btn ping-cmd-start"
          >
            <Send className="w-4 h-4" /> Start
          </button>
          <button
            type="button"
            onClick={onStopPing}
            disabled={!pingRunning}
            className="ping-cmd-btn ping-cmd-stop"
          >
            <OctagonAlert className="w-4 h-4" />
            Stop
          </button>
          <button
            type="button"
            onClick={onTracertFromTarget}
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

      <SpeedTestModal onStatusChange={onStatusChange} />
    </div>
  );
}
