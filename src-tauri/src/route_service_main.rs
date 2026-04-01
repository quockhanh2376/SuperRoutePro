// Prevents console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
#![cfg_attr(test, allow(dead_code))]

#[cfg(test)]
use std::collections::HashMap;
#[cfg(target_os = "windows")]
use std::ffi::OsString;
#[cfg(target_os = "windows")]
use std::sync::mpsc::{self, Receiver, RecvTimeoutError};
#[cfg(target_os = "windows")]
use std::time::Duration;
use super_route_pro_lib::route_apply::{
    apply_persist_config_with_adapters, resolve_nic_interface_index_from_adapters,
};
#[cfg(test)]
use super_route_pro_lib::route_apply::{
    build_nic_index_lookup, resolve_custom_route_interface_index,
};
#[cfg(target_os = "windows")]
use super_route_pro_lib::route_monitor::{
    inspect_config_drift, ROUTE_MONITOR_CONFIRMATION_PASSES, ROUTE_MONITOR_POLL_INTERVAL,
};
use super_route_pro_lib::route_persist::{self, NicIdentifier, PersistConfig};
#[cfg(target_os = "windows")]
use super_route_pro_lib::route_service_control::ROUTE_SERVICE_NAME;
#[cfg(test)]
use super_route_pro_lib::route_persist::CustomRoute;

#[cfg(target_os = "windows")]
use windows_service::define_windows_service;
#[cfg(target_os = "windows")]
use windows_service::service::{
    ServiceControl, ServiceControlAccept, ServiceExitCode, ServiceState, ServiceStatus,
    ServiceType,
};
#[cfg(target_os = "windows")]
use windows_service::service_control_handler::{self, ServiceControlHandlerResult};
#[cfg(target_os = "windows")]
use windows_service::service_dispatcher;

#[cfg(target_os = "windows")]
define_windows_service!(ffi_route_service_main, route_service_main);

#[cfg(target_os = "windows")]
#[derive(Default)]
struct RouteServiceState {
    last_applied_config_stamp: Option<String>,
    last_drift_signature: Option<String>,
    drift_passes: u8,
}

#[cfg(target_os = "windows")]
impl RouteServiceState {
    fn reset(&mut self) {
        self.last_applied_config_stamp = None;
        self.last_drift_signature = None;
        self.drift_passes = 0;
    }

    fn mark_applied(&mut self, config_stamp: String) {
        self.last_applied_config_stamp = Some(config_stamp);
        self.last_drift_signature = None;
        self.drift_passes = 0;
    }
}

fn main() {
    if let Err(err) = run_entrypoint() {
        eprintln!("[SuperRouteService] {err}");
        std::process::exit(1);
    }
}

fn run_entrypoint() -> Result<(), String> {
    let mut args = std::env::args().skip(1);
    if matches!(args.next().as_deref(), Some("--console")) {
        return run_console_loop();
    }

    #[cfg(target_os = "windows")]
    {
        service_dispatcher::start(ROUTE_SERVICE_NAME, ffi_route_service_main)
            .map_err(|err| format!("Unable to connect the route service to the Service Control Manager: {err}"))
    }

    #[cfg(not(target_os = "windows"))]
    {
        run_console_loop()
    }
}

#[cfg(target_os = "windows")]
fn route_service_main(_arguments: Vec<OsString>) {
    if let Err(err) = run_windows_service() {
        eprintln!("[SuperRouteService] {err}");
    }
}

#[cfg(target_os = "windows")]
fn run_windows_service() -> Result<(), String> {
    let (shutdown_tx, shutdown_rx) = mpsc::channel::<()>();
    let status_handle = service_control_handler::register(
        ROUTE_SERVICE_NAME,
        move |control_event| match control_event {
            ServiceControl::Stop => {
                let _ = shutdown_tx.send(());
                ServiceControlHandlerResult::NoError
            }
            ServiceControl::Interrogate => ServiceControlHandlerResult::NoError,
            _ => ServiceControlHandlerResult::NotImplemented,
        },
    )
    .map_err(|err| format!("Unable to register the route service control handler: {err}"))?;

    status_handle
        .set_service_status(service_status(ServiceState::StartPending))
        .map_err(|err| format!("Unable to set the route service start-pending status: {err}"))?;
    status_handle
        .set_service_status(service_status(ServiceState::Running))
        .map_err(|err| format!("Unable to set the route service running status: {err}"))?;

    let loop_result = run_route_service_loop(Some(&shutdown_rx));

    let final_status = if loop_result.is_ok() {
        service_status(ServiceState::Stopped)
    } else {
        service_status_with_exit_code(ServiceState::Stopped, 1)
    };
    status_handle
        .set_service_status(final_status)
        .map_err(|err| format!("Unable to set the route service stopped status: {err}"))?;

    loop_result
}

fn run_console_loop() -> Result<(), String> {
    run_route_service_loop(None)
}

