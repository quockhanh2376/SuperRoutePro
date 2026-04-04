import { useCallback, useState } from "react";

import { getBatterySummary, type BatterySummaryResult } from "../api";
import { formatErrorMessage, toErrorMessage } from "../errorUtils";
import { getBatteryWearLevel } from "../batteryUtils";
import { useModal } from "./useModal";

interface UseBatterySummaryOptions {
  setStatusMessage: (message: string) => void;
}

interface UseBatterySummaryResult {
  error: string;
  loading: boolean;
  modal: {
    close: () => void;
    isOpen: boolean;
    open: () => void;
  };
  summary: BatterySummaryResult | null;
  handleCloseModal: () => void;
  handleOpenModal: () => void;
  loadSummary: () => Promise<void>;
}

export function useBatterySummary({
  setStatusMessage,
}: UseBatterySummaryOptions): UseBatterySummaryResult {
  const modal = useModal();
  const [loading, setLoading] = useState(false);
  const [summary, setSummary] = useState<BatterySummaryResult | null>(null);
  const [error, setError] = useState("");

  const loadSummary = useCallback(async () => {
    setLoading(true);
    setError("");

    try {
      const nextSummary = await getBatterySummary();
      setSummary(nextSummary);
      if (nextSummary.present) {
        setStatusMessage(
          `Battery summary loaded (${getBatteryWearLevel(nextSummary.wear_percent)})`,
        );
      } else {
        setStatusMessage("Battery summary loaded (no battery detected)");
      }
    } catch (loadError: unknown) {
      setSummary(null);
      setError(toErrorMessage(loadError));
      setStatusMessage(formatErrorMessage("Battery summary error", loadError));
    } finally {
      setLoading(false);
    }
  }, [setStatusMessage]);

  const handleOpenModal = useCallback(() => {
    modal.open();
    void loadSummary();
  }, [loadSummary, modal]);

  const handleCloseModal = useCallback(() => {
    if (loading) {
      return;
    }
    modal.close();
  }, [loading, modal]);

  return {
    error,
    loading,
    modal,
    summary,
    handleCloseModal,
    handleOpenModal,
    loadSummary,
  };
}
