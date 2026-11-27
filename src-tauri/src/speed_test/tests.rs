use crate::speed_test_targets::resolve_speed_test_target;
use std::time::Duration;

use super::latency::{
    build_latency_baseline, build_latency_request_url, calculate_jitter,
    describe_latency_probe_failure, LATENCY_STAGE_TIMEOUT,
};
use super::transfer::{
    build_download_request_url, choose_calibration_download_bytes, download_stage_timeout,
    ensure_bytes_transferred, estimate_adaptive_download_bytes,
    is_transfer_duration_in_target_window, resolve_upload_bytes, sanitize_download_mb,
    upload_stage_timeout, ThroughputMeasurement, MAX_DOWNLOAD_STAGE_TIMEOUT,
    MAX_UPLOAD_STAGE_TIMEOUT, MEBIBYTE, MIN_DOWNLOAD_STAGE_TIMEOUT, MIN_UPLOAD_STAGE_TIMEOUT,
};
use super::{
    describe_transport_error, is_preferred_colo, parse_librespeed_public_ip, parse_trace_metadata,
    resolve_speed_test_provider_label, resolve_speed_test_route_fit,
    resolve_speed_test_server_label, SpeedTestRouteFit, SpeedTestTargetContext,
    SpeedTestTraceMetadata,
};

#[test]
fn sanitize_download_mb_respects_target_profiles() {
    let auto_asia = resolve_speed_test_target(None).expect("default target should resolve");
    let auto_au =
        resolve_speed_test_target(Some("auto_au")).expect("auto_au target should resolve");
    let jp_kr = resolve_speed_test_target(Some("jp_kr")).expect("jp_kr target should resolve");
    let eu = resolve_speed_test_target(Some("eu")).expect("eu target should resolve");

    assert_eq!(sanitize_download_mb(None, auto_asia), 24);
    assert_eq!(sanitize_download_mb(Some(2), auto_asia), 8);
    assert_eq!(sanitize_download_mb(Some(48), auto_asia), 32);
    assert_eq!(sanitize_download_mb(None, auto_au), 20);
    assert_eq!(sanitize_download_mb(None, jp_kr), 4);
    assert_eq!(sanitize_download_mb(Some(32), jp_kr), 8);
    assert_eq!(sanitize_download_mb(None, eu), 1);
    assert_eq!(sanitize_download_mb(Some(8), eu), 2);
}

#[test]
fn choose_calibration_download_bytes_stays_within_target_profiles() {
    let auto_asia = resolve_speed_test_target(None).expect("default target should resolve");
    let auto_au =
        resolve_speed_test_target(Some("auto_au")).expect("auto_au target should resolve");
    let jp_kr = resolve_speed_test_target(Some("jp_kr")).expect("jp_kr target should resolve");
    let eu = resolve_speed_test_target(Some("eu")).expect("eu target should resolve");

    assert_eq!(choose_calibration_download_bytes(auto_asia), 8 * MEBIBYTE);
    assert_eq!(choose_calibration_download_bytes(auto_au), 8 * MEBIBYTE);
    assert_eq!(choose_calibration_download_bytes(jp_kr), 2 * MEBIBYTE);
    assert_eq!(choose_calibration_download_bytes(eu), 1 * MEBIBYTE);
}

#[test]
fn estimate_adaptive_download_bytes_scales_with_calibration_result() {
    let auto_asia = resolve_speed_test_target(None).expect("default target should resolve");
    let eu = resolve_speed_test_target(Some("eu")).expect("eu target should resolve");

    let larger_payload = estimate_adaptive_download_bytes(
        auto_asia,
        &ThroughputMeasurement {
            mbps: 24.0,
            bytes_transferred: 8 * MEBIBYTE,
            elapsed_ms: 2_000,
        },
    );
    assert!(larger_payload > 8 * MEBIBYTE);
    assert!(larger_payload <= 32 * MEBIBYTE);

    let clamped_payload = estimate_adaptive_download_bytes(
        eu,
        &ThroughputMeasurement {
            mbps: 90.0,
            bytes_transferred: 1 * MEBIBYTE,
            elapsed_ms: 300,
        },
    );
    assert_eq!(clamped_payload, 2 * MEBIBYTE);
}

