#[cfg(target_os = "windows")]
use crate::process_exec::run_hidden_output_blocking;
#[cfg(target_os = "windows")]
use crate::route_service_control;
use crate::route_persist::{self, PersistConfig};

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
        route_service_control::ensure_route_service_running()?;
        cleanup_obsolete_startup_artifacts()?;
    }

    Ok(())
}

pub fn clear_persisted_startup_state() -> Result<(), String> {
    route_persist::delete_config()?;

    #[cfg(target_os = "windows")]
    {
        route_service_control::uninstall_route_service_if_present()?;
        cleanup_obsolete_startup_artifacts()?;
    }

    Ok(())
}

#[cfg(target_os = "windows")]
pub fn startup_task_exists() -> Result<bool, String> {
    route_service_control::route_service_is_installed()
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
enum TaskCommandStatus {
    Success,
    Missing,
    Failed(String),
}

#[cfg(target_os = "windows")]
fn run_task_command(program: &str, args: &[&str]) -> Result<std::process::Output, String> {
    run_hidden_output_blocking(program, args)
}

#[cfg(target_os = "windows")]
fn task_command_output(output: &std::process::Output) -> String {
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();

    match (stdout.is_empty(), stderr.is_empty()) {
        (true, true) => String::new(),
        (false, true) => stdout,
        (true, false) => stderr,
        (false, false) => format!("{stdout}\n{stderr}"),
    }
}

#[cfg(target_os = "windows")]
fn task_missing_marker_present(output: &std::process::Output) -> bool {
    let normalized = task_command_output(output).to_ascii_lowercase();
    normalized.contains("does not exist") || normalized.contains("cannot find")
}

#[cfg(target_os = "windows")]
fn task_command_status(output: &std::process::Output) -> TaskCommandStatus {
    if output.status.success() {
        TaskCommandStatus::Success
    } else if task_missing_marker_present(output) {
        TaskCommandStatus::Missing
    } else {
        TaskCommandStatus::Failed(task_command_output(output))
    }
}

#[cfg(target_os = "windows")]
fn delete_task_if_exists(task_name: &str) -> Result<(), String> {
    let output = run_task_command("schtasks", &["/Delete", "/TN", task_name, "/F"])?;

    if let TaskCommandStatus::Failed(detail) = task_command_status(&output) {
        return Err(format!("Task removal failed: {detail}"));
    }

    Ok(())
}

#[cfg(all(test, target_os = "windows"))]
mod tests {
    use super::{
        task_command_output, task_command_status, task_missing_marker_present, TaskCommandStatus,
    };
    use std::os::windows::process::ExitStatusExt;

    fn output(status: u32, stdout: &str, stderr: &str) -> std::process::Output {
        std::process::Output {
            status: std::process::ExitStatus::from_raw(status),
            stdout: stdout.as_bytes().to_vec(),
            stderr: stderr.as_bytes().to_vec(),
        }
    }

    #[test]
    fn task_missing_marker_detects_stdout_only_with_mixed_case() {
        let output = output(1, "ERROR: The system Cannot Find the file specified.", "");
        assert!(task_missing_marker_present(&output));
    }

    #[test]
    fn task_missing_marker_detects_stderr_only_missing_task() {
        let output = output(1, "", "WARNING: The task does not exist.");
        assert!(task_missing_marker_present(&output));
    }

    #[test]
    fn task_missing_marker_rejects_unrelated_errors() {
        let output = output(1, "", "Access is denied.");
        assert!(!task_missing_marker_present(&output));
    }

    #[test]
    fn task_command_output_combines_stdout_and_stderr() {
        let output = output(1, "stdout line", "stderr line");
        assert_eq!(task_command_output(&output), "stdout line\nstderr line");
    }

    #[test]
    fn task_command_status_distinguishes_success_missing_and_failure() {
        let success = output(0, "ok", "");
        assert!(matches!(
            task_command_status(&success),
            TaskCommandStatus::Success
        ));

        let missing = output(1, "", "WARNING: The task does not exist.");
        assert!(matches!(
            task_command_status(&missing),
            TaskCommandStatus::Missing
        ));

        let failed = output(1, "", "Access is denied.");
        assert!(matches!(
            task_command_status(&failed),
            TaskCommandStatus::Failed(message) if message == "Access is denied."
        ));
    }
}
