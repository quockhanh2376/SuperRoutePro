import test from "node:test";
import assert from "node:assert/strict";

import {
  getProfileSensitiveActionHint,
  getRepairModeBadgeLabel,
  isMachineRepairEnabled,
  isProfileSensitiveActionEnabled,
} from "../src/repairModeModel.ts";

test("repair mode badge defaults to locked", () => {
  assert.equal(getRepairModeBadgeLabel(true), "Repair Mode: Locked");
  assert.equal(getRepairModeBadgeLabel(false), "Repair Mode: Unlocked");
});

test("machine repair actions stay disabled while repair mode is locked", () => {
  assert.equal(isMachineRepairEnabled({ locked: true }), false);
  assert.equal(isMachineRepairEnabled({ locked: false }), true);
});

test("profile-sensitive actions become enabled immediately after unlock", () => {
  assert.equal(
    isProfileSensitiveActionEnabled({ locked: false, selectedTargetSid: null }),
    true,
  );
  assert.equal(
    getProfileSensitiveActionHint({ locked: false, selectedTargetSid: null }),
    "",
  );
  assert.equal(
    isProfileSensitiveActionEnabled({ locked: false, selectedTargetSid: "S-1-5-21-1001" }),
    true,
  );
  assert.equal(
    getProfileSensitiveActionHint({ locked: true, selectedTargetSid: "S-1-5-21-1001" }),
    "Unlock Repair Mode before running profile cleanup or app removal.",
  );
});
