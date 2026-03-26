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
  server_label: "Los Angeles, United States (Clouvider)",
  download_mbps: 226.8,
  upload_mbps: 81.3,
  ping_ms: 146.4,
  jitter_ms: 3.1,
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
        },
        {
          id: "us_west",
          label: "US West",
          description: "Fixed trans-Pacific backend pinned to Los Angeles, United States. Use this to compare long-haul performance against a stable US West endpoint.",
          provider: "LibreSpeed (regional fixed backend)",
        },
      ]}
    />,
  );

  assert.match(html, /Target/);
  assert.match(html, /US West/);
  assert.match(html, /Server/);
  assert.match(html, /Los Angeles, United States \(Clouvider\)/);
  assert.match(html, /LibreSpeed \(regional fixed backend\)/);
  assert.match(html, /Region Target/);
  assert.match(html, /Fixed regional backends are available now/);
});
