import test from "node:test";
import assert from "node:assert/strict";

import {
  getPersistStartupWriteMode,
  resolvePersistStartupEnabled,
} from "../src/persistStartupModel.ts";

test("disabling startup persistence clears persisted startup state", () => {
  assert.equal(getPersistStartupWriteMode(false), "clear");
});

test("enabling startup persistence saves startup config", () => {
  assert.equal(getPersistStartupWriteMode(true), "save");
});

test("startup checkbox stays enabled when persisted config is still active", () => {
  assert.equal(
    resolvePersistStartupEnabled({
      localPreference: false,
      legacyTaskEnabled: false,
      persistedConfigEnabled: true,
    }),
    true,
  );
});

test("startup checkbox stays enabled when a legacy startup task is still active", () => {
  assert.equal(
    resolvePersistStartupEnabled({
      localPreference: false,
      legacyTaskEnabled: true,
      persistedConfigEnabled: false,
    }),
    true,
  );
});

test("startup checkbox falls back to false when no persisted state remains", () => {
  assert.equal(
    resolvePersistStartupEnabled({
      localPreference: true,
      legacyTaskEnabled: false,
      persistedConfigEnabled: false,
    }),
    false,
  );
});
