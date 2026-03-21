import test from "node:test";
import assert from "node:assert/strict";

import { getNicTableMessage } from "../src/nicTableModel.ts";

test("shows a loading placeholder while the first NIC snapshot is still in flight", () => {
  assert.equal(
    getNicTableMessage({
      nicCount: 0,
      loading: true,
      hasLoadedOnce: false,
      activeOnly: true,
    }),
    "Loading interfaces...",
  );
});

test("distinguishes empty active-only results from the startup loading state", () => {
  assert.equal(
    getNicTableMessage({
      nicCount: 0,
      loading: false,
      hasLoadedOnce: true,
      activeOnly: true,
    }),
    "No active interfaces found",
  );
});

test("suppresses the placeholder once NIC rows exist", () => {
  assert.equal(
    getNicTableMessage({
      nicCount: 1,
      loading: false,
      hasLoadedOnce: true,
      activeOnly: true,
    }),
    null,
  );
});