#[cfg(target_os = "windows")]
fn run_route_service_loop(shutdown_rx: Option<&Receiver<()>>) -> Result<(), String> {
    let mut state = RouteServiceState::default();

    loop {
        if let Err(err) = route_service_tick(&mut state) {
            eprintln!("[SuperRouteService] {err}");
        }

        match shutdown_rx {
            Some(receiver) => match receiver.recv_timeout(ROUTE_MONITOR_POLL_INTERVAL) {
                Ok(()) | Err(RecvTimeoutError::Disconnected) => return Ok(()),
                Err(RecvTimeoutError::Timeout) => {}
            },
            None => std::thread::sleep(ROUTE_MONITOR_POLL_INTERVAL),
        }
    }
}

#[cfg(not(target_os = "windows"))]
fn run_route_service_loop(_shutdown_rx: Option<&()>) -> Result<(), String> {
    Ok(())
}

#[cfg(target_os = "windows")]
fn route_service_tick(state: &mut RouteServiceState) -> Result<(), String> {
    let Some(config) = route_persist::load_config()? else {
        state.reset();
        return Ok(());
    };
    if !config.enabled {
        state.reset();
        return Ok(());
    }

    let config_stamp = config.updated_at.clone().unwrap_or_else(|| persist_config_stamp(&config));
    if state.last_applied_config_stamp.as_deref() != Some(config_stamp.as_str()) {
        apply_config_and_mark_state(state, &config, config_stamp)?;
        return Ok(());
    }

    match inspect_config_drift(&config)? {
        Some(signature) => {
            if state
                .last_drift_signature
                .as_ref()
                .is_some_and(|last_signature| last_signature == &signature)
            {
                state.drift_passes = state.drift_passes.saturating_add(1);
            } else {
                state.last_drift_signature = Some(signature.clone());
                state.drift_passes = 1;
            }

            if state.drift_passes >= ROUTE_MONITOR_CONFIRMATION_PASSES {
                apply_config_and_mark_state(state, &config, config_stamp)?;
                eprintln!(
                    "[SuperRouteService] Re-applied persisted routes after drift '{}'.",
                    signature
                );
            }
        }
        None => {
            state.last_drift_signature = None;
            state.drift_passes = 0;
        }
    }

    Ok(())
}

#[cfg(target_os = "windows")]
fn apply_config_and_mark_state(
    state: &mut RouteServiceState,
    config: &PersistConfig,
    config_stamp: String,
) -> Result<(), String> {
    let (interface_index, adapters) = find_nic_interface_index(&config.nic)?;
    eprintln!(
        "[SuperRouteService] NIC '{}' resolved at InterfaceIndex {interface_index}",
        config.nic.description
    );
    apply_routes(config, &interface_index, &adapters)?;
    state.mark_applied(config_stamp);
    Ok(())
}

fn persist_config_stamp(config: &PersistConfig) -> String {
    serde_json::to_string(config).unwrap_or_else(|_| {
        format!(
            "{}:{}:{}:{}",
            config.enabled,
            config.nic.description,
            config.nic.mac_address,
            config.custom_routes.len()
        )
    })
}

fn find_nic_interface_index(
    nic: &NicIdentifier,
) -> Result<(String, Vec<super_route_pro_lib::win32_net::NativeNic>), String> {
    let adapters = super_route_pro_lib::win32_net::enumerate_adapters()?;
    let index = resolve_nic_interface_index_from_adapters(nic, &adapters)?;
    Ok((index, adapters))
}

fn apply_routes(
    config: &PersistConfig,
    interface_index: &str,
    adapters: &[super_route_pro_lib::win32_net::NativeNic],
) -> Result<(), String> {
    let report = apply_persist_config_with_adapters(config, adapters)?;
    for line in report.output_lines {
        eprintln!("[SuperRouteService] {line}");
    }
    eprintln!(
        "[SuperRouteService] Persist config applied on interface {interface_index} with {} custom route(s).",
        report.custom_route_count
    );
    Ok(())
}

#[cfg(target_os = "windows")]
fn service_status(current_state: ServiceState) -> ServiceStatus {
    service_status_with_exit_code(current_state, 0)
}

#[cfg(target_os = "windows")]
fn service_status_with_exit_code(current_state: ServiceState, exit_code: u32) -> ServiceStatus {
    ServiceStatus {
        service_type: ServiceType::OWN_PROCESS,
        current_state,
        controls_accepted: if current_state == ServiceState::Running {
            ServiceControlAccept::STOP
        } else {
            ServiceControlAccept::empty()
        },
        exit_code: ServiceExitCode::Win32(exit_code),
        checkpoint: 0,
        wait_hint: Duration::from_secs(10),
        process_id: None,
    }
}

#[cfg(test)]
pub(crate) fn test_build_nic_index_lookup(
    adapters: &[super_route_pro_lib::win32_net::NativeNic],
) -> HashMap<String, String> {
    build_nic_index_lookup(adapters)
}

#[cfg(test)]
pub(crate) fn test_resolve_nic_interface_index_from_adapters(
    nic: &NicIdentifier,
    adapters: &[super_route_pro_lib::win32_net::NativeNic],
) -> Result<String, String> {
    resolve_nic_interface_index_from_adapters(nic, adapters)
}

#[cfg(test)]
pub(crate) fn test_resolve_custom_route_interface_index(
    route: &CustomRoute,
    default_interface_index: &str,
    nic_index_lookup: &HashMap<String, String>,
) -> Result<String, String> {
    resolve_custom_route_interface_index(route, default_interface_index, nic_index_lookup)
}
