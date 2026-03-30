import { memo, type ElementType, type ReactNode, type RefObject } from "react";
import { Activity, ChevronDown, ChevronUp } from "lucide-react";

type OutputConsoleProps = {
  diagnosticView: "command" | "routing";
  routesCount: number;
  diagnosticsOutputText: string;
  pingOutputText: string;
  commandOutputRef: RefObject<HTMLPreElement | null>;
  pingOutputRef: RefObject<HTMLPreElement | null>;
  onShowCommand: () => void;
  onShowRouting: () => void;
  onClearCommand: () => void;
  onClearPing: () => void;
};

export const OutputConsole = memo(function OutputConsole({
  diagnosticView,
  routesCount,
  diagnosticsOutputText,
  pingOutputText,
  commandOutputRef,
  pingOutputRef,
  onShowCommand,
  onShowRouting,
  onClearCommand,
  onClearPing,
}: OutputConsoleProps) {
  return (
    <div className="output-console-shell flex flex-col flex-1 p-3 overflow-hidden">
      <div className="flex items-center gap-2 mb-2">
        <Activity className="w-4 h-4 text-blue-400" />
        <span className="text-xs font-bold text-slate-400 uppercase tracking-wider">Output Console</span>
        <span className="text-[0.62rem] text-slate-600 ml-auto">
          {diagnosticView === "routing" ? `${routesCount} routes snapshot` : "Command + Ping live logs"}
        </span>
      </div>

      <div className="output-console-grid flex-1 min-h-0">
        <div className="min-h-0 flex flex-col">
          <div className="flex items-center justify-between mb-1 gap-2">
            <span className="text-[0.72rem] text-slate-400 uppercase tracking-wider font-semibold">
              {diagnosticView === "routing" ? "Routing Table Output" : "Command Output"}
            </span>
            <div className="flex items-center gap-1.5">
              <button
                onClick={onShowCommand}
                className={`capsule-btn compact-pill console-chip console-chip-command ${
                  diagnosticView === "command" ? "console-chip-command-active" : ""
                }`}
              >
                Command
              </button>
              <button
                onClick={onShowRouting}
                className={`capsule-btn compact-pill console-chip console-chip-routing ${
                  diagnosticView === "routing" ? "console-chip-routing-active" : ""
                }`}
              >
                Routing
              </button>
              <button
                onClick={diagnosticView === "routing" ? onShowRouting : onClearCommand}
                className="capsule-btn compact-pill console-chip console-chip-refresh"
              >
                {diagnosticView === "routing" ? "Refresh" : "Clear"}
              </button>
            </div>
          </div>
          <pre
            ref={commandOutputRef}
            className="text-[0.76rem] font-mono bg-[#0c1220] border border-slate-700/50 rounded-xl p-3 flex-1 min-h-0 overflow-auto text-slate-300 whitespace-pre-wrap"
          >
            {diagnosticsOutputText}
          </pre>
        </div>

        <div className="min-h-0 flex flex-col">
          <div className="flex items-center justify-between mb-1 gap-2">
            <span className="text-[0.72rem] text-slate-400 uppercase tracking-wider font-semibold">
              Ping & Tracert Output
            </span>
            <button
              onClick={onClearPing}
              className="capsule-btn compact-pill output-clear-btn"
            >
              Clear
            </button>
          </div>
          <pre
            ref={pingOutputRef}
            className="text-[0.8rem] font-mono bg-[#0c1220] border border-slate-700/50 rounded-xl p-3 flex-1 min-h-0 overflow-auto text-slate-300 whitespace-pre-wrap"
          >
            {pingOutputText || "Ping log is ready. Click Start to run continuous ping."}
          </pre>
        </div>
      </div>
    </div>
  );
});

type FieldProps = {
  label: string;
  value: string;
  onChange: (value: string) => void;
  placeholder?: string;
};

