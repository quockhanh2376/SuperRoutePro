mod app_bootstrap;
mod battery;
mod bloatware_catalog;
pub mod cache_cleanup;
mod connectivity_probe;
mod network;
mod network_snapshot;
mod persist_commands;
pub mod persist_startup;
mod ping;
pub mod process_exec;
pub mod repair_actions;
mod repair_commands;
pub mod repair_ipc;
pub mod repair_protocol;
pub mod repair_session;
pub mod repair_targets;
pub mod route_persist;
mod speed_test;
mod speed_test_targets;
pub mod win32_consts;
pub mod win32_net;

use battery::{get_battery_report, get_battery_summary};
use network::{
    add_route, check_internet, delete_route, flush_routes, get_bloatware_candidates,
    run_network_command, set_default_gateway, test_tcp_port,
};
use network_snapshot::{
    get_network_interfaces, get_network_snapshot, get_routing_table,
    invalidate_network_adapter_cache,
};
use persist_commands::{
    persist_get_nic_stable_id, persist_get_nic_stable_ids, persist_load_config, persist_save_config,
};
use ping::{fping_scan, ping_host};
use repair_commands::{
    get_repair_service_health, get_repair_session_status, list_repair_targets, lock_repair_mode,
    repair_add_route, repair_clear_cache_targets, repair_clear_persist_config, repair_delete_route,
    repair_flush_routes, repair_remove_bloatware, repair_run_machine_action,
    repair_save_persist_config, repair_set_default_gateway, unlock_repair_mode,
};
use speed_test::run_speed_test;
use speed_test_targets::list_speed_test_targets;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    app_bootstrap::validate_or_exit();

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .on_window_event(repair_commands::handle_main_window_event)
        .setup(app_bootstrap::setup_main_window)
        .invoke_handler(tauri::generate_handler![
            get_network_interfaces,
            get_network_snapshot,
            get_routing_table,
            invalidate_network_adapter_cache,
            add_route,
            delete_route,
            flush_routes,
            set_default_gateway,
            run_network_command,
            ping_host,
            test_tcp_port,
            fping_scan,
            list_speed_test_targets,
            run_speed_test,
            check_internet,
            get_bloatware_candidates,
            get_battery_report,
            get_battery_summary,
            get_repair_service_health,
            get_repair_session_status,
            list_repair_targets,
            unlock_repair_mode,
            lock_repair_mode,
            repair_add_route,
            repair_delete_route,
            repair_flush_routes,
            repair_set_default_gateway,
            repair_save_persist_config,
            repair_clear_persist_config,
            repair_run_machine_action,
            repair_clear_cache_targets,
            repair_remove_bloatware,
            persist_save_config,
            persist_load_config,
            persist_get_nic_stable_id,
            persist_get_nic_stable_ids,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
