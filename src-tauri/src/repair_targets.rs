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
    let output = std::process::Command::new("powershell")
        .args([
            "-NoProfile",
            "-NonInteractive",
            "-Command",
            "$profiles = Get-CimInstance Win32_UserProfile | Where-Object { -not $_.Special -and $_.LocalPath -like '*\\\\Users\\\\*' }; \
             $profiles | ForEach-Object { \
               [PSCustomObject]@{ \
                 sid = $_.SID; \
                 account_name = Split-Path $_.LocalPath -Leaf; \
                 profile_path = $_.LocalPath; \
                 is_loaded = [bool]$_.Loaded \
               } \
             } | ConvertTo-Json -Compress",
        ])
        .output()
        .map_err(|err| format!("Unable to query Windows user profiles: {err}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(if stderr.is_empty() {
            "Windows profile query returned an unknown error.".to_string()
        } else {
            stderr
        });
    }

    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if stdout.is_empty() || stdout == "null" {
        return Ok(Vec::new());
    }

    let raw_targets: Vec<RawRepairTargetUser> = if stdout.starts_with('[') {
        serde_json::from_str(&stdout)
            .map_err(|err| format!("Unable to parse Windows profile list: {err}"))?
    } else {
        vec![
            serde_json::from_str(&stdout)
                .map_err(|err| format!("Unable to parse Windows profile record: {err}"))?,
        ]
    };

    let mut targets: Vec<RepairTargetUser> = raw_targets
        .into_iter()
        .filter_map(|target| {
            if !validate_target_sid(&target.sid) {
                return None;
            }

            let profile_path = normalize_profile_root(&target.profile_path)?;
            let account_name = target
                .account_name
                .filter(|name| !name.trim().is_empty())
                .unwrap_or_else(|| {
                    profile_path
                        .file_name()
                        .and_then(|name| name.to_str())
                        .unwrap_or("unknown")
                        .to_string()
                });

            Some(RepairTargetUser {
                sid: target.sid,
                account_name,
                profile_path: profile_path.to_string_lossy().to_string(),
                is_loaded: target.is_loaded,
            })
        })
        .collect();

    targets.sort_by(|left, right| {
        right.is_loaded
            .cmp(&left.is_loaded)
            .then_with(|| left.account_name.cmp(&right.account_name))
    });

    Ok(targets)
}

#[cfg(not(target_os = "windows"))]
pub fn list_repair_targets() -> Result<Vec<RepairTargetUser>, String> {
    Err("Repair targets are only available on Windows.".to_string())
}
