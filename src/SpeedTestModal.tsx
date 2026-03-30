import { useCallback, useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { Activity, ArrowDown, ArrowUp, Gauge, type LucideIcon } from "lucide-react";
import {
  listSpeedTestTargets,
  runSpeedTest,
  type SpeedTestProgress,
  type SpeedTestResult,
  type SpeedTestTargetOption,
} from "./api";
import { formatSpeedTestError } from "./speedTestError";
import {
  formatMetricAmount,
  splitMetricLabelLines,
  useAnimatedMetricValue,
  useSpeedTestMetricSnapshot,
} from "./speedTestMetricDisplay";
import { isTauriRuntime, runMockSpeedTest } from "./speedTestDemo";
import { SpeedTestModalDialog } from "./SpeedTestModalView";
import "./SpeedTestModal.css";

const SPEED_TEST_PROGRESS_EVENT = "speed-test://progress";
const DEFAULT_PROGRESS: SpeedTestProgress = {
  stage: "idle",
  percent: 0,
  current_speed_mbps: 0,
  message: "Ready to measure download, upload, ping, and stability.",
};

const TAURI_FALLBACK_TARGETS: SpeedTestTargetOption[] = [
  {
    id: "auto_asia",
    label: "Auto Asia",
    description: "Cloudflare auto-selects the nearest preferred Asia edge. Use this as the route-aware baseline close to the current network path.",
    provider: "Cloudflare (Asia auto-edge)",
    region_label: "Asia",
  },
  {
    id: "auto_au",
    label: "Auto Australia",
    description: "Cloudflare auto-selects the nearest preferred Australia edge. Use this to compare a southern hemisphere auto path against fixed regional backends.",
    provider: "Cloudflare (Australia auto-edge)",
    region_label: "Australia",
  },
  {
    id: "jp_kr",
    label: "JP/KR",
    description: "Fixed Northeast Asia backend pinned to Tokyo, Japan. Use this to compare against Auto Asia without Cloudflare auto-edge routing.",
    provider: "LibreSpeed (regional fixed backend)",
    region_label: "JP/KR",
  },
  {
    id: "us_west",
    label: "US West",
    description: "Fixed trans-Pacific backend pinned to Los Angeles, United States. Use this to compare long-haul performance against a stable US West endpoint.",
    provider: "LibreSpeed (regional fixed backend)",
    region_label: "US West",
  },
  {
    id: "eu",
    label: "EU",
    description: "Fixed Europe backend pinned to London, England. Payload sizes stay smaller here so long-haul runs from any distant region remain stable.",
    provider: "LibreSpeed (regional fixed backend)",
    region_label: "EU",
  },
];

const BROWSER_DEMO_TARGETS: SpeedTestTargetOption[] = [
  {
    id: "browser_preview",
    label: "Browser Preview",
    description: "Browser-safe preview flow for the speed test modal. Native desktop runtime will replace this with real target catalog entries.",
    provider: "Browser Demo",
    region_label: "Preview",
  },
];

const getInitialTargetOptions = (tauriRuntime: boolean): SpeedTestTargetOption[] =>
  tauriRuntime ? TAURI_FALLBACK_TARGETS : BROWSER_DEMO_TARGETS;

type LaunchMetricProps = {
  iconToneClassName: string;
  label: string;
  unit: string;
  value: number | null | undefined;
  Icon: LucideIcon;
};

type LaunchMetaRowProps = {
  label: string;
  value: string;
};

function LaunchMetric({
  iconToneClassName,
  label,
  value,
  unit,
  Icon,
}: LaunchMetricProps) {
  const displayValue = useAnimatedMetricValue(value);
  const amount = formatMetricAmount(displayValue);
  const labelLines = splitMetricLabelLines(label);

  return (
    <div className="speed-test-launch-metric">
      <div className={`speed-test-launch-icon-shell ${iconToneClassName}`}>
        <Icon
          aria-hidden="true"
          className="speed-test-launch-icon-glyph"
        />
        <div className="speed-test-launch-orb-value">{amount}</div>
        {unit && <div className="speed-test-launch-orb-unit">{unit}</div>}
        <div className="speed-test-launch-orb-label" aria-label={label}>
          {labelLines.map((labelLine, index) => (
            <span
              key={`${label}-${labelLine}`}
              className="speed-test-launch-orb-label-line"
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

function LaunchMetaRow({ label, value }: LaunchMetaRowProps) {
  return (
    <div className="speed-test-launch-server-row">
      <span className="speed-test-launch-label">{label}</span>
      <span className="speed-test-launch-server-value">{value}</span>
    </div>
  );
}

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
  const [targetOptions, setTargetOptions] = useState<SpeedTestTargetOption[]>(() => getInitialTargetOptions(tauriRuntime));
  const [selectedTargetId, setSelectedTargetId] = useState(() => getInitialTargetOptions(tauriRuntime)[0]?.id ?? "");

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

  useEffect(() => {
    const fallbackTargets = getInitialTargetOptions(tauriRuntime);
    setTargetOptions(fallbackTargets);
    setSelectedTargetId((current) => current || (fallbackTargets[0]?.id ?? ""));

    if (!tauriRuntime) return;

    let active = true;
    void listSpeedTestTargets()
      .then((targets) => {
        if (!active || targets.length === 0) {
          return;
        }
        setTargetOptions(targets);
        setSelectedTargetId((current) =>
          targets.some((target) => target.id === current) ? current : targets[0].id,
        );
      })
      .catch((loadErr) => {
        console.warn("Failed to load speed test targets:", loadErr);
      });

    return () => {
      active = false;
    };
  }, [tauriRuntime]);

  const handleOpen = useCallback(() => {
    setOpen(true);
  }, []);

  const handleClose = useCallback(() => {
    if (isTesting) return;
    setOpen(false);
  }, [isTesting]);

  const handleTargetChange = useCallback((targetId: string) => {
    setSelectedTargetId(targetId);
    setResult(null);
    setError("");
    setProgress(DEFAULT_PROGRESS);
  }, []);

  const selectedTarget =
    targetOptions.find((target) => target.id === selectedTargetId)
    ?? targetOptions[0]
    ?? null;
  const summaryMetrics = useSpeedTestMetricSnapshot({
    isTesting,
    progress,
    result,
  });

  const handleStart = useCallback(async () => {
    const targetLabel = selectedTarget?.label ?? (tauriRuntime ? "Auto" : "Browser Preview");

    setIsTesting(true);
    setError("");
    setResult(null);
    setProgress({
      stage: "preflight",
      percent: 3,
      current_speed_mbps: 0,
      message: tauriRuntime
        ? `Starting native speed test for ${targetLabel}...`
        : `Starting browser preview flow for ${targetLabel}...`,
    });
    onStatusChange?.(
      tauriRuntime
        ? `Speed test started: ${targetLabel}`
        : `Speed test demo started: ${targetLabel}`,
    );

    try {
      const response = tauriRuntime
        ? await runSpeedTest(undefined, selectedTargetId)
        : await runMockSpeedTest(setProgress);

      setResult(response);
      setProgress({
        stage: "finalize",
        percent: 100,
        current_speed_mbps: Math.max(response.download_mbps, response.upload_mbps),
        message: tauriRuntime
          ? `Speed test finished via ${response.provider}.`
          : `Speed test demo finished via ${response.provider}.`,
      });
      onStatusChange?.(
        tauriRuntime
          ? `Speed test done: ${response.download_mbps.toFixed(1)} Mbps down / ${response.upload_mbps.toFixed(1)} Mbps up`
          : `Speed test demo ready: ${response.download_mbps.toFixed(1)} Mbps down / ${response.upload_mbps.toFixed(1)} Mbps up`,
      );
    } catch (err) {
      const message = formatSpeedTestError(err);
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
  }, [onStatusChange, selectedTarget, selectedTargetId, tauriRuntime]);

  return (
    <>
      <div className="speed-test-launch-card">
        <div className="speed-test-launch-head">
          <button
            onClick={handleOpen}
            className="speed-test-launch-trigger"
            title={tauriRuntime ? "Open Speed Test" : "Preview Speed Test"}
          >
            <Activity className="w-4 h-4" />
            <span>Speed Test</span>
          </button>
        </div>

        <div className="speed-test-launch-body">
          {summaryMetrics.showSummary ? (
            <div className="speed-test-launch-results">
              <div className="speed-test-launch-metrics">
                <LaunchMetric
                  iconToneClassName="speed-test-launch-icon-download"
                  Icon={ArrowDown}
                  label="Download"
                  unit="Mbps"
                  value={summaryMetrics.downloadValue}
                />
                <LaunchMetric
                  iconToneClassName="speed-test-launch-icon-upload"
                  Icon={ArrowUp}
                  label="Upload"
                  unit="Mbps"
                  value={summaryMetrics.uploadValue}
                />
                <LaunchMetric
                  iconToneClassName="speed-test-launch-icon-ping"
                  Icon={Gauge}
                  label="Ping"
                  unit="ms"
                  value={summaryMetrics.pingValue}
                />
                <LaunchMetric
                  iconToneClassName="speed-test-launch-icon-jitter"
                  Icon={Activity}
                  label="Stability"
                  unit="ms"
                  value={summaryMetrics.stabilityValue}
                />
              </div>

              <div className="speed-test-launch-server-card">
                <div className="speed-test-launch-server-grid">
                  <LaunchMetaRow label="Server" value={summaryMetrics.serverLabel} />
                  <LaunchMetaRow label="Public IP" value={summaryMetrics.ipLabel} />
                  <LaunchMetaRow label="Route Fit" value={summaryMetrics.routeFitLabel} />
                  <LaunchMetaRow label="Resolved Edge" value={summaryMetrics.resolvedEdgeLabel} />
                  <LaunchMetaRow label="Latency Baseline" value={summaryMetrics.latencyBaselineLabel} />
                  <LaunchMetaRow label="Captured" value={summaryMetrics.capturedAtLabel} />
                </div>
              </div>
            </div>
          ) : (
            <div className="speed-test-launch-placeholder">
              {tauriRuntime
                ? "Start a native test run to capture live throughput and latency from a dedicated modal."
                : "Run a browser-safe mock sequence to preview the speed test flow before using the desktop runtime."}
            </div>
          )}
        </div>
      </div>

      {open && (
        <div
          className="speed-test-modal-backdrop"
          onClick={handleClose}
        >
          <SpeedTestModalDialog
            error={error}
            isTesting={isTesting}
            onClose={handleClose}
            onStart={() => void handleStart()}
            onTargetChange={handleTargetChange}
            progress={progress}
            result={result}
            selectedTargetId={selectedTargetId}
            tauriRuntime={tauriRuntime}
            targetOptions={targetOptions}
          />
        </div>
      )}
    </>
  );
}
