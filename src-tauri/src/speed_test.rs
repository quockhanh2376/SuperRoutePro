use crate::speed_test_targets::{
    resolve_speed_test_region_label, resolve_speed_test_target, SpeedTestBackendKind,
    SpeedTestTarget,
};
use chrono::Utc;
use futures_util::StreamExt;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter};

const SPEED_TEST_PROGRESS_EVENT: &str = "speed-test://progress";
const LATENCY_SAMPLES: usize = 8;
const LATENCY_WARMUP_SAMPLES: usize = 1;
const MIN_SUCCESSFUL_LATENCY_SAMPLES: usize = 5;
const LATENCY_OUTLIER_MAD_MULTIPLIER: f64 = 3.5;
const REQUEST_TIMEOUT: Duration = Duration::from_secs(90);
const CONNECT_TIMEOUT: Duration = Duration::from_secs(8);
const EMIT_INTERVAL: Duration = Duration::from_millis(180);

#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct SpeedTestTraceMetadata {
    ip: Option<String>,
    colo: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct SpeedTestTargetContext {
    public_ip: Option<String>,
    trace: Option<SpeedTestTraceMetadata>,
}

#[derive(Debug, Deserialize)]
struct LibreSpeedIpLookupResponse {
    #[serde(rename = "processedString")]
    processed_string: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SpeedTestRouteFit {
    PreferredRegion,
    GlobalFallback,
    Pending,
}

#[derive(Debug, Clone, PartialEq)]
struct LatencyMeasurement {
    attempted_samples: usize,
    successful_samples: usize,
    stable_samples: usize,
    ping_ms: f64,
    jitter_ms: f64,
}

#[derive(Debug, Clone, PartialEq)]
struct ThroughputMeasurement {
    mbps: f64,
    bytes_transferred: usize,
    elapsed_ms: u64,
}

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
    pub region_label: String,
    pub route_fit: String,
    pub resolved_colo: Option<String>,
    pub server_label: String,
    pub download_mbps: f64,
    pub upload_mbps: f64,
    pub ping_ms: f64,
    pub jitter_ms: f64,
    pub latency_samples: u32,
    pub successful_latency_samples: u32,
    pub stable_latency_samples: u32,
    pub download_bytes: u64,
    pub upload_bytes: u64,
    pub download_elapsed_ms: u64,
    pub upload_elapsed_ms: u64,
    pub ip: String,
    pub timestamp: String,
}

#[tauri::command]
pub async fn run_speed_test(
    app: AppHandle,
    download_mb: Option<u32>,
    target_id: Option<String>,
) -> Result<SpeedTestResult, String> {
    let target = resolve_speed_test_target(target_id.as_deref())?;
    let download_bytes = sanitize_download_mb(download_mb, target) as usize * 1024 * 1024;
    let upload_bytes = resolve_upload_bytes(target, download_bytes);
    let client = build_client()?;
    let target_context = fetch_target_context(&client, target).await.ok();
    let route_fit = resolve_speed_test_route_fit(target, target_context.as_ref().and_then(|context| context.trace.as_ref()));
    let resolved_colo = target_context
        .as_ref()
        .and_then(|context| context.trace.as_ref())
        .and_then(|trace| trace.colo.as_deref())
        .map(str::to_string);
    let provider_label = resolve_speed_test_provider_label(target);
    let region_label = resolve_speed_test_region_label(target).to_string();
    let server_label = resolve_speed_test_server_label(target, target_context.as_ref());
    let ip = target_context
        .as_ref()
        .and_then(|context| context.public_ip.as_deref())
        .unwrap_or("Unavailable")
        .to_string();

    emit_progress(
        &app,
        "preflight",
        4.0,
        0.0,
        build_preflight_message(target, &provider_label, target_context.as_ref()),
    )?;

    let latency_measurement = measure_latency(&client, target).await?;

    emit_progress(
        &app,
        "latency",
        18.0,
        0.0,
        format!(
            "Latency baseline captured: {:.1} ms ping / {:.1} ms jitter across {} stable sample(s).",
            latency_measurement.ping_ms,
            latency_measurement.jitter_ms,
            latency_measurement.stable_samples,
        ),
    )?;

    let download_measurement = measure_download(&client, &app, target, download_bytes).await?;
    let upload_measurement = measure_upload(&client, &app, target, upload_bytes).await?;

    emit_progress(
        &app,
        "finalize",
        100.0,
        download_measurement.mbps.max(upload_measurement.mbps),
        "Speed test finished.".to_string(),
    )?;

    Ok(SpeedTestResult {
        target_id: target.id.to_string(),
        target_label: target.target_label.to_string(),
        provider: provider_label,
        region_label,
        route_fit: route_fit_label(route_fit).to_string(),
        resolved_colo,
        server_label,
        download_mbps: download_measurement.mbps,
        upload_mbps: upload_measurement.mbps,
        ping_ms: latency_measurement.ping_ms,
        jitter_ms: latency_measurement.jitter_ms,
        latency_samples: latency_measurement.attempted_samples as u32,
        successful_latency_samples: latency_measurement.successful_samples as u32,
        stable_latency_samples: latency_measurement.stable_samples as u32,
        download_bytes: download_measurement.bytes_transferred as u64,
        upload_bytes: upload_measurement.bytes_transferred as u64,
        download_elapsed_ms: download_measurement.elapsed_ms,
        upload_elapsed_ms: upload_measurement.elapsed_ms,
        ip,
        timestamp: Utc::now().to_rfc3339(),
    })
}

