import { useAnimatedMetricValue, formatMetricAmount, splitMetricLabelLines, useSpeedTestMetricSnapshot } from "./speedTestMetricDisplay";
import { Activity, ArrowDown, ArrowUp, Gauge, RefreshCw, X, type LucideIcon } from "lucide-react";

import type { SpeedTestProgress, SpeedTestResult, SpeedTestTargetOption } from "./api";

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

const getLiveStatusLabel = (stage: string) => {
  switch (stage) {
    case "preflight":
      return "Preparing";
    case "latency":
      return "Measuring latency";
    case "download":
      return "Downloading";
    case "upload":
      return "Upload in progress";
    case "finalize":
      return "Finalizing";
    default:
      return "Running";
  }
};

const getLiveThroughputValue = (stage: string, currentSpeedMbps: number | null | undefined) => {
  if (stage !== "download" && stage !== "upload" && stage !== "finalize") {
    return "-- Mbps";
  }
  return formatMetric(currentSpeedMbps, "Mbps");
};

const getCondensedProgressMessage = (stage: string, message: string) => {
  const sizeMatch = message.match(/(?:~|\/\s*)(\d+)\s*MB/i);

  if (stage === "download") {
    return sizeMatch ? `Downloading ${sizeMatch[1]} MB...` : "Downloading...";
  }

  if (stage === "upload") {
    return sizeMatch ? `Uploading ${sizeMatch[1]} MB...` : "Uploading...";
  }

  return message;
};

export type SpeedTestModalDialogProps = {
  error: string;
  isTesting: boolean;
  selectedTargetId: string;
  progress: SpeedTestProgress;
  result: SpeedTestResult | null;
  tauriRuntime: boolean;
  onClose: () => void;
  onStart: () => void;
  onTargetChange: (targetId: string) => void;
  targetOptions: SpeedTestTargetOption[];
};

type ResultSummaryMetricProps = {
  iconToneClassName: string;
  label: string;
  unit: string;
  value: number | null | undefined;
  Icon: LucideIcon;
};

function ResultSummaryMetric({
  iconToneClassName,
  label,
  value,
  unit,
  Icon,
}: ResultSummaryMetricProps) {
  const displayValue = useAnimatedMetricValue(value);
  const amount = formatMetricAmount(displayValue);
  const labelLines = splitMetricLabelLines(label);

  return (
    <div className="speed-test-summary-card-rich">
      <div className={`speed-test-summary-icon-shell ${iconToneClassName}`}>
        <Icon
          aria-hidden="true"
          className="speed-test-summary-icon-glyph"
        />
        <div className="speed-test-summary-orb-value">{amount}</div>
        {unit && <div className="speed-test-summary-orb-unit">{unit}</div>}
        <div className="speed-test-summary-orb-label" aria-label={label}>
          {labelLines.map((labelLine, index) => (
            <span
              key={`${label}-${labelLine}`}
              className="speed-test-summary-orb-label-line"
            >
              {labelLine}
              {index === 0 && labelLines.length > 1 ? " /" : ""}
            </span>
          ))}
        </div>
      </div>
    </div>
  );
}

