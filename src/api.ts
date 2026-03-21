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

export interface RepairCommandResult {
  success: boolean;
  output: string;
  requires_unlock: boolean;
}

export interface BloatwareItem {
  package_name: string;
  label: string;
  installed: boolean;
}

export interface BatteryReportResult {
  html: string;
}

export interface BatterySummaryResult {
  present: boolean;
  status: string;
  charge_percent: number | null;
  design_capacity_mwh: number | null;
  full_charge_capacity_mwh: number | null;
  health_percent: number | null;
  wear_percent: number | null;
  cycle_count: number | null;
  estimated_runtime_minutes: number | null;
  estimated_runtime_full_minutes: number | null;
  note: string;
}

export interface RepairSessionStatus {
  locked: boolean;
  connected: boolean;
  target_sid: string | null;
  requires_unlock: boolean;
}

export interface RepairServiceHealth {
  connected: boolean;
  requires_unlock: boolean;
  detail: string | null;
}

export interface RepairTargetUser {
  sid: string;
  account_name: string;
  profile_path: string;
  is_loaded: boolean;
}

export type RepairMachineAction =
  | "FlushDns"
  | "RenewDhcpLease"
  | "ClearArpCache"
  | "ResetTcpIp"
  | "ResetWinsock"
  | "ResetFirewall"
  | "ResetWinHttpProxy"
  | "RestartActiveAdapters";

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

export async function setWanPersistOnStartup(
  interfaceIndex: string,
  enabled: boolean
): Promise<CommandResult> {
  return invoke<CommandResult>("set_wan_persist_on_startup", { interfaceIndex, enabled });
}

export async function getWanPersistOnStartupStatus(): Promise<boolean> {
  return invoke<boolean>("get_wan_persist_on_startup_status");
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

export async function repairRemoveBloatware(
  targetSid: string,
  packages: string[],
  removeProvisioned: boolean,
): Promise<RepairCommandResult> {
  return invoke<RepairCommandResult>("repair_remove_bloatware", {
    targetSid,
    packages,
    removeProvisioned,
  });
}

export async function clearCacheTargets(targets: string[]): Promise<CommandResult> {
  return invoke<CommandResult>("clear_cache_targets", { targets });
}

export async function repairClearCacheTargets(
  targetSid: string,
  targets: string[],
): Promise<RepairCommandResult> {
  return invoke<RepairCommandResult>("repair_clear_cache_targets", {
    targetSid,
    targets,
  });
}

export async function getBatteryReport(): Promise<BatteryReportResult> {
  return invoke<BatteryReportResult>("get_battery_report");
}

export async function getBatterySummary(): Promise<BatterySummaryResult> {
  return invoke<BatterySummaryResult>("get_battery_summary");
}

export async function getRepairServiceHealth(): Promise<RepairServiceHealth> {
  return invoke<RepairServiceHealth>("get_repair_service_health");
}

export async function getRepairSessionStatus(): Promise<RepairSessionStatus> {
  return invoke<RepairSessionStatus>("get_repair_session_status");
}

export async function listRepairTargets(): Promise<RepairTargetUser[]> {
  return invoke<RepairTargetUser[]>("list_repair_targets");
}

export async function unlockRepairMode(
  appInstanceId: string,
  connectionId: string,
): Promise<RepairSessionStatus> {
  return invoke<RepairSessionStatus>("unlock_repair_mode", {
    appInstanceId,
    connectionId,
  });
}

export async function lockRepairMode(): Promise<RepairSessionStatus> {
  return invoke<RepairSessionStatus>("lock_repair_mode");
}

export async function repairAddRoute(
  destination: string,
  mask: string,
  gateway: string,
  metric: string,
  interfaceIndex?: string,
): Promise<RepairCommandResult> {
  return invoke<RepairCommandResult>("repair_add_route", {
    destination,
    mask,
    gateway,
    metric,
    interfaceIndex: interfaceIndex || null,
  });
}

export async function repairDeleteRoute(
  destination: string,
  mask: string,
): Promise<RepairCommandResult> {
  return invoke<RepairCommandResult>("repair_delete_route", { destination, mask });
}

export async function repairFlushRoutes(): Promise<RepairCommandResult> {
  return invoke<RepairCommandResult>("repair_flush_routes");
}

export async function repairSetDefaultGateway(
  gateway: string,
  interfaceIndex: string,
): Promise<RepairCommandResult> {
  return invoke<RepairCommandResult>("repair_set_default_gateway", {
    gateway,
    interfaceIndex,
  });
}

export async function repairSetWanPersistOnStartup(
  interfaceIndex: string,
  enabled: boolean,
): Promise<RepairCommandResult> {
  return invoke<RepairCommandResult>("repair_set_wan_persist_on_startup", {
    interfaceIndex,
    enabled,
  });
}

export async function runRepairMachineAction(
  action: RepairMachineAction,
): Promise<RepairCommandResult> {
  return invoke<RepairCommandResult>("repair_run_machine_action", { action });
}

// ======================== PERSIST CONFIG ========================

export interface NicIdentifier {
  description: string;
  mac_address: string;
}

export interface WanConfig {
  gateway: string;
  metric: string;
}

export interface CustomRoute {
  destination: string;
  mask: string;
  gateway: string;
  metric: string;
}

export interface PersistConfig {
  schema_version: number;
  enabled: boolean;
  nic: NicIdentifier;
  wan?: WanConfig;
  custom_routes: CustomRoute[];
  updated_at?: string;
}

export async function persistSaveConfig(
  config: PersistConfig,
): Promise<void> {
  return invoke<void>("persist_save_config", { config });
}

export async function persistLoadConfig(): Promise<PersistConfig | null> {
  return invoke<PersistConfig | null>("persist_load_config");
}

export async function persistGetNicStableId(
  interfaceIndex: string,
): Promise<NicIdentifier> {
  return invoke<NicIdentifier>("persist_get_nic_stable_id", { interfaceIndex });
}
