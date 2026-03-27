use crate::process_exec::run_hidden_output_blocking;
use crate::windows_paths::{current_user_profile_root, program_data_dir, system_root_dir};
use std::path::{Path, PathBuf};

#[cfg(target_os = "windows")]
fn run_service_control_command(args: &[&str]) {
    let _ = run_hidden_output_blocking("net", args);
}

fn windows_temp_dir() -> PathBuf {
    system_root_dir().join("Temp")
}

fn windows_update_download_dir() -> PathBuf {
    system_root_dir().join(r"SoftwareDistribution\Download")
}

fn windows_prefetch_dir() -> PathBuf {
    system_root_dir().join("Prefetch")
}

fn windows_wer_reports_dir() -> PathBuf {
    program_data_dir().join(r"Microsoft\Windows\WER")
}

pub fn sanitize_cleanup_targets(targets: &[String]) -> Vec<String> {
    let mut seen = std::collections::HashSet::new();
    let mut selected = Vec::new();

    for target in targets {
        let trimmed = target.trim().to_lowercase();
        if trimmed.is_empty() {
            continue;
        }

        let is_safe_token = trimmed
            .chars()
            .all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '_' || ch == '-');
        if !is_safe_token || !seen.insert(trimmed.clone()) {
            continue;
        }

        if cleanup_paths_for_profile_root(Path::new(r"C:\Users\placeholder"), &trimmed).is_some() {
            selected.push(trimmed);
        }
    }

    selected
}

pub fn cleanup_paths_for_profile_root(profile_root: &Path, target: &str) -> Option<Vec<PathBuf>> {
    let profile_root = profile_root
        .to_string_lossy()
        .trim_end_matches(['\\', '/'])
        .to_string();
    let local = format!(r"{profile_root}\AppData\Local");

    match target {
        "user_temp" => Some(vec![PathBuf::from(format!(r"{local}\Temp"))]),
        "windows_temp" => Some(vec![windows_temp_dir()]),
        "windows_update_cache" => Some(vec![windows_update_download_dir()]),
        "prefetch" => Some(vec![windows_prefetch_dir()]),
        "explorer_cache" => Some(vec![PathBuf::from(format!(
            r"{local}\Microsoft\Windows\Explorer"
        ))]),
        "edge_cache" => Some(vec![
            PathBuf::from(format!(r"{local}\Microsoft\Edge\User Data\Default\Cache")),
            PathBuf::from(format!(
                r"{local}\Microsoft\Edge\User Data\Default\Code Cache"
            )),
            PathBuf::from(format!(
                r"{local}\Microsoft\Edge\User Data\Default\GPUCache"
            )),
        ]),
        "chrome_cache" => Some(vec![
            PathBuf::from(format!(r"{local}\Google\Chrome\User Data\Default\Cache")),
            PathBuf::from(format!(
                r"{local}\Google\Chrome\User Data\Default\Code Cache"
            )),
            PathBuf::from(format!(r"{local}\Google\Chrome\User Data\Default\GPUCache")),
        ]),
        "firefox_cache" => Some(vec![PathBuf::from(format!(
            r"{local}\Mozilla\Firefox\Profiles"
        ))]),
        "inet_cache" => Some(vec![PathBuf::from(format!(
            r"{local}\Microsoft\Windows\INetCache"
        ))]),
        "web_cache" => Some(vec![PathBuf::from(format!(
            r"{local}\Microsoft\Windows\WebCache"
        ))]),
        "crash_dumps" => Some(vec![PathBuf::from(format!(r"{local}\CrashDumps"))]),
        "wer_reports" => Some(vec![
            windows_wer_reports_dir(),
            PathBuf::from(format!(r"{local}\Microsoft\Windows\WER")),
        ]),
        "d3d_shader_cache" => Some(vec![PathBuf::from(format!(r"{local}\D3DSCache"))]),
        _ => None,
    }
}

