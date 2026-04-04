import { useCallback, useMemo, useRef, useState, type Dispatch, type SetStateAction } from "react";

import {
  getRepairSessionStatus,
  listRepairTargets,
  repairClearCacheTargets,
  type RepairSessionStatus,
} from "../api";
import {
  CACHE_CLEANUP_OPTIONS,
  DEFAULT_CACHE_SELECTION,
  type CacheCleanupOption,
} from "../constants/cacheTargets";
import { useModal } from "./useModal";
import { useProgressTracker } from "./useProgressTracker";

interface UseCacheCleanupManagerOptions {
  setStatusMessage: (message: string) => void;
  appendCommandOutput: (title: string, output: string) => void;
  openCommandDiagnostics: () => void;
  selectedRepairTargetSid: string | null;
  setSelectedRepairTargetSid: Dispatch<SetStateAction<string | null>>;
  setRepairSession: Dispatch<SetStateAction<RepairSessionStatus>>;
  loadRepairTargets: () => Promise<void>;
  openConfirm: (title: string, message: string, action: () => void | Promise<void>) => void;
}

interface UseCacheCleanupManagerResult {
  open: boolean;
  cleaning: boolean;
  stopPending: boolean;
  options: CacheCleanupOption[];
  selectedCaches: Set<string>;
  selectedCount: number;
  progressPercent: number;
  progressText: string;
  handleOpenModal: () => void;
  handleCloseModal: () => void;
  handleToggleCache: (cacheId: string) => void;
  handleSelectAll: () => void;
  handleClearSelection: () => void;
  handleForceStop: () => void;
  handleStartCleanup: () => void;
}

