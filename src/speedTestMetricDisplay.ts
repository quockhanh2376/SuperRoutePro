import { useEffect, useRef, useState } from "react";

import type { SpeedTestProgress, SpeedTestResult } from "./api";

const CAN_ANIMATE =
  typeof window !== "undefined"
  && typeof window.requestAnimationFrame === "function";

const sanitizeMetricValue = (value: number | null | undefined) => {
  if (value === null || value === undefined || Number.isNaN(value)) {
    return null;
  }
  return value;
};

const getAnimationDurationMs = (startValue: number, targetValue: number) =>
  Math.min(920, Math.max(320, 260 + Math.abs(targetValue - startValue) * 8));

type LiveTransferMetrics = {
  download: number;
  upload: number;
};

const EMPTY_LIVE_TRANSFER_METRICS: LiveTransferMetrics = {
  download: 0,
  upload: 0,
};

export const formatMetricAmount = (value: number | null | undefined) => {
  const safeValue = sanitizeMetricValue(value);
  return safeValue === null ? "--" : safeValue.toFixed(1);
};

export const splitMetricLabelLines = (label: string) =>
  label.includes(" / ") ? label.split(" / ") : [label];

export const formatSpeedTestRouteFit = (
  routeFit: SpeedTestResult["route_fit"],
  isTesting: boolean,
) => {
  switch (routeFit) {
    case "preferred_region":
      return "Preferred region";
    case "global_fallback":
      return "Global fallback";
    case "pending":
      return "Pending resolution";
    default:
      return isTesting ? "Resolving route fit..." : "--";
  }
};

export const formatSpeedTestResolvedEdge = (
  resolvedColo: string | null | undefined,
  isTesting: boolean,
) => {
  const edge = resolvedColo?.trim();
  if (edge) {
    return `${edge.toUpperCase()} edge`;
  }

  return isTesting ? "Awaiting edge trace..." : "--";
};

export const formatSpeedTestLatencyBaseline = (
  result: SpeedTestResult | null,
  isTesting: boolean,
) => {
  const stableSamples = result?.stable_latency_samples;
  const successfulSamples = result?.successful_latency_samples;
  const totalSamples = result?.latency_samples;

  if (
    stableSamples !== undefined
    || successfulSamples !== undefined
    || totalSamples !== undefined
  ) {
    const stable = stableSamples ?? 0;
    const successful = successfulSamples ?? stable;
    const total = totalSamples ?? successful;
    return `${stable} stable / ${successful} ok / ${total} total`;
  }

  return isTesting ? "Sampling baseline..." : "--";
};

export const formatSpeedTestCapturedAt = (
  timestamp: string | undefined,
  isTesting: boolean,
) => {
  if (timestamp) {
    return timestamp;
  }

  return isTesting ? "Pending final snapshot..." : "--";
};

export const useAnimatedMetricValue = (value: number | null | undefined) => {
  const targetValue = sanitizeMetricValue(value);
  const [displayValue, setDisplayValue] = useState<number | null>(() => {
    if (targetValue === null) {
      return null;
    }

    return CAN_ANIMATE ? 0 : targetValue;
  });
  const displayValueRef = useRef(displayValue);

  useEffect(() => {
    displayValueRef.current = displayValue;
  }, [displayValue]);

  useEffect(() => {
    if (!CAN_ANIMATE) {
      setDisplayValue(targetValue);
      displayValueRef.current = targetValue;
      return;
    }

    if (targetValue === null) {
      setDisplayValue(null);
      displayValueRef.current = null;
      return;
    }

    const startValue = displayValueRef.current ?? 0;
    if (Math.abs(targetValue - startValue) < 0.05) {
      setDisplayValue(targetValue);
      displayValueRef.current = targetValue;
      return;
    }

    const durationMs = getAnimationDurationMs(startValue, targetValue);
    let frameId = 0;
    let animationStartTime = 0;

    const animate = (timestamp: number) => {
      if (animationStartTime === 0) {
        animationStartTime = timestamp;
      }

      const progress = Math.min(1, (timestamp - animationStartTime) / durationMs);
      const easedProgress = 1 - Math.pow(1 - progress, 3);
      const nextValue = startValue + ((targetValue - startValue) * easedProgress);

      setDisplayValue(nextValue);
      displayValueRef.current = nextValue;

      if (progress < 1) {
        frameId = window.requestAnimationFrame(animate);
        return;
      }

      setDisplayValue(targetValue);
      displayValueRef.current = targetValue;
    };

    frameId = window.requestAnimationFrame(animate);
    return () => {
      window.cancelAnimationFrame(frameId);
    };
  }, [targetValue]);

  return displayValue;
};

export const useSpeedTestMetricSnapshot = ({
  isTesting,
  progress,
  result,
}: {
  isTesting: boolean;
  progress: SpeedTestProgress;
  result: SpeedTestResult | null;
}) => {
  const [liveTransferMetrics, setLiveTransferMetrics] = useState<LiveTransferMetrics>(
    EMPTY_LIVE_TRANSFER_METRICS,
  );

  useEffect(() => {
    if (result) {
      setLiveTransferMetrics({
        download: result.download_mbps,
        upload: result.upload_mbps,
      });
      return;
    }

    if (!isTesting) {
      setLiveTransferMetrics(EMPTY_LIVE_TRANSFER_METRICS);
      return;
    }

    const currentSpeed = sanitizeMetricValue(progress.current_speed_mbps) ?? 0;

    if (progress.stage === "preflight" || progress.stage === "latency") {
      setLiveTransferMetrics(EMPTY_LIVE_TRANSFER_METRICS);
      return;
    }

    if (progress.stage === "download") {
      setLiveTransferMetrics((current) => ({
        ...current,
        download: currentSpeed,
      }));
      return;
    }

    if (progress.stage === "upload" || progress.stage === "finalize") {
      setLiveTransferMetrics((current) => ({
        ...current,
        upload: currentSpeed,
      }));
    }
  }, [isTesting, progress.current_speed_mbps, progress.stage, result]);

  return {
    showSummary: isTesting || Boolean(result),
    downloadValue: result?.download_mbps ?? (isTesting ? liveTransferMetrics.download : null),
    uploadValue: result?.upload_mbps ?? (isTesting ? liveTransferMetrics.upload : null),
    pingValue: result?.ping_ms ?? null,
    stabilityValue: result?.jitter_ms ?? null,
    serverLabel: result?.server_label ?? (isTesting ? "Awaiting result" : "--"),
    ipLabel: result?.ip || (isTesting ? "Checking..." : "--"),
    routeFitLabel: formatSpeedTestRouteFit(result?.route_fit, isTesting),
    resolvedEdgeLabel: formatSpeedTestResolvedEdge(result?.resolved_colo, isTesting),
    latencyBaselineLabel: formatSpeedTestLatencyBaseline(result, isTesting),
    capturedAtLabel: formatSpeedTestCapturedAt(result?.timestamp, isTesting),
  };
};
