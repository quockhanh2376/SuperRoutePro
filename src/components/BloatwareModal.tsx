import { X } from "lucide-react";
import type { BloatwareItem } from "../api";

interface BloatwareModalProps {
  open: boolean;
  loading: boolean;
  removing: boolean;
  items: BloatwareItem[];
  selectedPackages: Set<string>;
  selectedCount: number;
  installedCount: number;
  progressPercent: number;
  progressText: string;
  onTogglePackage: (packageName: string) => void;
  onSelectAll: () => void;
  onSelectInstalled: () => void;
  onClearSelection: () => void;
  onRemoveSelected: () => void;
  onClose: () => void;
}

export function BloatwareModal(props: BloatwareModalProps) {
  if (!props.open) return null;

  return (
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
            onClick={props.onClose}
            disabled={props.removing}
            className="bloatware-close-btn capsule-btn"
            title="Close"
          >
            <X className="w-4 h-4" />
          </button>
        </div>

        <div className="bloatware-toolbar">
          <div className="flex items-center gap-2">
            <button
              onClick={props.onSelectAll}
              disabled={props.loading || props.removing}
              className="capsule-btn compact-pill bloatware-tool-btn"
            >
              Select All
            </button>
            <button
              onClick={props.onSelectInstalled}
              disabled={props.loading || props.removing}
              className="capsule-btn compact-pill bloatware-tool-btn"
            >
              Select Installed
            </button>
            <button
              onClick={props.onClearSelection}
              disabled={props.loading || props.removing}
              className="capsule-btn compact-pill bloatware-tool-btn"
            >
              Clear Selection
            </button>
          </div>
          <span className="text-[0.72rem] text-slate-400">
            {props.selectedCount} selected | {props.installedCount} installed
          </span>
        </div>

        <div className="bloatware-table-shell">
          {props.loading ? (
            <div className="bloatware-empty">Loading bloatware catalog...</div>
          ) : props.items.length === 0 ? (
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
                {props.items.map((item) => (
                  <tr key={item.package_name} className={!item.installed ? "bloatware-row-disabled" : ""}>
                    <td>
                      <input
                        type="checkbox"
                        checked={props.selectedPackages.has(item.package_name)}
                        onChange={() => props.onTogglePackage(item.package_name)}
                        disabled={props.removing}
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
              style={{ width: `${props.progressPercent}%` }}
            />
            <span className="remove-progress-value">{props.progressPercent}%</span>
          </div>
          <div className="remove-progress-text">
            {props.removing
              ? props.progressText
              : props.progressPercent > 0
                ? props.progressText
                : `Ready. ${props.selectedCount} app(s) selected.`}
          </div>
        </div>

        <div className="bloatware-modal-footer">
          <button
            onClick={props.onClose}
            disabled={props.removing}
            className="bloatware-footer-close-btn capsule-btn px-3 py-1.5 transition"
          >
            Close
          </button>
          <button
            onClick={props.onRemoveSelected}
            disabled={props.removing || props.selectedCount === 0 || props.loading}
            className="capsule-btn px-3 py-1.5 border border-rose-400/60 bg-rose-600/85 hover:bg-rose-500 text-white transition"
          >
            {props.removing ? "Removing..." : `Remove Selected (${props.selectedCount})`}
          </button>
        </div>
      </div>
    </div>
  );
}
