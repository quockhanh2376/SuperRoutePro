import { useState } from "react";
import { X } from "lucide-react";

const DONATE_QR_IMAGE_PATH = "/donate-qr-vpbank.png";

interface DonateModalProps {
  open: boolean;
  onClose: () => void;
}

export function DonateModal({ open, onClose }: DonateModalProps) {
  const [donateQrLoadError, setDonateQrLoadError] = useState(false);

  if (!open) return null;

  return (
    <div
      className="fixed inset-0 z-50 bg-slate-950/60 flex items-center justify-center px-4"
      onClick={onClose}
    >
      <div
        className="donate-modal"
        onClick={(event) => event.stopPropagation()}
        role="dialog"
        aria-modal="true"
        aria-label="Donate to the author Zozon"
      >
        <div className="donate-modal-header">
          <div>
            <h3 className="text-base font-bold text-slate-100">Donate</h3>
            <p className="text-xs text-slate-400 mt-0.5">
              Donate to the author Zozon.
            </p>
          </div>
          <button
            type="button"
            onClick={onClose}
            className="donate-close-btn capsule-btn"
            title="Close"
          >
            <X className="w-4 h-4" />
          </button>
        </div>

        <div className="donate-modal-body">
          <div className="donate-qr-shell">
            <img
              src={DONATE_QR_IMAGE_PATH}
              alt="Donate QR code"
              className={`donate-qr-image ${donateQrLoadError ? "hidden" : ""}`}
              onLoad={() => setDonateQrLoadError(false)}
              onError={() => setDonateQrLoadError(true)}
            />
            {donateQrLoadError && (
              <div className="donate-qr-missing">
                Unable to load donate QR image at <code>{DONATE_QR_IMAGE_PATH}</code>.
              </div>
            )}
          </div>
        </div>
      </div>
    </div>
  );
}
