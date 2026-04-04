import { Component, type ErrorInfo, type ReactNode } from "react";

import { toErrorMessage } from "../errorUtils";

type AppErrorBoundaryProps = {
  children: ReactNode;
};

type AppErrorBoundaryState = {
  errorMessage: string | null;
};

/**
 * React still requires a class component for render-time error boundaries.
 */
export class AppErrorBoundary extends Component<
  AppErrorBoundaryProps,
  AppErrorBoundaryState
> {
  public state: AppErrorBoundaryState = {
    errorMessage: null,
  };

  public static getDerivedStateFromError(error: unknown): AppErrorBoundaryState {
    return {
      errorMessage: toErrorMessage(error),
    };
  }

  public componentDidCatch(error: unknown, errorInfo: ErrorInfo): void {
    console.error("SuperRoutePro render error:", error, errorInfo);
  }

  public render(): ReactNode {
    if (!this.state.errorMessage) {
      return this.props.children;
    }

    return (
      <div className="min-h-screen bg-slate-950 text-slate-100 flex items-center justify-center p-6">
        <div
          className="max-w-xl w-full rounded-3xl border border-red-500/30 bg-slate-900/90 shadow-2xl p-6"
          role="alert"
          aria-live="assertive"
        >
          <p className="text-xs uppercase tracking-[0.24em] text-red-300 font-semibold">
            Application Error
          </p>
          <h1 className="mt-3 text-2xl font-bold">Super Route Pro hit an unexpected problem.</h1>
          <p className="mt-3 text-sm leading-6 text-slate-300">
            The UI crashed before it could recover safely. Reload the app to restore a clean session.
          </p>
          <pre className="mt-4 rounded-2xl border border-slate-700/60 bg-slate-950/70 p-4 text-xs text-red-200 whitespace-pre-wrap">
            {this.state.errorMessage}
          </pre>
          <button
            type="button"
            onClick={() => window.location.reload()}
            className="mt-5 rounded-xl border border-red-400/35 bg-red-500/10 px-4 py-2 text-sm font-semibold text-red-100 transition hover:bg-red-500/20"
          >
            Reload Application
          </button>
        </div>
      </div>
    );
  }
}
