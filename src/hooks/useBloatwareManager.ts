import { useCallback, useMemo, useState, type Dispatch, type SetStateAction } from "react";

import {
  getBloatwareCandidates,
  getRepairSessionStatus,
  listRepairTargets,
  repairRemoveBloatware,
  type BloatwareItem,
  type RepairSessionStatus,
} from "../api";
import { formatErrorMessage, formatOutputError } from "../errorUtils";
import { useModal } from "./useModal";
import { useProgressTracker } from "./useProgressTracker";

interface UseBloatwareManagerOptions {
  setStatusMessage: (message: string) => void;
  appendCommandOutput: (title: string, output: string) => void;
  openCommandDiagnostics: () => void;
  selectedRepairTargetSid: string | null;
  setSelectedRepairTargetSid: Dispatch<SetStateAction<string | null>>;
  setRepairSession: Dispatch<SetStateAction<RepairSessionStatus>>;
  loadRepairTargets: () => Promise<void>;
  openConfirm: (title: string, message: string, action: () => void | Promise<void>) => void;
}

interface UseBloatwareManagerResult {
  open: boolean;
  loading: boolean;
  removing: boolean;
  items: BloatwareItem[];
  selectedPackages: Set<string>;
  selectedCount: number;
  installedCount: number;
  progressPercent: number;
  progressText: string;
  handleOpenModal: () => void;
  handleCloseModal: () => void;
  handleTogglePackage: (packageName: string) => void;
  handleSelectInstalled: () => void;
  handleSelectAll: () => void;
  handleClearSelection: () => void;
  handleRemoveSelected: () => void;
}

