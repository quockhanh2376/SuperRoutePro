mod network;

use network::{
    get_network_interfaces, get_routing_table, add_route, delete_route,
    flush_routes, set_default_gateway, run_network_command, ping_host,
    check_internet, fping_scan, get_bloatware_candidates, remove_bloatware,
    clear_cache_targets, get_battery_report,
};
#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;
#[cfg(target_os = "windows")]
use std::process::Command;

#[cfg(target_os = "windows")]
const CREATE_NO_WINDOW: u32 = 0x08000000;
#[cfg(target_os = "windows")]
// Windows 10 RTM build. This also covers all Windows 11 builds.
const MIN_WINDOWS_BUILD: u32 = 10240;
#[cfg(target_os = "windows")]
const REQUIRED_COMMANDS: [&str; 5] = ["route", "netsh", "ipconfig", "ping", "powershell"];
#[cfg(target_os = "windows")]
const WEBVIEW2_CLIENT_GUID: &str = "{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}";
#[cfg(target_os = "windows")]
const DEV_DISABLE_ERROR_DIALOG_ENV: &str = "SRP_DEV_NO_DIALOG";

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    if let Err(reason) = validate_runtime_environment() {
        block_app_start(&reason);
    }

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
            get_bloatware_candidates,
            remove_bloatware,
            clear_cache_targets,
            get_battery_report,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[cfg(target_os = "windows")]
fn validate_runtime_environment() -> Result<(), String> {
    let mut failures: Vec<String> = Vec::new();

    match detect_windows_build_number() {
        Some(build) if build >= MIN_WINDOWS_BUILD => {}
        Some(build) => failures.push(format!(
            "Windows build {build} detected. This app supports Windows 10/11 (build >= {MIN_WINDOWS_BUILD})."
        )),
        None => failures.push("Unable to detect Windows build number.".to_string()),
    }

    match is_running_as_admin() {
        Some(true) => {}
        Some(false) => failures.push(
            "The app must run with Administrator privileges to manage routes and NIC settings."
                .to_string(),
        ),
        None => failures.push("Unable to verify Administrator privileges.".to_string()),
    }

    if !has_webview2_runtime() {
        failures.push("Microsoft Edge WebView2 Runtime is not installed.".to_string());
    }

    for command in REQUIRED_COMMANDS {
        if !command_exists(command) {
            failures.push(format!("Required system command is missing: {command}"));
        }
    }

    if failures.is_empty() {
        Ok(())
    } else {
        let bullet_list = failures
            .iter()
            .map(|item| format!("- {item}"))
            .collect::<Vec<String>>()
            .join("\n");
        Err(format!(
            "Environment check failed. Super Route Pro cannot start on this machine.\n\n{bullet_list}\n\nPlease fix the items above, then start the app again."
        ))
    }
}

#[cfg(not(target_os = "windows"))]
fn validate_runtime_environment() -> Result<(), String> {
    Err("This build only supports Windows.".to_string())
}

fn block_app_start(reason: &str) -> ! {
    #[cfg(target_os = "windows")]
    {
        if env_flag_enabled(DEV_DISABLE_ERROR_DIALOG_ENV) {
            eprintln!(
                "[DEV] {} enabled: skip startup MessageBox and print error to console.",
                DEV_DISABLE_ERROR_DIALOG_ENV
            );
        } else {
            show_windows_error_dialog("Super Route Pro - Unsupported Environment", reason);
        }
    }
    eprintln!("{reason}");
    std::process::exit(1);
}

#[cfg(target_os = "windows")]
fn env_flag_enabled(var_name: &str) -> bool {
    std::env::var(var_name)
        .map(|value| {
            matches!(
                value.trim().to_ascii_lowercase().as_str(),
                "1" | "true" | "yes" | "on"
            )
        })
        .unwrap_or(false)
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
fn detect_windows_build_number() -> Option<u32> {
    let output = run_hidden(
        "powershell",
        &[
            "-NoProfile",
            "-NonInteractive",
            "-Command",
            "(Get-CimInstance Win32_OperatingSystem).BuildNumber",
        ],
    )?;

    if !output.status.success() {
        return None;
    }

    String::from_utf8_lossy(&output.stdout)
        .trim()
        .parse::<u32>()
        .ok()
}

#[cfg(target_os = "windows")]
fn is_running_as_admin() -> Option<bool> {
    let output = run_hidden(
        "powershell",
        &[
            "-NoProfile",
            "-NonInteractive",
            "-Command",
            "([Security.Principal.WindowsPrincipal] [Security.Principal.WindowsIdentity]::GetCurrent()).IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)",
        ],
    )?;

    if !output.status.success() {
        return None;
    }

    let value = String::from_utf8_lossy(&output.stdout);
    Some(value.trim().eq_ignore_ascii_case("true"))
}

#[cfg(target_os = "windows")]
fn command_exists(name: &str) -> bool {
    run_hidden("where", &[name])
        .map(|output| output.status.success())
        .unwrap_or(false)
}

#[cfg(target_os = "windows")]
fn has_webview2_runtime() -> bool {
    let keys = [
        format!(
            r"HKLM\SOFTWARE\Microsoft\EdgeUpdate\Clients\{WEBVIEW2_CLIENT_GUID}"
        ),
        format!(
            r"HKLM\SOFTWARE\WOW6432Node\Microsoft\EdgeUpdate\Clients\{WEBVIEW2_CLIENT_GUID}"
        ),
        format!(
            r"HKCU\SOFTWARE\Microsoft\EdgeUpdate\Clients\{WEBVIEW2_CLIENT_GUID}"
        ),
    ];

    keys.iter()
        .any(|key| registry_value_exists(key, "pv"))
}

#[cfg(target_os = "windows")]
fn registry_value_exists(key: &str, value_name: &str) -> bool {
    run_hidden("reg", &["query", key, "/v", value_name])
        .map(|output| output.status.success())
        .unwrap_or(false)
}

#[cfg(target_os = "windows")]
fn show_windows_error_dialog(title: &str, message: &str) {
    use windows_sys::Win32::UI::WindowsAndMessaging::{MB_ICONERROR, MB_OK, MessageBoxW};

    let title_wide: Vec<u16> = title.encode_utf16().chain(std::iter::once(0)).collect();
    let message_wide: Vec<u16> = message.encode_utf16().chain(std::iter::once(0)).collect();

    unsafe {
        MessageBoxW(
            std::ptr::null_mut(),
            message_wide.as_ptr(),
            title_wide.as_ptr(),
            MB_OK | MB_ICONERROR,
        );
    }
}
