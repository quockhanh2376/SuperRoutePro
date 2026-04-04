import { memo, useMemo } from "react";
import { X } from "lucide-react";
import type { NetworkInterface, FpingHostResult } from "../api";
import type { IpScanPlan } from "../hooks/ipScanPlan";

type IpScanModalProps = {
  open: boolean;
  selectedNic: NetworkInterface | null;
  plan: IpScanPlan | null;
  running: boolean;
  stopPending: boolean;
  results: FpingHostResult[];
  progressPercent: number;
  progressText: string;
  onStart: () => void;
  onForceStop: () => void;
  onClose: () => void;
};

export const IpScanModal = memo(function IpScanModal({
  open,
  selectedNic,
  plan,
  running,
  stopPending,
  results,
  progressPercent,
  progressText,
  onStart,
  onForceStop,
  onClose,
}: IpScanModalProps) {
  const { scannedCount, reachableCount, displayRows } = useMemo(() => {
    const sortedRows = [...results].sort((left, right) => {
      if (left.success !== right.success) {
        return left.success ? -1 : 1;
      }
      return left.target.localeCompare(right.target);
    });

    return {
      scannedCount: results.length,
      reachableCount: results.filter((item) => item.success).length,
      displayRows: sortedRows,
    };
  }, [results]);

  if (!open) {
    return null;
  }

  return (
    <div className="fixed inset-0 z-50 bg-slate-950/60 flex items-center justify-center px-4">
      <div className="scan-ip-modal">
        <div className="scan-ip-modal-header">
          <div>
            <h3 className="text-base font-bold text-slate-100">Scan IP</h3>
            <p className="text-xs text-slate-400 mt-0.5">
              Scan active hosts in the selected interface subnet.
            </p>
            {plan && (
              <p className="scan-ip-subtitle">
                NIC {selectedNic?.index ?? "-"} | {selectedNic?.ip ?? "-"} | {plan.subnetLabel} | {plan.targets.length} targets
              </p>
            )}
          </div>
          <button
            type="button"
            onClick={onClose}
            disabled={running}
            className="scan-ip-close-btn capsule-btn"
            title="Close"
          >
            <X className="w-4 h-4" />
          </button>
        </div>

        <div className="scan-ip-toolbar">
          <span>
            {scannedCount} scanned | {reachableCount} reachable
          </span>
          {plan?.truncated && (
            <span className="scan-ip-truncated-note">
              Target list limited to {plan.targets.length} hosts
            </span>
          )}
        </div>

        <div className="scan-ip-table-shell">
          {displayRows.length === 0 ? (
            <div className="scan-ip-empty">
              {running ? "Scanning hosts..." : "No scan results yet. Click Start Scan."}
            </div>
          ) : (
            <table className="scan-ip-table">
              <thead>
                <tr>
                  <th scope="col" className="w-12">#</th>
                  <th scope="col">Host</th>
                  <th scope="col" className="w-28">Status</th>
                  <th scope="col" className="w-24">Latency</th>
                </tr>
              </thead>
              <tbody>
                {displayRows.map((host, index) => (
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
              style={{ width: `${progressPercent}%` }}
            />
            <span className="cache-progress-value">{progressPercent}%</span>
          </div>
          <div className="cache-progress-text">
            {running
              ? progressText
              : progressPercent > 0
                ? progressText
                : plan
                  ? `Ready. ${plan.targets.length} host target(s).`
                  : "Ready."}
          </div>
        </div>

        <div className="scan-ip-modal-footer">
          <button
            type="button"
            onClick={onStart}
            disabled={running || !plan}
            className="capsule-btn compact-pill cache-tool-btn"
          >
            {scannedCount > 0 ? "Rescan" : "Start Scan"}
          </button>
          <div className="flex items-center gap-2">
            {running && (
              <button
                type="button"
                onClick={onForceStop}
                disabled={stopPending}
                className="cache-force-stop-btn capsule-btn px-3 py-1.5 transition"
              >
                {stopPending ? "Stopping..." : "Force Stop"}
              </button>
            )}
            <button
              type="button"
              onClick={onClose}
              disabled={running}
              className="cache-footer-close-btn capsule-btn px-3 py-1.5 transition"
            >
              Close
            </button>
          </div>
        </div>
      </div>
    </div>
  );
});
