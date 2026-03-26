use chrono::Utc;
use futures_util::StreamExt;
use reqwest::Client;
use serde::Serialize;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter};

const SPEED_TEST_PROGRESS_EVENT: &str = "speed-test://progress";
const DEFAULT_DOWNLOAD_MB: u32 = 24;
const MIN_DOWNLOAD_MB: u32 = 8;
const MAX_DOWNLOAD_MB: u32 = 32;
const DEFAULT_UPLOAD_BYTES: usize = 8 * 1024 * 1024;
const LATENCY_SAMPLES: usize = 6;
const MIN_SUCCESSFUL_LATENCY_SAMPLES: usize = 3;
const REQUEST_TIMEOUT: Duration = Duration::from_secs(90);
const CONNECT_TIMEOUT: Duration = Duration::from_secs(8);
const EMIT_INTERVAL: Duration = Duration::from_millis(180);
const DEFAULT_SPEED_TEST_TARGET_ID: &str = "auto_asia";

#[derive(Clone, Copy, Debug)]
struct SpeedTestTarget {
    id: &'static str,
    target_label: &'static str,
    provider: &'static str,
    policy_label: &'static str,
    default_server_label: &'static str,
    preferred_asia_colos: &'static [&'static str],
    download_api_url: &'static str,
    upload_api_url: &'static str,
    trace_api_url: &'static str,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct SpeedTestTraceMetadata {
    ip: Option<String>,
    colo: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SpeedTestRouteFit {
    PreferredAsia,
    GlobalFallback,
    Pending,
}

const PREFERRED_ASIA_COLOS: [&str; 38] = [
    "SIN", "KUL", "BKK", "CGK", "DPS", "SUB", "SGN", "HAN", "PNH", "RGN", "VTE", "MNL",
    "HKG", "TPE", "KHH", "MFM", "NRT", "HND", "KIX", "ICN", "GMP", "PUS", "KTM", "DAC",
    "CCU", "DEL", "BOM", "AMD", "BLR", "MAA", "HYD", "COK", "CJB", "CMB", "DXB", "DOH",
    "MCT", "BAH",
];

