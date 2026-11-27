import { Monitor, RefreshCw } from "lucide-react";

import type { NetworkInterface } from "../api";

interface NicTablePanelProps {
  activeOnly: boolean;
  loading: boolean;
  nics: NetworkInterface[];
  selectedNic: NetworkInterface | null;
  nicTableMessage: string | null;
  onActiveOnlyChange: (nextValue: boolean) => void;
  onRefresh: () => void;
  onSelectNic: (nic: NetworkInterface) => void;
}

export function NicTablePanel({
  activeOnly,
  loading,
  nics,
  selectedNic,
  nicTableMessage,
  onActiveOnlyChange,
  onRefresh,
  onSelectNic,
}: NicTablePanelProps) {
  return (
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
              onChange={(e) => onActiveOnlyChange(e.target.checked)}
              className="w-3 h-3 rounded accent-blue-500"
            />
            <span className="text-[0.65rem] text-slate-500">Active only</span>
          </label>
          <button
            type="button"
            onClick={onRefresh}
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
                onClick={() => onSelectNic(nic)}
                onKeyDown={(event) => {
                  if (event.key === "Enter" || event.key === " ") {
                    event.preventDefault();
                    onSelectNic(nic);
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
  );
}
