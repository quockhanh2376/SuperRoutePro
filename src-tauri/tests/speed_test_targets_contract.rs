#[path = "../src/speed_test_targets.rs"]
mod speed_test_targets;

use speed_test_targets::{
    list_speed_test_targets, resolve_speed_test_target, SpeedTestBackendKind,
};

#[test]
fn speed_test_catalog_exposes_the_expected_labels_and_providers() {
    let targets = list_speed_test_targets();

    assert_eq!(
        targets
            .iter()
            .map(|target| (
                target.id.as_str(),
                target.label.as_str(),
                target.provider.as_str(),
                target.region_label.as_str(),
            ))
            .collect::<Vec<_>>(),
        vec![
            (
                "auto_asia",
                "Auto Asia",
                "Cloudflare (Asia auto-edge)",
                "Asia"
            ),
            (
                "auto_au",
                "Auto Australia",
                "Cloudflare (Australia auto-edge)",
                "Australia",
            ),
            (
                "jp_kr",
                "JP/KR",
                "LibreSpeed (regional fixed backend)",
                "JP/KR"
            ),
            (
                "us_west",
                "US West",
                "LibreSpeed (regional fixed backend)",
                "US West",
            ),
            ("eu", "EU", "LibreSpeed (regional fixed backend)", "EU"),
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
    assert_eq!(
        auto_au.backend_kind,
        SpeedTestBackendKind::CloudflareAutoEdge
    );

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

#[test]
fn speed_test_target_payload_profiles_stay_within_guardrails() {
    for id in ["auto_asia", "auto_au", "jp_kr", "us_west", "eu"] {
        let target = resolve_speed_test_target(Some(id)).expect("target should resolve");
        assert!(
            target.min_download_mb <= target.default_download_mb,
            "{id} default download size should stay above the minimum"
        );
        assert!(
            target.default_download_mb <= target.max_download_mb,
            "{id} default download size should stay below the maximum"
        );
        assert!(
            target.min_upload_bytes <= target.max_upload_bytes,
            "{id} upload bounds should stay ordered"
        );
    }

    let auto_asia = resolve_speed_test_target(Some("auto_asia")).expect("auto_asia should resolve");
    assert_eq!(auto_asia.default_download_mb, 24);
    assert_eq!(auto_asia.max_download_mb, 32);

    let auto_au = resolve_speed_test_target(Some("auto_au")).expect("auto_au should resolve");
    assert_eq!(auto_au.default_download_mb, 20);
    assert!(
        auto_au.max_download_mb <= 24,
        "Auto Australia should keep its HTTP-403 safety ceiling"
    );

    let eu = resolve_speed_test_target(Some("eu")).expect("eu should resolve");
    assert!(
        eu.max_download_mb <= 2,
        "EU should keep its smaller long-haul payload cap"
    );
}
