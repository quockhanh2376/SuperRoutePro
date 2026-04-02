#[cfg(target_os = "windows")]
use crate::process_exec::run_hidden_output_blocking;
use std::path::PathBuf;
#[cfg(target_os = "windows")]
use std::time::{Duration, Instant};

pub const ROUTE_SERVICE_NAME: &str = "SuperRouteProRouteService";
pub const ROUTE_SERVICE_DISPLAY_NAME: &str = "Super Route Pro Route Service";

#[cfg(target_os = "windows")]
const ROUTE_SERVICE_DESCRIPTION: &str =
    "Monitors and restores persisted WAN and custom routes for Super Route Pro.";
#[cfg(target_os = "windows")]
const ROUTE_SERVICE_STATE_POLL: Duration = Duration::from_millis(500);
#[cfg(target_os = "windows")]
const ROUTE_SERVICE_STATE_TIMEOUT: Duration = Duration::from_secs(15);

#[cfg(target_os = "windows")]
enum ServiceCommandStatus {
    Success(String),
    Missing,
    Failed(String),
}

#[cfg(target_os = "windows")]
pub fn ensure_route_service_running() -> Result<(), String> {
    let service_path = route_service_binary_path()?;
    if route_service_is_installed()? {
        configure_route_service(&service_path)?;
    } else {
        create_route_service(&service_path)?;
    }

    if !route_service_is_running()? {
        start_route_service()?;
    }

    Ok(())
}

#[cfg(not(target_os = "windows"))]
pub fn ensure_route_service_running() -> Result<(), String> {
    Ok(())
}

#[cfg(target_os = "windows")]
pub fn route_service_is_installed() -> Result<bool, String> {
    match run_service_command(&["query", ROUTE_SERVICE_NAME])? {
        ServiceCommandStatus::Success(_) => Ok(true),
        ServiceCommandStatus::Missing => Ok(false),
        ServiceCommandStatus::Failed(detail) => Err(format!("Route service query failed: {detail}")),
    }
}

#[cfg(not(target_os = "windows"))]
pub fn route_service_is_installed() -> Result<bool, String> {
    Ok(false)
}

#[cfg(target_os = "windows")]
pub fn route_service_is_running() -> Result<bool, String> {
    match run_service_command(&["query", ROUTE_SERVICE_NAME])? {
        ServiceCommandStatus::Success(detail) => Ok(service_is_running(&detail)),
        ServiceCommandStatus::Missing => Ok(false),
        ServiceCommandStatus::Failed(detail) => Err(format!("Route service query failed: {detail}")),
    }
}

#[cfg(not(target_os = "windows"))]
pub fn route_service_is_running() -> Result<bool, String> {
    Ok(false)
}

#[cfg(target_os = "windows")]
pub fn stop_route_service_if_present() -> Result<(), String> {
    if !route_service_is_installed()? {
        return Ok(());
    }
    if !route_service_is_running()? {
        return Ok(());
    }

    match run_service_command(&["stop", ROUTE_SERVICE_NAME])? {
        ServiceCommandStatus::Success(_) => wait_for_route_service_running(false),
        ServiceCommandStatus::Missing => Ok(()),
        ServiceCommandStatus::Failed(detail) => Err(format!("Route service stop failed: {detail}")),
    }
}

#[cfg(not(target_os = "windows"))]
pub fn stop_route_service_if_present() -> Result<(), String> {
    Ok(())
}

#[cfg(target_os = "windows")]
pub fn uninstall_route_service_if_present() -> Result<(), String> {
    if !route_service_is_installed()? {
        return Ok(());
    }

    stop_route_service_if_present()?;
    match run_service_command(&["delete", ROUTE_SERVICE_NAME])? {
        ServiceCommandStatus::Success(_) | ServiceCommandStatus::Missing => Ok(()),
        ServiceCommandStatus::Failed(detail) => Err(format!("Route service removal failed: {detail}")),
    }
}

