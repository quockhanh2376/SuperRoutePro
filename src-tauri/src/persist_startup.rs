use crate::route_persist::{self, PersistConfig};
#[cfg(target_os = "windows")]
use crate::win32_consts::CREATE_NO_WINDOW;
#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;
#[cfg(target_os = "windows")]
use std::process::Command;

#[cfg(target_os = "windows")]
const STARTUP_TASK_NAME: &str = "SuperRouteProPersist";
#[cfg(target_os = "windows")]
const LEGACY_WAN_TASK_NAME: &str = "SuperRoutePro-PersistWAN";
#[cfg(target_os = "windows")]
const LEGACY_WAN_SCRIPT_NAME: &str = "persist-wan.cmd";

pub fn save_enabled_config(config: &PersistConfig) -> Result<(), String> {
    if !config.enabled {
        return clear_persisted_startup_state();
    }

    route_persist::save_config(config)?;

    #[cfg(target_os = "windows")]
    {
        register_startup_task()?;
        cleanup_obsolete_startup_artifacts()?;
    }

    Ok(())
}

pub fn clear_persisted_startup_state() -> Result<(), String> {
    route_persist::delete_config()?;

    #[cfg(target_os = "windows")]
    {
        unregister_startup_task()?;
        cleanup_obsolete_startup_artifacts()?;
    }

    Ok(())
}

#[cfg(target_os = "windows")]
pub fn startup_task_exists() -> Result<bool, String> {
    query_task_exists(STARTUP_TASK_NAME)
}

#[cfg(not(target_os = "windows"))]
pub fn startup_task_exists() -> Result<bool, String> {
    Ok(false)
}

#[cfg(target_os = "windows")]
pub fn cleanup_obsolete_startup_artifacts() -> Result<(), String> {
    delete_task_if_exists(LEGACY_WAN_TASK_NAME)?;

    let legacy_script_path = route_persist::config_dir()?.join(LEGACY_WAN_SCRIPT_NAME);
    if legacy_script_path.exists() {
        std::fs::remove_file(&legacy_script_path).map_err(|err| {
            format!(
                "Failed to remove obsolete startup script {}: {err}",
                legacy_script_path.display()
            )
        })?;
    }

    Ok(())
}

#[cfg(not(target_os = "windows"))]
pub fn cleanup_obsolete_startup_artifacts() -> Result<(), String> {
    Ok(())
}

#[cfg(target_os = "windows")]
fn register_startup_task() -> Result<(), String> {
    let exe =
        std::env::current_exe().map_err(|e| format!("Failed to get current exe path: {e}"))?;
    let exe_dir = exe.parent().ok_or("No parent dir")?;
    let service_exe = exe_dir.join("SuperRouteService.exe");
    let service_path = service_exe.to_string_lossy();

    let _ = delete_task_if_exists(STARTUP_TASK_NAME);

    let output = run_hidden(
        "schtasks",
        &[
            "/Create",
            "/TN",
            STARTUP_TASK_NAME,
            "/TR",
            &format!("\"{}\"", service_path),
            "/SC",
            "ONLOGON",
            "/RL",
            "HIGHEST",
            "/F",
        ],
    )
    .ok_or_else(|| "Failed to run schtasks for task registration".to_string())?;

    if !output.status.success() {
        return Err(format!(
            "Task registration failed: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    Ok(())
}

#[cfg(target_os = "windows")]
fn unregister_startup_task() -> Result<(), String> {
    delete_task_if_exists(STARTUP_TASK_NAME)
}

#[cfg(target_os = "windows")]
fn run_hidden(program: &str, args: &[&str]) -> Option<std::process::Output> {
    Command::new(program)
        .args(args)
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .ok()
}

#[cfg(target_os = "windows")]
fn delete_task_if_exists(task_name: &str) -> Result<(), String> {
    let output = run_hidden("schtasks", &["/Delete", "/TN", task_name, "/F"])
        .ok_or_else(|| "Failed to run schtasks for task removal".to_string())?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        if !stderr.contains("does not exist") && !stderr.contains("cannot find") {
            return Err(format!("Task removal failed: {}", stderr));
        }
    }

    Ok(())
}

#[cfg(target_os = "windows")]
fn query_task_exists(task_name: &str) -> Result<bool, String> {
    let output = run_hidden("schtasks", &["/Query", "/TN", task_name])
        .ok_or_else(|| "Failed to run schtasks for task query".to_string())?;

    if output.status.success() {
        return Ok(true);
    }

    let stderr = String::from_utf8_lossy(&output.stderr);
    if stderr.contains("does not exist") || stderr.contains("cannot find") {
        Ok(false)
    } else {
        Err(format!("Task query failed: {}", stderr))
    }
}
