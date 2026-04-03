import { type RouteEntry } from "../api";

export const ROUTE_TABLE_COLUMNS: Array<{ key: keyof RouteEntry; label: string; width: number }> = [
  { key: "destination", label: "Destination", width: 18 },
  { key: "netmask", label: "Netmask", width: 18 },
  { key: "gateway", label: "Gateway", width: 18 },
  { key: "metric", label: "Met", width: 6 },
  { key: "interface_index", label: "IF", width: 6 },
];

const formatRouteCell = (value: string, width: number) => {
  if (value.length <= width) {
    return value.padEnd(width, " ");
  }
  if (width <= 3) {
    return value.slice(0, width);
  }
  return `${value.slice(0, width - 3)}...`;
};

export const formatRoutingSnapshot = (routeData: RouteEntry[]) => {
  const stamp = new Date().toLocaleTimeString("en-GB");
  if (!routeData.length) {
    return `[${stamp}] Routing table snapshot\nNo routes found.`;
  }

  const header = ROUTE_TABLE_COLUMNS
    .map((column) => formatRouteCell(column.label, column.width))
    .join(" ");
  const divider = ROUTE_TABLE_COLUMNS
    .map((column) => "-".repeat(column.width))
    .join(" ");
  const rows = routeData.map((route) =>
    ROUTE_TABLE_COLUMNS
      .map((column) => formatRouteCell(String(route[column.key] ?? ""), column.width))
      .join(" ")
  );

  return [
    `[${stamp}] Routing table snapshot (${routeData.length} routes)`,
    header,
    divider,
    ...rows,
  ].join("\n");
};
