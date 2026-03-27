use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use super_route_pro_lib::route_persist::{
    self, CustomRoute, NicIdentifier, PersistConfig, WanConfig,
};

fn unique_program_data_dir() -> std::path::PathBuf {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock should be monotonic")
        .as_nanos();
    env::temp_dir().join(format!("super-route-pro-programdata-{stamp}-{}", std::process::id()))
}

struct ProgramDataGuard {
    original_program_data: Option<String>,
    temp_dir: PathBuf,
}

impl ProgramDataGuard {
    fn new(temp_dir: PathBuf) -> Self {
        let original_program_data = env::var("ProgramData").ok();
        fs::create_dir_all(&temp_dir).expect("temp ProgramData root should be creatable");
        env::set_var("ProgramData", &temp_dir);
        Self {
            original_program_data,
            temp_dir,
        }
    }

    fn path(&self) -> &Path {
        &self.temp_dir
    }
}

impl Drop for ProgramDataGuard {
    fn drop(&mut self) {
        match self.original_program_data.take() {
            Some(value) => env::set_var("ProgramData", value),
            None => env::remove_var("ProgramData"),
        }
        let _ = fs::remove_dir_all(&self.temp_dir);
    }
}

#[test]
fn persist_config_save_load_and_clear_roundtrip() {
    let guard = ProgramDataGuard::new(unique_program_data_dir());

    let config = PersistConfig {
        schema_version: 1,
        enabled: true,
        nic: NicIdentifier {
            description: "Intel(R) Wi-Fi 6 AX201 160MHz".to_string(),
            mac_address: "A4:B1:C2:D3:E4:F5".to_string(),
        },
        wan: Some(WanConfig {
            gateway: "192.168.1.1".to_string(),
            metric: "1".to_string(),
        }),
        custom_routes: vec![CustomRoute {
            destination: "10.184.0.0".to_string(),
            mask: "255.255.255.0".to_string(),
            gateway: "192.168.1.1".to_string(),
            metric: "10".to_string(),
            nic: Some(NicIdentifier {
                description: "USB Ethernet".to_string(),
                mac_address: "11:22:33:44:55:66".to_string(),
            }),
        }],
        updated_at: Some("2026-03-27T08:00:00Z".to_string()),
    };

    let path = route_persist::config_path().expect("config path should resolve");
    assert!(
        !path.exists() && path.starts_with(guard.path()),
        "test config should start absent before saving"
    );

    route_persist::save_config(&config).expect("config should save");
    assert!(path.exists(), "config file should be created after save");

    let loaded = route_persist::load_config().expect("config should load");
    assert_eq!(loaded, Some(config.clone()));

    route_persist::delete_config().expect("config should delete");
    assert!(!path.exists(), "config file should be removed after clear");
    assert_eq!(
        route_persist::load_config().expect("loading after delete should succeed"),
        None
    );

    route_persist::delete_config().expect("delete should be idempotent");
}
