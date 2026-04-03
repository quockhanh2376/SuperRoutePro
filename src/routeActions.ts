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
  if (!args.formDest || !args.formGw) {
    args.setStatusMessage("Please fill Destination and Gateway");
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
      failureMessage: "Add Route - Failed",
    });
  } catch (err) {
    args.setStatusMessage(`Error: ${err}`);
  }
}

export async function executeDeleteRouteAction(args: DeleteRouteArgs): Promise<void> {
  if (!args.formDest) {
    args.setStatusMessage("Please fill Destination IP");
    return;
  }

  args.setStatusMessage("Deleting route...");
  try {
    const result = await repairDeleteRoute(args.formDest, args.formMask);
    await args.handleRepairCommandResult("Delete Route", result, {
      appendOutput: true,
      refresh: true,
      successMessage: "Route deleted!",
      failureMessage: "Delete Route - Failed",
    });
  } catch (err) {
    args.setStatusMessage(`Error: ${err}`);
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
          failureMessage: "Persist Startup Config - Failed",
        });
      } catch (persistErr) {
        console.warn("Failed to save persist config:", persistErr);
      }
    } else {
      try {
        const persistConfigResult = await repairClearPersistConfig();
        await args.handleRepairCommandResult("Persist Startup Config", persistConfigResult, {
          appendOutput: true,
          successMessage: "Default gateway set. Persist on startup disabled.",
          failureMessage: "Persist Startup Config - Failed",
        });
      } catch (persistErr) {
        console.warn("Failed to disable persist config:", persistErr);
      }
    }
  } catch (err) {
    args.setStatusMessage(`Error: ${err}`);
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
      failureMessage: "Flush Routes - Failed",
    });
  } catch (err) {
    args.setStatusMessage(`Error: ${err}`);
  }
}
