use std::path::{Path, PathBuf};

const DEFAULT_SYSTEM_ROOT: &str = r"C:\Windows";
const DEFAULT_PROGRAM_DATA: &str = r"C:\ProgramData";
const DEFAULT_USER_PROFILE: &str = r"C:\Users\Default";

fn env_path_or_default(value: Option<&str>, fallback: &str) -> PathBuf {
    let value = value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or(fallback);
    PathBuf::from(value)
}

pub fn system_root_dir() -> PathBuf {
    system_root_dir_from_value(std::env::var("SystemRoot").ok().as_deref())
}

pub(crate) fn system_root_dir_from_value(system_root: Option<&str>) -> PathBuf {
    env_path_or_default(system_root, DEFAULT_SYSTEM_ROOT)
}

pub fn program_data_dir() -> PathBuf {
    program_data_dir_from_value(std::env::var("ProgramData").ok().as_deref())
}

pub(crate) fn program_data_dir_from_value(program_data: Option<&str>) -> PathBuf {
    env_path_or_default(program_data, DEFAULT_PROGRAM_DATA)
}

pub(crate) fn default_user_profile_dir() -> PathBuf {
    PathBuf::from(DEFAULT_USER_PROFILE)
}

pub fn current_user_profile_root() -> PathBuf {
    current_user_profile_root_from_values(
        std::env::var("LOCALAPPDATA").ok().as_deref(),
        std::env::var("USERPROFILE").ok().as_deref(),
    )
}

pub(crate) fn current_user_profile_root_from_values(
    local_app_data: Option<&str>,
    user_profile: Option<&str>,
) -> PathBuf {
    let local_app_data = local_app_data
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            env_path_or_default(user_profile, DEFAULT_USER_PROFILE)
                .join("AppData")
                .join("Local")
        });

    local_app_data
        .parent()
        .and_then(Path::parent)
        .map(Path::to_path_buf)
        .unwrap_or_else(default_user_profile_dir)
}

#[cfg(test)]
mod tests {
    use super::{
        current_user_profile_root_from_values, program_data_dir_from_value,
        system_root_dir_from_value,
    };
    use std::path::Path;

    #[test]
    fn system_root_and_program_data_honor_configured_roots() {
        assert_eq!(
            system_root_dir_from_value(Some(r"D:\WindowsAlt")),
            Path::new(r"D:\WindowsAlt")
        );
        assert_eq!(
            program_data_dir_from_value(Some(r"E:\ProgramDataAlt")),
            Path::new(r"E:\ProgramDataAlt")
        );
    }

    #[test]
    fn system_root_and_program_data_fall_back_to_defaults() {
        assert_eq!(
            system_root_dir_from_value(Some("  ")),
            Path::new(r"C:\Windows")
        );
        assert_eq!(
            program_data_dir_from_value(None),
            Path::new(r"C:\ProgramData")
        );
    }

    #[test]
    fn current_user_profile_root_uses_local_app_data_when_available() {
        assert_eq!(
            current_user_profile_root_from_values(
                Some(r"D:\Profiles\demo\AppData\Local"),
                Some(r"D:\Profiles\ignored")
            ),
            Path::new(r"D:\Profiles\demo")
        );
    }

    #[test]
    fn current_user_profile_root_falls_back_to_user_profile() {
        assert_eq!(
            current_user_profile_root_from_values(None, Some(r"E:\Users\demo")),
            Path::new(r"E:\Users\demo")
        );
    }

    #[test]
    fn current_user_profile_root_falls_back_to_default_user_profile() {
        assert_eq!(
            current_user_profile_root_from_values(Some(""), None),
            Path::new(r"C:\Users\Default")
        );
    }
}
