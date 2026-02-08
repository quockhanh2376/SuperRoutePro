mod network;

use network::{
    get_network_interfaces, get_routing_table, add_route, delete_route,
    flush_routes, set_default_gateway, run_network_command, ping_host,
    check_internet, fping_scan,
};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            get_network_interfaces,
            get_routing_table,
            add_route,
            delete_route,
            flush_routes,
            set_default_gateway,
            run_network_command,
            ping_host,
            fping_scan,
            check_internet,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
