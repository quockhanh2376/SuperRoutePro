import { useCallback, useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { Activity } from "lucide-react";
import {
  listSpeedTestTargets,
  runSpeedTest,
  type SpeedTestProgress,
  type SpeedTestResult,
  type SpeedTestTargetOption,
} from "./api";
import { formatSpeedTestError } from "./speedTestError";
import { isTauriRuntime, runMockSpeedTest } from "./speedTestDemo";
import { SpeedTestModalDialog } from "./SpeedTestModalView";
import "./SpeedTestModal.css";

const SPEED_TEST_PROGRESS_EVENT = "speed-test://progress";
const DEFAULT_PROGRESS: SpeedTestProgress = {
  stage: "idle",
  percent: 0,
  current_speed_mbps: 0,
  message: "Ready to measure download, upload, ping, and jitter.",
};

const TAURI_FALLBACK_TARGETS: SpeedTestTargetOption[] = [
  {
    id: "auto_asia",
    label: "Auto Asia",
    description: "Cloudflare auto-selects the nearest preferred Asia edge. Country pinning will sit on top of this target catalog once real region targets are available.",
    provider: "Cloudflare (Asia auto-edge)",
  },
];

const BROWSER_DEMO_TARGETS: SpeedTestTargetOption[] = [
  {
    id: "browser_preview",
    label: "Browser Preview",
    description: "Browser-safe preview flow for the speed test modal. Native desktop runtime will replace this with real target catalog entries.",
    provider: "Browser Demo",
  },
];

const getInitialTargetOptions = (tauriRuntime: boolean): SpeedTestTargetOption[] =>
  tauriRuntime ? TAURI_FALLBACK_TARGETS : BROWSER_DEMO_TARGETS;

const formatMetric = (value: number | null | undefined, suffix: string) => {
  if (value === null || value === undefined || Number.isNaN(value)) {
    return `-- ${suffix}`;
  }
  return `${value.toFixed(1)} ${suffix}`;
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

  const handleStart = useCallback(async () => {
    const targetLabel = selectedTarget?.label ?? (tauriRuntime ? "Auto Asia" : "Browser Preview");

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
        ? await runSpeedTest(24, selectedTargetId)
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
          <div>
            <div className="speed-test-launch-title">
              <Activity className="w-4 h-4" />
              <span>Speed Test</span>
            </div>
            <p className="speed-test-launch-subtitle">
              {tauriRuntime
                ? "Open a dedicated modal for download, upload, ping, and jitter."
                : "Preview the dedicated modal in browser demo mode before the native desktop runtime is available."}
            </p>
          </div>
          <button
            onClick={handleOpen}
            className="speed-test-open-btn capsule-btn"
          >
            {tauriRuntime ? "Open" : "Preview"}
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
