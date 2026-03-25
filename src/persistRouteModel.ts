import type { CustomRoute, NicIdentifier, RouteEntry } from "./api";

export function isPersistableCustomRoute(route: RouteEntry): boolean {
  return (
    route.destination !== "0.0.0.0" &&
    route.gateway !== "" &&
    route.gateway !== "0.0.0.0" &&
    route.gateway !== "On-link" &&
    route.interface_index.trim() !== ""
  );
}

export function getPersistRouteInterfaceIndexes(routes: RouteEntry[]): string[] {
  const uniqueIndexes = new Set<string>();
  for (const route of routes) {
    if (isPersistableCustomRoute(route)) {
      uniqueIndexes.add(route.interface_index);
    }
  }
  return [...uniqueIndexes];
}

export function buildPersistCustomRoutes(
  routes: RouteEntry[],
  nicByInterfaceIndex: Map<string, NicIdentifier>,
): CustomRoute[] {
  return routes
    .filter(isPersistableCustomRoute)
    .map((route) => {
      const nic = nicByInterfaceIndex.get(route.interface_index);
      return {
        destination: route.destination,
        mask: route.netmask,
        gateway: route.gateway,
        metric: route.metric,
        ...(nic ? { nic } : {}),
      };
    });
}
