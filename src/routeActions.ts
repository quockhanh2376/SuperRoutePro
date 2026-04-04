import {
  persistGetNicStableIds,
  repairAddRoute,
  repairClearPersistConfig,
  repairDeleteRoute,
  repairFlushRoutes,
  repairSavePersistConfig,
  repairSetDefaultGateway,
  type NetworkInterface,
  type PersistConfig,
  type RouteEntry,
} from "./api";
import { formatErrorMessage, getFirstValidationError } from "./errorUtils";
import { validateRouteDeleteInput, validateRouteForm } from "./networkValidation";
import { buildPersistCustomRoutes, getPersistRouteInterfaceIndexes } from "./persistRouteModel";
import { getPersistStartupWriteMode } from "./persistStartupModel";
import {
  type RepairCommandResultLike,
  type RepairCommandResultOptions,
} from "./repairActions";

interface RouteActionContext {
  setStatusMessage: (message: string) => void;
  handleRepairCommandResult: (
    title: string,
    result: RepairCommandResultLike,
    options?: RepairCommandResultOptions,
  ) => Promise<boolean>;
}

interface AddRouteArgs extends RouteActionContext {
  formDest: string;
  formMask: string;
  formGw: string;
  formMetric: string;
  selectedNicIndex?: string;
}

interface DeleteRouteArgs extends RouteActionContext {
  formDest: string;
  formMask: string;
}

interface SetInternetArgs extends RouteActionContext {
  selectedNic: NetworkInterface | null;
  persistWanOnStartup: boolean;
  routes: RouteEntry[];
}

interface FlushRoutesArgs extends RouteActionContext {}

export async function executeAddRouteAction(args: AddRouteArgs): Promise<void> {
  const validationError = getFirstValidationError(validateRouteForm({
    dest: args.formDest,
    mask: args.formMask,
    gw: args.formGw,
    metric: args.formMetric,
  }));
  if (validationError) {
    args.setStatusMessage(validationError);
    return;
  }

  args.setStatusMessage("Adding route...");
  try {
    const result = await repairAddRoute(
      args.formDest,
      args.formMask,
      args.formGw,
      args.formMetric,
      args.selectedNicIndex,
    );
    await args.handleRepairCommandResult("Add Route", result, {
      appendOutput: true,
      refresh: true,
      successMessage: "Route added successfully!",
      failureMessage: "Add Route failed.",
    });
  } catch (error: unknown) {
    args.setStatusMessage(formatErrorMessage("Add Route failed", error));
  }
}

export async function executeDeleteRouteAction(args: DeleteRouteArgs): Promise<void> {
  const validationError = getFirstValidationError(validateRouteDeleteInput({
    dest: args.formDest,
    mask: args.formMask,
  }));
  if (validationError) {
    args.setStatusMessage(validationError);
    return;
  }

  args.setStatusMessage("Deleting route...");
  try {
    const result = await repairDeleteRoute(args.formDest, args.formMask);
    await args.handleRepairCommandResult("Delete Route", result, {
      appendOutput: true,
      refresh: true,
      successMessage: "Route deleted!",
      failureMessage: "Delete Route failed.",
    });
  } catch (error: unknown) {
    args.setStatusMessage(formatErrorMessage("Delete Route failed", error));
  }
}

export async function executeSetInternetAction(args: SetInternetArgs): Promise<void> {
  const { selectedNic, persistWanOnStartup, routes } = args;
  if (!selectedNic || !selectedNic.gateway) {
    args.setStatusMessage("Select a NIC with a gateway first");
    return;
  }

  args.setStatusMessage("Setting default gateway...");
  try {
    const gatewayResult = await repairSetDefaultGateway(selectedNic.gateway, selectedNic.index);
    const gatewayApplied = await args.handleRepairCommandResult(
      "Set Default Gateway",
      gatewayResult,
      {
        appendOutput: true,
        successMessage: "Default gateway set.",
        failureMessage: "Set Default Gateway - Failed",
      },
    );
    if (!gatewayApplied) {
      return;
    }

    const persistWriteMode = getPersistStartupWriteMode(persistWanOnStartup);
    if (persistWriteMode === "save") {
      try {
        const persistRouteInterfaceIndexes = getPersistRouteInterfaceIndexes(routes);
        const stableIdIndexes = Array.from(new Set([selectedNic.index, ...persistRouteInterfaceIndexes]));
        const stableIds = await persistGetNicStableIds(stableIdIndexes);
        const nicId = stableIds[0];
        const routeNicEntries = new Map(
          [
            [selectedNic.index, nicId] as const,
            ...stableIdIndexes.slice(1).map((interfaceIndex, index) => [
              interfaceIndex,
              stableIds[index + 1],
            ] as const),
          ],
        );
        const config: PersistConfig = {
          schema_version: 1,
          enabled: true,
          nic: nicId,
          wan: { gateway: selectedNic.gateway, metric: "1" },
          custom_routes: buildPersistCustomRoutes(routes, routeNicEntries),
          updated_at: new Date().toISOString(),
        };
        const persistConfigResult = await repairSavePersistConfig(config);
        await args.handleRepairCommandResult("Persist Startup Config", persistConfigResult, {
          appendOutput: true,
          successMessage: "Default gateway set. Persist on startup enabled.",
          failureMessage: "Persist Startup Config failed.",
        });
      } catch (persistError: unknown) {
        console.warn("Failed to save persist config:", persistError);
      }
    } else {
      try {
        const persistConfigResult = await repairClearPersistConfig();
        await args.handleRepairCommandResult("Persist Startup Config", persistConfigResult, {
          appendOutput: true,
          successMessage: "Default gateway set. Persist on startup disabled.",
          failureMessage: "Persist Startup Config failed.",
        });
      } catch (persistError: unknown) {
        console.warn("Failed to disable persist config:", persistError);
      }
    }
  } catch (error: unknown) {
    args.setStatusMessage(formatErrorMessage("Set Default Gateway failed", error));
  }
}

export async function executeFlushRoutesAction(args: FlushRoutesArgs): Promise<void> {
  args.setStatusMessage("Flushing routes...");
  try {
    const result = await repairFlushRoutes();
    await args.handleRepairCommandResult("Flush Routes", result, {
      appendOutput: true,
      refresh: true,
      successMessage: "All routes flushed!",
      failureMessage: "Flush Routes failed.",
    });
  } catch (error: unknown) {
    args.setStatusMessage(formatErrorMessage("Flush Routes failed", error));
  }
}
