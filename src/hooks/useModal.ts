import { useState, useCallback, useMemo } from "react";

/**
 * Custom hook for managing modal state with open/close handlers.
 * Eliminates the duplicate pattern of:
 *   const [modalOpen, setModalOpen] = useState(false);
 *   const handleOpenModal = useCallback(() => setModalOpen(true), []);
 *   const handleCloseModal = useCallback(() => setModalOpen(false), []);
 * 
 * @param onOpen - Optional callback to run when opening modal (e.g., load data)
 * @param onClose - Optional callback to run when closing modal
 * @param canClose - Optional function to check if modal can be closed (e.g., !loading)
 * @returns Object with isOpen state and open/close handlers
 * 
 * @example
 * // Simple modal
 * const donateModal = useModal();
 * 
 * // Modal with onOpen data loading
 * const batteryModal = useModal(loadBatterySummary);
 * 
 * // Modal with conditional close (e.g., prevent close while loading)
 * const batteryModal = useModal(loadBatterySummary, undefined, () => !batteryLoading);
 */
export function useModal(
  onOpen?: () => void,
  onClose?: () => void,
  canClose?: () => boolean
) {
  const [isOpen, setIsOpen] = useState(false);

  const open = useCallback(() => {
    setIsOpen(true);
    onOpen?.();
  }, [onOpen]);

  const close = useCallback(() => {
    if (canClose && !canClose()) return;
    setIsOpen(false);
    onClose?.();
  }, [onClose, canClose]);

  return useMemo(() => ({
    isOpen,
    open,
    close,
  }), [close, isOpen, open]);
}
