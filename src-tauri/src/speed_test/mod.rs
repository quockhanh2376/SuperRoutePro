mod latency;
#[cfg(test)]
mod tests;
mod transfer;

use crate::speed_test_targets::{
    resolve_speed_test_region_label, resolve_speed_test_target, SpeedTestBackendKind,
    SpeedTestTarget,
};
use chrono::Utc;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::sync::OnceLock;
use std::time::Duration;
use tauri::{AppHandle, Emitter};

const SPEED_TEST_PROGRESS_EVENT: &str = "speed-test://progress";
const CONNECT_TIMEOUT: Duration = Duration::from_secs(8);
const PUBLIC_IP_LOOKUP_TIMEOUT: Duration = Duration::from_secs(6);
static SPEED_TEST_CLIENT: OnceLock<Client> = OnceLock::new();

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(super) struct SpeedTestTraceMetadata {
    pub(super) ip: Option<String>,
    pub(super) colo: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(super) struct SpeedTestTargetContext {
    pub(super) public_ip: Option<String>,
    pub(super) trace: Option<SpeedTestTraceMetadata>,
}

#[derive(Debug, Deserialize)]
struct LibreSpeedIpLookupResponse {
    #[serde(rename = "processedString")]
    processed_string: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum SpeedTestRouteFit {
    PreferredRegion,
    GlobalFallback,
    Pending,
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
    run_speed_test_internal(Some(&app), download_mb, target_id.as_deref()).await
}

pub async fn run_speed_test_snapshot(
    download_mb: Option<u32>,
    target_id: Option<&str>,
) -> Result<SpeedTestResult, String> {
    run_speed_test_internal(None, download_mb, target_id).await
}

async fn run_speed_test_internal(
    app: Option<&AppHandle>,
    download_mb: Option<u32>,
    target_id: Option<&str>,
) -> Result<SpeedTestResult, String> {
    let target = resolve_speed_test_target(target_id)?;
    let client = shared_client()?;
    let target_context = fetch_target_context(client, target).await.ok();
    let route_fit = resolve_speed_test_route_fit(
        target,
        target_context
            .as_ref()
            .and_then(|context| context.trace.as_ref()),
    );
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
        app,
        "preflight",
        4.0,
        0.0,
        build_preflight_message(target, &provider_label, target_context.as_ref()),
    )?;

    let latency_measurement = latency::measure_latency(client, target).await?;

    emit_progress(
        app,
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

    let download_measurement = transfer::measure_download(client, app, target, download_mb).await?;
    let upload_bytes = transfer::resolve_upload_bytes(target, download_measurement.bytes_transferred);
    let upload_measurement = transfer::measure_upload(client, app, target, upload_bytes).await?;

    emit_progress(
        app,
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

pub(super) fn resolve_speed_test_provider_label(target: SpeedTestTarget) -> String {
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
        .user_agent("SuperRoutePro-SpeedTest/1.0")
        .build()
        .map_err(|error| describe_reqwest_error("Speed test client setup", &error))
}

fn shared_client() -> Result<&'static Client, String> {
    if let Some(client) = SPEED_TEST_CLIENT.get() {
        return Ok(client);
    }
    let client = build_client()?;
    Ok(SPEED_TEST_CLIENT.get_or_init(|| client))
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
        .timeout(PUBLIC_IP_LOOKUP_TIMEOUT)
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
        .timeout(PUBLIC_IP_LOOKUP_TIMEOUT)
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

pub(super) fn emit_progress(
    app: Option<&AppHandle>,
    stage: &str,
    percent: f64,
    current_speed_mbps: f64,
    message: String,
) -> Result<(), String> {
    let Some(app_handle) = app else {
        return Ok(());
    };

    app_handle
        .emit(
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

pub(super) fn describe_reqwest_error(stage: &str, error: &reqwest::Error) -> String {
    describe_transport_error(
        stage,
        error.is_timeout(),
        error.is_connect(),
        error.status().map(|status| status.as_u16()),
        &error.to_string(),
    )
}

pub(super) fn describe_transport_error(
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

pub(super) fn parse_trace_metadata(trace: &str) -> SpeedTestTraceMetadata {
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

pub(super) fn parse_librespeed_public_ip(body: &str) -> Option<String> {
    serde_json::from_str::<LibreSpeedIpLookupResponse>(body)
        .ok()
        .and_then(|response| response.processed_string)
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

pub(super) fn resolve_speed_test_route_fit(
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

pub(super) fn is_preferred_colo(target: SpeedTestTarget, colo: &str) -> bool {
    target
        .preferred_colos
        .iter()
        .any(|candidate| *candidate == colo)
}

pub(super) fn resolve_speed_test_server_label(
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
