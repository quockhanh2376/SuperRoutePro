import { OctagonAlert } from "lucide-react";

interface ConfirmDialogProps {
  open: boolean;
  title: string;
  message: string;
  onConfirm: () => void;
  onCancel: () => void;
}

export function ConfirmDialog({
  open,
  title,
  message,
  onConfirm,
  onCancel,
}: ConfirmDialogProps) {
  if (!open) return null;

  return (
    <div className="fixed inset-0 z-50 bg-slate-950/60 flex items-center justify-center px-4">
      <div
        className="w-full max-w-md rounded-xl border border-slate-600 bg-slate-900 shadow-2xl"
        role="dialog"
        aria-modal="true"
        aria-labelledby="confirm-dialog-title"
      >
        <div className="flex items-center gap-2 px-4 py-3 border-b border-slate-700">
          <OctagonAlert className="w-4 h-4 text-amber-400" />
          <h3 id="confirm-dialog-title" className="text-sm font-bold text-slate-100">{title}</h3>
        </div>
        <div className="confirm-dialog-body px-4 py-4 text-sm">{message}</div>
        <div className="flex justify-end gap-2 px-4 py-3 border-t border-slate-700">
          <button
            type="button"
            onClick={onCancel}
            className="capsule-btn px-3 py-1.5 min-w-[84px] border border-slate-500 bg-slate-700/70 text-white font-semibold hover:bg-slate-600 transition"
          >
            Cancel
          </button>
          <button
            type="button"
            onClick={onConfirm}
            className="capsule-btn px-3 py-1.5 bg-blue-600 text-white hover:bg-blue-500 transition"
          >
            Confirm
          </button>
        </div>
      </div>
    </div>
  );
}
