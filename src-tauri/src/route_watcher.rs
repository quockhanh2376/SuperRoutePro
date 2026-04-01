#[cfg(target_os = "windows")]
use crate::repair_ipc;
#[cfg(target_os = "windows")]
use crate::repair_protocol::{PersistConfigRequest, RepairMachineAction};
#[cfg(target_os = "windows")]
use crate::route_apply;
#[cfg(target_os = "windows")]
use crate::route_monitor::{
    inspect_config_drift, ROUTE_MONITOR_CONFIRMATION_PASSES, ROUTE_MONITOR_POLL_INTERVAL,
};
#[cfg(target_os = "windows")]
use crate::route_persist::PersistConfig;
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

                if drift_passes >= ROUTE_MONITOR_CONFIRMATION_PASSES {
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
    let mut remaining = ROUTE_MONITOR_POLL_INTERVAL;
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

    match inspect_config_drift(&config)? {
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