const DEFAULT_SPEED_TEST_TARGET: SpeedTestTarget = SpeedTestTarget {
    id: DEFAULT_SPEED_TEST_TARGET_ID,
    target_label: "Auto Asia",
    provider: "Cloudflare",
    policy_label: "Asia auto-edge",
    default_server_label: "Cloudflare auto edge",
    preferred_asia_colos: &PREFERRED_ASIA_COLOS,
    download_api_url: "https://speed.cloudflare.com/__down",
    upload_api_url: "https://speed.cloudflare.com/__up",
    trace_api_url: "https://speed.cloudflare.com/cdn-cgi/trace",
};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct SpeedTestProgress {
    pub stage: String,
    pub percent: f64,
    pub current_speed_mbps: f64,
    pub message: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct SpeedTestResult {
    pub target_id: String,
    pub target_label: String,
    pub provider: String,
    pub server_label: String,
    pub download_mbps: f64,
    pub upload_mbps: f64,
    pub ping_ms: f64,
    pub jitter_ms: f64,
    pub ip: String,
    pub timestamp: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct SpeedTestCatalogEntry {
    pub id: String,
    pub label: String,
    pub description: String,
    pub provider: String,
}

#[tauri::command]
pub fn list_speed_test_targets() -> Vec<SpeedTestCatalogEntry> {
    vec![build_speed_test_catalog_entry(DEFAULT_SPEED_TEST_TARGET)]
}

#[tauri::command]
pub async fn run_speed_test(
    app: AppHandle,
    download_mb: Option<u32>,
    target_id: Option<String>,
) -> Result<SpeedTestResult, String> {
    let target = resolve_speed_test_target(target_id.as_deref())?;
    let download_bytes = sanitize_download_mb(download_mb) as usize * 1024 * 1024;
    let upload_bytes = DEFAULT_UPLOAD_BYTES.min((download_bytes / 2).max(2 * 1024 * 1024));
    let client = build_client()?;
    let trace_metadata = fetch_trace_metadata(&client, target.trace_api_url)
        .await
        .ok();
    let provider_label = resolve_speed_test_provider_label(target);
    let server_label = resolve_speed_test_server_label(target, trace_metadata.as_ref());
    let ip = trace_metadata
        .as_ref()
        .and_then(|trace| trace.ip.as_deref())
        .unwrap_or("Unavailable")
        .to_string();

    emit_progress(
        &app,
        "preflight",
        4.0,
        0.0,
        build_preflight_message(target, &provider_label, trace_metadata.as_ref()),
    )?;

    let latency_points = measure_latency(&client, target).await?;
    let ping_ms = average(&latency_points);
    let jitter_ms = calculate_jitter(&latency_points);

    emit_progress(
        &app,
        "latency",
        18.0,
        0.0,
        format!(
            "Latency baseline captured: {:.1} ms ping / {:.1} ms jitter.",
            ping_ms, jitter_ms
        ),
    )?;

    let download_mbps = measure_download(&client, &app, target, download_bytes).await?;
    let upload_mbps = measure_upload(&client, &app, target, upload_bytes).await?;

    emit_progress(
        &app,
        "finalize",
        100.0,
        download_mbps.max(upload_mbps),
        "Speed test finished.".to_string(),
    )?;

    Ok(SpeedTestResult {
        target_id: target.id.to_string(),
        target_label: target.target_label.to_string(),
        provider: provider_label,
        server_label,
        download_mbps,
        upload_mbps,
        ping_ms,
        jitter_ms,
        ip,
        timestamp: Utc::now().to_rfc3339(),
    })
}

fn build_speed_test_catalog_entry(target: SpeedTestTarget) -> SpeedTestCatalogEntry {
    SpeedTestCatalogEntry {
        id: target.id.to_string(),
        label: target.target_label.to_string(),
        description: "Cloudflare auto-selects the nearest preferred Asia edge. Country pinning will layer on top of this catalog when real region targets are available.".to_string(),
        provider: resolve_speed_test_provider_label(target),
    }
}

fn resolve_speed_test_target(target_id: Option<&str>) -> Result<SpeedTestTarget, String> {
    match target_id.unwrap_or(DEFAULT_SPEED_TEST_TARGET_ID) {
        DEFAULT_SPEED_TEST_TARGET_ID => Ok(DEFAULT_SPEED_TEST_TARGET),
        unknown => Err(format!("Unknown speed test target: {unknown}")),
    }
}

fn resolve_speed_test_provider_label(target: SpeedTestTarget) -> String {
    format!("{} ({})", target.provider, target.policy_label)
}

fn build_preflight_message(
    target: SpeedTestTarget,
    provider_label: &str,
    trace: Option<&SpeedTestTraceMetadata>,
) -> String {
    match resolve_speed_test_route_fit(target, trace) {
        SpeedTestRouteFit::PreferredAsia => {
            let colo = trace
                .and_then(|trace| trace.colo.as_deref())
                .unwrap_or("preferred Asia edge");
            format!("Preparing native speed test via {provider_label}. Preferred Asia edge resolved at {colo}.")
        }
        SpeedTestRouteFit::GlobalFallback => {
            let colo = trace
                .and_then(|trace| trace.colo.as_deref())
                .unwrap_or("Cloudflare edge");
            format!("Preparing native speed test via {provider_label}. Using a global fallback edge at {colo}.")
        }
        SpeedTestRouteFit::Pending => {
            format!("Preparing native speed test via {provider_label}. Cloudflare edge will resolve automatically.")
        }
    }
}

fn build_client() -> Result<Client, String> {
    Client::builder()
        .connect_timeout(CONNECT_TIMEOUT)
        .timeout(REQUEST_TIMEOUT)
        .user_agent("SuperRoutePro-SpeedTest/1.0")
        .build()
        .map_err(|error| describe_reqwest_error("Speed test client setup", &error))
}

async fn measure_latency(client: &Client, target: SpeedTestTarget) -> Result<Vec<f64>, String> {
    let mut points = Vec::with_capacity(LATENCY_SAMPLES);
    let mut last_error: Option<String> = None;

    for sample in 0..LATENCY_SAMPLES {
        let url = format!("{}?bytes=0&sample={sample}", target.download_api_url);
        let started = Instant::now();
        let response = client
            .get(&url)
            .send()
            .await
            .map_err(|error| describe_reqwest_error("Latency probe", &error));

        match response.and_then(|response| {
            response
                .error_for_status()
                .map_err(|error| describe_reqwest_error("Latency probe", &error))
        }) {
            Ok(_) => points.push(started.elapsed().as_secs_f64() * 1000.0),
            Err(error) => last_error = Some(error),
        }
    }

    if points.len() < MIN_SUCCESSFUL_LATENCY_SAMPLES {
        return Err(describe_latency_probe_failure(
            points.len(),
            last_error.as_deref(),
        ));
    }

    Ok(points)
}

async fn measure_download(
    client: &Client,
    app: &AppHandle,
    target: SpeedTestTarget,
    download_bytes: usize,
) -> Result<f64, String> {
    emit_progress(
        app,
        "download",
        24.0,
        0.0,
        format!(
            "Downloading ~{} MB test payload...",
            download_bytes / 1024 / 1024
        ),
    )?;

    let url = format!("{}?bytes={download_bytes}", target.download_api_url);
    let response = client
        .get(&url)
        .send()
        .await
        .map_err(|error| describe_reqwest_error("Download test", &error))?
        .error_for_status()
        .map_err(|error| describe_reqwest_error("Download test", &error))?;

    let mut total_bytes = 0usize;
    let started = Instant::now();
    let mut last_emit = Instant::now();
    let mut stream = response.bytes_stream();

    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|error| format!("Download stream failed: {error}"))?;
        total_bytes += chunk.len();

        if last_emit.elapsed() >= EMIT_INTERVAL {
            let elapsed = started.elapsed().as_secs_f64().max(0.001);
            let current_speed_mbps = bytes_to_mbps(total_bytes, elapsed);
            let percent =
                24.0 + ((total_bytes as f64 / download_bytes as f64).clamp(0.0, 1.0) * 44.0);
            emit_progress(
                app,
                "download",
                percent,
                current_speed_mbps,
                format!(
                    "Downloading test payload... {} / {} MB",
                    total_bytes / 1024 / 1024,
                    download_bytes / 1024 / 1024
                ),
            )?;
            last_emit = Instant::now();
        }
    }

    ensure_bytes_transferred("Download test", total_bytes)?;
    let elapsed = started.elapsed().as_secs_f64().max(0.001);
    Ok(bytes_to_mbps(total_bytes, elapsed))
}

