use crate::route_persist::{self, PersistConfig};
#[cfg(target_os = "windows")]
use crate::win32_consts::CREATE_NO_WINDOW;
#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;
#[cfg(target_os = "windows")]
use std::process::Command;

#[cfg(target_os = "windows")]
const STARTUP_TASK_NAME: &str = "SuperRouteProPersist";

pub fn save_enabled_config(config: &PersistConfig) -> Result<(), String> {
    if !config.enabled {
        return clear_persisted_startup_state();
    }

    route_persist::save_config(config)?;

    #[cfg(target_os = "windows")]
    {
        register_startup_task()?;
    }

    Ok(())
}

pub fn clear_persisted_startup_state() -> Result<(), String> {
    route_persist::delete_config()?;

    #[cfg(target_os = "windows")]
    {
        unregister_startup_task()?;
    }

    Ok(())
}

#[cfg(target_os = "windows")]
fn register_startup_task() -> Result<(), String> {
    let exe =
        std::env::current_exe().map_err(|e| format!("Failed to get current exe path: {e}"))?;
    let exe_dir = exe.parent().ok_or("No parent dir")?;
    let service_exe = exe_dir.join("SuperRouteService.exe");
    let service_path = service_exe.to_string_lossy();

    let _ = run_hidden("schtasks", &["/Delete", "/TN", STARTUP_TASK_NAME, "/F"]);

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
    let output = run_hidden("schtasks", &["/Delete", "/TN", STARTUP_TASK_NAME, "/F"])
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
fn run_hidden(program: &str, args: &[&str]) -> Option<std::process::Output> {
    Command::new(program)
        .args(args)
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .ok()
}
