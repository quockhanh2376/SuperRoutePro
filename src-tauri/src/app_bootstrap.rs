#[cfg(target_os = "windows")]
use crate::process_exec::command_exists_on_path;
#[cfg(target_os = "windows")]
use std::path::{Path, PathBuf};
use tauri::{Manager, Runtime, WebviewWindowBuilder};

#[cfg(target_os = "windows")]
const MIN_WINDOWS_BUILD: u32 = 10240;
#[cfg(target_os = "windows")]
const REQUIRED_COMMANDS: [&str; 4] = ["route", "netsh", "ipconfig", "ping"];
#[cfg(target_os = "windows")]
const WEBVIEW2_CLIENT_GUID: &str = "{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}";
#[cfg(target_os = "windows")]
const DEV_DISABLE_ERROR_DIALOG_ENV: &str = "SRP_DEV_NO_DIALOG";

pub(crate) fn validate_or_exit() {
    if let Err(reason) = validate_runtime_environment() {
        block_app_start(&reason);
    }
}

pub(crate) fn setup_main_window<R: Runtime>(
    app: &mut tauri::App<R>,
) -> Result<(), Box<dyn std::error::Error>> {
    let main_window_config = app
        .config()
        .app
        .windows
        .iter()
        .find(|window| window.label == "main")
        .cloned()
        .expect("main window config should exist");

    let primary_webview_data_dir = app.path().app_local_data_dir()?.join("main-webview");
    let build_window = |data_dir: PathBuf| {
        WebviewWindowBuilder::from_config(app.handle(), &main_window_config)?
            .data_directory(data_dir)
            .build()
    };

    prepare_webview_data_directory(&primary_webview_data_dir)
        .or_else(|_| {
            reset_webview_data_directory(&primary_webview_data_dir)?;
            prepare_webview_data_directory(&primary_webview_data_dir)
        })
        .map_err(std::io::Error::other)?;

    match build_window(primary_webview_data_dir.clone()) {
        Ok(_) => {}
        Err(first_err) => {
            if !should_retry_webview_data_dir(&first_err.to_string()) {
                return Err(first_err.into());
            }

            let retry_dir = reset_webview_data_directory(&primary_webview_data_dir)
                .map(|_| primary_webview_data_dir.clone())
                .or_else(|_| create_fallback_webview_data_directory(&primary_webview_data_dir))
                .map_err(std::io::Error::other)?;

            prepare_webview_data_directory(&retry_dir).map_err(std::io::Error::other)?;
            build_window(retry_dir)?;
        }
    }

    Ok(())
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

    if !has_webview2_runtime() {
        failures.push("Microsoft Edge WebView2 Runtime is not installed.".to_string());
    }

    for command in REQUIRED_COMMANDS {
        if !command_exists_on_path(command) {
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
fn detect_windows_build_number() -> Option<u32> {
    read_registry_string(
        windows_sys::Win32::System::Registry::HKEY_LOCAL_MACHINE,
        "SOFTWARE\\Microsoft\\Windows NT\\CurrentVersion",
        "CurrentBuildNumber",
    )
    .and_then(|value| value.trim().parse::<u32>().ok())
}

#[cfg(target_os = "windows")]
fn has_webview2_runtime() -> bool {
    use windows_sys::Win32::System::Registry::{HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE};

    let subkeys = [
        (
            HKEY_LOCAL_MACHINE,
            format!("SOFTWARE\\Microsoft\\EdgeUpdate\\Clients\\{WEBVIEW2_CLIENT_GUID}"),
        ),
        (
            HKEY_LOCAL_MACHINE,
            format!(
                "SOFTWARE\\WOW6432Node\\Microsoft\\EdgeUpdate\\Clients\\{WEBVIEW2_CLIENT_GUID}"
            ),
        ),
        (
            HKEY_CURRENT_USER,
            format!("SOFTWARE\\Microsoft\\EdgeUpdate\\Clients\\{WEBVIEW2_CLIENT_GUID}"),
        ),
    ];

    subkeys
        .iter()
        .any(|(root, subkey)| read_registry_string(*root, subkey, "pv").is_some())
}

#[cfg(target_os = "windows")]
fn read_registry_string(
    root: windows_sys::Win32::System::Registry::HKEY,
    subkey: &str,
    value_name: &str,
) -> Option<String> {
    use windows_sys::Win32::Foundation::ERROR_SUCCESS;
    use windows_sys::Win32::System::Registry::{
        RegCloseKey, RegOpenKeyExW, RegQueryValueExW, KEY_READ, REG_SZ,
    };

    let subkey_wide: Vec<u16> = subkey.encode_utf16().chain(std::iter::once(0)).collect();
    let value_wide: Vec<u16> = value_name
        .encode_utf16()
        .chain(std::iter::once(0))
        .collect();

    let mut hkey: windows_sys::Win32::System::Registry::HKEY = std::ptr::null_mut();
    let status = unsafe { RegOpenKeyExW(root, subkey_wide.as_ptr(), 0, KEY_READ, &mut hkey) };
    if status as u32 != ERROR_SUCCESS {
        return None;
    }

    let mut data_type: u32 = 0;
    let mut data_size: u32 = 0;
    let status = unsafe {
        RegQueryValueExW(
            hkey,
            value_wide.as_ptr(),
            std::ptr::null(),
            &mut data_type,
            std::ptr::null_mut(),
            &mut data_size,
        )
    };
    if status as u32 != ERROR_SUCCESS || data_type != REG_SZ {
        unsafe { RegCloseKey(hkey) };
        return None;
    }

    let mut buffer: Vec<u8> = vec![0u8; data_size as usize];
    let status = unsafe {
        RegQueryValueExW(
            hkey,
            value_wide.as_ptr(),
            std::ptr::null(),
            &mut data_type,
            buffer.as_mut_ptr(),
            &mut data_size,
        )
    };
    unsafe { RegCloseKey(hkey) };

    if status as u32 != ERROR_SUCCESS {
        return None;
    }

    let wide_slice = unsafe {
        std::slice::from_raw_parts(buffer.as_ptr() as *const u16, data_size as usize / 2)
    };
    let trimmed = wide_slice
        .iter()
        .copied()
        .take_while(|&c| c != 0)
        .collect::<Vec<u16>>();
    Some(String::from_utf16_lossy(&trimmed))
}

#[cfg(target_os = "windows")]
fn show_windows_error_dialog(title: &str, message: &str) {
    use windows_sys::Win32::UI::WindowsAndMessaging::{MessageBoxW, MB_ICONERROR, MB_OK};

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

#[cfg(target_os = "windows")]
fn prepare_webview_data_directory(path: &Path) -> Result<(), String> {
    std::fs::create_dir_all(path).map_err(|err| {
        format!(
            "Unable to create WebView data directory {}: {err}",
            path.display()
        )
    })?;

    let webview_runtime_root = path.join("EBWebView");
    std::fs::create_dir_all(&webview_runtime_root).map_err(|err| {
        format!(
            "Unable to create WebView runtime directory {}: {err}",
            webview_runtime_root.display()
        )
    })?;

    let probe_path = webview_runtime_root.join(".srp-write-probe");
    std::fs::write(&probe_path, b"probe").map_err(|err| {
        format!(
            "Unable to write WebView probe file {}: {err}",
            probe_path.display()
        )
    })?;
    let _ = std::fs::remove_file(&probe_path);

    Ok(())
}

#[cfg(target_os = "windows")]
fn next_webview_recovery_path(primary: &Path, label: &str) -> PathBuf {
    let parent = primary
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."));
    let stem = primary
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("main-webview");

    let mut attempt = 1u32;
    loop {
        let suffix = if attempt == 1 {
            format!("{stem}-{label}")
        } else {
            format!("{stem}-{label}-{attempt}")
        };
        let candidate = parent.join(suffix);
        if !candidate.exists() {
            return candidate;
        }
        attempt += 1;
    }
}

#[cfg(target_os = "windows")]
fn reset_webview_data_directory(primary: &Path) -> Result<PathBuf, String> {
    let reset_path = next_webview_recovery_path(primary, "reset");

    if primary.exists() {
        std::fs::rename(primary, &reset_path).map_err(|err| {
            format!(
                "Unable to rotate stale WebView data directory {} -> {}: {err}",
                primary.display(),
                reset_path.display()
            )
        })?;
    }

    std::fs::create_dir_all(primary).map_err(|err| {
        format!(
            "Unable to recreate fresh WebView data directory {}: {err}",
            primary.display()
        )
    })?;

    Ok(reset_path)
}

#[cfg(target_os = "windows")]
fn create_fallback_webview_data_directory(primary: &Path) -> Result<PathBuf, String> {
    let fallback_path = next_webview_recovery_path(primary, "recovery");
    std::fs::create_dir_all(&fallback_path).map_err(|err| {
        format!(
            "Unable to create fallback WebView data directory {}: {err}",
            fallback_path.display()
        )
    })?;
    Ok(fallback_path)
}

#[cfg(target_os = "windows")]
pub(crate) fn should_retry_webview_data_dir(message: &str) -> bool {
    let normalized = message.to_ascii_lowercase();
    normalized.contains("data directory")
        || normalized.contains("user data folder")
        || normalized.contains("can't read and write")
        || normalized.contains("ebwebview")
}

#[cfg(test)]
mod tests {
    use serde_json::Value;
    use std::path::PathBuf;

    #[test]
    fn main_window_is_opted_out_of_auto_creation() {
        let config_text =
            std::fs::read_to_string("tauri.conf.json").expect("tauri.conf.json should be readable");
        let config: Value =
            serde_json::from_str(&config_text).expect("tauri.conf.json should contain valid JSON");

        let create = config["app"]["windows"][0]["create"].as_bool();
        assert_eq!(
            create,
            Some(false),
            "main window must be created from Rust setup so we can set a writable WebView2 data directory"
        );
    }

    #[test]
    fn startup_contract_keeps_ui_as_standard_user_process() {
        let manifest_text = std::fs::read_to_string("super-route-pro.exe.manifest")
            .expect("manifest should be readable");
        assert!(
            manifest_text.contains(r#"<requestedExecutionLevel level="asInvoker" uiAccess="false"/>"#),
            "UI manifest must stay asInvoker so the app can start inside the logged-in standard-user session"
        );

        let lib_text = std::fs::read_to_string("src/lib.rs").expect("lib.rs should be readable");
        let production_code = lib_text
            .split("#[cfg(test)]")
            .next()
            .expect("lib.rs should contain production code before tests");
        assert!(
            !production_code.contains("RELAUNCH_AS_ADMIN_SIGNAL"),
            "UI startup should not relaunch the full app as admin"
        );
        assert!(
            !production_code.contains("relaunch_as_admin()"),
            "UI startup should not call relaunch_as_admin during app boot"
        );

        let build_text = std::fs::read_to_string("build.rs").expect("build.rs should be readable");
        assert!(
            !build_text.contains("admin manifest"),
            "build script should not describe the UI process as using an admin manifest"
        );
    }

    #[test]
    fn webview_data_dir_errors_trigger_a_recovery_retry() {
        assert!(super::should_retry_webview_data_dir(
            "We couldn't create the data directory. Microsoft Edge can't read and write to its data directory: C:\\Users\\demo\\AppData\\Local\\com.superroute.pro\\main-webview\\EBWebView"
        ));
        assert!(super::should_retry_webview_data_dir(
            "webview creation failed because the user data folder is not writable"
        ));
        assert!(
            !super::should_retry_webview_data_dir(
                "Microsoft Edge WebView2 Runtime is not installed."
            ),
            "runtime install errors should not trigger a directory reset retry"
        );
    }

    #[test]
    fn reset_webview_data_directory_moves_stale_state_out_of_the_way() {
        let temp_root =
            std::env::temp_dir().join(format!("srp-webview-reset-test-{}", std::process::id()));
        let primary = temp_root.join("main-webview");
        let stale_file = primary.join("stale.txt");
        std::fs::create_dir_all(&primary).expect("primary webview dir should be creatable");
        std::fs::write(&stale_file, "stale").expect("stale marker should be writable");

        let reset_dir = super::reset_webview_data_directory(&primary)
            .expect("reset should rotate the stale webview directory");

        assert_eq!(primary, PathBuf::from(&primary));
        assert!(
            primary.exists(),
            "primary webview directory should be recreated"
        );
        assert!(
            !stale_file.exists(),
            "stale files should be moved out of the fresh primary directory"
        );
        assert!(
            reset_dir.exists(),
            "rotated reset directory should remain on disk for troubleshooting"
        );
        assert!(
            reset_dir.join("stale.txt").exists(),
            "rotated reset directory should contain the previous stale content"
        );

        let _ = std::fs::remove_dir_all(&temp_root);
    }
}
