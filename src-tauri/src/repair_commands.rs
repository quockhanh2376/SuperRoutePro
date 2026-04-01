use crate::privilege::{current_privilege_context, PrivilegeContext};
#[cfg(target_os = "windows")]
use crate::process_exec::run_hidden_output_blocking;
use crate::repair_ipc::{
    auto_unlock_local_session, complete_unlock_request,
    get_repair_service_health as read_repair_service_health,
    get_repair_session_status as read_repair_session_status, issue_unlock_request,
    lock_repair_mode as lock_repair_mode_state, run_appx_removal as dispatch_appx_removal,
    run_machine_action as dispatch_repair_machine_action,
    run_profile_cleanup as dispatch_profile_cleanup,
};
use crate::repair_protocol::{
    AppxRemovalRequest, PersistConfigRequest, ProfileCleanupRequest, RepairCommandResult,
    RepairMachineAction, RepairServiceHealth, RepairSessionStatus, UnlockRepairSessionRequest,
};
use crate::repair_targets::{list_repair_targets as read_repair_targets, RepairTargetUser};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AutoUnlockStrategy {
    UnlockLocalSession,
    PromptForElevation,
    LeaveLocked,
}

fn resolve_auto_unlock_strategy(context: PrivilegeContext) -> AutoUnlockStrategy {
    if context.process_is_elevated {
        AutoUnlockStrategy::UnlockLocalSession
    } else if context.account_is_local_admin {
        AutoUnlockStrategy::PromptForElevation
    } else {
        AutoUnlockStrategy::LeaveLocked
    }
}

pub(crate) fn handle_main_window_event<R: tauri::Runtime>(
    window: &tauri::Window<R>,
    event: &tauri::WindowEvent,
) {
    if window.label() != "main" {
        return;
    }

    if matches!(
        event,
        tauri::WindowEvent::CloseRequested { .. } | tauri::WindowEvent::Destroyed
    ) {
        crate::route_watcher::stop();
        let _ = lock_repair_mode_state();
    }
}

#[tauri::command]
pub(crate) fn get_repair_service_health() -> RepairServiceHealth {
    read_repair_service_health()
}

#[tauri::command]
pub(crate) fn get_repair_session_status() -> RepairSessionStatus {
    read_repair_session_status()
}

#[tauri::command]
pub(crate) fn list_repair_targets() -> Result<Vec<RepairTargetUser>, String> {
    read_repair_targets()
}

#[tauri::command]
pub(crate) fn auto_unlock_repair_mode(
    app_instance_id: String,
    connection_id: String,
) -> Result<RepairSessionStatus, String> {
    match resolve_auto_unlock_strategy(current_privilege_context()?) {
        AutoUnlockStrategy::UnlockLocalSession => {
            auto_unlock_local_session(&app_instance_id, &connection_id)
        }
        AutoUnlockStrategy::PromptForElevation => {
            unlock_repair_mode_via_broker(&app_instance_id, &connection_id)
        }
        AutoUnlockStrategy::LeaveLocked => Ok(read_repair_session_status()),
    }
}

#[tauri::command]
pub(crate) fn unlock_repair_mode(
    app_instance_id: String,
    connection_id: String,
) -> Result<RepairSessionStatus, String> {
    if crate::privilege::is_process_elevated()? {
        return auto_unlock_local_session(&app_instance_id, &connection_id);
    }

    unlock_repair_mode_via_broker(&app_instance_id, &connection_id)
}

#[tauri::command]
pub(crate) fn lock_repair_mode() -> RepairSessionStatus {
    lock_repair_mode_state()
}

#[tauri::command]
pub(crate) async fn repair_add_route(
    destination: String,
    mask: String,
    gateway: String,
    metric: String,
    interface_index: Option<String>,
) -> Result<RepairCommandResult, String> {
    dispatch_repair_machine_action(RepairMachineAction::AddRoute(
        crate::repair_protocol::AddRouteRequest {
            destination,
            mask,
            gateway,
            metric,
            interface_index,
        },
    ))
}

#[tauri::command]
pub(crate) async fn repair_delete_route(
    destination: String,
    mask: String,
) -> Result<RepairCommandResult, String> {
    dispatch_repair_machine_action(RepairMachineAction::DeleteRoute(
        crate::repair_protocol::DeleteRouteRequest { destination, mask },
    ))
}

#[tauri::command]
pub(crate) async fn repair_flush_routes() -> Result<RepairCommandResult, String> {
    dispatch_repair_machine_action(RepairMachineAction::FlushRoutes)
}

#[tauri::command]
pub(crate) async fn repair_set_default_gateway(
    gateway: String,
    interface_index: String,
) -> Result<RepairCommandResult, String> {
    dispatch_repair_machine_action(RepairMachineAction::SetDefaultGateway(
        crate::repair_protocol::SetDefaultGatewayRequest {
            gateway,
            interface_index,
        },
    ))
}

