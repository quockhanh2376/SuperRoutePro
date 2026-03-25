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
  message: "Speed test finished via Cloudflare.",
};

const result: SpeedTestResult = {
  provider: "Cloudflare",
  server_label: "Asia Preferred (SIN edge)",
  download_mbps: 226.8,
  upload_mbps: 81.3,
  ping_ms: 27.4,
  jitter_ms: 1.8,
  ip: "203.0.113.7",
  timestamp: "2026-03-25T07:45:00.000Z",
};

test("SpeedTestModalDialog renders the resolved server label in the modal metadata", () => {
  const html = renderToStaticMarkup(
    <SpeedTestModalDialog
      error=""
      isTesting={false}
      onClose={() => {}}
      onStart={() => {}}
      progress={progress}
      result={result}
      tauriRuntime
    />,
  );

  assert.match(html, /Server/);
  assert.match(html, /Asia Preferred \(SIN edge\)/);
  assert.match(html, /Cloudflare/);
});
