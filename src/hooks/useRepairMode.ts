import { useCallback, useEffect, useMemo, useState, type Dispatch, type SetStateAction } from "react";

import {
  autoUnlockRepairMode,
  getRepairSessionStatus,
  listRepairTargets,
  lockRepairMode,
  type RepairSessionStatus,
  unlockRepairMode,
} from "../api";
import { formatErrorMessage, toErrorMessage } from "../errorUtils";

interface UseRepairModeOptions {
  setStatusMessage: (message: string) => void;
}

interface UseRepairModeResult {
  repairSession: RepairSessionStatus;
  setRepairSession: Dispatch<SetStateAction<RepairSessionStatus>>;
  selectedRepairTargetSid: string | null;
  setSelectedRepairTargetSid: Dispatch<SetStateAction<string | null>>;
  repairLoading: boolean;
  repairUnlocking: boolean;
  loadRepairTargets: () => Promise<void>;
  handleUnlockRepair: () => Promise<void>;
  handleLockRepair: () => Promise<void>;
}

const INITIAL_REPAIR_SESSION: RepairSessionStatus = {
  locked: true,
  connected: false,
  target_sid: null,
  requires_unlock: true,
};

/**
 * Manages repair-mode session state, available targets, and unlock/lock flows for
 * privileged machine repair actions.
 */
export function useRepairMode({ setStatusMessage }: UseRepairModeOptions): UseRepairModeResult {
  const [repairSession, setRepairSession] = useState<RepairSessionStatus>(INITIAL_REPAIR_SESSION);
  const [selectedRepairTargetSid, setSelectedRepairTargetSid] = useState<string | null>(null);
  const [repairLoading, setRepairLoading] = useState(true);
  const [repairUnlocking, setRepairUnlocking] = useState(false);

  const repairAppInstanceId = useMemo(
    () => globalThis.crypto?.randomUUID?.() ?? `srp-ui-${Date.now()}`,
    [],
  );
  const repairConnectionId = useMemo(
    () => globalThis.crypto?.randomUUID?.() ?? `srp-conn-${Date.now()}`,
    [],
  );

  const loadRepairTargets = useCallback(async () => {
    try {
      const targets = await listRepairTargets();
      if (targets.length > 0) {
        const activeTarget = targets.find((target) => target.is_loaded) || targets[0];
        setSelectedRepairTargetSid(activeTarget.sid);
        console.debug("Auto-selected target user:", activeTarget.account_name, activeTarget.sid);
      }
    } catch (error: unknown) {
      console.warn("Could not load repair targets:", error);
    }
  }, []);

  const refreshRepairContext = useCallback(async () => {
    setRepairLoading(true);
    try {
      let autoUnlockFailure: string | null = null;
      const [sessionResult, targetsResult] = await Promise.allSettled([
        getRepairSessionStatus(),
        listRepairTargets(),
      ]);
      let nextRepairSession = sessionResult.status === "fulfilled" ? sessionResult.value : null;
      if (sessionResult.status === "fulfilled") {
        setRepairSession(sessionResult.value);
      }
      if (targetsResult.status === "fulfilled" && targetsResult.value.length > 0) {
        const targets = targetsResult.value;
        const activeTarget = targets.find((target) => target.is_loaded) || targets[0];
        setSelectedRepairTargetSid(activeTarget.sid);
        console.debug("Auto-selected target user:", activeTarget.account_name, activeTarget.sid);
      }

      if (nextRepairSession?.locked) {
        try {
          const autoUnlocked = await autoUnlockRepairMode(repairAppInstanceId, repairConnectionId);
          nextRepairSession = autoUnlocked;
          setRepairSession(autoUnlocked);
          if (!autoUnlocked.locked) {
            setStatusMessage("Repair Mode unlocked automatically for this app session.");
          }
        } catch (autoUnlockError: unknown) {
          autoUnlockFailure = toErrorMessage(autoUnlockError);
          console.warn("Auto-unlock repair mode skipped:", autoUnlockError);
        }
      }

      const sessionFailure = sessionResult.status === "rejected" ? sessionResult.reason : null;
      const targetsFailure = targetsResult.status === "rejected" ? targetsResult.reason : null;
      const failure = sessionFailure ?? targetsFailure;
      if (failure) {
        setStatusMessage(`Repair context error: ${failure}`);
      } else if (autoUnlockFailure) {
        setStatusMessage(`Repair Mode stayed locked: ${autoUnlockFailure}`);
      } else if (nextRepairSession?.locked) {
        setStatusMessage("Repair Mode is locked.");
      }
    } catch (error: unknown) {
      setStatusMessage(formatErrorMessage("Repair context error", error));
    } finally {
      setRepairLoading(false);
    }
  }, [repairAppInstanceId, repairConnectionId, setStatusMessage]);

  useEffect(() => {
    void refreshRepairContext();
  }, [refreshRepairContext]);

  const handleUnlockRepair = useCallback(async () => {
    setRepairUnlocking(true);
    setStatusMessage("Unlocking Repair Mode...");
    try {
      const status = await unlockRepairMode(repairAppInstanceId, repairConnectionId);
      setRepairSession(status);
      setStatusMessage("Repair Mode unlocked for this app session.");
    } catch (error: unknown) {
      setStatusMessage(formatErrorMessage("Repair unlock error", error));
    } finally {
      setRepairUnlocking(false);
    }
  }, [repairAppInstanceId, repairConnectionId, setStatusMessage]);

  const handleLockRepair = useCallback(async () => {
    try {
      const status = await lockRepairMode();
      setRepairSession(status);
      setStatusMessage("Repair Mode locked.");
    } catch (error: unknown) {
      setStatusMessage(formatErrorMessage("Repair lock error", error));
    }
  }, [setStatusMessage]);

  return {
    repairSession,
    setRepairSession,
    selectedRepairTargetSid,
    setSelectedRepairTargetSid,
    repairLoading,
    repairUnlocking,
    loadRepairTargets,
    handleUnlockRepair,
    handleLockRepair,
  };
}
