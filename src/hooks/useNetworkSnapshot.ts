import { useCallback, useRef, useState } from "react";

import {
  getNetworkSnapshot,
  invalidateNetworkAdapterCache,
  persistGetNicStableIds,
  type NetworkInterface,
  type RouteEntry,
} from "../api";
import { formatErrorMessage } from "../errorUtils";
import {
  mergeNicDescriptions,
  stabilizeNicSnapshotDescriptions,
  syncSelectedNicToList,
} from "../nicDescriptionModel";

type SnapshotState = {
  activeOnly: boolean;
  hasLoadedNicSnapshot: boolean;
  loading: boolean;
  nics: NetworkInterface[];
  routes: RouteEntry[];
  selectedNic: NetworkInterface | null;
};

interface UseNetworkSnapshotOptions {
  setStatusMessage: (message: string) => void;
}

interface UseNetworkSnapshotResult {
  activeOnly: boolean;
  hasLoadedNicSnapshot: boolean;
  loading: boolean;
  nics: NetworkInterface[];
  routes: RouteEntry[];
  selectedNic: NetworkInterface | null;
  loadData: (options?: { invalidateNicCache?: boolean }) => Promise<void>;
  setActiveOnly: (activeOnly: boolean) => void;
  setRoutes: (value: RouteEntry[] | ((current: RouteEntry[]) => RouteEntry[])) => void;
  setSelectedNic: (
    value:
      | NetworkInterface
      | null
      | ((current: NetworkInterface | null) => NetworkInterface | null)
  ) => void;
}

const INITIAL_SNAPSHOT_STATE: SnapshotState = {
  activeOnly: true,
  hasLoadedNicSnapshot: false,
  loading: true,
  nics: [],
  routes: [],
  selectedNic: null,
};

export function useNetworkSnapshot({
  setStatusMessage,
}: UseNetworkSnapshotOptions): UseNetworkSnapshotResult {
  const [snapshot, setSnapshot] = useState<SnapshotState>(INITIAL_SNAPSHOT_STATE);
  const latestLoadRequestRef = useRef(0);
  const latestNicsRef = useRef<NetworkInterface[]>([]);

  const setActiveOnly = useCallback((activeOnly: boolean) => {
    setSnapshot((current) => ({
      ...current,
      activeOnly,
    }));
  }, []);

  const setRoutes = useCallback((
    value: RouteEntry[] | ((current: RouteEntry[]) => RouteEntry[]),
  ) => {
    setSnapshot((current) => ({
      ...current,
      routes: typeof value === "function"
        ? (value as (current: RouteEntry[]) => RouteEntry[])(current.routes)
        : value,
    }));
  }, []);

  const setSelectedNic = useCallback((
    value:
      | NetworkInterface
      | null
      | ((current: NetworkInterface | null) => NetworkInterface | null),
  ) => {
    setSnapshot((current) => ({
      ...current,
      selectedNic: typeof value === "function"
        ? (value as (current: NetworkInterface | null) => NetworkInterface | null)(current.selectedNic)
        : value,
    }));
  }, []);

  const loadData = useCallback(async (options?: { invalidateNicCache?: boolean }) => {
    const requestId = latestLoadRequestRef.current + 1;
    latestLoadRequestRef.current = requestId;
    setSnapshot((current) => ({
      ...current,
      loading: true,
    }));
    setStatusMessage("Loading data...");

    try {
      if (options?.invalidateNicCache) {
        await invalidateNetworkAdapterCache();
        if (requestId !== latestLoadRequestRef.current) {
          return;
        }
      }

      const currentActiveOnly = snapshot.activeOnly;
      const networkSnapshot = await getNetworkSnapshot(currentActiveOnly);
      if (requestId !== latestLoadRequestRef.current) {
        return;
      }

      const stabilizedInterfaces = stabilizeNicSnapshotDescriptions(
        latestNicsRef.current,
        networkSnapshot.interfaces,
      );
      latestNicsRef.current = stabilizedInterfaces;

      setSnapshot((current) => ({
        ...current,
        hasLoadedNicSnapshot: true,
        loading: false,
        nics: stabilizedInterfaces,
        routes: networkSnapshot.routes,
        selectedNic: syncSelectedNicToList(current.selectedNic, stabilizedInterfaces),
      }));
      setStatusMessage(
        `Loaded ${stabilizedInterfaces.length} NICs, ${networkSnapshot.routes.length} routes`,
      );

      const interfaceIndexes = stabilizedInterfaces.map((nic) => nic.index);
      if (interfaceIndexes.length === 0) {
        return;
      }

      void (async () => {
        try {
          const stableIds = await persistGetNicStableIds(interfaceIndexes);
          if (requestId !== latestLoadRequestRef.current) {
            return;
          }

          const descriptionEntries = interfaceIndexes.map((interfaceIndex, index) => ({
            interfaceIndex,
            description: stableIds[index]?.description ?? "",
          }));

          setSnapshot((current) => {
            const enrichedNics = mergeNicDescriptions(current.nics, descriptionEntries);
            latestNicsRef.current = enrichedNics;
            const selectedNic = current.selectedNic
              ? mergeNicDescriptions([current.selectedNic], descriptionEntries)[0] ?? current.selectedNic
              : current.selectedNic;

            return {
              ...current,
              nics: enrichedNics,
              selectedNic,
            };
          });
        } catch (enrichError: unknown) {
          console.warn("Failed to enrich NIC descriptions:", enrichError);
        }
      })();
    } catch (error: unknown) {
      if (requestId !== latestLoadRequestRef.current) {
        return;
      }

      setSnapshot((current) => ({
        ...current,
        hasLoadedNicSnapshot: true,
        loading: false,
      }));
      setStatusMessage(formatErrorMessage("Loading network snapshot failed", error));
    }
  }, [setStatusMessage, snapshot.activeOnly]);

  return {
    activeOnly: snapshot.activeOnly,
    hasLoadedNicSnapshot: snapshot.hasLoadedNicSnapshot,
    loading: snapshot.loading,
    nics: snapshot.nics,
    routes: snapshot.routes,
    selectedNic: snapshot.selectedNic,
    loadData,
    setActiveOnly,
    setRoutes,
    setSelectedNic,
  };
}
