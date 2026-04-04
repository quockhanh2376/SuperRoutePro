import { X } from "lucide-react";

import { HELP_GUIDE_CONTENT, type HelpLanguage } from "../constants/helpContent";

interface HelpModalProps {
  open: boolean;
  language: HelpLanguage;
  onLanguageChange: (language: HelpLanguage) => void;
  onClose: () => void;
}

export function HelpModal({
  open,
  language,
  onLanguageChange,
  onClose,
}: HelpModalProps) {
  if (!open) {
    return null;
  }

  const helpContent = HELP_GUIDE_CONTENT[language];

  return (
    <div
      className="fixed inset-0 z-50 bg-slate-950/60 flex items-center justify-center px-4"
      onClick={onClose}
    >
      <div
        className="help-modal"
        onClick={(event) => event.stopPropagation()}
        role="dialog"
        aria-modal="true"
        aria-label="Application Help"
      >
        <div className="help-modal-header">
          <div className="help-modal-heading">
            <h3 className="text-base font-bold text-slate-100">{helpContent.modalTitle}</h3>
            <p className="text-xs text-slate-400 mt-0.5">
              {helpContent.modalSubtitle}
            </p>
          </div>
          <div className="help-lang-switch" role="group" aria-label="Help language">
            <button
              type="button"
              onClick={() => onLanguageChange("en")}
              className={`help-lang-btn capsule-btn ${language === "en" ? "help-lang-btn-active" : ""}`}
            >
              ENG
            </button>
            <button
              type="button"
              onClick={() => onLanguageChange("vi")}
              className={`help-lang-btn capsule-btn ${language === "vi" ? "help-lang-btn-active" : ""}`}
            >
              VN
            </button>
          </div>
          <button
            type="button"
            onClick={onClose}
            className="help-close-btn capsule-btn"
            title="Close"
          >
            <X className="w-4 h-4" />
          </button>
        </div>

        <div className="help-modal-body">
          {helpContent.sections.map((section) => (
            <section key={section.title} className="help-section">
              <h4 className="help-section-title">{section.title}</h4>
              <ul className="help-list">
                {section.items.map((item) => (
                  <li key={`${section.title}-${item.name}`} className="help-item">
                    <span className="help-item-name">{item.name}</span>
                    <span className="help-item-detail">{item.detail}</span>
                  </li>
                ))}
              </ul>
            </section>
          ))}
        </div>
      </div>
    </div>
  );
}