pub fn run_cleanup_for_profile_root(profile_root: &Path, target: &str) -> Option<(bool, String)> {
    let profile_root = profile_root
        .to_string_lossy()
        .trim_end_matches(['\\', '/'])
        .to_string();
    let local = format!(r"{profile_root}\AppData\Local");

    match target {
        "user_temp" => {
            let path = Path::new(&local).join("Temp");
            let (deleted, failed) = clean_directory_contents(&path);
            Some((
                failed == 0,
                format!("[OK] User Temp cleaned ({deleted} items removed, {failed} failed)."),
            ))
        }
        "windows_temp" => {
            let path = windows_temp_dir();
            let (deleted, failed) = clean_directory_contents(&path);
            Some((
                failed == 0,
                format!("[OK] Windows Temp cleaned ({deleted} items, {failed} failed)."),
            ))
        }
        "windows_update_cache" => {
            #[cfg(target_os = "windows")]
            {
                run_service_control_command(&["stop", "wuauserv", "/y"]);
                run_service_control_command(&["stop", "bits", "/y"]);
            }
            let path = windows_update_download_dir();
            let (deleted, failed) = clean_directory_contents(&path);
            #[cfg(target_os = "windows")]
            {
                run_service_control_command(&["start", "wuauserv"]);
                run_service_control_command(&["start", "bits"]);
            }
            Some((
                failed == 0,
                format!("[OK] Windows Update cache cleaned ({deleted} items, {failed} failed)."),
            ))
        }
        "prefetch" => {
            let path = windows_prefetch_dir();
            let (deleted, failed) = clean_directory_contents(&path);
            Some((
                failed == 0,
                format!("[OK] Prefetch cleaned ({deleted} items, {failed} failed)."),
            ))
        }
        "explorer_cache" => {
            let explorer_dir = Path::new(&local).join(r"Microsoft\Windows\Explorer");
            let (d1, f1) = clean_files_with_prefix(&explorer_dir, "thumbcache_", ".db");
            let (d2, f2) = clean_files_with_prefix(&explorer_dir, "iconcache_", ".db");
            let deleted = d1 + d2;
            let failed = f1 + f2;
            Some((
                failed == 0,
                format!("[OK] Explorer cache cleaned ({deleted} items, {failed} failed)."),
            ))
        }
        "edge_cache" => {
            let base = Path::new(&local).join(r"Microsoft\Edge\User Data\Default");
            let (deleted, failed) = clean_many_subdirs(&base, &["Cache", "Code Cache", "GPUCache"]);
            Some((
                failed == 0,
                format!("[OK] Microsoft Edge cache cleaned ({deleted} items, {failed} failed)."),
            ))
        }
        "chrome_cache" => {
            let base = Path::new(&local).join(r"Google\Chrome\User Data\Default");
            let (deleted, failed) = clean_many_subdirs(&base, &["Cache", "Code Cache", "GPUCache"]);
            Some((
                failed == 0,
                format!("[OK] Google Chrome cache cleaned ({deleted} items, {failed} failed)."),
            ))
        }
        "firefox_cache" => {
            let profiles_dir = Path::new(&local).join(r"Mozilla\Firefox\Profiles");
            let mut deleted = 0u64;
            let mut failed = 0u64;
            if let Ok(entries) = std::fs::read_dir(&profiles_dir) {
                for entry in entries.flatten() {
                    let (d, f) = clean_directory_contents(&entry.path().join("cache2"));
                    deleted += d;
                    failed += f;
                }
            }
            Some((
                failed == 0,
                format!("[OK] Mozilla Firefox cache cleaned ({deleted} items, {failed} failed)."),
            ))
        }
        "inet_cache" => {
            let path = Path::new(&local).join(r"Microsoft\Windows\INetCache");
            let (deleted, failed) = clean_directory_contents(&path);
            Some((
                failed == 0,
                format!("[OK] INetCache cleaned ({deleted} items, {failed} failed)."),
            ))
        }
        "web_cache" => {
            let path = Path::new(&local).join(r"Microsoft\Windows\WebCache");
            let (deleted, failed) = clean_directory_contents(&path);
            Some((
                failed == 0,
                format!("[OK] WebCache cleaned ({deleted} items, {failed} failed)."),
            ))
        }
        "crash_dumps" => {
            let path = Path::new(&local).join("CrashDumps");
            let (deleted, failed) = clean_directory_contents(&path);
            Some((
                failed == 0,
                format!("[OK] Crash dumps cleaned ({deleted} items, {failed} failed)."),
            ))
        }
        "wer_reports" => {
            let (d1, f1) = clean_directory_contents(&windows_wer_reports_dir());
            let (d2, f2) =
                clean_directory_contents(&Path::new(&local).join(r"Microsoft\Windows\WER"));
            let deleted = d1 + d2;
            let failed = f1 + f2;
            Some((
                failed == 0,
                format!("[OK] Windows Error Reporting cleaned ({deleted} items, {failed} failed)."),
            ))
        }
        "d3d_shader_cache" => {
            let path = Path::new(&local).join("D3DSCache");
            let (deleted, failed) = clean_directory_contents(&path);
            Some((
                failed == 0,
                format!("[OK] DirectX Shader Cache cleaned ({deleted} items, {failed} failed)."),
            ))
        }
        _ => None,
    }
}