#[cfg(not(target_os = "windows"))]
pub fn uninstall_route_service_if_present() -> Result<(), String> {
    Ok(())
}

#[cfg(target_os = "windows")]
pub fn route_service_binary_path() -> Result<PathBuf, String> {
    let current_exe =
        std::env::current_exe().map_err(|err| format!("Failed to locate the app binary: {err}"))?;
    let exe_dir = current_exe
        .parent()
        .ok_or_else(|| "App binary has no parent directory.".to_string())?;
    let service_exe = exe_dir.join("SuperRouteService.exe");
    if service_exe.exists() {
        Ok(service_exe)
    } else {
        Err(format!(
            "Route service binary is missing from the installation: {}",
            service_exe.display()
        ))
    }
}

#[cfg(not(target_os = "windows"))]
pub fn route_service_binary_path() -> Result<PathBuf, String> {
    Err("Route service is only available on Windows.".to_string())
}

#[cfg(target_os = "windows")]
fn create_route_service(service_path: &std::path::Path) -> Result<(), String> {
    let args = build_create_route_service_args(service_path);
    let args_refs: Vec<&str> = args.iter().map(String::as_str).collect();
    match run_service_command(&args_refs)? {
        ServiceCommandStatus::Success(_) => {
            let _ = run_service_command(&[
                "description",
                ROUTE_SERVICE_NAME,
                ROUTE_SERVICE_DESCRIPTION,
            ]);
            Ok(())
        }
        ServiceCommandStatus::Missing => Err("Route service creation returned a missing-service marker.".to_string()),
        ServiceCommandStatus::Failed(detail) => Err(format!("Route service creation failed: {detail}")),
    }
}

#[cfg(target_os = "windows")]
fn configure_route_service(service_path: &std::path::Path) -> Result<(), String> {
    let args = build_configure_route_service_args(service_path);
    let args_refs: Vec<&str> = args.iter().map(String::as_str).collect();
    match run_service_command(&args_refs)? {
        ServiceCommandStatus::Success(_) => {
            let _ = run_service_command(&[
                "description",
                ROUTE_SERVICE_NAME,
                ROUTE_SERVICE_DESCRIPTION,
            ]);
            Ok(())
        }
        ServiceCommandStatus::Missing => create_route_service(service_path),
        ServiceCommandStatus::Failed(detail) => Err(format!("Route service update failed: {detail}")),
    }
}

#[cfg(target_os = "windows")]
fn build_create_route_service_args(service_path: &std::path::Path) -> Vec<String> {
    build_route_service_command_args("create", service_path)
}

#[cfg(target_os = "windows")]
fn build_configure_route_service_args(service_path: &std::path::Path) -> Vec<String> {
    build_route_service_command_args("config", service_path)
}

#[cfg(target_os = "windows")]
fn build_route_service_command_args(
    operation: &str,
    service_path: &std::path::Path,
) -> Vec<String> {
    let service_path = service_path.to_string_lossy().to_string();
    vec![
        operation.to_string(),
        ROUTE_SERVICE_NAME.to_string(),
        "binPath=".to_string(),
        format!("\"{}\"", service_path),
        "start=".to_string(),
        "auto".to_string(),
        "DisplayName=".to_string(),
        format!("\"{}\"", ROUTE_SERVICE_DISPLAY_NAME),
    ]
}

#[cfg(target_os = "windows")]
fn start_route_service() -> Result<(), String> {
    match run_service_command(&["start", ROUTE_SERVICE_NAME])? {
        ServiceCommandStatus::Success(detail) => {
            if detail.to_ascii_lowercase().contains("already running") {
                return Ok(());
            }
            wait_for_route_service_running(true)
        }
        ServiceCommandStatus::Missing => Err("Route service is not installed.".to_string()),
        ServiceCommandStatus::Failed(detail) => {
            if detail.to_ascii_lowercase().contains("already running") {
                Ok(())
            } else {
                Err(format!("Route service start failed: {detail}"))
            }
        }
    }
}

