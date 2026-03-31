#[cfg(target_os = "windows")]
use crate::network_snapshot::{get_routing_table_blocking_with_adapters, RouteEntry};
#[cfg(target_os = "windows")]
use crate::repair_ipc;
#[cfg(target_os = "windows")]
use crate::repair_protocol::{PersistConfigRequest, RepairMachineAction};
#[cfg(target_os = "windows")]
use crate::route_apply;
#[cfg(target_os = "windows")]
use crate::route_persist::{CustomRoute, PersistConfig};
#[cfg(target_os = "windows")]
use crate::win32_net::NativeNic;
#[cfg(target_os = "windows")]
use serde::Serialize;
#[cfg(target_os = "windows")]
use std::sync::atomic::{AtomicBool, Ordering};
#[cfg(target_os = "windows")]
use std::sync::{Arc, Mutex, OnceLock};
#[cfg(target_os = "windows")]
use std::thread::{self, JoinHandle};
#[cfg(target_os = "windows")]
use std::time::Duration;
#[cfg(target_os = "windows")]
use tauri::{Emitter, Runtime};

#[cfg(target_os = "windows")]
const ROUTE_WATCHER_POLL_INTERVAL: Duration = Duration::from_secs(8);
#[cfg(target_os = "windows")]
const ROUTE_WATCHER_CONFIRMATION_PASSES: u8 = 2;
#[cfg(target_os = "windows")]
pub const ROUTE_WATCHER_STATUS_EVENT: &str = "route-watcher://status";

#[cfg(target_os = "windows")]
type EmitRouteWatcherEvent = Arc<dyn Fn(RouteWatcherStatusEvent) + Send + Sync>;

#[cfg(target_os = "windows")]
#[derive(Debug, Clone, Serialize)]
pub struct RouteWatcherStatusEvent {
    pub status: String,
    pub title: String,
    pub message: String,
    pub detail: Option<String>,
    pub used_repair_host: bool,
}

#[cfg(target_os = "windows")]
struct ReapplyOutcome {
    detail: String,
    used_repair_host: bool,
}

#[cfg(target_os = "windows")]
struct RouteWatcherHandle {
    stop_requested: Arc<AtomicBool>,
    join_handle: JoinHandle<()>,
}

#[cfg(target_os = "windows")]
enum WatcherState {
    Idle,
    Healthy,
    Drifted(String),
}

#[cfg(target_os = "windows")]
fn watcher_handle() -> &'static Mutex<Option<RouteWatcherHandle>> {
    static ROUTE_WATCHER_HANDLE: OnceLock<Mutex<Option<RouteWatcherHandle>>> = OnceLock::new();
    ROUTE_WATCHER_HANDLE.get_or_init(|| Mutex::new(None))
}

#[cfg(target_os = "windows")]
pub fn start<R: Runtime + 'static>(app_handle: tauri::AppHandle<R>) -> Result<(), String> {
    let mut handle_slot = watcher_handle()
        .lock()
        .expect("route watcher handle mutex should not be poisoned");
    if handle_slot.is_some() {
        return Ok(());
    }

    let stop_requested = Arc::new(AtomicBool::new(false));
    let stop_signal = Arc::clone(&stop_requested);
    let emit_event: EmitRouteWatcherEvent = Arc::new(move |payload| {
        if let Err(err) = app_handle.emit(ROUTE_WATCHER_STATUS_EVENT, payload) {
            eprintln!("[RouteWatcher] Failed to emit watcher event: {err}");
        }
    });
    let join_handle = thread::Builder::new()
        .name("route-watcher".to_string())
        .spawn(move || run_loop(stop_signal, emit_event))
        .map_err(|err| format!("Failed to start route watcher thread: {err}"))?;

    *handle_slot = Some(RouteWatcherHandle {
        stop_requested,
        join_handle,
    });
    Ok(())
}

#[cfg(not(target_os = "windows"))]
pub fn start<R: tauri::Runtime>(_app_handle: tauri::AppHandle<R>) -> Result<(), String> {
    Ok(())
}

#[cfg(target_os = "windows")]
pub fn stop() {
    let handle = watcher_handle()
        .lock()
        .expect("route watcher handle mutex should not be poisoned")
        .take();

    if let Some(handle) = handle {
        handle.stop_requested.store(true, Ordering::Relaxed);
        let _ = handle.join_handle.join();
    }
}

#[cfg(not(target_os = "windows"))]
pub fn stop() {}

