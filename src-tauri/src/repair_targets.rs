use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RepairTargetUser {
    pub sid: String,
    pub account_name: String,
    pub profile_path: String,
    pub is_loaded: bool,
}

#[derive(Debug, Deserialize)]
struct RawRepairTargetUser {
    sid: String,
    account_name: Option<String>,
    profile_path: String,
    is_loaded: bool,
}

pub fn validate_target_sid(sid: &str) -> bool {
    let mut parts = sid.trim().split('-');

    if parts.next() != Some("S") {
        return false;
    }

    let Some(revision) = parts.next() else {
        return false;
    };

    if revision.parse::<u64>().is_err() {
        return false;
    }

    let mut saw_identifier_authority = false;
    for part in parts {
        if part.is_empty() || part.parse::<u64>().is_err() {
            return false;
        }
        saw_identifier_authority = true;
    }

    saw_identifier_authority
}

pub fn normalize_profile_root(raw_path: &str) -> Option<PathBuf> {
    let normalized = raw_path.trim().replace('/', "\\");
    if normalized.is_empty() || normalized.contains("..") {
        return None;
    }

    let trimmed = normalized.trim_end_matches('\\');
    let mut parts = trimmed.split('\\').filter(|part| !part.is_empty());

    let drive = parts.next()?;
    let users_dir = parts.next()?;
    let profile_name = parts.next()?;

    if drive.len() != 2 || !drive.ends_with(':') {
        return None;
    }

    if !users_dir.eq_ignore_ascii_case("users") {
        return None;
    }

    if profile_name == "." || profile_name == ".." {
        return None;
    }

    if parts.next().is_some() {
        return None;
    }

    let drive_letter = drive.chars().next()?.to_ascii_uppercase();
    Some(PathBuf::from(format!(
        "{drive_letter}:\\Users\\{profile_name}"
    )))
}

#[cfg(target_os = "windows")]
pub fn list_repair_targets() -> Result<Vec<RepairTargetUser>, String> {
    use windows_sys::Win32::Foundation::*;
    use windows_sys::Win32::System::Registry::*;

    const PROFILE_LIST_PATH: &[u8] =
        b"SOFTWARE\\Microsoft\\Windows NT\\CurrentVersion\\ProfileList\0";

    unsafe {
        let mut hkey: HKEY = std::ptr::null_mut();
        let result = RegOpenKeyExA(
            HKEY_LOCAL_MACHINE,
            PROFILE_LIST_PATH.as_ptr(),
            0,
            KEY_READ,
            &mut hkey,
        );
        if result != 0 {
            return Err(format!(
                "Unable to open ProfileList registry key (error {result})"
            ));
        }

        let mut targets: Vec<RepairTargetUser> = Vec::new();
        let mut index = 0u32;
        let mut name_buf = [0u8; 256];

        loop {
            let mut name_len = name_buf.len() as u32;
            let enum_result = RegEnumKeyExA(
                hkey,
                index,
                name_buf.as_mut_ptr(),
                &mut name_len,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
            );
            if enum_result != 0 {
                break;
            }
            index += 1;

            let sid = String::from_utf8_lossy(&name_buf[..name_len as usize]).to_string();

            // Only include real user profiles (S-1-5-21-*)
            if !sid.starts_with("S-1-5-21-") {
                continue;
            }

            if !validate_target_sid(&sid) {
                continue;
            }

            // Read ProfileImagePath from the subkey
            let subkey_path =
                format!("SOFTWARE\\Microsoft\\Windows NT\\CurrentVersion\\ProfileList\\{sid}\0");
            let mut subkey: HKEY = std::ptr::null_mut();
            let open_result = RegOpenKeyExA(
                HKEY_LOCAL_MACHINE,
                subkey_path.as_ptr(),
                0,
                KEY_READ,
                &mut subkey,
            );
            if open_result != 0 {
                continue;
            }

            let mut path_buf = [0u8; 512];
            let mut path_len = path_buf.len() as u32;
            let mut value_type: u32 = 0;
            let value_name = b"ProfileImagePath\0";
            let query_result = RegQueryValueExA(
                subkey,
                value_name.as_ptr(),
                std::ptr::null_mut(),
                &mut value_type,
                path_buf.as_mut_ptr(),
                &mut path_len,
            );
            let _ = RegCloseKey(subkey);

            if query_result != 0 || path_len == 0 {
                continue;
            }

            // Remove trailing null byte if present
            let actual_len = if path_len > 0 && path_buf[path_len as usize - 1] == 0 {
                (path_len - 1) as usize
            } else {
                path_len as usize
            };
            let raw_path = String::from_utf8_lossy(&path_buf[..actual_len]).to_string();

            // Expand %SystemDrive% etc.
            let expanded_path = if raw_path.contains('%') {
                let mut expand_buf = [0u8; 512];
                let raw_cstr = format!("{raw_path}\0");
                let expanded_len =
                    windows_sys::Win32::System::Environment::ExpandEnvironmentStringsA(
                        raw_cstr.as_ptr(),
                        expand_buf.as_mut_ptr(),
                        expand_buf.len() as u32,
                    );
                if expanded_len > 0 && (expanded_len as usize) <= expand_buf.len() {
                    String::from_utf8_lossy(&expand_buf[..(expanded_len as usize - 1)]).to_string()
                } else {
                    raw_path
                }
            } else {
                raw_path
            };

            // Filter: must be under \Users\
            let path_lower = expanded_path.to_lowercase();
            if !path_lower.contains("\\users\\") {
                continue;
            }

            let Some(profile_path) = normalize_profile_root(&expanded_path) else {
                continue;
            };

            let account_name = profile_path
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("unknown")
                .to_string();

            // Check if profile is loaded by checking if the SID hive is loaded in HKU
            let hku_path = format!("{sid}\0");
            let mut hku_test: HKEY = std::ptr::null_mut();
            let is_loaded =
                RegOpenKeyExA(HKEY_USERS, hku_path.as_ptr(), 0, KEY_READ, &mut hku_test) == 0;
            if is_loaded {
                let _ = RegCloseKey(hku_test);
            }

            targets.push(RepairTargetUser {
                sid,
                account_name,
                profile_path: profile_path.to_string_lossy().to_string(),
                is_loaded,
            });
        }

        let _ = RegCloseKey(hkey);

        targets.sort_by(|left, right| {
            right
                .is_loaded
                .cmp(&left.is_loaded)
                .then_with(|| left.account_name.cmp(&right.account_name))
        });

        Ok(targets)
    }
}

#[cfg(target_os = "windows")]
pub fn resolve_repair_target_by_sid(target_sid: &str) -> Result<RepairTargetUser, String> {
    if !validate_target_sid(target_sid) {
        return Err("Invalid target SID".to_string());
    }

    list_repair_targets()?
        .into_iter()
        .find(|target| target.sid == target_sid)
        .ok_or_else(|| format!("Unable to resolve target profile for SID {target_sid}"))
}

#[cfg(not(target_os = "windows"))]
pub fn resolve_repair_target_by_sid(_target_sid: &str) -> Result<RepairTargetUser, String> {
    Err("Repair targets are only available on Windows.".to_string())
}

#[cfg(not(target_os = "windows"))]
pub fn list_repair_targets() -> Result<Vec<RepairTargetUser>, String> {
    Err("Repair targets are only available on Windows.".to_string())
}