export function useBloatwareManager({
  setStatusMessage,
  appendCommandOutput,
  openCommandDiagnostics,
  selectedRepairTargetSid,
  setSelectedRepairTargetSid,
  setRepairSession,
  loadRepairTargets,
  openConfirm,
}: UseBloatwareManagerOptions): UseBloatwareManagerResult {
  const modal = useModal();
  const [loading, setLoading] = useState(false);
  const [removing, setRemoving] = useState(false);
  const [items, setItems] = useState<BloatwareItem[]>([]);
  const [selectedPackages, setSelectedPackages] = useState<Set<string>>(new Set());
  const {
    percent: progressPercent,
    text: progressText,
    update: updateProgress,
    setMessage: setProgressText,
    reset: resetProgress,
  } = useProgressTracker();

  const loadBloatwareList = useCallback(async () => {
    setLoading(true);
    try {
      const nextItems = await getBloatwareCandidates();
      setItems(nextItems);
      setSelectedPackages((previous) => {
        if (previous.size === 0) return previous;

        const available = new Set(nextItems.map((item) => item.package_name));
        const next = new Set<string>();
        previous.forEach((name) => {
          if (available.has(name)) {
            next.add(name);
          }
        });
        return next;
      });
    } catch (error: unknown) {
      setStatusMessage(formatErrorMessage("Bloatware list error", error));
    } finally {
      setLoading(false);
    }
  }, [setStatusMessage]);

  const handleOpenModal = useCallback(() => {
    resetProgress();
    modal.open();
    void loadBloatwareList();
    void loadRepairTargets();
  }, [loadBloatwareList, loadRepairTargets, modal, resetProgress]);

  const handleCloseModal = useCallback(() => {
    if (removing) return;
    modal.close();
  }, [modal, removing]);

  const handleTogglePackage = useCallback((packageName: string) => {
    setSelectedPackages((previous) => {
      const next = new Set(previous);
      if (next.has(packageName)) {
        next.delete(packageName);
      } else {
        next.add(packageName);
      }
      return next;
    });
  }, []);

  const handleSelectInstalled = useCallback(() => {
    const next = new Set<string>();
    for (const item of items) {
      if (item.installed) {
        next.add(item.package_name);
      }
    }
    setSelectedPackages(next);
  }, [items]);

  const handleSelectAll = useCallback(() => {
    setSelectedPackages(new Set(items.map((item) => item.package_name)));
  }, [items]);

  const handleClearSelection = useCallback(() => {
    setSelectedPackages(new Set());
  }, []);

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

  const executeRemoveSelected = useCallback(async () => {
    const packages = Array.from(selectedPackages);
    if (!packages.length) {
      setStatusMessage("Select at least one app to remove");
      return;
    }

    const targetSid = await resolveTargetSid();
    if (!targetSid) {
      return;
    }

    setRemoving(true);
    openCommandDiagnostics();
    updateProgress(0, `Starting removal... 0/${packages.length} (0%)`);
    setStatusMessage(`Removing ${packages.length} selected app(s)...`);

    let successCount = 0;
    let failedCount = 0;

    try {
      for (let index = 0; index < packages.length; index += 1) {
        const packageName = packages[index];
        const appLabel = items.find((item) => item.package_name === packageName)?.label ?? packageName;
        const beforePercent = Math.round((index / packages.length) * 100);
        updateProgress(
          beforePercent,
          `Removing ${appLabel}... ${index}/${packages.length} (${beforePercent}%)`,
        );

        try {
          const result = await repairRemoveBloatware(targetSid, [packageName], true);
          appendCommandOutput(`Remove Apps - ${appLabel}`, result.output);
          if (result.requires_unlock) {
            failedCount += 1;
            setStatusMessage("Unlock Repair Mode first to remove apps");
            setProgressText("Error: Repair Mode is locked. Unlock first, then retry.");
            const status = await getRepairSessionStatus();
            setRepairSession(status);
            break;
          }

          if (result.success) {
            successCount += 1;
          } else {
            failedCount += 1;
          }
        } catch (error: unknown) {
          failedCount += 1;
          appendCommandOutput(`Remove Apps - ${appLabel}`, formatOutputError(error));
        }

        const processed = index + 1;
        const percent = Math.round((processed / packages.length) * 100);
        updateProgress(percent, `Processed ${processed}/${packages.length} (${percent}%)`);
      }

      setStatusMessage(
        failedCount === 0
          ? `Remove Apps completed (${successCount}/${packages.length})`
          : `Remove Apps completed with warnings (${failedCount} failed)`,
      );
      setProgressText(`Done: ${successCount} success, ${failedCount} failed`);
      setSelectedPackages(new Set());
      await loadBloatwareList();
    } catch (error: unknown) {
      appendCommandOutput("Remove Apps", formatOutputError(error));
      setStatusMessage(formatErrorMessage("Remove Apps error", error));
      setProgressText("Removal aborted by error.");
    } finally {
      setRemoving(false);
    }
  }, [
    appendCommandOutput,
    items,
    loadBloatwareList,
    openCommandDiagnostics,
    resolveTargetSid,
    selectedPackages,
    setProgressText,
    setRepairSession,
    setStatusMessage,
    updateProgress,
  ]);

  const handleRemoveSelected = useCallback(() => {
    if (selectedPackages.size === 0) {
      setStatusMessage("Select at least one app to remove");
      return;
    }

    openConfirm(
      "Remove Selected Apps",
      `Remove ${selectedPackages.size} selected app(s)? This operation may require Administrator privileges.`,
      executeRemoveSelected,
    );
  }, [executeRemoveSelected, openConfirm, selectedPackages.size, setStatusMessage]);

  const installedCount = useMemo(
    () => items.filter((item) => item.installed).length,
    [items],
  );

  return {
    open: modal.isOpen,
    loading,
    removing,
    items,
    selectedPackages,
    selectedCount: selectedPackages.size,
    installedCount,
    progressPercent,
    progressText,
    handleOpenModal,
    handleCloseModal,
    handleTogglePackage,
    handleSelectInstalled,
    handleSelectAll,
    handleClearSelection,
    handleRemoveSelected,
  };
}
