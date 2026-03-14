export type RepairModeUiState = {
  locked: boolean;
  selectedTargetSid: string | null;
};

export function getRepairModeBadgeLabel(locked: boolean): string {
  return locked ? "Repair Mode: Locked" : "Repair Mode: Unlocked";
}

export function isMachineRepairEnabled(state: Pick<RepairModeUiState, "locked">): boolean {
  return !state.locked;
}

export function isProfileSensitiveActionEnabled(state: Required<Pick<RepairModeUiState, "locked">>): boolean {
  return !state.locked;
}

export function getProfileSensitiveActionHint(state: Required<Pick<RepairModeUiState, "locked">>): string {
  if (state.locked) {
    return "Unlock Repair Mode before running profile cleanup or app removal.";
  }

  return "";
}
