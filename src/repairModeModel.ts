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

export function isProfileSensitiveActionEnabled(state: RepairModeUiState): boolean {
  return !state.locked && Boolean(state.selectedTargetSid);
}

export function getProfileSensitiveActionHint(state: RepairModeUiState): string {
  if (state.locked) {
    return "Unlock Repair Mode before running profile cleanup or app removal.";
  }

  if (!state.selectedTargetSid) {
    return "Select a target user before running profile cleanup or app removal.";
  }

  return "";
}
