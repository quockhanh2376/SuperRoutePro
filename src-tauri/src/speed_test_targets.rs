use serde::Serialize;

pub const DEFAULT_SPEED_TEST_TARGET_ID: &str = "auto_asia";

const PREFERRED_ASIA_COLOS: [&str; 38] = [
    "SIN", "KUL", "BKK", "CGK", "DPS", "SUB", "SGN", "HAN", "PNH", "RGN", "VTE", "MNL", "HKG",
    "TPE", "KHH", "MFM", "NRT", "HND", "KIX", "ICN", "GMP", "PUS", "KTM", "DAC", "CCU", "DEL",
    "BOM", "AMD", "BLR", "MAA", "HYD", "COK", "CJB", "CMB", "DXB", "DOH", "MCT", "BAH",
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SpeedTestBackendKind {
    CloudflareAutoEdge,
    LibreSpeedRegional,
}

#[derive(Clone, Copy, Debug)]
pub struct SpeedTestTarget {
    pub backend_kind: SpeedTestBackendKind,
    pub id: &'static str,
    pub target_label: &'static str,
    pub description: &'static str,
    pub provider_label: &'static str,
    pub default_server_label: &'static str,
    pub preferred_colos: &'static [&'static str],
    pub latency_url: &'static str,
    pub download_url: &'static str,
    pub upload_url: &'static str,
    pub ip_lookup_url: &'static str,
    pub default_download_mb: u32,
    pub min_download_mb: u32,
    pub max_download_mb: u32,
    pub min_upload_bytes: usize,
    pub max_upload_bytes: usize,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct SpeedTestCatalogEntry {
    pub id: String,
    pub label: String,
    pub description: String,
    pub provider: String,
}

const AUTO_ASIA_TARGET: SpeedTestTarget = SpeedTestTarget {
    backend_kind: SpeedTestBackendKind::CloudflareAutoEdge,
    id: DEFAULT_SPEED_TEST_TARGET_ID,
    target_label: "Auto Asia",
    description: "Cloudflare auto-selects the nearest preferred Asia edge. Use this as the route-aware baseline close to the current network path.",
    provider_label: "Cloudflare (Asia auto-edge)",
    default_server_label: "Cloudflare auto edge",
    preferred_colos: &PREFERRED_ASIA_COLOS,
    latency_url: "https://speed.cloudflare.com/__down",
    download_url: "https://speed.cloudflare.com/__down",
    upload_url: "https://speed.cloudflare.com/__up",
    ip_lookup_url: "https://speed.cloudflare.com/cdn-cgi/trace",
    default_download_mb: 24,
    min_download_mb: 8,
    max_download_mb: 32,
    min_upload_bytes: 2 * 1024 * 1024,
    max_upload_bytes: 8 * 1024 * 1024,
};

const JP_KR_TARGET: SpeedTestTarget = SpeedTestTarget {
    backend_kind: SpeedTestBackendKind::LibreSpeedRegional,
    id: "jp_kr",
    target_label: "JP/KR",
    description: "Fixed Northeast Asia backend pinned to Tokyo, Japan. Use this to compare against Auto Asia without Cloudflare auto-edge routing.",
    provider_label: "LibreSpeed (regional fixed backend)",
    default_server_label: "Tokyo, Japan (A573)",
    preferred_colos: &[],
    latency_url: "https://librespeed.a573.net/backend/empty.php",
    download_url: "https://librespeed.a573.net/backend/garbage.php",
    upload_url: "https://librespeed.a573.net/backend/empty.php",
    ip_lookup_url: "https://librespeed.a573.net/backend/getIP.php",
    default_download_mb: 4,
    min_download_mb: 1,
    max_download_mb: 8,
    min_upload_bytes: 512 * 1024,
    max_upload_bytes: 2 * 1024 * 1024,
};

const US_WEST_TARGET: SpeedTestTarget = SpeedTestTarget {
    backend_kind: SpeedTestBackendKind::LibreSpeedRegional,
    id: "us_west",
    target_label: "US West",
    description: "Fixed trans-Pacific backend pinned to Los Angeles, United States. Use this to compare long-haul performance against a stable US West endpoint.",
    provider_label: "LibreSpeed (regional fixed backend)",
    default_server_label: "Los Angeles, United States (Clouvider)",
    preferred_colos: &[],
    latency_url: "https://la.speedtest.clouvider.net/backend/empty.php",
    download_url: "https://la.speedtest.clouvider.net/backend/garbage.php",
    upload_url: "https://la.speedtest.clouvider.net/backend/empty.php",
    ip_lookup_url: "https://la.speedtest.clouvider.net/backend/getIP.php",
    default_download_mb: 4,
    min_download_mb: 1,
    max_download_mb: 8,
    min_upload_bytes: 512 * 1024,
    max_upload_bytes: 2 * 1024 * 1024,
};

const EU_TARGET: SpeedTestTarget = SpeedTestTarget {
    backend_kind: SpeedTestBackendKind::LibreSpeedRegional,
    id: "eu",
    target_label: "EU",
    description: "Fixed Europe backend pinned to London, England. Payload sizes stay smaller here so long-haul runs from Southeast Asia remain stable.",
    provider_label: "LibreSpeed (regional fixed backend)",
    default_server_label: "London, England (Clouvider)",
    preferred_colos: &[],
    latency_url: "https://lon.speedtest.clouvider.net/backend/empty.php",
    download_url: "https://lon.speedtest.clouvider.net/backend/garbage.php",
    upload_url: "https://lon.speedtest.clouvider.net/backend/empty.php",
    ip_lookup_url: "https://lon.speedtest.clouvider.net/backend/getIP.php",
    default_download_mb: 1,
    min_download_mb: 1,
    max_download_mb: 2,
    min_upload_bytes: 256 * 1024,
    max_upload_bytes: 512 * 1024,
};

const SPEED_TEST_TARGETS: [SpeedTestTarget; 4] =
    [AUTO_ASIA_TARGET, JP_KR_TARGET, US_WEST_TARGET, EU_TARGET];

fn build_speed_test_catalog_entry(target: SpeedTestTarget) -> SpeedTestCatalogEntry {
    SpeedTestCatalogEntry {
        id: target.id.to_string(),
        label: target.target_label.to_string(),
        description: target.description.to_string(),
        provider: target.provider_label.to_string(),
    }
}

#[tauri::command]
pub fn list_speed_test_targets() -> Vec<SpeedTestCatalogEntry> {
    SPEED_TEST_TARGETS
        .iter()
        .copied()
        .map(build_speed_test_catalog_entry)
        .collect()
}

pub fn resolve_speed_test_target(target_id: Option<&str>) -> Result<SpeedTestTarget, String> {
    let target_id = target_id.unwrap_or(DEFAULT_SPEED_TEST_TARGET_ID);
    SPEED_TEST_TARGETS
        .iter()
        .copied()
        .find(|target| target.id == target_id)
        .ok_or_else(|| format!("Unknown speed test target: {target_id}"))
}

#[cfg(test)]
mod tests {
    use super::{list_speed_test_targets, resolve_speed_test_target, SpeedTestBackendKind};

    #[test]
    fn list_speed_test_targets_exposes_real_regional_entries() {
        let targets = list_speed_test_targets();
        let ids = targets
            .iter()
            .map(|target| target.id.as_str())
            .collect::<Vec<_>>();

        assert_eq!(ids, vec!["auto_asia", "jp_kr", "us_west", "eu"]);
        assert_eq!(targets[0].provider, "Cloudflare (Asia auto-edge)");
        assert!(targets[1].description.contains("Tokyo"));
        assert!(targets[2].description.contains("Los Angeles"));
        assert!(targets[3].description.contains("London"));
    }

    #[test]
    fn resolve_speed_test_target_defaults_and_rejects_unknown_values() {
        let default_target =
            resolve_speed_test_target(None).expect("default target should resolve");
        assert_eq!(default_target.id, "auto_asia");
        assert_eq!(
            default_target.backend_kind,
            SpeedTestBackendKind::CloudflareAutoEdge
        );

        let eu_target = resolve_speed_test_target(Some("eu")).expect("eu target should resolve");
        assert_eq!(
            eu_target.default_server_label,
            "London, England (Clouvider)"
        );
        assert_eq!(
            eu_target.backend_kind,
            SpeedTestBackendKind::LibreSpeedRegional
        );

        let error =
            resolve_speed_test_target(Some("invalid")).expect_err("unknown target should fail");
        assert_eq!(error, "Unknown speed test target: invalid");
    }
}
