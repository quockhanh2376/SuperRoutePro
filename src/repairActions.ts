import {
  getRepairSessionStatus,
  runRepairMachineAction,
  type RepairMachineAction,
  type RepairSessionStatus,
} from "./api";

export interface RepairCommandResultLike {
  success: boolean;
  output: string;
  requires_unlock: boolean;
}

export interface RepairCommandResultOptions {
  refresh?: boolean;
  invalidateNicCache?: boolean;
  appendOutput?: boolean;
  successMessage?: string;
  failureMessage?: string;
}

interface RepairActionContext {
  appendCommandOutput: (title: string, output: string) => void;
  setStatusMessage: (message: string) => void;
  setRepairSession: (status: RepairSessionStatus) => void;
  loadData: (options?: { invalidateNicCache?: boolean }) => Promise<void>;
  setDiagnosticView: (view: "command" | "routing") => void;
}

export async function handleRepairCommandResult(
  context: Omit<RepairActionContext, "setDiagnosticView">,
  title: string,
  result: RepairCommandResultLike,
  options?: RepairCommandResultOptions,
): Promise<boolean> {
  if (options?.appendOutput !== false) {
    context.appendCommandOutput(title, result.output);
  }

  if (result.requires_unlock) {
    context.setStatusMessage("Unlock Repair Mode first to run admin fixes.");
    const status = await getRepairSessionStatus();
    context.setRepairSession(status);
    return false;
  }

  context.setStatusMessage(
    result.success
      ? (options?.successMessage ?? `${title} - Success!`)
      : (options?.failureMessage ?? `${title} - Failed`),
  );

  if (result.success && options?.refresh) {
    await context.loadData({ invalidateNicCache: options.invalidateNicCache });
  }
  return result.success;
}

export async function executeRepairAction(
  context: RepairActionContext,
  action: RepairMachineAction,
  title: string,
  options?: { refresh?: boolean; invalidateNicCache?: boolean },
): Promise<void> {
  context.setDiagnosticView("command");
  context.setStatusMessage(`Running ${title}...`);
  try {
    const result = await runRepairMachineAction(action);
    await handleRepairCommandResult(context, title, result, {
      appendOutput: true,
      refresh: options?.refresh,
      invalidateNicCache: options?.invalidateNicCache,
    });
  } catch (err) {
    context.appendCommandOutput(title, `Error: ${err}`);
    context.setStatusMessage(`Error: ${err}`);
  }
}