async fn measure_upload(
    client: &Client,
    app: &AppHandle,
    target: SpeedTestTarget,
    upload_bytes: usize,
) -> Result<f64, String> {
    emit_progress(
        app,
        "upload",
        72.0,
        0.0,
        format!(
            "Uploading ~{} MB validation payload...",
            upload_bytes / 1024 / 1024
        ),
    )?;

    let payload = vec![0u8; upload_bytes];
    let started = Instant::now();
    client
        .post(format!("{}?bytes={upload_bytes}", target.upload_api_url))
        .body(payload)
        .send()
        .await
        .map_err(|error| describe_reqwest_error("Upload test", &error))?
        .error_for_status()
        .map_err(|error| describe_reqwest_error("Upload test", &error))?;

    let elapsed = started.elapsed().as_secs_f64().max(0.001);
    let upload_mbps = bytes_to_mbps(upload_bytes, elapsed);

    emit_progress(
        app,
        "upload",
        96.0,
        upload_mbps,
        "Upload stage complete.".to_string(),
    )?;

    Ok(upload_mbps)
}

async fn fetch_trace_metadata(
    client: &Client,
    trace_api_url: &str,
) -> Result<SpeedTestTraceMetadata, String> {
    let trace = client
        .get(trace_api_url)
        .send()
        .await
        .map_err(|error| describe_reqwest_error("Public IP lookup", &error))?
        .error_for_status()
        .map_err(|error| describe_reqwest_error("Public IP lookup", &error))?
        .text()
        .await
        .map_err(|error| format!("Public IP lookup response could not be read: {error}"))?;

    Ok(parse_trace_metadata(&trace))
}

fn emit_progress(
    app: &AppHandle,
    stage: &str,
    percent: f64,
    current_speed_mbps: f64,
    message: String,
) -> Result<(), String> {
    app.emit(
        SPEED_TEST_PROGRESS_EVENT,
        SpeedTestProgress {
            stage: stage.to_string(),
            percent,
            current_speed_mbps,
            message,
        },
    )
    .map_err(|error| format!("Could not emit speed test progress: {error}"))
}

fn sanitize_download_mb(value: Option<u32>) -> u32 {
    value
        .unwrap_or(DEFAULT_DOWNLOAD_MB)
        .clamp(MIN_DOWNLOAD_MB, MAX_DOWNLOAD_MB)
}

