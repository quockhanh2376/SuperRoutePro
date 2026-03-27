#[path = "../src/speed_test_targets.rs"]
mod speed_test_targets;

use speed_test_targets::{list_speed_test_targets, resolve_speed_test_target, SpeedTestBackendKind};

#[test]
fn speed_test_catalog_exposes_the_expected_labels_and_providers() {
    let targets = list_speed_test_targets();

    assert_eq!(
        targets
            .iter()
            .map(|target| (target.id.as_str(), target.label.as_str(), target.provider.as_str()))
            .collect::<Vec<_>>(),
        vec![
            ("auto_asia", "Auto Asia", "Cloudflare (Asia auto-edge)"),
            ("auto_au", "Auto Australia", "Cloudflare (Australia auto-edge)"),
            ("jp_kr", "JP/KR", "LibreSpeed (regional fixed backend)"),
            ("us_west", "US West", "LibreSpeed (regional fixed backend)"),
            ("eu", "EU", "LibreSpeed (regional fixed backend)"),
        ]
    );
    assert!(targets[0].description.contains("Cloudflare auto-selects"));
    assert!(targets[1].description.contains("Australia"));
    assert!(targets[2].description.contains("Tokyo"));
    assert!(targets[3].description.contains("Los Angeles"));
    assert!(targets[4].description.contains("London"));
}

#[test]
fn speed_test_target_resolution_preserves_region_specific_server_labels() {
    let auto = resolve_speed_test_target(None).expect("default target should resolve");
    assert_eq!(auto.id, "auto_asia");
    assert_eq!(auto.backend_kind, SpeedTestBackendKind::CloudflareAutoEdge);

    let jp_kr = resolve_speed_test_target(Some("jp_kr")).expect("jp_kr should resolve");
    assert_eq!(jp_kr.target_label, "JP/KR");
    assert_eq!(jp_kr.default_server_label, "Tokyo, Japan (A573)");
    assert_eq!(jp_kr.backend_kind, SpeedTestBackendKind::LibreSpeedRegional);

    let auto_au = resolve_speed_test_target(Some("auto_au")).expect("auto_au should resolve");
    assert_eq!(auto_au.target_label, "Auto Australia");
    assert_eq!(auto_au.backend_kind, SpeedTestBackendKind::CloudflareAutoEdge);

    let us_west = resolve_speed_test_target(Some("us_west")).expect("us_west should resolve");
    assert_eq!(us_west.target_label, "US West");
    assert_eq!(
        us_west.default_server_label,
        "Los Angeles, United States (Clouvider)"
    );

    let eu = resolve_speed_test_target(Some("eu")).expect("eu should resolve");
    assert_eq!(eu.target_label, "EU");
    assert_eq!(eu.default_server_label, "London, England (Clouvider)");
}
