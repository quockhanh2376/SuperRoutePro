import test from "node:test";
import assert from "node:assert/strict";

import {
  buildMockSpeedTestResult,
  isTauriRuntime,
  runMockSpeedTest,
} from "../src/speedTestDemo.ts";

test("isTauriRuntime is false in plain node/browserless execution", () => {
  assert.equal(isTauriRuntime(), false);
});

test("buildMockSpeedTestResult returns stable preview metadata", () => {
  const now = new Date("2026-03-24T08:30:00.000Z");
  const result = buildMockSpeedTestResult(now);

  assert.equal(result.target_id, "browser_preview");
  assert.equal(result.target_label, "Browser Preview");
  assert.equal(result.provider, "Browser Demo");
  assert.equal(result.region_label, "Preview");
  assert.equal(result.server_label, "Local Preview");
  assert.equal(result.download_mbps, 226.8);
  assert.equal(result.upload_mbps, 81.3);
  assert.equal(result.timestamp, now.toISOString());
});

test("runMockSpeedTest emits staged progress before returning final result", async () => {
  const frames: string[] = [];
  const now = new Date("2026-03-24T08:30:00.000Z");

  const result = await runMockSpeedTest((progress) => {
    frames.push(`${progress.stage}:${Math.round(progress.percent)}`);
  }, {
    delayMs: 0,
    now,
  });

  assert.deepEqual(frames, [
    "preflight:4",
    "latency:18",
    "download:42",
    "download:68",
    "upload:92",
    "finalize:100",
  ]);
  assert.equal(result.target_id, "browser_preview");
  assert.equal(result.provider, "Browser Demo");
  assert.equal(result.region_label, "Preview");
  assert.equal(result.timestamp, now.toISOString());
});