export function useCacheCleanupManager({
  setStatusMessage,
  appendCommandOutput,
  openCommandDiagnostics,
  selectedRepairTargetSid,
  setSelectedRepairTargetSid,
  setRepairSession,
  loadRepairTargets,
  openConfirm,
}: UseCacheCleanupManagerOptions): UseCacheCleanupManagerResult {
  const modal = useModal();
  const stopRequestedRef = useRef(false);
  const [cleaning, setCleaning] = useState(false);
  const [stopPending, setStopPending] = useState(false);
  const [selectedCaches, setSelectedCaches] = useState<Set<string>>(
    () => new Set(DEFAULT_CACHE_SELECTION),
  );
  const {
    percent: progressPercent,
    text: progressText,
    update: updateProgress,
    setMessage: setProgressText,
    reset: resetProgress,
  } = useProgressTracker();

  const selectedTargets = useMemo(
    () => CACHE_CLEANUP_OPTIONS.filter((option) => selectedCaches.has(option.id)),
    [selectedCaches],
  );

  const handleOpenModal = useCallback(() => {
    setSelectedCaches(new Set(DEFAULT_CACHE_SELECTION));
    resetProgress();
    setStopPending(false);
    stopRequestedRef.current = false;
    modal.open();
    void loadRepairTargets();
  }, [loadRepairTargets, modal, resetProgress]);

  const handleCloseModal = useCallback(() => {
    if (cleaning) return;
    modal.close();
  }, [cleaning, modal]);

  const handleToggleCache = useCallback((cacheId: string) => {
    setSelectedCaches((previous) => {
      const next = new Set(previous);
      if (next.has(cacheId)) {
        next.delete(cacheId);
      } else {
        next.add(cacheId);
      }
      return next;
    });
  }, []);

  const handleSelectAll = useCallback(() => {
    setSelectedCaches(new Set(CACHE_CLEANUP_OPTIONS.map((option) => option.id)));
  }, []);

  const handleClearSelection = useCallback(() => {
    setSelectedCaches(new Set());
  }, []);

  const handleForceStop = useCallback(() => {
    if (!cleaning || stopPending) return;
    stopRequestedRef.current = true;
    setStopPending(true);
    setStatusMessage("Force stop requested. Waiting for current task to finish...");
    setProgressText("Stopping... waiting for current task to finish.");
  }, [cleaning, setProgressText, setStatusMessage, stopPending]);

  const resolveTargetSid = useCallback(async (): Promise<string | null> => {
    if (selectedRepairTargetSid) {
      return selectedRepairTargetSid;
    }

    try {
      const targets = await listRepairTargets();
      if (targets.length === 0) {
        setProgressText("Error: No repair target found. Unlock Repair Mode first.");
        return null;
      }

      const activeTarget = targets.find((target) => target.is_loaded) || targets[0];
      setSelectedRepairTargetSid(activeTarget.sid);
      return activeTarget.sid;
    } catch {
      setProgressText("Error: Could not load repair targets. Unlock Repair Mode first.");
      return null;
    }
  }, [selectedRepairTargetSid, setProgressText, setSelectedRepairTargetSid]);

  const executeCleanup = useCallback(async () => {
    if (!selectedTargets.length) {
      setStatusMessage("Select at least one cache target");
      return;
    }

    const targetSid = await resolveTargetSid();
    if (!targetSid) {
      return;
    }

    setCleaning(true);
    setStopPending(false);
    stopRequestedRef.current = false;
    openCommandDiagnostics();
    updateProgress(0, `Starting cleanup... 0/${selectedTargets.length} (0%)`);
    setStatusMessage(`Cleaning ${selectedTargets.length} cache target(s)...`);

    let successCount = 0;
    let failedCount = 0;
    let processedCount = 0;

    try {
      for (let index = 0; index < selectedTargets.length; index += 1) {
        if (stopRequestedRef.current) {
          break;
        }

        const target = selectedTargets[index];
        const beforePercent = Math.round((index / selectedTargets.length) * 100);
        updateProgress(
          beforePercent,
          `Cleaning ${target.label}... ${index}/${selectedTargets.length} (${beforePercent}%)`,
        );

        try {
          const result = await repairClearCacheTargets(targetSid, [target.id]);
          appendCommandOutput(`Clear Cache - ${target.label}`, result.output);
          if (result.requires_unlock) {
            failedCount += 1;
            setStatusMessage("Unlock Repair Mode first to clean profile caches");
            const status = await getRepairSessionStatus();
            setRepairSession(status);
            break;
          }

          if (result.success) {
            successCount += 1;
          } else {
            failedCount += 1;
          }
        } catch (err) {
          failedCount += 1;
          appendCommandOutput(`Clear Cache - ${target.label}`, `Error: ${err}`);
        }

        processedCount = index + 1;
        const percent = Math.round((processedCount / selectedTargets.length) * 100);
        updateProgress(
          percent,
          `Processed ${processedCount}/${selectedTargets.length} (${percent}%)`,
        );

        if (stopRequestedRef.current) {
          break;
        }
      }

      const stoppedEarly = stopRequestedRef.current && processedCount < selectedTargets.length;
      if (stoppedEarly) {
        setStatusMessage(`Cleanup stopped by user (${processedCount}/${selectedTargets.length})`);
        setProgressText(
          `Stopped: processed ${processedCount}/${selectedTargets.length}, success ${successCount}, failed ${failedCount}`,
        );
      } else {
        setStatusMessage(
          failedCount === 0
            ? `Clear Cache completed (${successCount}/${selectedTargets.length})`
            : `Clear Cache completed with warnings (${failedCount} failed)`,
        );
        setProgressText(`Done: ${successCount} success, ${failedCount} failed`);
      }
    } catch (err) {
      appendCommandOutput("Clear Cache", `Error: ${err}`);
      setStatusMessage(`Clear Cache error: ${err}`);
      setProgressText("Cleanup aborted by error.");
    } finally {
      setCleaning(false);
      setStopPending(false);
      stopRequestedRef.current = false;
    }
  }, [
    appendCommandOutput,
    openCommandDiagnostics,
    resolveTargetSid,
    selectedTargets,
    setProgressText,
    setRepairSession,
    setStatusMessage,
    updateProgress,
  ]);

  const handleStartCleanup = useCallback(() => {
    if (selectedTargets.length === 0) {
      setStatusMessage("Select at least one cache target");
      return;
    }

    openConfirm(
      "Start Cache Cleanup",
      `Clean ${selectedTargets.length} selected cache target(s)?`,
      executeCleanup,
    );
  }, [executeCleanup, openConfirm, selectedTargets.length, setStatusMessage]);

  return {
    open: modal.isOpen,
    cleaning,
    stopPending,
    options: CACHE_CLEANUP_OPTIONS,
    selectedCaches,
    selectedCount: selectedTargets.length,
    progressPercent,
    progressText,
    handleOpenModal,
    handleCloseModal,
    handleToggleCache,
    handleSelectAll,
    handleClearSelection,
    handleForceStop,
    handleStartCleanup,
  };
}
