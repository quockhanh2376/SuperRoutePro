import { invoke } from "@tauri-apps/api/core";

// ======================== TYPES ========================

export interface NetworkInterface {
  index: string;
  ip: string;
  gateway: string;
  description: string;
}

export interface RouteEntry {
  destination: string;
  netmask: string;
  gateway: string;
  metric: string;
  interface_index: string;
}

export interface PingResult {
  success: boolean;
  latency_ms: number;
  output: string;
}

export interface FpingHostResult {
  target: string;
  success: boolean;
  latency_ms: number;
  output: string;
}

export interface FpingScanResult {
  sent: number;
  received: number;
  loss_percent: number;
  min_ms: number;
  avg_ms: number;
  max_ms: number;
  hosts: FpingHostResult[];
}

export interface CommandResult {
  success: boolean;
  output: string;
}

export interface BloatwareItem {
  package_name: string;
  label: string;
  installed: boolean;
}

// ======================== API CALLS ========================

export async function getNetworkInterfaces(activeOnly: boolean): Promise<NetworkInterface[]> {
  return invoke<NetworkInterface[]>("get_network_interfaces", { activeOnly });
}

export async function getRoutingTable(): Promise<RouteEntry[]> {
  return invoke<RouteEntry[]>("get_routing_table");
}

export async function addRoute(
  destination: string,
  mask: string,
  gateway: string,
  metric: string,
  interfaceIndex?: string
): Promise<CommandResult> {
  return invoke<CommandResult>("add_route", {
    destination,
    mask,
    gateway,
    metric,
    interfaceIndex: interfaceIndex || null,
  });
}

export async function deleteRoute(destination: string, mask: string): Promise<CommandResult> {
  return invoke<CommandResult>("delete_route", { destination, mask });
}

export async function flushRoutes(): Promise<CommandResult> {
  return invoke<CommandResult>("flush_routes");
}

export async function setDefaultGateway(
  gateway: string,
  interfaceIndex: string
): Promise<CommandResult> {
  return invoke<CommandResult>("set_default_gateway", { gateway, interfaceIndex });
}

export async function runNetworkCommand(command: string): Promise<CommandResult> {
  return invoke<CommandResult>("run_network_command", { command });
}

export async function pingHost(target: string, count?: number): Promise<PingResult> {
  return invoke<PingResult>("ping_host", { target, count: count || null });
}

export async function fpingScan(
  targets: string[],
  timeoutMs?: number
): Promise<FpingScanResult> {
  return invoke<FpingScanResult>("fping_scan", {
    targets,
    timeoutMs: timeoutMs || null,
  });
}

export async function checkInternet(): Promise<boolean> {
  return invoke<boolean>("check_internet");
}

export async function getBloatwareCandidates(): Promise<BloatwareItem[]> {
  return invoke<BloatwareItem[]>("get_bloatware_candidates");
}

export async function removeBloatware(packages: string[]): Promise<CommandResult> {
  return invoke<CommandResult>("remove_bloatware", { packages });
}

export async function clearCacheTargets(targets: string[]): Promise<CommandResult> {
  return invoke<CommandResult>("clear_cache_targets", { targets });
}