fn describe_reqwest_error(stage: &str, error: &reqwest::Error) -> String {
    describe_transport_error(
        stage,
        error.is_timeout(),
        error.is_connect(),
        error.status().map(|status| status.as_u16()),
        &error.to_string(),
    )
}

fn describe_transport_error(
    stage: &str,
    is_timeout: bool,
    is_connect: bool,
    status_code: Option<u16>,
    raw: &str,
) -> String {
    if is_timeout {
        return format!("{stage} timed out. Check internet connectivity or try again in a moment.");
    }

    if is_connect {
        return format!(
            "{stage} could not reach the test server. Verify the network path and retry."
        );
    }

    if let Some(status_code) = status_code {
        return format!("{stage} returned HTTP {status_code}. The test server may be unavailable.");
    }

    format!("{stage} failed: {raw}")
}

fn ensure_bytes_transferred(stage: &str, total_bytes: usize) -> Result<(), String> {
    if total_bytes == 0 {
        return Err(format!(
            "{stage} returned no payload bytes. Check connectivity and try again."
        ));
    }

    Ok(())
}

fn describe_latency_probe_failure(sample_count: usize, last_error: Option<&str>) -> String {
    let mut message = format!(
        "Latency check collected only {sample_count} stable sample(s). At least {MIN_SUCCESSFUL_LATENCY_SAMPLES} successful probes are required."
    );

    if let Some(last_error) = last_error {
        message.push(' ');
        message.push_str(last_error);
    }

    message
}

fn bytes_to_mbps(total_bytes: usize, elapsed_seconds: f64) -> f64 {
    if total_bytes == 0 || elapsed_seconds <= 0.0 {
        return 0.0;
    }
    (total_bytes as f64 * 8.0) / elapsed_seconds / 1_000_000.0
}

fn average(points: &[f64]) -> f64 {
    if points.is_empty() {
        return 0.0;
    }
    points.iter().sum::<f64>() / points.len() as f64
}

fn calculate_jitter(points: &[f64]) -> f64 {
    if points.len() < 2 {
        return 0.0;
    }

    let deltas: Vec<f64> = points
        .windows(2)
        .map(|pair| (pair[1] - pair[0]).abs())
        .collect();

    average(&deltas)
}

fn parse_trace_metadata(trace: &str) -> SpeedTestTraceMetadata {
    let mut metadata = SpeedTestTraceMetadata::default();

    for line in trace.lines() {
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };

        match key {
            "ip" => metadata.ip = Some(value.trim().to_string()),
            "colo" => metadata.colo = Some(value.trim().to_ascii_uppercase()),
            _ => {}
        }
    }

    metadata
}

fn resolve_speed_test_route_fit(
    target: SpeedTestTarget,
    trace: Option<&SpeedTestTraceMetadata>,
) -> SpeedTestRouteFit {
    match trace.and_then(|trace| trace.colo.as_deref()) {
        Some(colo) if is_preferred_asia_colo(target, colo) => SpeedTestRouteFit::PreferredAsia,
        Some(_) => SpeedTestRouteFit::GlobalFallback,
        None => SpeedTestRouteFit::Pending,
    }
}

fn is_preferred_asia_colo(target: SpeedTestTarget, colo: &str) -> bool {
    target
        .preferred_asia_colos
        .iter()
        .any(|candidate| *candidate == colo)
}