export function SpeedTestModalDialog({
  error,
  isTesting,
  selectedTargetId,
  progress,
  result,
  tauriRuntime,
  onClose,
  onStart,
  onTargetChange,
  targetOptions,
}: SpeedTestModalDialogProps) {
  const activeTarget =
    targetOptions.find((target) => target.id === selectedTargetId)
    ?? targetOptions[0]
    ?? null;
  const targetLabel = result?.target_label ?? activeTarget?.label ?? "Auto";
  const providerLabel = result?.provider ?? activeTarget?.provider ?? "Auto";
  const regionLabel = result?.region_label ?? activeTarget?.region_label ?? targetLabel;
  const progressPercent = Math.min(100, Math.max(0, progress.percent));
  const progressStageLabel = getStageLabel(progress.stage);
  const liveStatusLabel = getLiveStatusLabel(progress.stage);
  const liveThroughputValue = getLiveThroughputValue(progress.stage, progress.current_speed_mbps);
  const condensedProgressMessage = getCondensedProgressMessage(progress.stage, progress.message);
  const summaryMetrics = useSpeedTestMetricSnapshot({
    isTesting,
    progress,
    result,
  });

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

        <div className={`speed-test-top-layout ${summaryMetrics.showSummary ? "speed-test-top-layout-with-results" : ""}`}>
          <div className="speed-test-target-shell">
            <div className="speed-test-target-title-row">
              <div className="speed-test-target-title">Test Target</div>
              <div className="speed-test-target-chip">{activeTarget?.label ?? "Auto"}</div>
            </div>
            <label className="speed-test-target-control">
              <span className="speed-test-target-label">Region Target</span>
            <select
              className="speed-test-target-select"
              disabled={isTesting || targetOptions.length <= 1}
              onChange={(event) => onTargetChange(event.target.value)}
              value={selectedTargetId}
              >
                {targetOptions.map((target) => (
                  <option key={target.id} value={target.id}>
                    {target.label}
                  </option>
                ))}
              </select>
            </label>
          </div>

          {summaryMetrics.showSummary && (
            <div className="speed-test-result-sidebar">
              <div className="speed-test-summary-grid speed-test-summary-grid-top">
                <ResultSummaryMetric
                  iconToneClassName="speed-test-summary-icon-download"
                  Icon={ArrowDown}
                  label="Download"
                  unit="Mbps"
                  value={summaryMetrics.downloadValue}
                />
                <ResultSummaryMetric
                  iconToneClassName="speed-test-summary-icon-upload"
                  Icon={ArrowUp}
                  label="Upload"
                  unit="Mbps"
                  value={summaryMetrics.uploadValue}
                />
                <ResultSummaryMetric
                  iconToneClassName="speed-test-summary-icon-ping"
                  Icon={Gauge}
                  label="Ping"
                  unit="ms"
                  value={summaryMetrics.pingValue}
                />
                <ResultSummaryMetric
                  iconToneClassName="speed-test-summary-icon-jitter"
                  Icon={Activity}
                  label="Stability"
                  unit="ms"
                  value={summaryMetrics.stabilityValue}
                />
              </div>

              <div className="speed-test-meta-grid speed-test-meta-grid-top">
                <div className="speed-test-meta-row">
                  <span className="speed-test-meta-label">Server</span>
                  <span className="speed-test-meta-value">{summaryMetrics.serverLabel}</span>
                </div>
                <div className="speed-test-meta-row">
                  <span className="speed-test-meta-label">Public IP</span>
                  <span className="speed-test-meta-value">{summaryMetrics.ipLabel}</span>
                </div>
              </div>
            </div>
          )}
        </div>

        {isTesting ? (
          <div className="speed-test-live-shell">
            <div className="speed-test-live-meter">
              <div className="speed-test-progress-topline">
                <span className="speed-test-stage-chip">{progressStageLabel}</span>
                <span className="speed-test-progress-percent">{Math.round(progress.percent)}%</span>
              </div>
              <div className="speed-test-progress-track">
                <div
                  className={`speed-test-progress-fill ${isTesting ? "speed-test-progress-fill-active" : ""}`}
                  style={{ width: `${progressPercent}%` }}
                />
              </div>
              <div className="speed-test-progress-message">{condensedProgressMessage}</div>
            </div>

            <div className="speed-test-live-grid">
              <div className="speed-test-live-card speed-test-live-card-primary">
                <span className="speed-test-live-label">Live Throughput</span>
                <span className="speed-test-live-value">{liveThroughputValue}</span>
                <span className="speed-test-live-copy">Current transfer rate</span>
              </div>
              <div className="speed-test-live-card">
                <span className="speed-test-live-label">Target</span>
                <span className="speed-test-live-value">{targetLabel}</span>
                <span className="speed-test-live-copy">{providerLabel}</span>
              </div>
              <div className="speed-test-live-card">
                <span className="speed-test-live-label">Status</span>
                <span className="speed-test-live-value">{liveStatusLabel}</span>
                <span className="speed-test-live-copy">{condensedProgressMessage}</span>
              </div>
            </div>
          </div>
        ) : (
          <div className="speed-test-progress-shell">
            <div className="speed-test-progress-topline">
              <span className="speed-test-stage-chip">{progressStageLabel}</span>
              <span className="speed-test-progress-percent">{Math.round(progress.percent)}%</span>
            </div>
            <div className="speed-test-progress-track">
                <div
                  className={`speed-test-progress-fill ${isTesting ? "speed-test-progress-fill-active" : ""}`}
                  style={{ width: `${progressPercent}%` }}
                />
            </div>
            <div className="speed-test-progress-message">{progress.message}</div>
            <div className="speed-test-live-speed">
              Live speed: <strong>{formatMetric(progress.current_speed_mbps, "Mbps")}</strong>
            </div>
          </div>
        )}

        {result && (
          <>
            <div className="speed-test-identity-grid">
              <div className="speed-test-identity-card">
                <span className="speed-test-identity-label">Target</span>
                <span className="speed-test-identity-value">{targetLabel}</span>
                <span className="speed-test-identity-copy">Selected test profile</span>
              </div>
              <div className="speed-test-identity-card">
                <span className="speed-test-identity-label">Provider</span>
                <span className="speed-test-identity-value">{providerLabel}</span>
                <span className="speed-test-identity-copy">Backend policy</span>
              </div>
              <div className="speed-test-identity-card speed-test-identity-card-region">
                <span className="speed-test-identity-label">Region</span>
                <span className="speed-test-identity-value">{regionLabel}</span>
              </div>
            </div>
          </>
        )}

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
