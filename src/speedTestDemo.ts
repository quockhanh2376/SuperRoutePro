import type { SpeedTestProgress, SpeedTestResult } from "./api";

type ProgressSink = (progress: SpeedTestProgress) => void;

const MOCK_PROGRESS_FRAMES: SpeedTestProgress[] = [
  {
    stage: "preflight",
    percent: 4,
    current_speed_mbps: 0,
    message: "Browser demo mode is preparing the speed test flow.",
  },
  {
    stage: "latency",
    percent: 18,
    current_speed_mbps: 0,
    message: "Simulating latency sampling for dev preview.",
  },
  {
    stage: "download",
    percent: 42,
    current_speed_mbps: 146.2,
    message: "Streaming demo download progress...",
  },
  {
    stage: "download",
    percent: 68,
    current_speed_mbps: 218.4,
    message: "Download sample is stabilizing in preview mode.",
  },
  {
    stage: "upload",
    percent: 92,
    current_speed_mbps: 81.3,
    message: "Uploading demo validation payload...",
  },
];

declare global {
  interface Window {
    __TAURI_INTERNALS__?: unknown;
  }
}

export function isTauriRuntime(): boolean {
  return typeof window !== "undefined" && typeof window.__TAURI_INTERNALS__ !== "undefined";
}

export function buildMockSpeedTestResult(now = new Date()): SpeedTestResult {
  return {
    provider: "Browser Demo",
    server_label: "Local Preview",
    download_mbps: 226.8,
    upload_mbps: 81.3,
    ping_ms: 27.4,
    jitter_ms: 1.8,
    ip: "Preview mode",
    timestamp: now.toISOString(),
  };
}

async function delay(ms: number): Promise<void> {
  if (ms <= 0) {
    return;
  }
  await new Promise((resolve) => setTimeout(resolve, ms));
}

export async function runMockSpeedTest(
  onProgress: ProgressSink,
  options?: {
    delayMs?: number;
    now?: Date;
  },
): Promise<SpeedTestResult> {
  const delayMs = options?.delayMs ?? 420;

  for (const frame of MOCK_PROGRESS_FRAMES) {
    onProgress(frame);
    await delay(delayMs);
  }

  const result = buildMockSpeedTestResult(options?.now);
  onProgress({
    stage: "finalize",
    percent: 100,
    current_speed_mbps: result.download_mbps,
    message: "Browser demo finished. Native runtime will use the Rust backend.",
  });
  return result;
}
