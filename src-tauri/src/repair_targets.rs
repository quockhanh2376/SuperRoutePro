use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RepairTargetUser {
    pub sid: String,
    pub account_name: String,
    pub profile_path: String,
    pub is_loaded: bool,
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
fn encode_wide_nul(input: &str) -> Vec<u16> {
    input.encode_utf16().chain(std::iter::once(0)).collect()
}

#[cfg(target_os = "windows")]
fn decode_wide_nul(buffer: &[u16]) -> String {
    let end = buffer.iter().position(|&unit| unit == 0).unwrap_or(buffer.len());
    String::from_utf16_lossy(&buffer[..end])
}

#[cfg(target_os = "windows")]
fn read_registry_wide_string(
    root: windows_sys::Win32::System::Registry::HKEY,
    subkey: &str,
    value_name: &str,
) -> Option<String> {
    use windows_sys::Win32::System::Registry::{
        RegCloseKey, RegOpenKeyExW, RegQueryValueExW, HKEY, KEY_READ, REG_EXPAND_SZ, REG_SZ,
    };

    let subkey_wide = encode_wide_nul(subkey);
    let value_wide = encode_wide_nul(value_name);

    let mut hkey: HKEY = std::ptr::null_mut();
    let open_result =
        unsafe { RegOpenKeyExW(root, subkey_wide.as_ptr(), 0, KEY_READ, &mut hkey) };
    if open_result != 0 {
        return None;
    }

    let mut value_type: u32 = 0;
    let mut byte_len: u32 = 0;
    let size_result = unsafe {
        RegQueryValueExW(
            hkey,
            value_wide.as_ptr(),
            std::ptr::null_mut(),
            &mut value_type,
            std::ptr::null_mut(),
            &mut byte_len,
        )
    };
    if size_result != 0 || (value_type != REG_SZ && value_type != REG_EXPAND_SZ) {
        unsafe {
            let _ = RegCloseKey(hkey);
        }
        return None;
    }

    if byte_len == 0 {
        unsafe {
            let _ = RegCloseKey(hkey);
        }
        return Some(String::new());
    }

    let mut wide_buf = vec![0u16; (byte_len as usize).div_ceil(2)];
    let mut query_len = (wide_buf.len() * std::mem::size_of::<u16>()) as u32;
    let query_result = unsafe {
        RegQueryValueExW(
            hkey,
            value_wide.as_ptr(),
            std::ptr::null_mut(),
            &mut value_type,
            wide_buf.as_mut_ptr() as *mut u8,
            &mut query_len,
        )
    };
    unsafe {
        let _ = RegCloseKey(hkey);
    }

    if query_result != 0 {
        return None;
    }

    let wide_len = (query_len as usize / 2).min(wide_buf.len());
    Some(decode_wide_nul(&wide_buf[..wide_len]))
}

#[cfg(target_os = "windows")]
fn expand_environment_wide(raw_path: &str) -> String {
    use windows_sys::Win32::System::Environment::ExpandEnvironmentStringsW;

    let raw_wide = encode_wide_nul(raw_path);
    let required = unsafe { ExpandEnvironmentStringsW(raw_wide.as_ptr(), std::ptr::null_mut(), 0) };
    if required == 0 {
        return raw_path.to_string();
    }

    let mut expand_buf = vec![0u16; required as usize];
    let expanded_len = unsafe {
        ExpandEnvironmentStringsW(
            raw_wide.as_ptr(),
            expand_buf.as_mut_ptr(),
            expand_buf.len() as u32,
        )
    };
    if expanded_len > 0 && (expanded_len as usize) <= expand_buf.len() {
        decode_wide_nul(&expand_buf[..(expanded_len as usize)])
    } else {
        raw_path.to_string()
    }
}

#[cfg(target_os = "windows")]
pub fn list_repair_targets() -> Result<Vec<RepairTargetUser>, String> {
    use windows_sys::Win32::System::Registry::{
        RegCloseKey, RegEnumKeyExW, RegOpenKeyExW, HKEY, HKEY_LOCAL_MACHINE, HKEY_USERS, KEY_READ,
    };

    const PROFILE_LIST_SUBKEY: &str =
        r"SOFTWARE\Microsoft\Windows NT\CurrentVersion\ProfileList";
    let profile_list_wide = encode_wide_nul(PROFILE_LIST_SUBKEY);

    unsafe {
        let mut hkey: HKEY = std::ptr::null_mut();
        let result = RegOpenKeyExW(
            HKEY_LOCAL_MACHINE,
            profile_list_wide.as_ptr(),
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
        let mut name_buf = [0u16; 256];

        loop {
            let mut name_len = name_buf.len() as u32;
            let enum_result = RegEnumKeyExW(
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

            let sid = String::from_utf16_lossy(&name_buf[..name_len as usize]);

            // Only include real user profiles (S-1-5-21-*)
            if !sid.starts_with("S-1-5-21-") {
                continue;
            }

            if !validate_target_sid(&sid) {
                continue;
            }

            // Read ProfileImagePath from the subkey using UTF-16 Registry APIs
            let subkey_path = format!("{PROFILE_LIST_SUBKEY}\\{sid}");
            let Some(raw_path) =
                read_registry_wide_string(HKEY_LOCAL_MACHINE, &subkey_path, "ProfileImagePath")
            else {
                continue;
            };
            if raw_path.is_empty() {
                continue;
            }

            // Expand %SystemDrive% etc.
            let expanded_path = if raw_path.contains('%') {
                expand_environment_wide(&raw_path)
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
            let hku_path_wide = encode_wide_nul(&sid);
            let mut hku_test: HKEY = std::ptr::null_mut();
            let is_loaded =
                RegOpenKeyExW(HKEY_USERS, hku_path_wide.as_ptr(), 0, KEY_READ, &mut hku_test) == 0;
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

#[cfg(test)]
mod tests {
    use super::{normalize_profile_root, validate_target_sid};
    use std::path::PathBuf;

    #[test]
    fn validate_target_sid_accepts_realistic_user_sids() {
        assert!(validate_target_sid(
            "S-1-5-21-123456789-234567890-345678901-1001"
        ));
        assert!(validate_target_sid(" S-1-5-18 "));
    }

    #[test]
    fn validate_target_sid_rejects_malformed_values() {
        assert!(!validate_target_sid(""));
        assert!(!validate_target_sid("demo"));
        assert!(!validate_target_sid("S-1-5-21-abc"));
        assert!(!validate_target_sid("S-1-5-21-1234-"));
    }

    #[test]
    fn normalize_profile_root_accepts_users_paths_and_normalizes_drive_case() {
        assert_eq!(
            normalize_profile_root(r"c:\Users\Demo\"),
            Some(PathBuf::from(r"C:\Users\Demo"))
        );
        assert_eq!(
            normalize_profile_root("D:/Users/AnotherUser"),
            Some(PathBuf::from(r"D:\Users\AnotherUser"))
        );
    }

    #[test]
    fn normalize_profile_root_rejects_non_user_roots_and_traversal() {
        assert_eq!(normalize_profile_root(r"C:\Windows\System32"), None);
        assert_eq!(normalize_profile_root(r"C:\Users\demo\..\Admin"), None);
        assert_eq!(normalize_profile_root(r"\\server\share\demo"), None);
        assert_eq!(normalize_profile_root(r"C:\Users\demo\Desktop"), None);
    }

    #[test]
    fn normalize_profile_root_preserves_unicode_user_profile_paths() {
        assert_eq!(
            normalize_profile_root(r"C:\Users\Quốc Khánh\"),
            Some(PathBuf::from(r"C:\Users\Quốc Khánh"))
        );
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn wide_string_helpers_preserve_unicode_and_handle_empty_registry_values() {
        let encoded = super::encode_wide_nul(r"C:\Users\Quốc Khánh");
        assert_eq!(super::decode_wide_nul(&encoded), r"C:\Users\Quốc Khánh");
        assert_eq!(super::decode_wide_nul(&[0u16]), "");
        assert_eq!(super::decode_wide_nul(&[]), "");
    }
}
