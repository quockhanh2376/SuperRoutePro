use serde::{Deserialize, Serialize};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use crate::windows_paths::program_data_dir;

const CONFIG_DIR_NAME: &str = "SuperRoutePro";
const CONFIG_FILE_NAME: &str = "persist.json";
const SERVICE_LOG_FILE_NAME: &str = "service.log";
const SERVICE_LOG_MAX_BYTES: u64 = 512 * 1024;
const SCHEMA_VERSION: u32 = 1;
static TEMP_FILE_SEQ: AtomicU64 = AtomicU64::new(1);

/// Stable NIC identifier that survives reboots (unlike InterfaceIndex).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct NicIdentifier {
    pub description: String,
    pub mac_address: String,
}

/// WAN (default gateway) configuration.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WanConfig {
    pub gateway: String,
    #[serde(default)]
    pub metric: String,
}

/// A single custom route entry.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CustomRoute {
    pub destination: String,
    pub mask: String,
    pub gateway: String,
    #[serde(default)]
    pub metric: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub nic: Option<NicIdentifier>,
}

/// Full persist configuration written by the UI and read by the service.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PersistConfig {
    pub schema_version: u32,
    pub enabled: bool,
    pub nic: NicIdentifier,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wan: Option<WanConfig>,
    #[serde(default)]
    pub custom_routes: Vec<CustomRoute>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<String>,
}

impl PersistConfig {
    pub fn new(nic: NicIdentifier) -> Self {
        Self {
            schema_version: SCHEMA_VERSION,
            enabled: true,
            nic,
            wan: None,
            custom_routes: Vec::new(),
            updated_at: None,
        }
    }
}

/// Resolve the config directory: `%ProgramData%\SuperRoutePro\`.
pub fn config_dir() -> Result<PathBuf, String> {
    Ok(program_data_dir().join(CONFIG_DIR_NAME))
}

/// Full path to `persist.json`.
pub fn config_path() -> Result<PathBuf, String> {
    Ok(config_dir()?.join(CONFIG_FILE_NAME))
}

fn load_config_from_path(path: &Path) -> Result<Option<PersistConfig>, String> {
    if !path.exists() {
        return Ok(None);
    }
    let content = std::fs::read_to_string(path)
        .map_err(|e| format!("Failed to read {}: {e}", path.display()))?;
    let config: PersistConfig = serde_json::from_str(&content)
        .map_err(|e| format!("Failed to parse {}: {e}", path.display()))?;
    Ok(Some(config))
}

fn save_config_to_path(path: &Path, config: &PersistConfig) -> Result<(), String> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)
            .map_err(|e| format!("Failed to create {}: {e}", dir.display()))?;
    }

    let json = serde_json::to_string_pretty(config)
        .map_err(|e| format!("Failed to serialize config: {e}"))?;

    let seq = TEMP_FILE_SEQ.fetch_add(1, Ordering::Relaxed);
    let pid = std::process::id();
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or(CONFIG_FILE_NAME);
    let tmp_path = path.with_file_name(format!("{file_name}.{pid}.{seq}.tmp"));

    {
        let mut tmp_file = std::fs::File::create(&tmp_path)
            .map_err(|e| format!("Failed to create temp file {}: {e}", tmp_path.display()))?;
        tmp_file
            .write_all(json.as_bytes())
            .map_err(|e| format!("Failed to write temp file {}: {e}", tmp_path.display()))?;
        let _ = tmp_file.sync_all();
    }

    let mut last_err = None;
    for attempt in 0..5 {
        match std::fs::rename(&tmp_path, path) {
            Ok(()) => return Ok(()),
            Err(err) => {
                last_err = Some(err);
                if attempt < 4 {
                    std::thread::sleep(Duration::from_millis(10));
                }
            }
        }
    }

    let _ = std::fs::remove_file(&tmp_path);
    Err(format!(
        "Failed to atomically replace {}: {}",
        path.display(),
        last_err
            .map(|e| e.to_string())
            .unwrap_or_else(|| "unknown error".to_string())
    ))
}

/// Read and parse the persist config from disk. Returns `None` if the file
/// does not exist. Returns `Err` if the file exists but cannot be read or parsed.
pub fn load_config() -> Result<Option<PersistConfig>, String> {
    let path = config_path()?;
    load_config_from_path(&path)
}

/// Write the persist config to disk atomically. Creates the config directory if needed.
pub fn save_config(config: &PersistConfig) -> Result<(), String> {
    let path = config_path()?;
    save_config_to_path(&path, config)
}

/// Delete the persist config file. No error if the file is already missing.
pub fn delete_config() -> Result<(), String> {
    let path = config_path()?;
    if path.exists() {
        std::fs::remove_file(&path)
            .map_err(|e| format!("Failed to remove {}: {e}", path.display()))?;
    }
    Ok(())
}

/// Append a log line to a size-capped rotating log file (rotates to `.log.1` when >= `max_bytes`).
pub fn append_rotating_log(path: &Path, line: &str, max_bytes: u64) {
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }

    if let Ok(meta) = std::fs::metadata(path) {
        if meta.len() >= max_bytes {
            let rotated = path.with_extension("log.1");
            let _ = std::fs::remove_file(&rotated);
            let _ = std::fs::rename(path, &rotated);
        }
    }

    if let Ok(mut file) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
    {
        let _ = writeln!(file, "{line}");
    }
}