#[cfg(target_os = "windows")]
fn run_loop(stop_requested: Arc<AtomicBool>, emit_event: EmitRouteWatcherEvent) {
    let mut last_drift_signature: Option<String> = None;
    let mut drift_passes: u8 = 0;

    while !stop_requested.load(Ordering::Relaxed) {
        match inspect_persist_config_drift() {
            Ok(WatcherState::Idle | WatcherState::Healthy) => {
                last_drift_signature = None;
                drift_passes = 0;
            }
            Ok(WatcherState::Drifted(signature)) => {
                if last_drift_signature.as_deref() == Some(signature.as_str()) {
                    drift_passes = drift_passes.saturating_add(1);
                } else {
                    last_drift_signature = Some(signature.clone());
                    drift_passes = 1;
                }

                if drift_passes >= ROUTE_WATCHER_CONFIRMATION_PASSES {
                    match crate::route_persist::load_config() {
                        Ok(Some(config)) if config.enabled => match reapply_persist_config(&config) {
                            Ok(outcome) => {
                                eprintln!(
                                    "[RouteWatcher] Re-applied persisted routes after drift '{}'. {}",
                                    signature, outcome.detail
                                );
                                emit_event(RouteWatcherStatusEvent {
                                    status: "reapplied".to_string(),
                                    title: "Route Watcher Restored Routes".to_string(),
                                    message: if outcome.used_repair_host {
                                        "Persisted routes were restored after Windows changed the route table. Repair Mode handled the restore.".to_string()
                                    } else {
                                        "Persisted routes were restored after Windows changed the route table.".to_string()
                                    },
                                    detail: Some(outcome.detail),
                                    used_repair_host: outcome.used_repair_host,
                                });
                            }
                            Err(err) => {
                                eprintln!(
                                    "[RouteWatcher] Failed to re-apply persisted routes after drift '{}': {err}",
                                    signature
                                );
                                emit_event(RouteWatcherStatusEvent {
                                    status: "failed".to_string(),
                                    title: "Route Watcher Needs Attention".to_string(),
                                    message: "Persisted routes drifted but could not be restored automatically. Unlock Repair Mode or run the app as Administrator.".to_string(),
                                    detail: Some(err),
                                    used_repair_host: false,
                                });
                            }
                        },
                        Ok(_) => {}
                        Err(err) => eprintln!(
                            "[RouteWatcher] Failed to reload persist config before re-apply: {err}"
                        ),
                    }
                    last_drift_signature = None;
                    drift_passes = 0;
                }
            }
            Err(err) => eprintln!("[RouteWatcher] Drift inspection failed: {err}"),
        }

        sleep_until_next_poll(&stop_requested);
    }
}

#[cfg(target_os = "windows")]
fn sleep_until_next_poll(stop_requested: &AtomicBool) {
    let mut remaining = ROUTE_WATCHER_POLL_INTERVAL;
    let step = Duration::from_millis(500);
    while remaining > Duration::ZERO {
        if stop_requested.load(Ordering::Relaxed) {
            return;
        }
        let current_step = remaining.min(step);
        thread::sleep(current_step);
        remaining = remaining.saturating_sub(current_step);
    }
}

#[cfg(target_os = "windows")]
fn inspect_persist_config_drift() -> Result<WatcherState, String> {
    let Some(config) = crate::route_persist::load_config()? else {
        return Ok(WatcherState::Idle);
    };
    if !config.enabled {
        return Ok(WatcherState::Idle);
    }

    let adapters = crate::win32_net::enumerate_adapters()?;
    let routes = get_routing_table_blocking_with_adapters(&adapters)?;
    match build_drift_signature(&config, &routes, &adapters)? {
        Some(signature) => Ok(WatcherState::Drifted(signature)),
        None => Ok(WatcherState::Healthy),
    }
}

#[cfg(target_os = "windows")]
fn reapply_persist_config(config: &PersistConfig) -> Result<ReapplyOutcome, String> {
    match route_apply::apply_persist_config(config) {
        Ok(report) => Ok(ReapplyOutcome {
            detail: report.summary(),
            used_repair_host: false,
        }),
        Err(direct_err) => match repair_ipc::run_machine_action(
            RepairMachineAction::ApplyPersistConfig(PersistConfigRequest {
                config: config.clone(),
            }),
        ) {
            Ok(result) if result.success => Ok(ReapplyOutcome {
                detail: result.output,
                used_repair_host: true,
            }),
            Ok(result) => Err(format!(
                "Direct apply failed: {direct_err}; repair host apply failed: {}",
                result.output
            )),
            Err(host_err) => Err(format!(
                "Direct apply failed: {direct_err}; repair host unavailable: {host_err}"
            )),
        },
    }
}

