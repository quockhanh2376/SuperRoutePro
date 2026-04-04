import assert from "node:assert/strict";
import test from "node:test";

import {
  formatActionResultMessage,
  formatErrorMessage,
  formatOutputError,
  getFirstValidationError,
  isAdminElevationError,
  toErrorMessage,
} from "../src/errorUtils.ts";

test("normalizes thrown values into readable error messages", () => {
  assert.equal(toErrorMessage(new Error("boom")), "boom");
  assert.equal(toErrorMessage("plain string"), "plain string");
  assert.equal(toErrorMessage({ message: "object message" }), "object message");
});

test("builds consistent action and output messages", () => {
  assert.equal(formatActionResultMessage("Flush DNS", true), "Flush DNS completed successfully.");
  assert.equal(formatActionResultMessage("Flush DNS", false), "Flush DNS failed.");
  assert.equal(formatErrorMessage("Load failed", new Error("offline")), "Load failed: offline");
  assert.equal(formatOutputError(new Error("offline")), "Error: offline");
});

test("extracts the first validation error and detects elevation failures", () => {
  assert.equal(
    getFirstValidationError({
      dest: undefined,
      gw: "Gateway is required.",
      metric: "Metric is required.",
    }),
    "Gateway is required.",
  );
  assert.equal(isAdminElevationError("This command requires elevation."), true);
  assert.equal(isAdminElevationError("Completed without issues"), false);
});