/// Append a formatted service log line to `%ProgramData%\SuperRoutePro\service.log`.
pub fn append_service_log_line(line: &str) {
    if let Ok(dir) = config_dir() {
        let log_path = dir.join(SERVICE_LOG_FILE_NAME);
        append_rotating_log(&log_path, line, SERVICE_LOG_MAX_BYTES);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_config() -> PersistConfig {
        PersistConfig {
            schema_version: 1,
            enabled: true,
            nic: NicIdentifier {
                description: "Intel(R) Wi-Fi 6 AX200 160MHz".into(),
                mac_address: "A4:B1:C2:D3:E4:F5".into(),
            },
            wan: Some(WanConfig {
                gateway: "192.168.1.1".into(),
                metric: "1".into(),
            }),
            custom_routes: vec![CustomRoute {
                destination: "10.0.0.0".into(),
                mask: "255.255.255.0".into(),
                gateway: "192.168.1.1".into(),
                metric: "10".into(),
                nic: Some(NicIdentifier {
                    description: "USB Ethernet".into(),
                    mac_address: "11:22:33:44:55:66".into(),
                }),
            }],
            updated_at: Some("2026-03-21T09:00:00Z".into()),
        }
    }

    #[test]
    fn config_serializes_to_valid_json() {
        let config = sample_config();
        let json = serde_json::to_string_pretty(&config).expect("should serialize");
        assert!(json.contains("\"schema_version\": 1"));
        assert!(json.contains("Intel(R) Wi-Fi 6 AX200 160MHz"));
        assert!(json.contains("A4:B1:C2:D3:E4:F5"));
        assert!(json.contains("192.168.1.1"));
        assert!(json.contains("10.0.0.0"));
        assert!(json.contains("USB Ethernet"));
    }

    #[test]
    fn config_roundtrips_through_json() {
        let original = sample_config();
        let json = serde_json::to_string(&original).expect("should serialize");
        let restored: PersistConfig = serde_json::from_str(&json).expect("should deserialize");
        assert_eq!(original, restored);
    }

    #[test]
    fn config_deserializes_minimal_json() {
        let json = r#"{
            "schema_version": 1,
            "enabled": false,
            "nic": { "description": "Ethernet", "mac_address": "00:11:22:33:44:55" }
        }"#;
        let config: PersistConfig = serde_json::from_str(json).expect("should parse minimal");
        assert!(!config.enabled);
        assert!(config.wan.is_none());
        assert!(config.custom_routes.is_empty());
        assert!(config.updated_at.is_none());
    }

    #[test]
    fn config_dir_resolves_under_programdata() {
        let dir = config_dir().expect("should resolve");
        let dir_str = dir.to_string_lossy().to_lowercase();
        assert!(
            dir_str.contains("superroutepro"),
            "config dir should contain SuperRoutePro: {dir_str}"
        );
    }

    #[test]
    fn new_config_has_defaults() {
        let config = PersistConfig::new(NicIdentifier {
            description: "Test".into(),
            mac_address: "00:00:00:00:00:00".into(),
        });
        assert_eq!(config.schema_version, 1);
        assert!(config.enabled);
        assert!(config.wan.is_none());
        assert!(config.custom_routes.is_empty());
    }

    #[test]
    fn atomic_save_config_prevents_partial_reads_under_concurrency() {
        let dir = std::env::temp_dir().join(format!(
            "srp-atomic-persist-test-{}",
            std::process::id()
        ));
        let path = dir.join("persist.json");
        let _ = std::fs::remove_dir_all(&dir);
        let initial = sample_config();
        save_config_to_path(&path, &initial).expect("initial save should succeed");

        let writer_path = path.clone();
        let writer = std::thread::spawn(move || {
            for i in 0..40 {
                let mut cfg = sample_config();
                cfg.updated_at = Some(format!("2026-11-27T00:00:{i:02}Z"));
                save_config_to_path(&writer_path, &cfg).expect("atomic save should succeed");
            }
        });

        let reader_path = path.clone();
        let reader = std::thread::spawn(move || {
            for _ in 0..80 {
                match load_config_from_path(&reader_path) {
                    Ok(Some(cfg)) => assert_eq!(cfg.schema_version, 1),
                    Ok(None) => {}
                    Err(err) => {
                        // On Windows, a transient sharing violation while renaming is possible,
                        // but JSON must NEVER be truncated or corrupted.
                        assert!(
                            !err.contains("Failed to parse"),
                            "atomic write must never expose truncated/corrupt JSON: {err}"
                        );
                    }
                }
            }
        });

        writer.join().expect("writer thread should succeed");
        reader.join().expect("reader thread should succeed");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn rotating_service_log_rotates_when_max_size_exceeded() {
        let dir = std::env::temp_dir().join(format!("srp-service-log-test-{}", std::process::id()));
        let log_path = dir.join("service.log");
        let rotated_path = dir.join("service.log.1");
        let _ = std::fs::remove_dir_all(&dir);

        append_rotating_log(&log_path, "first-line-exceeding-limit", 10);
        append_rotating_log(&log_path, "second-line", 10);

        assert!(rotated_path.exists(), "rotated log file should be created");
        let current = std::fs::read_to_string(&log_path).expect("current log should read");
        assert!(current.contains("second-line"));
        assert!(!current.contains("first-line-exceeding-limit"));

        let _ = std::fs::remove_dir_all(&dir);
    }
}