#[tauri::command]
pub(crate) async fn repair_save_persist_config(
    config: crate::route_persist::PersistConfig,
) -> Result<RepairCommandResult, String> {
    dispatch_repair_machine_action(RepairMachineAction::SavePersistConfig(
        PersistConfigRequest { config },
    ))
}

#[tauri::command]
pub(crate) async fn repair_clear_persist_config() -> Result<RepairCommandResult, String> {
    dispatch_repair_machine_action(RepairMachineAction::ClearPersistConfig)
}

#[tauri::command]
pub(crate) async fn repair_run_machine_action(
    action: RepairMachineAction,
) -> Result<RepairCommandResult, String> {
    dispatch_repair_machine_action(action)
}

#[tauri::command]
pub(crate) async fn repair_clear_cache_targets(
    target_sid: String,
    targets: Vec<String>,
) -> Result<RepairCommandResult, String> {
    dispatch_profile_cleanup(ProfileCleanupRequest {
        target_sid,
        targets,
    })
}

#[tauri::command]
pub(crate) async fn repair_remove_bloatware(
    target_sid: String,
    packages: Vec<String>,
    remove_provisioned: bool,
) -> Result<RepairCommandResult, String> {
    dispatch_appx_removal(AppxRemovalRequest {
        target_sid,
        packages,
        remove_provisioned,
    })
}

fn unlock_repair_mode_via_broker(
    app_instance_id: &str,
    connection_id: &str,
) -> Result<RepairSessionStatus, String> {
    let request = issue_unlock_request(app_instance_id, connection_id)?;
    launch_repair_broker(&request)?;

    let response = complete_unlock_request(request);
    if response.unlocked {
        Ok(read_repair_session_status())
    } else {
        Err(response
            .detail
            .unwrap_or_else(|| "Repair mode unlock failed.".to_string()))
    }
}

#[cfg(target_os = "windows")]
fn launch_repair_broker(request: &UnlockRepairSessionRequest) -> Result<(), String> {
    let broker_path = std::env::current_exe()
        .map_err(|err| format!("Unable to locate the current executable: {err}"))?
        .with_file_name("SuperRouteRepairBroker.exe");

    if !broker_path.exists() {
        return Err(format!(
            "Repair broker is missing from the installation: {}",
            broker_path.display()
        ));
    }

    let broker_escaped = broker_path.to_string_lossy().replace('\'', "''");
    let app_id = request.app_instance_id.replace('\'', "''");
    let connection_id = request.connection_id.replace('\'', "''");
    let nonce = request.nonce.replace('\'', "''");
    let parent_process_id = request.parent_process_id;
    let script = format!(
        "Start-Process -FilePath '{broker_escaped}' -Verb RunAs -WindowStyle Hidden -ArgumentList @('--serve','{port}','{nonce}','{app_id}','{connection_id}','{parent_process_id}')",
        port = request.port
    );
    let output = run_hidden_output_blocking(
        "powershell",
        &["-NoProfile", "-NonInteractive", "-Command", script.as_str()],
    )?;

    if output.status.success() {
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if !stderr.is_empty() {
            Err(stderr)
        } else if !stdout.is_empty() {
            Err(stdout)
        } else {
            Err("Repair broker elevation was cancelled or returned an unknown error.".to_string())
        }
    }
}

#[cfg(not(target_os = "windows"))]
fn launch_repair_broker(_request: &UnlockRepairSessionRequest) -> Result<(), String> {
    Err("Repair mode unlock is only available on Windows.".to_string())
}

#[cfg(test)]
mod tests {
    use super::{resolve_auto_unlock_strategy, AutoUnlockStrategy};
    use crate::privilege::PrivilegeContext;

    #[test]
    fn auto_unlock_prefers_in_process_session_when_already_elevated() {
        let strategy = resolve_auto_unlock_strategy(PrivilegeContext {
            process_is_elevated: true,
            account_is_local_admin: true,
        });

        assert_eq!(strategy, AutoUnlockStrategy::UnlockLocalSession);
    }

    #[test]
    fn auto_unlock_prompts_for_uac_when_account_is_admin_but_process_isnt() {
        let strategy = resolve_auto_unlock_strategy(PrivilegeContext {
            process_is_elevated: false,
            account_is_local_admin: true,
        });

        assert_eq!(strategy, AutoUnlockStrategy::PromptForElevation);
    }

    #[test]
    fn auto_unlock_leaves_standard_users_locked() {
        let strategy = resolve_auto_unlock_strategy(PrivilegeContext {
            process_is_elevated: false,
            account_is_local_admin: false,
        });

        assert_eq!(strategy, AutoUnlockStrategy::LeaveLocked);
    }
}
