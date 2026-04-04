import { X } from "lucide-react";
import { type CacheCleanupOption } from "../constants/cacheTargets";

interface CacheModalProps {
  open: boolean;
  cleaning: boolean;
  stopPending: boolean;
  options: CacheCleanupOption[];
  selectedCaches: Set<string>;
  selectedCount: number;
  progressPercent: number;
  progressText: string;
  onToggleCache: (cacheId: string) => void;
  onSelectAll: () => void;
  onClearSelection: () => void;
  onForceStop: () => void;
  onStartCleanup: () => void;
  onClose: () => void;
}

export function CacheModal(props: CacheModalProps) {
  if (!props.open) return null;

  return (
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
            type="button"
            onClick={props.onClose}
            disabled={props.cleaning}
            className="cache-close-btn capsule-btn"
            title="Close"
          >
            <X className="w-4 h-4" />
          </button>
        </div>

        <div className="cache-options-grid">
          {props.options.map((option) => (
            <label key={option.id} className="cache-option-item">
              <input
                type="checkbox"
                checked={props.selectedCaches.has(option.id)}
                onChange={() => props.onToggleCache(option.id)}
                disabled={props.cleaning}
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
              style={{ width: `${props.progressPercent}%` }}
            />
            <span className="cache-progress-value">{props.progressPercent}%</span>
          </div>
          <div className="cache-progress-text">
            {props.cleaning
              ? props.progressText
              : props.progressPercent > 0
                ? props.progressText
                : `Ready. ${props.selectedCount} cache target(s) selected.`}
          </div>
        </div>

        <div className="cache-modal-footer">
          <div className="flex items-center gap-2">
            <button
              type="button"
              onClick={props.onSelectAll}
              disabled={props.cleaning}
              className="capsule-btn compact-pill cache-tool-btn"
            >
              Select All
            </button>
            <button
              type="button"
              onClick={props.onClearSelection}
              disabled={props.cleaning}
              className="capsule-btn compact-pill cache-tool-btn"
            >
              Clear Selection
            </button>
          </div>

          <div className="flex items-center gap-2">
            {props.cleaning && (
              <button
                type="button"
                onClick={props.onForceStop}
                disabled={props.stopPending}
                className="cache-force-stop-btn capsule-btn px-3 py-1.5 transition"
              >
                {props.stopPending ? "Stopping..." : "Force Stop"}
              </button>
            )}
            <button
              type="button"
              onClick={props.onClose}
              disabled={props.cleaning}
              className="cache-footer-close-btn capsule-btn px-3 py-1.5 transition"
            >
              Close
            </button>
            <button
              type="button"
              onClick={props.onStartCleanup}
              disabled={props.cleaning || props.selectedCount === 0}
              className="capsule-btn px-3 py-1.5 border border-amber-400/60 bg-amber-600/90 hover:bg-amber-500 text-white transition"
            >
              {props.cleaning ? "Cleaning..." : `Start Cleanup (${props.selectedCount})`}
            </button>
          </div>
        </div>
      </div>
    </div>
  );
}