export const Field = memo(function Field({ label, value, onChange, placeholder }: FieldProps) {
  return (
    <div>
      <label className="text-[0.6rem] text-slate-500 uppercase tracking-wider font-bold">{label}</label>
      <input
        type="text"
        value={value}
        onChange={(event) => onChange(event.target.value)}
        placeholder={placeholder}
        className="w-full mt-0.5 px-2.5 py-1.5 text-xs font-mono bg-[#0c1220] border border-slate-700/50 rounded-md focus:border-blue-500/50 focus:outline-none text-slate-200 placeholder:text-slate-700"
      />
    </div>
  );
});

type ActionBtnProps = {
  icon: ElementType;
  label: string;
  color: string;
  onClick: () => void;
  disabled?: boolean;
  compact?: boolean;
};

export const ActionBtn = memo(function ActionBtn({
  icon: Icon,
  label,
  color,
  onClick,
  disabled = false,
  compact = false,
}: ActionBtnProps) {
  const colors: Record<string, string> = {
    emerald: "bg-emerald-600/80 hover:bg-emerald-500 border-emerald-700/50",
    red: "bg-red-600/80 hover:bg-red-500 border-red-700/50",
    blue: "bg-blue-600/80 hover:bg-blue-500 border-blue-700/50",
    orange: "bg-orange-600/80 hover:bg-orange-500 border-orange-700/50",
    slate: "bg-slate-700/80 hover:bg-slate-600 border-slate-600/70",
  };
  const sizeClass = compact
    ? "action-btn-compact min-w-[54px] px-1.5 gap-1 py-1 text-[0.66rem]"
    : "min-w-[72px] px-2.5 gap-1.5 py-1.5 text-[0.76rem]";

  return (
    <button
      onClick={onClick}
      disabled={disabled}
      className={`capsule-btn flex items-center justify-center font-bold text-white border transition disabled:opacity-45 disabled:cursor-not-allowed ${sizeClass} ${colors[color] || colors.blue}`}
    >
      <Icon className={compact ? "w-3 h-3" : "w-3.5 h-3.5"} /> {label}
    </button>
  );
});

type SectionProps = {
  icon: ElementType;
  title: string;
  open: boolean;
  onToggle: () => void;
  children: ReactNode;
};

export const Section = memo(function Section({
  icon: Icon,
  title,
  open,
  onToggle,
  children,
}: SectionProps) {
  return (
    <div className="bg-[#1e293b]/50 border border-slate-700/30 rounded-xl overflow-hidden">
      <button
        onClick={onToggle}
        className="capsule-btn-soft flex items-center justify-between w-full px-4 py-3 hover:bg-slate-700/20 transition"
      >
        <div className="flex items-center gap-2">
          <Icon className="w-4 h-4 text-blue-400" />
          <span className="text-sm font-bold text-slate-300">{title}</span>
        </div>
        {open ? <ChevronUp className="w-4 h-4 text-slate-500" /> : <ChevronDown className="w-4 h-4 text-slate-500" />}
      </button>
      {open && <div className="px-4 pb-4">{children}</div>}
    </div>
  );
});

type ToolBtnProps = {
  icon: ElementType;
  label: string;
  desc: string;
  onClick: () => void;
  tone?: "safe" | "system" | "danger";
  compact?: boolean;
  disabled?: boolean;
};

export const ToolBtn = memo(function ToolBtn({
  icon: Icon,
  label,
  desc,
  onClick,
  tone,
  compact,
  disabled = false,
}: ToolBtnProps) {
  const toneClass = tone ?? "safe";
  return (
    <button
      onClick={onClick}
      disabled={disabled}
      className={`tool-card tool-card-${toneClass} ${compact ? "tool-card-compact" : ""} disabled:opacity-45 disabled:cursor-not-allowed`}
    >
      <span className="tool-icon-shell">
        <Icon className="w-3.5 h-3.5" />
      </span>
      <div className="min-w-0">
        <div className="tool-title">{label}</div>
        <div className="tool-desc">{desc}</div>
      </div>
    </button>
  );
});
