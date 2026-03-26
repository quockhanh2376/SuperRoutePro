import test from "node:test";
import assert from "node:assert/strict";

import { formatSpeedTestError } from "../src/speedTestError.ts";

test("formatSpeedTestError maps timeouts to a user-facing message", () => {
  const message = formatSpeedTestError("Download test failed to start: operation timed out");

  assert.equal(
    message,
    "Speed test timed out while talking to the test server. Check internet, VPN, or firewall and try again.",
  );
});

test("formatSpeedTestError explains latency probe failures", () => {
  const message = formatSpeedTestError("Latency probe failed: dns lookup failed");

  assert.equal(
    message,
    "Latency check failed before the speed test could start. dns lookup failed",
  );
});

test("formatSpeedTestError keeps unknown messages intact", () => {
  const message = formatSpeedTestError("Unexpected native panic");

  assert.equal(message, "Unexpected native panic");
});
