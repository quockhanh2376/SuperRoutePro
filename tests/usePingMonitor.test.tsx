import "./setupDom.ts";

import assert from "node:assert/strict";
import { afterEach, test } from "node:test";
import { act, cleanup, renderHook, waitFor } from "@testing-library/react";

import { usePingMonitor } from "../src/hooks/usePingMonitor.ts";

const originalSetInterval = window.setInterval;
const originalClearInterval = window.clearInterval;

afterEach(() => {
  cleanup();
  window.setInterval = originalSetInterval;
  window.clearInterval = originalClearInterval;
});

test("usePingMonitor runs the single-host ping path immediately on start", async () => {
  const lines: string[] = [];
  const statuses: string[] = [];
  window.setInterval = (() => 1) as unknown as typeof window.setInterval;
  window.clearInterval = (() => {}) as typeof window.clearInterval;

  const { result } = renderHook(() => usePingMonitor({
    appendLine: (line) => {
      lines.push(line);
    },
    appendLines: (nextLines) => {
      lines.push(...nextLines);
    },
    setStatusMessage: (message) => {
      statuses.push(message);
    },
    initialTarget: "1.1.1.1",
    pingHostFn: async () => ({
      success: true,
      latency_ms: 14,
      output: "reply",
    }),
  }));

  act(() => {
    result.current.handleStartPing();
  });

  await waitFor(() => {
    assert.ok(lines.some((line) => line.includes("Reply from 1.1.1.1")));
  });
  assert.equal(result.current.pingRunning, true);
  assert.ok(statuses.some((message) => message.includes("continuously")));
});

test("usePingMonitor supports fping mode with batch host output", async () => {
  const lines: string[] = [];
  window.setInterval = (() => 1) as unknown as typeof window.setInterval;
  window.clearInterval = (() => {}) as typeof window.clearInterval;

  const { result } = renderHook(() => usePingMonitor({
    appendLine: (line) => {
      lines.push(line);
    },
    appendLines: (nextLines) => {
      lines.push(...nextLines);
    },
    setStatusMessage: () => {},
    initialTarget: "1.1.1.1",
    fpingScanFn: async () => ({
      sent: 2,
      received: 1,
      loss_percent: 50,
      min_ms: 10,
      avg_ms: 10,
      max_ms: 10,
      hosts: [
        { target: "1.1.1.1", success: true, latency_ms: 10, output: "ok" },
        { target: "8.8.8.8", success: false, latency_ms: 0, output: "timeout" },
      ],
    }),
  }));

  act(() => {
    result.current.setPingMode("fping");
    result.current.setPingTarget("1.1.1.1 8.8.8.8");
    result.current.handleStartPing();
  });

  await waitFor(() => {
    assert.ok(lines.some((line) => line.includes("[UP] 1.1.1.1 10 ms")));
    assert.ok(lines.some((line) => line.includes("[DOWN] 8.8.8.8 timeout")));
  });
});