fn resolve_speed_test_provider_label(target: SpeedTestTarget) -> String {
    target.provider_label.to_string()
}

fn route_fit_label(route_fit: SpeedTestRouteFit) -> &'static str {
    match route_fit {
        SpeedTestRouteFit::PreferredRegion => "preferred_region",
        SpeedTestRouteFit::GlobalFallback => "global_fallback",
        SpeedTestRouteFit::Pending => "pending",
    }
}

fn build_preflight_message(
    target: SpeedTestTarget,
    provider_label: &str,
    context: Option<&SpeedTestTargetContext>,
) -> String {
    match target.backend_kind {
        SpeedTestBackendKind::CloudflareAutoEdge => match resolve_speed_test_route_fit(
            target,
            context.and_then(|context| context.trace.as_ref()),
        ) {
            SpeedTestRouteFit::PreferredRegion => {
                let colo = context
                    .and_then(|context| context.trace.as_ref())
                    .and_then(|trace| trace.colo.as_deref())
                    .unwrap_or("preferred regional edge");
                let region_label = preferred_region_label(target);
                format!(
                    "Preparing native speed test via {provider_label}. Preferred {region_label} edge resolved at {colo}."
                )
            }
            SpeedTestRouteFit::GlobalFallback => {
                let colo = context
                    .and_then(|context| context.trace.as_ref())
                    .and_then(|trace| trace.colo.as_deref())
                    .unwrap_or("Cloudflare edge");
                let region_label = preferred_region_label(target);
                format!(
                    "Preparing native speed test via {provider_label}. Using a global fallback edge at {colo}, outside the preferred {region_label} region."
                )
            }
            SpeedTestRouteFit::Pending => {
                format!(
                    "Preparing native speed test via {provider_label}. Cloudflare edge will resolve automatically."
                )
            }
        },
        SpeedTestBackendKind::LibreSpeedRegional => {
            format!(
                "Preparing native speed test via {provider_label}. Fixed regional backend: {}.",
                target.default_server_label
            )
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

async fn measure_latency(
    client: &Client,
    target: SpeedTestTarget,
) -> Result<LatencyMeasurement, String> {
    let mut points = Vec::with_capacity(LATENCY_SAMPLES);
    let mut last_error: Option<String> = None;

    for sample in 0..LATENCY_SAMPLES {
        let url = build_latency_request_url(target, sample);
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

    let stable_points = build_latency_baseline(&points);
    if stable_points.len() < MIN_SUCCESSFUL_LATENCY_SAMPLES {
        return Err(describe_latency_probe_failure(
            stable_points.len(),
            last_error.as_deref(),
        ));
    }

    Ok(LatencyMeasurement {
        attempted_samples: LATENCY_SAMPLES,
        successful_samples: points.len(),
        stable_samples: stable_points.len(),
        ping_ms: median(&stable_points),
        jitter_ms: calculate_jitter(&stable_points),
    })
}

async fn measure_download(
    client: &Client,
    app: &AppHandle,
    target: SpeedTestTarget,
    download_bytes: usize,
) -> Result<ThroughputMeasurement, String> {
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

    let url = build_download_request_url(target, download_bytes);
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
    let elapsed = started.elapsed();
    Ok(ThroughputMeasurement {
        mbps: bytes_to_mbps(total_bytes, elapsed.as_secs_f64().max(0.001)),
        bytes_transferred: total_bytes,
        elapsed_ms: milliseconds_from_duration(elapsed),
    })
}

async fn measure_upload(
    client: &Client,
    app: &AppHandle,
    target: SpeedTestTarget,
    upload_bytes: usize,
) -> Result<ThroughputMeasurement, String> {
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
    build_upload_request(client, target, payload)
        .send()
        .await
        .map_err(|error| describe_reqwest_error("Upload test", &error))?
        .error_for_status()
        .map_err(|error| describe_reqwest_error("Upload test", &error))?;

    let elapsed = started.elapsed();
    let upload_mbps = bytes_to_mbps(upload_bytes, elapsed.as_secs_f64().max(0.001));

    emit_progress(
        app,
        "upload",
        96.0,
        upload_mbps,
        "Upload stage complete.".to_string(),
    )?;

    Ok(ThroughputMeasurement {
        mbps: upload_mbps,
        bytes_transferred: upload_bytes,
        elapsed_ms: milliseconds_from_duration(elapsed),
    })
}

fn build_latency_request_url(target: SpeedTestTarget, sample: usize) -> String {
    match target.backend_kind {
        SpeedTestBackendKind::CloudflareAutoEdge => {
            format!("{}?bytes=0&sample={sample}", target.download_url)
        }
        SpeedTestBackendKind::LibreSpeedRegional => format!("{}?r={sample}", target.latency_url),
    }
}

fn build_download_request_url(target: SpeedTestTarget, download_bytes: usize) -> String {
    match target.backend_kind {
        SpeedTestBackendKind::CloudflareAutoEdge => {
            format!("{}?bytes={download_bytes}", target.download_url)
        }
        SpeedTestBackendKind::LibreSpeedRegional => {
            format!(
                "{}?ckSize={}",
                target.download_url,
                bytes_to_mebibytes(download_bytes)
            )
        }
    }
}

fn build_upload_request(
    client: &Client,
    target: SpeedTestTarget,
    payload: Vec<u8>,
) -> reqwest::RequestBuilder {
    let payload_len = payload.len();
    match target.backend_kind {
        SpeedTestBackendKind::CloudflareAutoEdge => client
            .post(format!("{}?bytes={payload_len}", target.upload_url))
            .body(payload),
        SpeedTestBackendKind::LibreSpeedRegional => client.post(target.upload_url).body(payload),
    }
}

async fn fetch_target_context(
    client: &Client,
    target: SpeedTestTarget,
) -> Result<SpeedTestTargetContext, String> {
    match target.backend_kind {
        SpeedTestBackendKind::CloudflareAutoEdge => {
            let trace = fetch_trace_metadata(client, target.ip_lookup_url).await?;
            Ok(SpeedTestTargetContext {
                public_ip: trace.ip.clone(),
                trace: Some(trace),
            })
        }
        SpeedTestBackendKind::LibreSpeedRegional => {
            let public_ip = fetch_librespeed_public_ip(client, target.ip_lookup_url).await?;
            Ok(SpeedTestTargetContext {
                public_ip: Some(public_ip),
                trace: None,
            })
        }
    }
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

async fn fetch_librespeed_public_ip(
    client: &Client,
    ip_lookup_url: &str,
) -> Result<String, String> {
    let body = client
        .get(ip_lookup_url)
        .send()
        .await
        .map_err(|error| describe_reqwest_error("Public IP lookup", &error))?
        .error_for_status()
        .map_err(|error| describe_reqwest_error("Public IP lookup", &error))?
        .text()
        .await
        .map_err(|error| format!("Public IP lookup response could not be read: {error}"))?;

    parse_librespeed_public_ip(&body)
        .ok_or_else(|| "Public IP lookup returned an unrecognized LibreSpeed response.".to_string())
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

fn sanitize_download_mb(value: Option<u32>, target: SpeedTestTarget) -> u32 {
    value
        .unwrap_or(target.default_download_mb)
        .clamp(target.min_download_mb, target.max_download_mb)
}

fn resolve_upload_bytes(target: SpeedTestTarget, download_bytes: usize) -> usize {
    (download_bytes / 2)
        .max(target.min_upload_bytes)
        .min(target.max_upload_bytes)
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

fn milliseconds_from_duration(duration: Duration) -> u64 {
    duration.as_millis().min(u64::MAX as u128) as u64
}

fn bytes_to_mebibytes(total_bytes: usize) -> usize {
    (total_bytes / 1024 / 1024).max(1)
}

fn median(points: &[f64]) -> f64 {
    if points.is_empty() {
        return 0.0;
    }

    let mut sorted = points.to_vec();
    sorted.sort_by(|left, right| left.total_cmp(right));
    let middle = sorted.len() / 2;

    if sorted.len() % 2 == 0 {
        (sorted[middle - 1] + sorted[middle]) / 2.0
    } else {
        sorted[middle]
    }
}

fn median_absolute_deviation(points: &[f64], median_value: f64) -> f64 {
    if points.is_empty() {
        return 0.0;
    }

    let deviations = points
        .iter()
        .map(|point| (point - median_value).abs())
        .collect::<Vec<_>>();

    median(&deviations)
}

fn filter_latency_outliers(points: &[f64]) -> Vec<f64> {
    if points.len() <= MIN_SUCCESSFUL_LATENCY_SAMPLES {
        return points.to_vec();
    }

    let median_value = median(points);
    let mad = median_absolute_deviation(points, median_value);
    if mad <= f64::EPSILON {
        return points.to_vec();
    }

    let max_deviation = mad * LATENCY_OUTLIER_MAD_MULTIPLIER;
    points
        .iter()
        .copied()
        .filter(|point| (point - median_value).abs() <= max_deviation)
        .collect()
}

fn build_latency_baseline(points: &[f64]) -> Vec<f64> {
    let candidate_points = if points.len() > MIN_SUCCESSFUL_LATENCY_SAMPLES {
        points
            .iter()
            .skip(LATENCY_WARMUP_SAMPLES)
            .copied()
            .collect::<Vec<_>>()
    } else {
        points.to_vec()
    };

    let filtered_points = filter_latency_outliers(&candidate_points);
    if filtered_points.len() >= MIN_SUCCESSFUL_LATENCY_SAMPLES {
        filtered_points
    } else {
        candidate_points
    }
}

fn calculate_jitter(points: &[f64]) -> f64 {
    if points.len() < 2 {
        return 0.0;
    }

    let deltas: Vec<f64> = points
        .windows(2)
        .map(|pair| (pair[1] - pair[0]).abs())
        .collect();

    median(&deltas)
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

fn parse_librespeed_public_ip(body: &str) -> Option<String> {
    serde_json::from_str::<LibreSpeedIpLookupResponse>(body)
        .ok()
        .and_then(|response| response.processed_string)
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn resolve_speed_test_route_fit(
    target: SpeedTestTarget,
    trace: Option<&SpeedTestTraceMetadata>,
) -> SpeedTestRouteFit {
    if target.backend_kind != SpeedTestBackendKind::CloudflareAutoEdge {
        return SpeedTestRouteFit::Pending;
    }

    match trace.and_then(|trace| trace.colo.as_deref()) {
        Some(colo) if is_preferred_colo(target, colo) => SpeedTestRouteFit::PreferredRegion,
        Some(_) => SpeedTestRouteFit::GlobalFallback,
        None => SpeedTestRouteFit::Pending,
    }
}

fn preferred_region_label(target: SpeedTestTarget) -> &'static str {
    target.preferred_region_label.unwrap_or("preferred")
}

fn is_preferred_colo(target: SpeedTestTarget, colo: &str) -> bool {
    target
        .preferred_colos
        .iter()
        .any(|candidate| *candidate == colo)
}

fn resolve_speed_test_server_label(
    target: SpeedTestTarget,
    context: Option<&SpeedTestTargetContext>,
) -> String {
    if target.backend_kind != SpeedTestBackendKind::CloudflareAutoEdge {
        return target.default_server_label.to_string();
    }

    let trace = context.and_then(|context| context.trace.as_ref());
    match (
        resolve_speed_test_route_fit(target, trace),
        trace.and_then(|trace| trace.colo.as_deref()),
    ) {
        (SpeedTestRouteFit::PreferredRegion, Some(colo)) => {
            format!("{} Preferred ({colo} edge)", preferred_region_label(target))
        }
        (SpeedTestRouteFit::GlobalFallback, Some(colo)) => {
            format!(
                "Global Fallback ({colo} edge, outside {} preference)",
                preferred_region_label(target)
            )
        }
        _ => target.default_server_label.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use crate::speed_test_targets::resolve_speed_test_target;

    use super::{
        build_download_request_url, build_latency_baseline, build_latency_request_url,
        calculate_jitter, describe_latency_probe_failure, describe_transport_error,
        ensure_bytes_transferred, is_preferred_colo, parse_librespeed_public_ip,
        parse_trace_metadata,
        resolve_speed_test_provider_label, resolve_speed_test_route_fit,
        resolve_speed_test_server_label, resolve_upload_bytes, sanitize_download_mb,
        SpeedTestRouteFit, SpeedTestTargetContext, SpeedTestTraceMetadata,
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

        assert_eq!(label, "Global Fallback (LAX edge, outside Asia preference)");
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
}