fn resolve_speed_test_server_label(
    target: SpeedTestTarget,
    trace: Option<&SpeedTestTraceMetadata>,
) -> String {
    match (
        resolve_speed_test_route_fit(target, trace),
        trace.and_then(|trace| trace.colo.as_deref()),
    ) {
        (SpeedTestRouteFit::PreferredAsia, Some(colo)) => format!("Asia Preferred ({colo} edge)"),
        (SpeedTestRouteFit::GlobalFallback, Some(colo)) => {
            format!("Global Fallback ({colo} edge, outside Asia preference)")
        }
        _ => target.default_server_label.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::{
        calculate_jitter, describe_latency_probe_failure, describe_transport_error,
        ensure_bytes_transferred, is_preferred_asia_colo, list_speed_test_targets,
        parse_trace_metadata, resolve_speed_test_provider_label, resolve_speed_test_route_fit,
        resolve_speed_test_server_label, resolve_speed_test_target, sanitize_download_mb,
        SpeedTestRouteFit, SpeedTestTraceMetadata,
    };

    #[test]
    fn sanitize_download_mb_clamps_supported_range() {
        assert_eq!(sanitize_download_mb(None), 24);
        assert_eq!(sanitize_download_mb(Some(2)), 8);
        assert_eq!(sanitize_download_mb(Some(48)), 32);
        assert_eq!(sanitize_download_mb(Some(16)), 16);
    }

    #[test]
    fn parse_trace_metadata_reads_cloudflare_trace_format() {
        let trace =
            "fl=29f64\nh=speed.cloudflare.com\nip=203.0.113.7\ncolo=SIN\nts=1711111111.111\n";
        assert_eq!(
            parse_trace_metadata(trace),
            SpeedTestTraceMetadata {
                ip: Some("203.0.113.7".to_string()),
                colo: Some("SIN".to_string()),
            }
        );
    }

    #[test]
    fn calculate_jitter_uses_average_gap_between_samples() {
        let points = [10.0, 14.0, 15.0, 21.0];
        let jitter = calculate_jitter(&points);
        assert!((jitter - 3.6666666667).abs() < 0.0001);
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
    fn resolve_speed_test_target_defaults_to_cloudflare_asia_policy() {
        let target = resolve_speed_test_target(None).expect("default target should resolve");
        assert_eq!(target.id, "auto_asia");
        assert_eq!(target.target_label, "Auto Asia");
        assert_eq!(target.provider, "Cloudflare");
        assert_eq!(target.policy_label, "Asia auto-edge");
        assert_eq!(target.default_server_label, "Cloudflare auto edge");
        assert_eq!(
            target.download_api_url,
            "https://speed.cloudflare.com/__down"
        );
        assert_eq!(target.upload_api_url, "https://speed.cloudflare.com/__up");
        assert_eq!(
            target.trace_api_url,
            "https://speed.cloudflare.com/cdn-cgi/trace"
        );
    }

    #[test]
    fn resolve_speed_test_server_label_uses_resolved_edge_when_available() {
        let label = resolve_speed_test_server_label(
            resolve_speed_test_target(None).expect("default target should resolve"),
            Some(&SpeedTestTraceMetadata {
                ip: Some("203.0.113.7".to_string()),
                colo: Some("SIN".to_string()),
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
            Some(&SpeedTestTraceMetadata {
                ip: Some("203.0.113.7".to_string()),
                colo: Some("LAX".to_string()),
            }),
        );

        assert_eq!(label, "Global Fallback (LAX edge, outside Asia preference)");
    }

    #[test]
    fn resolve_speed_test_provider_label_surfaces_policy_name() {
        let provider = resolve_speed_test_provider_label(
            resolve_speed_test_target(None).expect("default target should resolve"),
        );

        assert_eq!(provider, "Cloudflare (Asia auto-edge)");
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
            SpeedTestRouteFit::PreferredAsia
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

        assert_eq!(resolve_speed_test_route_fit(target, None), SpeedTestRouteFit::Pending);
    }

    #[test]
    fn is_preferred_asia_colo_accepts_common_edge_codes() {
        let target = resolve_speed_test_target(None).expect("default target should resolve");

        assert!(is_preferred_asia_colo(target, "SIN"));
        assert!(is_preferred_asia_colo(target, "SGN"));
        assert!(is_preferred_asia_colo(target, "NRT"));
        assert!(!is_preferred_asia_colo(target, "LAX"));
    }

    #[test]
    fn describe_latency_probe_failure_mentions_threshold_and_last_error() {
        let message = describe_latency_probe_failure(1, Some("Latency probe timed out."));
        assert_eq!(
            message,
            "Latency check collected only 1 stable sample(s). At least 3 successful probes are required. Latency probe timed out."
        );
    }

    #[test]
    fn list_speed_test_targets_exposes_the_auto_asia_catalog_entry() {
        let targets = list_speed_test_targets();

        assert_eq!(targets.len(), 1);
        assert_eq!(targets[0].id, "auto_asia");
        assert_eq!(targets[0].label, "Auto Asia");
        assert_eq!(targets[0].provider, "Cloudflare (Asia auto-edge)");
        assert!(targets[0].description.contains("Country pinning"));
    }

    #[test]
    fn resolve_speed_test_target_rejects_unknown_target_ids() {
        let error = resolve_speed_test_target(Some("us_west")).expect_err("target should be rejected");
        assert_eq!(error, "Unknown speed test target: us_west");
    }
}