#[cfg(target_os = "windows")]
fn wait_for_route_service_running(expected_running: bool) -> Result<(), String> {
    let deadline = Instant::now() + ROUTE_SERVICE_STATE_TIMEOUT;
    while Instant::now() < deadline {
        let is_running = route_service_is_running()?;
        if is_running == expected_running {
            return Ok(());
        }
        std::thread::sleep(ROUTE_SERVICE_STATE_POLL);
    }

    let state_label = if expected_running { "running" } else { "stopped" };
    Err(format!(
        "Timed out waiting for the route service to become {state_label}."
    ))
}

#[cfg(target_os = "windows")]
fn run_service_command(args: &[&str]) -> Result<ServiceCommandStatus, String> {
    let output = run_hidden_output_blocking("sc", args)?;
    let detail = service_command_output(&output);
    if output.status.success() {
        Ok(ServiceCommandStatus::Success(detail))
    } else if service_missing_marker_present(&detail) {
        Ok(ServiceCommandStatus::Missing)
    } else {
        Ok(ServiceCommandStatus::Failed(detail))
    }
}

#[cfg(target_os = "windows")]
fn service_command_output(output: &std::process::Output) -> String {
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
fn service_missing_marker_present(detail: &str) -> bool {
    let normalized = detail.to_ascii_lowercase();
    normalized.contains("does not exist as an installed service")
        || normalized.contains("specified service does not exist")
}

#[cfg(target_os = "windows")]
fn service_is_running(detail: &str) -> bool {
    let normalized = detail.to_ascii_uppercase();
    normalized.contains("STATE") && normalized.contains("RUNNING")
}

#[cfg(all(test, target_os = "windows"))]
mod tests {
    use super::{
        build_configure_route_service_args, build_create_route_service_args, service_is_running,
        service_missing_marker_present, ROUTE_SERVICE_DISPLAY_NAME, ROUTE_SERVICE_NAME,
    };
    use std::path::Path;

    #[test]
    fn service_missing_marker_matches_sc_output() {
        assert!(service_missing_marker_present(
            "[SC] OpenService FAILED 1060:\nThe specified service does not exist as an installed service."
        ));
        assert!(!service_missing_marker_present("[SC] OpenService FAILED 5: Access is denied."));
    }

    #[test]
    fn service_running_parser_detects_state_marker() {
        assert!(service_is_running(
            "STATE              : 4  RUNNING\nWIN32_EXIT_CODE    : 0  (0x0)"
        ));
        assert!(!service_is_running(
            "STATE              : 1  STOPPED\nWIN32_EXIT_CODE    : 0  (0x0)"
        ));
    }

    #[test]
    fn create_service_args_keep_sc_option_keys_and_values_separate() {
        let args = build_create_route_service_args(Path::new(
            r"C:\Program Files\SuperRoutePro\SuperRouteService.exe",
        ));

        assert_eq!(args[0], "create");
        assert_eq!(args[1], ROUTE_SERVICE_NAME);
        assert_eq!(args[2], "binPath=");
        assert_eq!(args[3], r#""C:\Program Files\SuperRoutePro\SuperRouteService.exe""#);
        assert_eq!(args[4], "start=");
        assert_eq!(args[5], "auto");
        assert_eq!(args[6], "DisplayName=");
        assert_eq!(args[7], format!(r#""{}""#, ROUTE_SERVICE_DISPLAY_NAME));
    }

    #[test]
    fn config_service_args_keep_sc_option_keys_and_values_separate() {
        let args = build_configure_route_service_args(Path::new(
            r"C:\Program Files\SuperRoutePro\SuperRouteService.exe",
        ));

        assert_eq!(args[0], "config");
        assert_eq!(args[4], "start=");
        assert_eq!(args[5], "auto");
    }
}
