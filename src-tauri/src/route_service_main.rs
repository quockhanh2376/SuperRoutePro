// Prevents console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
#![cfg_attr(test, allow(dead_code))]

//! SuperRouteService — Run-once startup service that re-applies WAN and custom
//! routes using stable NIC identifiers (description / MAC) to survive
//! InterfaceIndex changes across reboots.

#[cfg(test)]
use std::collections::HashMap;
use std::thread;
use std::time::Duration;
use super_route_pro_lib::route_apply::{
    apply_persist_config_with_adapters, resolve_nic_interface_index_from_adapters,
};
#[cfg(test)]
use super_route_pro_lib::route_apply::{
    build_nic_index_lookup, resolve_custom_route_interface_index,
};
use super_route_pro_lib::route_persist::{self, NicIdentifier, PersistConfig};
#[cfg(test)]
use super_route_pro_lib::route_persist::CustomRoute;

const MAX_RETRY_SECONDS: u64 = 60;
const RETRY_INTERVAL_SECONDS: u64 = 5;

fn main() {
    let config = match route_persist::load_config() {
        Ok(Some(c)) => c,
        Ok(None) => {
            eprintln!("[SuperRouteService] No persist.json found. Exiting.");
            return;
        }
        Err(e) => {
            eprintln!("[SuperRouteService] Failed to load config: {e}");
            return;
        }
    };

    if !config.enabled {
        eprintln!("[SuperRouteService] Persist disabled. Exiting.");
        return;
    }

    // Retry loop: wait for NIC to come up after boot.
    let mut elapsed: u64 = 0;
    loop {
        match find_nic_interface_index(&config.nic) {
            Ok((index, adapters)) => {
                eprintln!(
                    "[SuperRouteService] NIC '{}' found at InterfaceIndex {index}",
                    config.nic.description
                );
                apply_routes(&config, &index, &adapters);
                eprintln!("[SuperRouteService] Routes applied. Exiting.");
                return;
            }
            Err(e) => {
                if elapsed >= MAX_RETRY_SECONDS {
                    let msg = format!(
                        "Super Route Pro: NIC '{}' not found after {MAX_RETRY_SECONDS}s. Routes not applied.",
                        config.nic.description
                    );
                    eprintln!("[SuperRouteService] {msg}");
                    show_balloon_tip("Super Route Pro", &msg);
                    return;
                }
                eprintln!(
                    "[SuperRouteService] NIC not found ({e}), retrying in {RETRY_INTERVAL_SECONDS}s... ({elapsed}/{MAX_RETRY_SECONDS}s)"
                );
                thread::sleep(Duration::from_secs(RETRY_INTERVAL_SECONDS));
                elapsed += RETRY_INTERVAL_SECONDS;
            }
        }
    }
}

// ---------------------------------------------------------------------------
// NIC matching
// ---------------------------------------------------------------------------

/// Find the current InterfaceIndex of a NIC by matching its description (primary)
/// or MAC address (fallback).
fn find_nic_interface_index(
    nic: &NicIdentifier,
) -> Result<(String, Vec<super_route_pro_lib::win32_net::NativeNic>), String> {
    let adapters = super_route_pro_lib::win32_net::enumerate_adapters()?;
    let index = resolve_nic_interface_index_from_adapters(nic, &adapters)?;
    Ok((index, adapters))
}

// ---------------------------------------------------------------------------
// Route application
// ---------------------------------------------------------------------------

fn apply_routes(
    config: &PersistConfig,
    interface_index: &str,
    adapters: &[super_route_pro_lib::win32_net::NativeNic],
) {
    match apply_persist_config_with_adapters(config, adapters) {
        Ok(report) => {
            for line in report.output_lines {
                eprintln!("[SuperRouteService] {line}");
            }
            eprintln!(
                "[SuperRouteService] Persist config applied on interface {interface_index} with {} custom route(s).",
                report.custom_route_count
            );
        }
        Err(err) => eprintln!("[SuperRouteService] Failed to apply persisted routes: {err}"),
    }
}

