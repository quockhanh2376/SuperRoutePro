import test from "node:test";
import assert from "node:assert/strict";
import React from "react";
import { renderToStaticMarkup } from "react-dom/server";

import type { SpeedTestProgress, SpeedTestResult } from "../src/api.ts";
import { SpeedTestModalDialog } from "../src/SpeedTestModalView.tsx";

const progress: SpeedTestProgress = {
  stage: "finalize",
  percent: 100,
  current_speed_mbps: 226.8,
  message: "Speed test finished via LibreSpeed.",
};

const result: SpeedTestResult = {
  target_id: "us_west",
  target_label: "US West",
  provider: "LibreSpeed (regional fixed backend)",
  region_label: "US West",
  route_fit: "global_fallback",
  resolved_colo: "sjc",
  server_label: "Los Angeles, United States (Clouvider)",
  download_mbps: 226.8,
  upload_mbps: 81.3,
  ping_ms: 146.4,
  jitter_ms: 3.1,
  latency_samples: 8,
  successful_latency_samples: 7,
  stable_latency_samples: 6,
  ip: "203.0.113.7",
  timestamp: "2026-03-26T08:20:00.000Z",
};

test("SpeedTestModalDialog renders regional targets and the resolved fixed backend metadata", () => {
  const html = renderToStaticMarkup(
    <SpeedTestModalDialog
      error=""
      isTesting={false}
      onClose={() => {}}
      onStart={() => {}}
      onTargetChange={() => {}}
      progress={progress}
      result={result}
      selectedTargetId="us_west"
      tauriRuntime
      targetOptions={[
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
          id: "us_west",
          label: "US West",
          description: "Fixed trans-Pacific backend pinned to Los Angeles, United States. Use this to compare long-haul performance against a stable US West endpoint.",
          provider: "LibreSpeed (regional fixed backend)",
          region_label: "US West",
        },
      ]}
    />,
  );

  assert.match(html, /Target/);
  assert.match(html, /US West/);
  assert.match(html, /Region/);
  assert.match(html, /Server/);
  assert.match(html, /Los Angeles, United States \(Clouvider\)/);
  assert.match(html, /LibreSpeed \(regional fixed backend\)/);
  assert.match(html, /Route Fit/);
  assert.match(html, /Global fallback/);
  assert.match(html, /Resolved Edge/);
  assert.match(html, /SJC edge/);
  assert.match(html, /Latency Baseline/);
  assert.match(html, /6 stable \/ 7 ok \/ 8 total/);
  assert.match(html, /Captured/);
  assert.match(html, /2026-03-26T08:20:00.000Z/);
  assert.match(html, /Region Target/);
  assert.match(html, /Auto Australia/);
  assert.match(html, /Stability/);
  assert.doesNotMatch(html, /Jitter/);
});

test("SpeedTestModalDialog renders live metric cards while a run is active", () => {
  const html = renderToStaticMarkup(
    <SpeedTestModalDialog
      error=""
      isTesting
      onClose={() => {}}
      onStart={() => {}}
      onTargetChange={() => {}}
      progress={{
        stage: "download",
        percent: 42,
        current_speed_mbps: 188.6,
        message: "Downloading test payload... 0 / 24 MB",
      }}
      result={null}
      selectedTargetId="auto_au"
      tauriRuntime
      targetOptions={[
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
      ]}
    />,
  );

  assert.match(html, /Live Throughput/);
  assert.match(html, /Auto Australia/);
  assert.match(html, /Downloading/);
  assert.match(html, /Downloading 24 MB/);
  assert.match(html, /Awaiting result/);
  assert.match(html, /Checking\.\.\./);
  assert.match(html, /Resolving route fit\.\.\./);
  assert.match(html, /Awaiting edge trace\.\.\./);
  assert.match(html, /Sampling baseline\.\.\./);
  assert.match(html, /Stability/);
});

test("SpeedTestModalDialog derives a stage-aware live status label", () => {
  const html = renderToStaticMarkup(
    <SpeedTestModalDialog
      error=""
      isTesting
      onClose={() => {}}
      onStart={() => {}}
      onTargetChange={() => {}}
      progress={{
        stage: "latency",
        percent: 18,
        current_speed_mbps: 0,
        message: "Checking latency before throughput run...",
      }}
      result={null}
      selectedTargetId="auto_asia"
      tauriRuntime
      targetOptions={[
        {
          id: "auto_asia",
          label: "Auto Asia",
          description: "Cloudflare auto-selects the nearest preferred Asia edge. Use this as the route-aware baseline close to the current network path.",
          provider: "Cloudflare (Asia auto-edge)",
          region_label: "Asia",
        },
      ]}
    />,
  );

  assert.match(html, /Measuring latency/);
  assert.match(html, /-- Mbps/);
  assert.doesNotMatch(html, />Streaming</);
});