#[cfg(target_os = "windows")]
fn build_drift_signature(
    config: &PersistConfig,
    routes: &[RouteEntry],
    adapters: &[NativeNic],
) -> Result<Option<String>, String> {
    let default_interface_index =
        route_apply::resolve_nic_interface_index_from_adapters(&config.nic, adapters)?;

    if let Some(wan) = &config.wan {
        let default_routes: Vec<&RouteEntry> = routes
            .iter()
            .filter(|route| route.destination == "0.0.0.0" && route.netmask == "0.0.0.0")
            .collect();

        if default_routes.len() != 1 {
            return Ok(Some(format!("default-route-count:{}", default_routes.len())));
        }

        let active_default = default_routes[0];
        if active_default.gateway != wan.gateway
            || active_default.interface_index != default_interface_index
        {
            return Ok(Some(format!(
                "default-route-mismatch:{}:{}:{}:{}",
                active_default.gateway,
                active_default.interface_index,
                wan.gateway,
                default_interface_index
            )));
        }
    }

    let nic_index_lookup = route_apply::build_nic_index_lookup(adapters);
    for route in &config.custom_routes {
        let expected_interface_index = route_apply::resolve_custom_route_interface_index(
            route,
            &default_interface_index,
            &nic_index_lookup,
        )?;
        if !has_matching_custom_route(route, &expected_interface_index, routes) {
            return Ok(Some(format!(
                "missing-custom-route:{}/{}/{}:{}",
                route.destination, route.mask, route.gateway, expected_interface_index
            )));
        }
    }

    Ok(None)
}

#[cfg(target_os = "windows")]
fn has_matching_custom_route(
    expected_route: &CustomRoute,
    expected_interface_index: &str,
    routes: &[RouteEntry],
) -> bool {
    routes.iter().any(|route| {
        route.destination == expected_route.destination
            && route.netmask == expected_route.mask
            && route.gateway == expected_route.gateway
            && route.interface_index == expected_interface_index
    })
}

#[cfg(all(test, target_os = "windows"))]
mod tests {
    use super::build_drift_signature;
    use crate::network_snapshot::RouteEntry;
    use crate::route_persist::{CustomRoute, NicIdentifier, PersistConfig, WanConfig};
    use crate::win32_net::NativeNic;

    fn sample_adapter() -> NativeNic {
        NativeNic {
            interface_index: 7,
            description: "Intel(R) Wi-Fi 6".to_string(),
            mac_address: "AA-BB-CC-DD-EE-FF".to_string(),
            friendly_name: "Wi-Fi".to_string(),
            ip_addresses: vec!["192.168.1.25".to_string()],
            gateways: vec!["192.168.1.1".to_string()],
            oper_status_up: true,
        }
    }

    fn sample_config() -> PersistConfig {
        PersistConfig {
            schema_version: 1,
            enabled: true,
            nic: NicIdentifier {
                description: "Intel(R) Wi-Fi 6".to_string(),
                mac_address: "AA-BB-CC-DD-EE-FF".to_string(),
            },
            wan: Some(WanConfig {
                gateway: "192.168.1.1".to_string(),
                metric: "1".to_string(),
            }),
            custom_routes: vec![CustomRoute {
                destination: "10.0.0.0".to_string(),
                mask: "255.0.0.0".to_string(),
                gateway: "192.168.1.1".to_string(),
                metric: "10".to_string(),
                nic: None,
            }],
            updated_at: None,
        }
    }

    #[test]
    fn drift_signature_is_empty_when_routes_match_config() {
        let routes = vec![
            RouteEntry {
                destination: "0.0.0.0".to_string(),
                netmask: "0.0.0.0".to_string(),
                gateway: "192.168.1.1".to_string(),
                metric: "25".to_string(),
                interface_index: "7".to_string(),
            },
            RouteEntry {
                destination: "10.0.0.0".to_string(),
                netmask: "255.0.0.0".to_string(),
                gateway: "192.168.1.1".to_string(),
                metric: "10".to_string(),
                interface_index: "7".to_string(),
            },
        ];

        assert_eq!(
            build_drift_signature(&sample_config(), &routes, &[sample_adapter()]).unwrap(),
            None
        );
    }

    #[test]
    fn drift_signature_detects_extra_default_route() {
        let routes = vec![
            RouteEntry {
                destination: "0.0.0.0".to_string(),
                netmask: "0.0.0.0".to_string(),
                gateway: "192.168.1.1".to_string(),
                metric: "25".to_string(),
                interface_index: "7".to_string(),
            },
            RouteEntry {
                destination: "0.0.0.0".to_string(),
                netmask: "0.0.0.0".to_string(),
                gateway: "10.8.0.1".to_string(),
                metric: "5".to_string(),
                interface_index: "77".to_string(),
            },
        ];

        assert_eq!(
            build_drift_signature(&sample_config(), &routes, &[sample_adapter()]).unwrap(),
            Some("default-route-count:2".to_string())
        );
    }

    #[test]
    fn drift_signature_detects_missing_custom_route() {
        let routes = vec![RouteEntry {
            destination: "0.0.0.0".to_string(),
            netmask: "0.0.0.0".to_string(),
            gateway: "192.168.1.1".to_string(),
            metric: "25".to_string(),
            interface_index: "7".to_string(),
        }];

        assert_eq!(
            build_drift_signature(&sample_config(), &routes, &[sample_adapter()]).unwrap(),
            Some("missing-custom-route:10.0.0.0/255.0.0.0/192.168.1.1:7".to_string())
        );
    }
}