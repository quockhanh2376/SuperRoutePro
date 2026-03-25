import { RefreshCw, X } from "lucide-react";

import type { SpeedTestProgress, SpeedTestResult } from "./api";

const formatMetric = (value: number | null | undefined, suffix: string) => {
  if (value === null || value === undefined || Number.isNaN(value)) {
    return `-- ${suffix}`;
  }
  return `${value.toFixed(1)} ${suffix}`;
};

const getStageLabel = (stage: string) => {
  switch (stage) {
    case "preflight":
      return "Preflight";
    case "latency":
      return "Latency";
    case "download":
      return "Download";
    case "upload":
      return "Upload";
    case "finalize":
      return "Finalize";
    default:
      return "Idle";
  }
};

export type SpeedTestModalDialogProps = {
  error: string;
  isTesting: boolean;
  progress: SpeedTestProgress;
  result: SpeedTestResult | null;
  tauriRuntime: boolean;
  onClose: () => void;
  onStart: () => void;
};

export function SpeedTestModalDialog({
  error,
  isTesting,
  progress,
  result,
  tauriRuntime,
  onClose,
  onStart,
}: SpeedTestModalDialogProps) {
  return (
    <div
      className="speed-test-modal"
      onClick={(event) => event.stopPropagation()}
      role="dialog"
      aria-modal="true"
      aria-label="Speed Test"
    >
      <div className="speed-test-modal-header">
        <div>
          <h3 className="text-base font-bold text-slate-100">Speed Test</h3>
          <p className="text-xs text-slate-400 mt-0.5">
            {tauriRuntime
              ? "Dedicated throughput modal using native backend progress events."
              : "Browser preview mode is active. Start Demo runs a mock flow for UI preview."}
          </p>
        </div>
        <button
          onClick={onClose}
          disabled={isTesting}
          className="speed-test-close-btn capsule-btn"
          title="Close"
        >
          <X className="w-4 h-4" />
        </button>
      </div>

      <div className="speed-test-modal-body">
        {!tauriRuntime && (
          <div className="speed-test-demo-banner">
            Browser demo mode is active. Progress and throughput are mocked here so the UI can be reviewed before the native Tauri runtime is available.
          </div>
        )}

        <div className="speed-test-progress-shell">
          <div className="speed-test-progress-topline">
            <span className="speed-test-stage-chip">{getStageLabel(progress.stage)}</span>
            <span className="speed-test-progress-percent">{Math.round(progress.percent)}%</span>
          </div>
          <div className="speed-test-progress-track">
            <div
              className="speed-test-progress-fill"
              style={{ width: `${Math.min(100, Math.max(0, progress.percent))}%` }}
            />
          </div>
          <div className="speed-test-progress-message">{progress.message}</div>
          <div className="speed-test-live-speed">
            Live speed: <strong>{formatMetric(progress.current_speed_mbps, "Mbps")}</strong>
          </div>
        </div>

        <div className="speed-test-summary-grid">
          <div className="speed-test-summary-card">
            <div className="speed-test-summary-label">Download</div>
            <div className="speed-test-summary-value">
              {result ? formatMetric(result.download_mbps, "Mbps") : "-- Mbps"}
            </div>
          </div>
          <div className="speed-test-summary-card">
            <div className="speed-test-summary-label">Upload</div>
            <div className="speed-test-summary-value">
              {result ? formatMetric(result.upload_mbps, "Mbps") : "-- Mbps"}
            </div>
          </div>
          <div className="speed-test-summary-card">
            <div className="speed-test-summary-label">Ping</div>
            <div className="speed-test-summary-value">
              {result ? formatMetric(result.ping_ms, "ms") : "-- ms"}
            </div>
          </div>
          <div className="speed-test-summary-card">
            <div className="speed-test-summary-label">Jitter</div>
            <div className="speed-test-summary-value">
              {result ? formatMetric(result.jitter_ms, "ms") : "-- ms"}
            </div>
          </div>
        </div>

        <div className="speed-test-meta-grid">
          <div className="speed-test-meta-row">
            <span className="speed-test-meta-label">Provider</span>
            <span className="speed-test-meta-value">{result?.provider ?? "Auto"}</span>
          </div>
          <div className="speed-test-meta-row">
            <span className="speed-test-meta-label">Server</span>
            <span className="speed-test-meta-value">{result?.server_label ?? "Auto"}</span>
          </div>
          <div className="speed-test-meta-row">
            <span className="speed-test-meta-label">Public IP</span>
            <span className="speed-test-meta-value">{result?.ip || "--"}</span>
          </div>
          <div className="speed-test-meta-row">
            <span className="speed-test-meta-label">Timestamp</span>
            <span className="speed-test-meta-value">{result?.timestamp ?? "--"}</span>
          </div>
        </div>

        {error && (
          <div className="speed-test-error-box">
            {error}
          </div>
        )}
      </div>

      <div className="speed-test-modal-footer">
        <div className="speed-test-footer-note">
          {isTesting
            ? tauriRuntime
              ? "Running native network measurement. Keep the modal open until it finishes."
              : "Running browser preview flow. This simulates progress before desktop runtime is available."
            : tauriRuntime
              ? "Results stream live during the run and return as a final snapshot when complete."
              : "Browser demo mode mirrors the modal flow. Native Tauri runtime will replace the mocked measurements."}
        </div>
        <div className="flex items-center gap-2">
          <button
            onClick={onClose}
            disabled={isTesting}
            className="speed-test-footer-close-btn capsule-btn"
          >
            Close
          </button>
          <button
            onClick={onStart}
            disabled={isTesting}
            className="speed-test-run-btn capsule-btn"
          >
            <RefreshCw className={`w-4 h-4 ${isTesting ? "animate-spin" : ""}`} />
            {result ? (tauriRuntime ? "Retest" : "Replay Demo") : tauriRuntime ? "Start Test" : "Start Demo"}
          </button>
        </div>
      </div>
    </div>
  );
}