// ---------------------------------------------------------------------------
// Balloon tip notification (Windows Shell_NotifyIcon)
// ---------------------------------------------------------------------------

#[cfg(target_os = "windows")]
fn show_balloon_tip(title: &str, message: &str) {
    use std::mem;
    use std::ptr;

    // Shell_NotifyIconW constants
    const NIM_ADD: u32 = 0x00000000;
    const NIM_MODIFY: u32 = 0x00000001;
    const NIM_DELETE: u32 = 0x00000002;
    const NIF_ICON: u32 = 0x00000002;
    const NIF_TIP: u32 = 0x00000004;
    const NIF_INFO: u32 = 0x00000010;
    const NIIF_INFO: u32 = 0x00000001;

    #[repr(C)]
    #[allow(non_snake_case)]
    struct NOTIFYICONDATAW {
        cbSize: u32,
        hWnd: *mut std::ffi::c_void,
        uID: u32,
        uFlags: u32,
        uCallbackMessage: u32,
        hIcon: *mut std::ffi::c_void,
        szTip: [u16; 128],
        dwState: u32,
        dwStateMask: u32,
        szInfo: [u16; 256],
        uTimeoutOrVersion: u32,
        szInfoTitle: [u16; 64],
        dwInfoFlags: u32,
        guidItem: [u8; 16],
        hBalloonIcon: *mut std::ffi::c_void,
    }

    #[link(name = "shell32")]
    extern "system" {
        fn Shell_NotifyIconW(message: u32, data: *mut NOTIFYICONDATAW) -> i32;
    }

    fn to_wide_fixed<const N: usize>(s: &str) -> [u16; N] {
        let mut buf = [0u16; N];
        for (i, ch) in s.encode_utf16().take(N - 1).enumerate() {
            buf[i] = ch;
        }
        buf
    }

    unsafe {
        let mut nid: NOTIFYICONDATAW = mem::zeroed();
        nid.cbSize = mem::size_of::<NOTIFYICONDATAW>() as u32;
        nid.hWnd = ptr::null_mut();
        nid.uID = 1;
        nid.uFlags = NIF_ICON | NIF_TIP | NIF_INFO;
        nid.szTip = to_wide_fixed::<128>("Super Route Pro");
        nid.szInfo = to_wide_fixed::<256>(message);
        nid.szInfoTitle = to_wide_fixed::<64>(title);
        nid.dwInfoFlags = NIIF_INFO;
        nid.uTimeoutOrVersion = 5000; // 5 seconds

        Shell_NotifyIconW(NIM_ADD, &mut nid);
        Shell_NotifyIconW(NIM_MODIFY, &mut nid);

        // Keep alive long enough for user to see the balloon
        thread::sleep(Duration::from_secs(5));

        Shell_NotifyIconW(NIM_DELETE, &mut nid);
    }
}

#[cfg(not(target_os = "windows"))]
fn show_balloon_tip(_title: &str, message: &str) {
    eprintln!("[SuperRouteService] NOTIFICATION: {message}");
}

#[cfg(test)]
pub(crate) fn test_build_nic_index_lookup(
    adapters: &[super_route_pro_lib::win32_net::NativeNic],
) -> HashMap<String, String> {
    build_nic_index_lookup(adapters)
}

#[cfg(test)]
pub(crate) fn test_resolve_nic_interface_index_from_adapters(
    nic: &NicIdentifier,
    adapters: &[super_route_pro_lib::win32_net::NativeNic],
) -> Result<String, String> {
    resolve_nic_interface_index_from_adapters(nic, adapters)
}

#[cfg(test)]
pub(crate) fn test_resolve_custom_route_interface_index(
    route: &CustomRoute,
    default_interface_index: &str,
    nic_index_lookup: &HashMap<String, String>,
) -> Result<String, String> {
    resolve_custom_route_interface_index(route, default_interface_index, nic_index_lookup)
}
