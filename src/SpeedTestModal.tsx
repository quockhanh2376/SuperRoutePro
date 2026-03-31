import { useCallback, useEffect, useRef, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { Activity, ArrowDown, ArrowUp, Check, ChevronDown, Gauge, type LucideIcon } from "lucide-react";
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
  isTesting: boolean;
};

const FIXED_LAUNCH_METRIC_ROTATION_CONFIG: Record<string, { cssDuration: string; durationMs: number }> = {
  Download: { cssDuration: "0.84s", durationMs: 840 },
  Upload: { cssDuration: "1.02s", durationMs: 1020 },
  Ping: { cssDuration: "1.26s", durationMs: 1260 },
  Stability: { cssDuration: "1.48s", durationMs: 1480 },
};

const getLaunchMetricRotationConfig = (label: string) =>
  FIXED_LAUNCH_METRIC_ROTATION_CONFIG[label] ?? { cssDuration: "1.2s", durationMs: 1200 };

function LaunchMetric({
  iconToneClassName,
  label,
  value,
  unit,
  Icon,
  isTesting,
}: LaunchMetricProps) {
  const rotationConfig = getLaunchMetricRotationConfig(label);
  const displayValue = useAnimatedMetricValue(value, {
    durationMs: rotationConfig.durationMs,
  });
  const amount = formatMetricAmount(displayValue);
  const labelLines = splitMetricLabelLines(label);
  const rotationDuration = rotationConfig.cssDuration;

  return (
    <div className="speed-test-launch-metric">
      <div className={`speed-test-launch-metric-aura ${iconToneClassName}`} />
      <div
        aria-hidden="true"
        className={`speed-test-launch-metric-track ${isTesting ? "speed-test-launch-metric-track-live" : ""} ${iconToneClassName}`}
        style={{ animationDuration: rotationDuration }}
      />
      {isTesting && (
        <div
          aria-hidden="true"
          className={`speed-test-launch-metric-head ${iconToneClassName}`}
          style={{ animationDuration: rotationDuration }}
        />
      )}
      <div className={`speed-test-launch-icon-shell ${iconToneClassName}`}>
        <div className="speed-test-launch-icon-inner-ring" />
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

export function SpeedTestModal({
  onStatusChange,
}: {
  onStatusChange?: (message: string) => void;
}) {
  const tauriRuntime = isTauriRuntime();
  const [isTesting, setIsTesting] = useState(false);
  const [isTargetMenuOpen, setIsTargetMenuOpen] = useState(false);
  const [progress, setProgress] = useState<SpeedTestProgress>(DEFAULT_PROGRESS);
  const [result, setResult] = useState<SpeedTestResult | null>(null);
  const [error, setError] = useState("");
  const [targetOptions, setTargetOptions] = useState<SpeedTestTargetOption[]>(() => getInitialTargetOptions(tauriRuntime));
  const [selectedTargetId, setSelectedTargetId] = useState(() => getInitialTargetOptions(tauriRuntime)[0]?.id ?? "");
  const targetMenuRef = useRef<HTMLDivElement | null>(null);

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

  useEffect(() => {
    if (!isTargetMenuOpen) {
      return;
    }

    const handlePointerDown = (event: MouseEvent) => {
      if (!targetMenuRef.current?.contains(event.target as Node)) {
        setIsTargetMenuOpen(false);
      }
    };

    const handleEscape = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        setIsTargetMenuOpen(false);
      }
    };

    document.addEventListener("mousedown", handlePointerDown);
    document.addEventListener("keydown", handleEscape);

    return () => {
      document.removeEventListener("mousedown", handlePointerDown);
      document.removeEventListener("keydown", handleEscape);
    };
  }, [isTargetMenuOpen]);

  useEffect(() => {
    if (isTesting || targetOptions.length <= 1) {
      setIsTargetMenuOpen(false);
    }
  }, [isTesting, targetOptions.length]);

  const handleTargetChange = useCallback((targetId: string) => {
    setSelectedTargetId(targetId);
    setIsTargetMenuOpen(false);
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
  const activeProviderLabel = result?.provider ?? selectedTarget?.provider ?? (tauriRuntime ? "Native backend" : "Browser preview");

  const handleStart = useCallback(async () => {
    if (isTesting) {
      return;
    }

    const targetLabel = selectedTarget?.label ?? (tauriRuntime ? "Auto" : "Browser Preview");

    setIsTargetMenuOpen(false);
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
  }, [isTesting, onStatusChange, selectedTarget, selectedTargetId, tauriRuntime]);

  return (
    <div className="speed-test-launch-card">
      <div className="speed-test-launch-head">
        <div className="speed-test-launch-title-cluster">
          <div className="speed-test-launch-title-row">
            <button
              className="speed-test-launch-trigger speed-test-launch-trigger-action"
              disabled={isTesting}
              onClick={() => void handleStart()}
              title={tauriRuntime ? "Start the native Speed Test" : "Start the browser preview run"}
              type="button"
            >
              <Activity className="w-4 h-4" />
              <span>
                {isTesting ? (
                  <>
                    Analyzing <span className="speed-test-launch-heading-accent">...</span>
                  </>
                ) : (
                  <>
                    Speed <span className="speed-test-launch-heading-accent">test</span>
                  </>
                )}
              </span>
            </button>

            <div className="speed-test-launch-target-shell">
              <div className="speed-test-launch-target-menu" ref={targetMenuRef}>
                <button
                  aria-expanded={isTargetMenuOpen}
                  aria-haspopup="listbox"
                  className="speed-test-launch-target-button"
                  disabled={isTesting || targetOptions.length <= 1}
                  onClick={() => setIsTargetMenuOpen((current) => !current)}
                  title={activeProviderLabel}
                  type="button"
                >
                  <span className="speed-test-launch-target-button-value">{selectedTarget?.label ?? "Auto"}</span>
                  <ChevronDown
                    className={`speed-test-launch-target-button-chevron ${isTargetMenuOpen ? "speed-test-launch-target-button-chevron-open" : ""}`}
                  />
                </button>

                {isTargetMenuOpen && (
                  <div className="speed-test-launch-target-list" role="listbox">
                    {targetOptions.map((target) => {
                      const isSelected = target.id === (selectedTarget?.id ?? "");

                      return (
                        <button
                          aria-selected={isSelected}
                          className={`speed-test-launch-target-option ${isSelected ? "speed-test-launch-target-option-selected" : ""}`}
                          key={target.id}
                          onClick={() => handleTargetChange(target.id)}
                          role="option"
                          title={`${target.region_label} | ${target.provider}`}
                          type="button"
                        >
                          <span className="speed-test-launch-target-button-value">{target.label}</span>
                          {isSelected && <Check className="speed-test-launch-target-check" />}
                        </button>
                      );
                    })}
                  </div>
                )}
              </div>
            </div>
          </div>

          {error && <div className="speed-test-launch-error">{error}</div>}
        </div>
      </div>

      <div className="speed-test-launch-body">
        <div className="speed-test-launch-surface">
          <div className="speed-test-launch-metrics">
            <LaunchMetric
              iconToneClassName="speed-test-launch-icon-download"
              Icon={ArrowDown}
              isTesting={isTesting}
              label="Download"
              unit="Mbps"
              value={summaryMetrics.downloadValue}
            />
            <LaunchMetric
              iconToneClassName="speed-test-launch-icon-upload"
              Icon={ArrowUp}
              isTesting={isTesting}
              label="Upload"
              unit="Mbps"
              value={summaryMetrics.uploadValue}
            />
            <LaunchMetric
              iconToneClassName="speed-test-launch-icon-ping"
              Icon={Gauge}
              isTesting={isTesting}
              label="Ping"
              unit="ms"
              value={summaryMetrics.pingValue}
            />
            <LaunchMetric
              iconToneClassName="speed-test-launch-icon-jitter"
              Icon={Activity}
              isTesting={isTesting}
              label="Stability"
              unit="ms"
              value={summaryMetrics.stabilityValue}
            />
          </div>
        </div>

      </div>
    </div>
  );
}
