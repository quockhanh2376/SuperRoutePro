use crate::{persist_startup, route_persist, win32_net};

#[tauri::command]
pub(crate) async fn persist_save_config(
    config: route_persist::PersistConfig,
) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || persist_startup::save_enabled_config(&config))
        .await
        .map_err(|err| format!("Persist save config task join error: {err}"))?
}

#[tauri::command]
pub(crate) async fn persist_load_config() -> Result<Option<route_persist::PersistConfig>, String> {
    tauri::async_runtime::spawn_blocking(route_persist::load_config)
        .await
        .map_err(|err| format!("Persist load config task join error: {err}"))?
}

#[tauri::command]
pub(crate) async fn persist_get_nic_stable_id(
    interface_index: String,
) -> Result<route_persist::NicIdentifier, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let target_idx: u32 = interface_index
            .parse()
            .map_err(|_| format!("Invalid interface index: {interface_index}"))?;

        let adapters = win32_net::enumerate_adapters()?;
        let nic = adapters
            .iter()
            .find(|a| a.interface_index == target_idx)
            .ok_or_else(|| format!("No adapter found with InterfaceIndex {target_idx}"))?;

        Ok(route_persist::NicIdentifier {
            description: nic.description.clone(),
            mac_address: nic.mac_address.clone(),
        })
    })
    .await
    .map_err(|err| format!("Persist get NIC stable ID task join error: {err}"))?
}

#[tauri::command]
pub(crate) async fn persist_get_nic_stable_ids(
    interface_indexes: Vec<String>,
) -> Result<Vec<route_persist::NicIdentifier>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let requested_indexes: Vec<u32> = interface_indexes
            .iter()
            .map(|interface_index| {
                interface_index
                    .parse::<u32>()
                    .map_err(|_| format!("Invalid interface index: {interface_index}"))
            })
            .collect::<Result<_, _>>()?;

        let adapters = win32_net::enumerate_adapters()?;

        requested_indexes
            .iter()
            .map(|target_idx| {
                let nic = adapters
                    .iter()
                    .find(|a| a.interface_index == *target_idx)
                    .ok_or_else(|| format!("No adapter found with InterfaceIndex {target_idx}"))?;

                Ok(route_persist::NicIdentifier {
                    description: nic.description.clone(),
                    mac_address: nic.mac_address.clone(),
                })
            })
            .collect()
    })
    .await
    .map_err(|err| format!("Persist get NIC stable IDs task join error: {err}"))?
}
