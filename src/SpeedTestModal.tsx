import { useCallback, useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { Activity, RefreshCw, X } from "lucide-react";
import {
  runSpeedTest,
  type SpeedTestProgress,
  type SpeedTestResult,
} from "./api";
import { isTauriRuntime, runMockSpeedTest } from "./speedTestDemo";
import "./SpeedTestModal.css";

const SPEED_TEST_PROGRESS_EVENT = "speed-test://progress";
const DEFAULT_PROGRESS: SpeedTestProgress = {
  stage: "idle",
  percent: 0,
  current_speed_mbps: 0,
  message: "Ready to measure download, upload, ping, and jitter.",
};

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

export function SpeedTestModal({
  onStatusChange,
}: {
  onStatusChange?: (message: string) => void;
}) {
  const tauriRuntime = isTauriRuntime();
  const [open, setOpen] = useState(false);
  const [isTesting, setIsTesting] = useState(false);
  const [progress, setProgress] = useState<SpeedTestProgress>(DEFAULT_PROGRESS);
  const [result, setResult] = useState<SpeedTestResult | null>(null);
  const [error, setError] = useState("");

  useEffect(() => {
    if (!tauriRuntime) return;

    let active = true;
    let cleanup: (() => void) | null = null;

    void listen<SpeedTestProgress>(SPEED_TEST_PROGRESS_EVENT, (event) => {
      if (!active) return;
      setProgress(event.payload);
    }).then((unlisten) => {
      if (!active) {
        unlisten();
        return;
      }
      cleanup = unlisten;
    });

    return () => {
      active = false;
      cleanup?.();
    };
  }, [tauriRuntime]);

  const handleOpen = useCallback(() => {
    setOpen(true);
  }, []);

  const handleClose = useCallback(() => {
    if (isTesting) return;
    setOpen(false);
  }, [isTesting]);

  const handleStart = useCallback(async () => {
    setIsTesting(true);
    setError("");
    setResult(null);
    setProgress({
      stage: "preflight",
      percent: 3,
      current_speed_mbps: 0,
      message: tauriRuntime
        ? "Starting native speed test..."
        : "Starting browser preview flow...",
    });
    onStatusChange?.("Speed test started...");

    try {
      const response = tauriRuntime
        ? await runSpeedTest(24)
        : await runMockSpeedTest(setProgress);
      setResult(response);
      setProgress({
        stage: "finalize",
        percent: 100,
        current_speed_mbps: Math.max(response.download_mbps, response.upload_mbps),
        message: `Speed test finished via ${response.provider}.`,
      });
      onStatusChange?.(
        `Speed test done: ${response.download_mbps.toFixed(1)} Mbps down / ${response.upload_mbps.toFixed(1)} Mbps up`,
      );
    } catch (err) {
      const message = err instanceof Error ? err.message : String(err);
      setError(message);
      setProgress({
        stage: "idle",
        percent: 0,
        current_speed_mbps: 0,
        message: "Speed test failed.",
      });
      onStatusChange?.(`Speed test error: ${message}`);
    } finally {
      setIsTesting(false);
    }
  }, [onStatusChange, tauriRuntime]);

  return (
    <>
      <div className="speed-test-launch-card">
        <div className="speed-test-launch-head">
          <div>
            <div className="speed-test-launch-title">
              <Activity className="w-4 h-4" />
              <span>Speed Test</span>
            </div>
            <p className="speed-test-launch-subtitle">
              Open a dedicated modal for download, upload, ping, and jitter.
            </p>
          </div>
          <button
            onClick={handleOpen}
            className="speed-test-open-btn capsule-btn"
          >
            Open
          </button>
        </div>

        <div className="speed-test-launch-body">
          {result ? (
            <div className="speed-test-launch-metrics">
              <div className="speed-test-launch-metric">
                <span className="speed-test-launch-label">Down</span>
                <span className="speed-test-launch-value">{formatMetric(result.download_mbps, "Mbps")}</span>
              </div>
              <div className="speed-test-launch-metric">
                <span className="speed-test-launch-label">Up</span>
                <span className="speed-test-launch-value">{formatMetric(result.upload_mbps, "Mbps")}</span>
              </div>
              <div className="speed-test-launch-metric">
                <span className="speed-test-launch-label">Ping</span>
                <span className="speed-test-launch-value">{formatMetric(result.ping_ms, "ms")}</span>
              </div>
              <div className="speed-test-launch-metric">
                <span className="speed-test-launch-label">Jitter</span>
                <span className="speed-test-launch-value">{formatMetric(result.jitter_ms, "ms")}</span>
              </div>
            </div>
          ) : (
            <div className="speed-test-launch-placeholder">
              Start a native test run to capture live throughput and latency from a dedicated modal.
            </div>
          )}
        </div>
      </div>

      {open && (
        <div
          className="speed-test-modal-backdrop"
          onClick={handleClose}
        >
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
                    : "Browser preview mode is active. Start Test runs a mock flow for UI demo."}
                </p>
              </div>
              <button
                onClick={handleClose}
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
                  Browser preview mode is active. Progress and result values are mocked so anh có thể xem demo flow trước khi chạy Tauri runtime thật.
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
                  : "Results stream live during the run and return as a final snapshot when complete."}
              </div>
              <div className="flex items-center gap-2">
                <button
                  onClick={handleClose}
                  disabled={isTesting}
                  className="speed-test-footer-close-btn capsule-btn"
                >
                  Close
                </button>
                <button
                  onClick={() => void handleStart()}
                  disabled={isTesting}
                  className="speed-test-run-btn capsule-btn"
                >
                  <RefreshCw className={`w-4 h-4 ${isTesting ? "animate-spin" : ""}`} />
                  {result ? "Retest" : "Start Test"}
                </button>
              </div>
            </div>
          </div>
        </div>
      )}
    </>
  );
}
