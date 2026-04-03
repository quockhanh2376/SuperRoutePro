import { useCallback, useRef, useState } from "react";

interface UseConfirmDialogOptions {
  onErrorMessage: (message: string) => void;
}

interface UseConfirmDialogResult {
  confirmOpen: boolean;
  confirmTitle: string;
  confirmMessage: string;
  openConfirm: (title: string, message: string, action: () => void | Promise<void>) => void;
  onConfirm: () => void;
  onCancelConfirm: () => void;
}

export function useConfirmDialog({ onErrorMessage }: UseConfirmDialogOptions): UseConfirmDialogResult {
  const [confirmOpen, setConfirmOpen] = useState(false);
  const [confirmTitle, setConfirmTitle] = useState("Confirm");
  const [confirmMessage, setConfirmMessage] = useState("");
  const confirmActionRef = useRef<(() => void | Promise<void>) | null>(null);

  const openConfirm = useCallback((title: string, message: string, action: () => void | Promise<void>) => {
    confirmActionRef.current = action;
    setConfirmTitle(title);
    setConfirmMessage(message);
    setConfirmOpen(true);
  }, []);

  const onConfirm = useCallback(() => {
    const action = confirmActionRef.current;
    confirmActionRef.current = null;
    setConfirmOpen(false);
    if (!action) return;
    Promise.resolve(action()).catch((err) => onErrorMessage(`Error: ${err}`));
  }, [onErrorMessage]);

  const onCancelConfirm = useCallback(() => {
    confirmActionRef.current = null;
    setConfirmOpen(false);
  }, []);

  return {
    confirmOpen,
    confirmTitle,
    confirmMessage,
    openConfirm,
    onConfirm,
    onCancelConfirm,
  };
}
