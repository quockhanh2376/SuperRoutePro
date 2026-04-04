import "./setupDom.ts";

import assert from "node:assert/strict";
import { afterEach, test } from "node:test";
import { act, cleanup, renderHook } from "@testing-library/react";

import { useModal } from "../src/hooks/useModal.ts";

afterEach(() => {
  cleanup();
});

test("useModal opens and closes with stable state updates", () => {
  const { result } = renderHook(() => useModal());

  assert.equal(result.current.isOpen, false);

  act(() => {
    result.current.open();
  });
  assert.equal(result.current.isOpen, true);

  act(() => {
    result.current.close();
  });
  assert.equal(result.current.isOpen, false);
});

test("useModal runs lifecycle callbacks and obeys canClose guard", () => {
  const events: string[] = [];
  let allowClose = false;
  const { result } = renderHook(() => useModal(
    () => {
      events.push("open");
    },
    () => {
      events.push("close");
    },
    () => allowClose,
  ));

  act(() => {
    result.current.open();
  });
  assert.deepEqual(events, ["open"]);

  act(() => {
    result.current.close();
  });
  assert.equal(result.current.isOpen, true);
  assert.deepEqual(events, ["open"]);

  allowClose = true;
  act(() => {
    result.current.close();
  });
  assert.equal(result.current.isOpen, false);
  assert.deepEqual(events, ["open", "close"]);
});
