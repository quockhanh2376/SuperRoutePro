import { invoke } from "@tauri-apps/api/core";
import { toErrorMessage } from "./errorUtils";

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

export interface NetworkSnapshot {
  interfaces: NetworkInterface[];
  routes: RouteEntry[];
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

export interface SpeedTestProgress {
  stage: string;
  percent: number;
  current_speed_mbps: number;
  message: string;
}

export interface SpeedTestTargetOption {
  id: string;
  label: string;
  description: string;
  provider: string;
  region_label?: string;
}

export interface SpeedTestResult {
  target_id: string;
  target_label: string;
  provider: string;
  region_label: string;
  route_fit?: "preferred_region" | "global_fallback" | "pending";
  resolved_colo?: string | null;
  server_label: string;
  download_mbps: number;
  upload_mbps: number;
  ping_ms: number;
  jitter_ms: number;
  latency_samples?: number;
  successful_latency_samples?: number;
  stable_latency_samples?: number;
  download_bytes?: number;
  upload_bytes?: number;
  download_elapsed_ms?: number;
  upload_elapsed_ms?: number;
  ip: string;
  timestamp: string;
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

async function invokeCommand<T>(
  command: string,
  args?: Record<string, unknown>,
): Promise<T> {
  try {
    return await invoke<T>(command, args);
  } catch (error: unknown) {
    throw new Error(toErrorMessage(error));
  }
}

/**
 * Loads the filtered NIC list from the Rust backend.
 */
export async function getNetworkInterfaces(activeOnly: boolean): Promise<NetworkInterface[]> {
  return invokeCommand<NetworkInterface[]>("get_network_interfaces", { activeOnly });
}

/**
 * Loads NICs and routes together so the UI can stay in sync.
 */
export async function getNetworkSnapshot(activeOnly: boolean): Promise<NetworkSnapshot> {
  return invokeCommand<NetworkSnapshot>("get_network_snapshot", { activeOnly });
}

/**
 * Clears the backend NIC cache before the next snapshot refresh.
 */
export async function invalidateNetworkAdapterCache(): Promise<void> {
  return invokeCommand<void>("invalidate_network_adapter_cache");
}

/**
 * Fetches the current Windows routing table.
 */
export async function getRoutingTable(): Promise<RouteEntry[]> {
  return invokeCommand<RouteEntry[]>("get_routing_table");
}

/**
 * Adds a route through the non-elevated command path.
 */
export async function addRoute(
  destination: string,
  mask: string,
  gateway: string,
  metric: string,
  interfaceIndex?: string
): Promise<CommandResult> {
  return invokeCommand<CommandResult>("add_route", {
    destination,
    mask,
    gateway,
    metric,
    interfaceIndex: interfaceIndex || null,
  });
}

/**
 * Deletes a route through the non-elevated command path.
 */
export async function deleteRoute(destination: string, mask: string): Promise<CommandResult> {
  return invokeCommand<CommandResult>("delete_route", { destination, mask });
}

/**
 * Flushes all routes through the non-elevated command path.
 */
export async function flushRoutes(): Promise<CommandResult> {
  return invokeCommand<CommandResult>("flush_routes");
}

/**
 * Sets a NIC gateway through the non-elevated command path.
 */
export async function setDefaultGateway(
  gateway: string,
  interfaceIndex: string
): Promise<CommandResult> {
  return invokeCommand<CommandResult>("set_default_gateway", { gateway, interfaceIndex });
}

/**
 * Runs an arbitrary network command and captures stdout/stderr.
 */
export async function runNetworkCommand(command: string): Promise<CommandResult> {
  return invokeCommand<CommandResult>("run_network_command", { command });
}

/**
 * Executes a single ping request.
 */
export async function pingHost(target: string, count?: number): Promise<PingResult> {
  return invokeCommand<PingResult>("ping_host", { target, count: count || null });
}

/**
 * Tests a TCP host:port from the backend so PowerShell stays centralized.
 */
export async function testTcpPort(host: string, port: number): Promise<CommandResult> {
  return invokeCommand<CommandResult>("test_tcp_port", { host, port });
}

/**
 * Runs a batch ICMP scan for one subnet chunk.
 */
export async function fpingScan(
  targets: string[],
  timeoutMs?: number
): Promise<FpingScanResult> {
  return invokeCommand<FpingScanResult>("fping_scan", {
    targets,
    timeoutMs: timeoutMs || null,
  });
}

/**
 * Lists speed-test targets supported by the native backend.
 */
export async function listSpeedTestTargets(): Promise<SpeedTestTargetOption[]> {
  return invokeCommand<SpeedTestTargetOption[]>("list_speed_test_targets");
}

/**
 * Starts the native speed test.
 */
export async function runSpeedTest(
  downloadMb?: number,
  targetId?: string,
): Promise<SpeedTestResult> {
  return invokeCommand<SpeedTestResult>("run_speed_test", {
    downloadMb: downloadMb || null,
    targetId: targetId || null,
  });
}

/**
 * Checks whether the machine currently has internet access.
 */
export async function checkInternet(): Promise<boolean> {
  return invokeCommand<boolean>("check_internet");
}

/**
 * Loads removable Windows app candidates from the repair broker.
 */
export async function getBloatwareCandidates(): Promise<BloatwareItem[]> {
  return invokeCommand<BloatwareItem[]>("get_bloatware_candidates");
}

/**
 * Removes selected Windows apps for the chosen repair target user.
 */
export async function repairRemoveBloatware(
  targetSid: string,
  packages: string[],
  removeProvisioned: boolean,
): Promise<RepairCommandResult> {
  return invokeCommand<RepairCommandResult>("repair_remove_bloatware", {
    targetSid,
    packages,
    removeProvisioned,
  });
}

/**
 * Clears selected cache targets for the chosen repair target user.
 */
export async function repairClearCacheTargets(
  targetSid: string,
  targets: string[],
): Promise<RepairCommandResult> {
  return invokeCommand<RepairCommandResult>("repair_clear_cache_targets", {
    targetSid,
    targets,
  });
}

/**
 * Reads the generated battery report HTML.
 */
export async function getBatteryReport(): Promise<BatteryReportResult> {
  return invokeCommand<BatteryReportResult>("get_battery_report");
}

/**
 * Returns the condensed battery health summary used by the modal.
 */
export async function getBatterySummary(): Promise<BatterySummaryResult> {
  return invokeCommand<BatterySummaryResult>("get_battery_summary");
}

/**
 * Fetches repair service connectivity and lock state.
 */
export async function getRepairServiceHealth(): Promise<RepairServiceHealth> {
  return invokeCommand<RepairServiceHealth>("get_repair_service_health");
}

/**
 * Returns the current repair session state for this UI client.
 */
export async function getRepairSessionStatus(): Promise<RepairSessionStatus> {
  return invokeCommand<RepairSessionStatus>("get_repair_session_status");
}

/**
 * Tries to auto-unlock repair mode for the current app instance.
 */
export async function autoUnlockRepairMode(
  appInstanceId: string,
  connectionId: string,
): Promise<RepairSessionStatus> {
  return invokeCommand<RepairSessionStatus>("auto_unlock_repair_mode", {
    appInstanceId,
    connectionId,
  });
}

/**
 * Lists available user targets that support profile-sensitive repair actions.
 */
export async function listRepairTargets(): Promise<RepairTargetUser[]> {
  return invokeCommand<RepairTargetUser[]>("list_repair_targets");
}

/**
 * Explicitly unlocks repair mode for the current UI client.
 */
export async function unlockRepairMode(
  appInstanceId: string,
  connectionId: string,
): Promise<RepairSessionStatus> {
  return invokeCommand<RepairSessionStatus>("unlock_repair_mode", {
    appInstanceId,
    connectionId,
  });
}

/**
 * Locks the current repair session.
 */
export async function lockRepairMode(): Promise<RepairSessionStatus> {
  return invokeCommand<RepairSessionStatus>("lock_repair_mode");
}

/**
 * Adds a route through the elevated repair broker path.
 */
export async function repairAddRoute(
  destination: string,
  mask: string,
  gateway: string,
  metric: string,
  interfaceIndex?: string,
): Promise<RepairCommandResult> {
  return invokeCommand<RepairCommandResult>("repair_add_route", {
    destination,
    mask,
    gateway,
    metric,
    interfaceIndex: interfaceIndex || null,
  });
}

/**
 * Deletes a route through the elevated repair broker path.
 */
export async function repairDeleteRoute(
  destination: string,
  mask: string,
): Promise<RepairCommandResult> {
  return invokeCommand<RepairCommandResult>("repair_delete_route", { destination, mask });
}

/**
 * Flushes routes through the elevated repair broker path.
 */
export async function repairFlushRoutes(): Promise<RepairCommandResult> {
  return invokeCommand<RepairCommandResult>("repair_flush_routes");
}

/**
 * Sets a NIC gateway through the elevated repair broker path.
 */
export async function repairSetDefaultGateway(
  gateway: string,
  interfaceIndex: string,
): Promise<RepairCommandResult> {
  return invokeCommand<RepairCommandResult>("repair_set_default_gateway", {
    gateway,
    interfaceIndex,
  });
}

/**
 * Persists startup replay config through the repair broker.
 */
export async function repairSavePersistConfig(
  config: PersistConfig,
): Promise<RepairCommandResult> {
  return invokeCommand<RepairCommandResult>("repair_save_persist_config", { config });
}

/**
 * Clears startup replay config through the repair broker.
 */
export async function repairClearPersistConfig(): Promise<RepairCommandResult> {
  return invokeCommand<RepairCommandResult>("repair_clear_persist_config");
}

/**
 * Executes a predefined repair action on the broker.
 */
export async function runRepairMachineAction(
  action: RepairMachineAction,
): Promise<RepairCommandResult> {
  return invokeCommand<RepairCommandResult>("repair_run_machine_action", { action });
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
  nic?: NicIdentifier;
}

export interface PersistConfig {
  schema_version: number;
  enabled: boolean;
  nic: NicIdentifier;
  wan?: WanConfig;
  custom_routes: CustomRoute[];
  updated_at?: string;
}

/**
 * Writes the persisted startup config from the non-elevated frontend path.
 */
export async function persistSaveConfig(
  config: PersistConfig,
): Promise<void> {
  return invokeCommand<void>("persist_save_config", { config });
}

/**
 * Reads the current persisted startup config.
 */
export async function persistLoadConfig(): Promise<PersistConfig | null> {
  return invokeCommand<PersistConfig | null>("persist_load_config");
}

/**
 * Resolves the stable NIC identifier for one interface index.
 */
export async function persistGetNicStableId(
  interfaceIndex: string,
): Promise<NicIdentifier> {
  return invokeCommand<NicIdentifier>("persist_get_nic_stable_id", { interfaceIndex });
}

/**
 * Resolves stable NIC identifiers for multiple interface indexes in one round-trip.
 */
export async function persistGetNicStableIds(
  interfaceIndexes: string[],
): Promise<NicIdentifier[]> {
  return invokeCommand<NicIdentifier[]>("persist_get_nic_stable_ids", { interfaceIndexes });
}
