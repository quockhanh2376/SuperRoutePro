use serde::{Deserialize, Serialize};
use std::path::PathBuf;

const CONFIG_DIR_NAME: &str = "SuperRoutePro";
const CONFIG_FILE_NAME: &str = "persist.json";
const SCHEMA_VERSION: u32 = 1;

/// Stable NIC identifier that survives reboots (unlike InterfaceIndex).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct NicIdentifier {
    pub description: String,
    pub mac_address: String,
}

/// WAN (default gateway) configuration.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WanConfig {
    pub gateway: String,
    #[serde(default)]
    pub metric: String,
}

/// A single custom route entry.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CustomRoute {
    pub destination: String,
    pub mask: String,
    pub gateway: String,
    #[serde(default)]
    pub metric: String,
}

/// Full persist configuration written by the UI and read by the service.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
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
    let program_data =
        std::env::var("ProgramData").unwrap_or_else(|_| r"C:\ProgramData".to_string());
    let dir = PathBuf::from(program_data).join(CONFIG_DIR_NAME);
    Ok(dir)
}

/// Full path to `persist.json`.
pub fn config_path() -> Result<PathBuf, String> {
    Ok(config_dir()?.join(CONFIG_FILE_NAME))
}

/// Read and parse the persist config from disk. Returns `None` if the file
/// does not exist. Returns `Err` if the file exists but cannot be read or parsed.
pub fn load_config() -> Result<Option<PersistConfig>, String> {
    let path = config_path()?;
    if !path.exists() {
        return Ok(None);
    }
    let content = std::fs::read_to_string(&path)
        .map_err(|e| format!("Failed to read {}: {e}", path.display()))?;
    let config: PersistConfig = serde_json::from_str(&content)
        .map_err(|e| format!("Failed to parse {}: {e}", path.display()))?;
    Ok(Some(config))
}

/// Write the persist config to disk. Creates the config directory if needed.
pub fn save_config(config: &PersistConfig) -> Result<(), String> {
    let dir = config_dir()?;
    std::fs::create_dir_all(&dir)
        .map_err(|e| format!("Failed to create {}: {e}", dir.display()))?;

    let path = config_path()?;
    let json = serde_json::to_string_pretty(config)
        .map_err(|e| format!("Failed to serialize config: {e}"))?;
    std::fs::write(&path, json).map_err(|e| format!("Failed to write {}: {e}", path.display()))?;
    Ok(())
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
}