pub fn run_cleanup_for_current_user(target: &str) -> Option<(&'static str, bool, String)> {
    let profile_root = current_user_profile_root();
    let label = label_for_target(target)?;
    let result = run_cleanup_for_profile_root(&profile_root, target)?;
    Some((label, result.0, result.1))
}

pub fn label_for_target(target: &str) -> Option<&'static str> {
    match target {
        "user_temp" => Some("User Temp"),
        "windows_temp" => Some("Windows Temp"),
        "windows_update_cache" => Some("Windows Update Cache"),
        "prefetch" => Some("Prefetch"),
        "explorer_cache" => Some("Explorer Cache (thumbnail/icon)"),
        "edge_cache" => Some("Microsoft Edge Cache"),
        "chrome_cache" => Some("Google Chrome Cache"),
        "firefox_cache" => Some("Mozilla Firefox Cache"),
        "inet_cache" => Some("INetCache"),
        "web_cache" => Some("WebCache"),
        "crash_dumps" => Some("Crash Dumps"),
        "wer_reports" => Some("Windows Error Reporting (WER)"),
        "d3d_shader_cache" => Some("DirectX Shader Cache (D3DSCache)"),
        _ => None,
    }
}

fn clean_many_subdirs(base: &Path, subdirs: &[&str]) -> (u64, u64) {
    let mut deleted = 0u64;
    let mut failed = 0u64;
    for subdir in subdirs {
        let (d, f) = clean_directory_contents(&base.join(subdir));
        deleted += d;
        failed += f;
    }
    (deleted, failed)
}

pub fn clean_directory_contents(path: &Path) -> (u64, u64) {
    let mut deleted = 0u64;
    let mut failed = 0u64;
    let Ok(entries) = std::fs::read_dir(path) else {
        return (0, 0);
    };
    for entry in entries.flatten() {
        let entry_path = entry.path();
        if entry_path.is_dir() {
            match std::fs::remove_dir_all(&entry_path) {
                Ok(_) => deleted += 1,
                Err(_) => failed += 1,
            }
        } else {
            match std::fs::remove_file(&entry_path) {
                Ok(_) => deleted += 1,
                Err(_) => failed += 1,
            }
        }
    }
    (deleted, failed)
}

pub fn clean_files_with_prefix(dir: &Path, prefix: &str, suffix: &str) -> (u64, u64) {
    let mut deleted = 0u64;
    let mut failed = 0u64;
    let Ok(entries) = std::fs::read_dir(dir) else {
        return (0, 0);
    };
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        if name.starts_with(prefix) && name.ends_with(suffix) {
            match std::fs::remove_file(entry.path()) {
                Ok(_) => deleted += 1,
                Err(_) => failed += 1,
            }
        }
    }
    (deleted, failed)
}

#[cfg(test)]
mod tests {
    use super::{cleanup_paths_for_profile_root, label_for_target, sanitize_cleanup_targets};
    use std::path::Path;

    #[test]
    fn sanitize_cleanup_targets_deduplicates_and_filters_invalid_tokens() {
        let sanitized = sanitize_cleanup_targets(&[
            "user_temp".to_string(),
            "user_temp".to_string(),
            "bad token".to_string(),
            "edge_cache".to_string(),
        ]);

        assert_eq!(sanitized, vec!["user_temp", "edge_cache"]);
    }

    #[test]
    fn cleanup_paths_resolve_under_profile_root() {
        let paths = cleanup_paths_for_profile_root(Path::new(r"C:\Users\demo"), "chrome_cache")
            .expect("chrome cache should resolve");

        assert!(paths.iter().any(|path| path
            .to_string_lossy()
            .contains(r"C:\Users\demo\AppData\Local\Google\Chrome")));
    }

    #[test]
    fn labels_exist_for_known_targets() {
        assert_eq!(label_for_target("user_temp"), Some("User Temp"));
        assert_eq!(label_for_target("unknown"), None);
    }
}