#[test]
fn transfer_window_and_stage_timeouts_stay_bounded() {
    assert!(is_transfer_duration_in_target_window(Duration::from_secs(5)));
    assert!(!is_transfer_duration_in_target_window(Duration::from_secs(
        7
    )));

    let download_timeout = download_stage_timeout(32 * MEBIBYTE);
    let upload_timeout = upload_stage_timeout(8 * MEBIBYTE);

    assert!(LATENCY_STAGE_TIMEOUT < download_timeout);
    assert!(download_timeout >= MIN_DOWNLOAD_STAGE_TIMEOUT);
    assert!(download_timeout <= MAX_DOWNLOAD_STAGE_TIMEOUT);
    assert!(upload_timeout >= MIN_UPLOAD_STAGE_TIMEOUT);
    assert!(upload_timeout <= MAX_UPLOAD_STAGE_TIMEOUT);
}

#[test]
fn parse_trace_metadata_reads_cloudflare_trace_format() {
    let trace = "fl=29f64\nh=speed.cloudflare.com\nip=203.0.113.7\ncolo=SIN\nts=1711111111.111\n";
    assert_eq!(
        parse_trace_metadata(trace),
        SpeedTestTraceMetadata {
            ip: Some("203.0.113.7".to_string()),
            colo: Some("SIN".to_string()),
        }
    );
}

