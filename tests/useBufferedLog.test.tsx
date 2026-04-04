import "./setupDom.ts";

import assert from "node:assert/strict";
import { afterEach, test } from "node:test";
import { act, cleanup, renderHook, waitFor } from "@testing-library/react";

import { useBufferedLog } from "../src/hooks/useBufferedLog.ts";

afterEach(() => {
  cleanup();
});

test("useBufferedLog appends lines and trims to the configured max length", async () => {
  const { result } = renderHook(() => useBufferedLog(3));

  act(() => {
    result.current.appendLines(["one", "two", "three", "four"]);
  });

  await waitFor(() => {
    assert.equal(result.current.text, "two\nthree\nfour");
  });
});

test("useBufferedLog clears the buffered output", async () => {
  const { result } = renderHook(() => useBufferedLog(5));

  act(() => {
    result.current.appendLine("alpha");
  });

  await waitFor(() => {
    assert.equal(result.current.text, "alpha");
  });

  act(() => {
    result.current.clear();
  });

  await waitFor(() => {
    assert.equal(result.current.text, "");
  });
});