#[test]
fn parse_librespeed_public_ip_reads_json_lookup_format() {
    assert_eq!(
        parse_librespeed_public_ip(r#"{"processedString":"203.0.113.7","rawIspInfo":""}"#),
        Some("203.0.113.7".to_string())
    );
    assert_eq!(
        parse_librespeed_public_ip(r#"{"processedString":"","rawIspInfo":""}"#),
        None
    );
}

#[test]
fn calculate_jitter_uses_median_gap_between_samples() {
    let points = [10.0, 14.0, 15.0, 21.0];
    let jitter = calculate_jitter(&points);
    assert!((jitter - 4.0).abs() < 0.0001);
}

#[test]
fn build_latency_baseline_drops_warmup_and_large_outliers() {
    let points = [58.0, 24.0, 25.0, 24.5, 25.1, 78.0, 24.8];
    let baseline = build_latency_baseline(&points);

    assert_eq!(baseline, vec![24.0, 25.0, 24.5, 25.1, 24.8]);
}

#[test]
fn describe_transport_error_prefers_timeout_message() {
    let message =
        describe_transport_error("Download test", true, false, None, "request timed out");
    assert_eq!(
        message,
        "Download test timed out. Check internet connectivity or try again in a moment."
    );
}

#[test]
fn describe_transport_error_handles_connectivity_and_status() {
    let connect = describe_transport_error("Upload test", false, true, None, "dns failed");
    assert_eq!(
        connect,
        "Upload test could not reach the test server. Verify the network path and retry."
    );

    let status = describe_transport_error(
        "Latency probe",
        false,
        false,
        Some(503),
        "service unavailable",
    );
    assert_eq!(
        status,
        "Latency probe returned HTTP 503. The test server may be unavailable."
    );
}

#[test]
fn ensure_bytes_transferred_rejects_empty_payloads() {
    assert_eq!(
        ensure_bytes_transferred("Download test", 0),
        Err(
            "Download test returned no payload bytes. Check connectivity and try again."
                .to_string()
        )
    );
    assert_eq!(ensure_bytes_transferred("Download test", 128), Ok(()));
}

#[test]
fn resolve_speed_test_server_label_uses_resolved_edge_when_available() {
    let label = resolve_speed_test_server_label(
        resolve_speed_test_target(None).expect("default target should resolve"),
        Some(&SpeedTestTargetContext {
            public_ip: Some("203.0.113.7".to_string()),
            trace: Some(SpeedTestTraceMetadata {
                ip: Some("203.0.113.7".to_string()),
                colo: Some("SIN".to_string()),
            }),
        }),
    );

    assert_eq!(label, "Asia Preferred (SIN edge)");
}

#[test]
fn resolve_speed_test_server_label_falls_back_when_trace_metadata_is_missing() {
    let label = resolve_speed_test_server_label(
        resolve_speed_test_target(None).expect("default target should resolve"),
        None,
    );

    assert_eq!(label, "Cloudflare auto edge");
}

#[test]
fn resolve_speed_test_server_label_marks_non_asia_edges_as_global_fallback() {
    let label = resolve_speed_test_server_label(
        resolve_speed_test_target(None).expect("default target should resolve"),
        Some(&SpeedTestTargetContext {
            public_ip: Some("203.0.113.7".to_string()),
            trace: Some(SpeedTestTraceMetadata {
                ip: Some("203.0.113.7".to_string()),
                colo: Some("LAX".to_string()),
            }),
        }),
    );

    assert_eq!(
        label,
        "Global Fallback (LAX edge, outside Asia preference)"
    );
}

#[test]
fn resolve_speed_test_server_label_keeps_fixed_regional_backend_names() {
    let label = resolve_speed_test_server_label(
        resolve_speed_test_target(Some("jp_kr")).expect("jp_kr target should resolve"),
        Some(&SpeedTestTargetContext {
            public_ip: Some("203.0.113.7".to_string()),
            trace: None,
        }),
    );

    assert_eq!(label, "Tokyo, Japan (A573)");
}

#[test]
fn resolve_speed_test_provider_label_surfaces_backend_policy() {
    let provider = resolve_speed_test_provider_label(
        resolve_speed_test_target(None).expect("default target should resolve"),
    );
    let auto_au_provider = resolve_speed_test_provider_label(
        resolve_speed_test_target(Some("auto_au")).expect("auto_au target should resolve"),
    );
    let eu_provider = resolve_speed_test_provider_label(
        resolve_speed_test_target(Some("eu")).expect("eu target should resolve"),
    );

    assert_eq!(provider, "Cloudflare (Asia auto-edge)");
    assert_eq!(auto_au_provider, "Cloudflare (Australia auto-edge)");
    assert_eq!(eu_provider, "LibreSpeed (regional fixed backend)");
}

#[test]
fn resolve_speed_test_route_fit_distinguishes_preferred_fallback_and_pending() {
    let target = resolve_speed_test_target(None).expect("default target should resolve");

    assert_eq!(
        resolve_speed_test_route_fit(
            target,
            Some(&SpeedTestTraceMetadata {
                ip: Some("203.0.113.7".to_string()),
                colo: Some("SIN".to_string()),
            })
        ),
        SpeedTestRouteFit::PreferredRegion
    );

    assert_eq!(
        resolve_speed_test_route_fit(
            target,
            Some(&SpeedTestTraceMetadata {
                ip: Some("203.0.113.7".to_string()),
                colo: Some("LAX".to_string()),
            })
        ),
        SpeedTestRouteFit::GlobalFallback
    );

    assert_eq!(
        resolve_speed_test_route_fit(target, None),
        SpeedTestRouteFit::Pending
    );
}

#[test]
fn is_preferred_colo_accepts_common_edge_codes() {
    let target = resolve_speed_test_target(None).expect("default target should resolve");

    assert!(is_preferred_colo(target, "SIN"));
    assert!(is_preferred_colo(target, "SGN"));
    assert!(is_preferred_colo(target, "NRT"));
    assert!(!is_preferred_colo(target, "LAX"));

    let auto_au =
        resolve_speed_test_target(Some("auto_au")).expect("auto_au target should resolve");
    assert!(is_preferred_colo(auto_au, "SYD"));
    assert!(is_preferred_colo(auto_au, "MEL"));
    assert!(!is_preferred_colo(auto_au, "NRT"));
}

#[test]
fn describe_latency_probe_failure_mentions_threshold_and_last_error() {
    let message = describe_latency_probe_failure(1, Some("Latency probe timed out."));
    assert_eq!(
        message,
        "Latency check collected only 1 stable sample(s). At least 5 successful probes are required. Latency probe timed out."
    );
}

#[test]
fn request_builders_use_backend_specific_semantics() {
    let auto_asia = resolve_speed_test_target(None).expect("default target should resolve");
    let jp_kr = resolve_speed_test_target(Some("jp_kr")).expect("jp_kr target should resolve");

    assert_eq!(
        build_latency_request_url(auto_asia, 3),
        "https://speed.cloudflare.com/__down?bytes=0&sample=3"
    );
    assert_eq!(
        build_download_request_url(auto_asia, 8 * 1024 * 1024),
        "https://speed.cloudflare.com/__down?bytes=8388608"
    );
    assert_eq!(
        build_latency_request_url(jp_kr, 3),
        "https://librespeed.a573.net/backend/empty.php?r=3"
    );
    assert_eq!(
        build_download_request_url(jp_kr, 4 * 1024 * 1024),
        "https://librespeed.a573.net/backend/garbage.php?ckSize=4"
    );
}

#[test]
fn resolve_upload_bytes_respects_per_target_caps() {
    let auto_asia = resolve_speed_test_target(None).expect("default target should resolve");
    let eu = resolve_speed_test_target(Some("eu")).expect("eu target should resolve");

    assert_eq!(
        resolve_upload_bytes(auto_asia, 32 * 1024 * 1024),
        8 * 1024 * 1024
    );
    assert_eq!(resolve_upload_bytes(eu, 2 * 1024 * 1024), 512 * 1024);
    assert_eq!(resolve_upload_bytes(eu, 512 * 1024), 256 * 1024);
}
