use super::{describe_reqwest_error, emit_progress};
use crate::speed_test_targets::{SpeedTestBackendKind, SpeedTestTarget};
use futures_util::{stream, StreamExt};
use reqwest::Client;
use std::time::{Duration, Instant};
use tauri::AppHandle;

const EMIT_INTERVAL: Duration = Duration::from_millis(180);
pub(super) const MIN_DOWNLOAD_STAGE_TIMEOUT: Duration = Duration::from_secs(25);
pub(super) const MAX_DOWNLOAD_STAGE_TIMEOUT: Duration = Duration::from_secs(90);
pub(super) const MIN_UPLOAD_STAGE_TIMEOUT: Duration = Duration::from_secs(15);
pub(super) const MAX_UPLOAD_STAGE_TIMEOUT: Duration = Duration::from_secs(75);
const ASSUMED_MIN_DOWNLOAD_MBPS: f64 = 3.0;
const ASSUMED_MIN_UPLOAD_MBPS: f64 = 1.25;
const TARGET_TRANSFER_DURATION_SECS: f64 = 5.0;
const TARGET_TRANSFER_DURATION_MIN_SECS: f64 = 4.0;
const TARGET_TRANSFER_DURATION_MAX_SECS: f64 = 6.0;
const CALIBRATION_DOWNLOAD_FLOOR_MB: u32 = 2;
pub(super) const MEBIBYTE: usize = 1024 * 1024;
const UPLOAD_CHUNK_BYTES: usize = 64 * 1024;
static UPLOAD_CHUNK_BUFFER: [u8; UPLOAD_CHUNK_BYTES] = [0u8; UPLOAD_CHUNK_BYTES];

#[derive(Debug, Clone, PartialEq)]
pub(super) struct ThroughputMeasurement {
    pub(super) mbps: f64,
    pub(super) bytes_transferred: usize,
    pub(super) elapsed_ms: u64,
}

pub(super) async fn measure_download(
    client: &Client,
    app: Option<&AppHandle>,
    target: SpeedTestTarget,
    requested_download_mb: Option<u32>,
) -> Result<ThroughputMeasurement, String> {
    let requested_download_bytes =
        mebibytes_to_bytes(sanitize_download_mb(requested_download_mb, target));

    if requested_download_mb.is_some() {
        emit_progress(
            app,
            "download",
            24.0,
            0.0,
            format!(
                "Downloading ~{} MB test payload...",
                requested_download_bytes / MEBIBYTE,
            ),
        )?;

        return run_download_attempt(client, app, target, requested_download_bytes).await;
    }

    let calibration_bytes = choose_calibration_download_bytes(target);
    emit_progress(
        app,
        "download",
        24.0,
        0.0,
        format!(
            "Calibrating adaptive download payload with ~{} MB sample...",
            calibration_bytes / MEBIBYTE,
        ),
    )?;

    let calibration_measurement =
        run_download_attempt(client, None, target, calibration_bytes).await?;
    if is_transfer_duration_in_target_window(Duration::from_millis(
        calibration_measurement.elapsed_ms,
    )) {
        emit_progress(
            app,
            "download",
            68.0,
            calibration_measurement.mbps,
            format!(
                "Download stage completed within the target accuracy window using {} MB.",
                bytes_to_mebibytes(calibration_measurement.bytes_transferred),
            ),
        )?;

        return Ok(calibration_measurement);
    }

    let adaptive_download_bytes = estimate_adaptive_download_bytes(target, &calibration_measurement);
    if adaptive_download_bytes == calibration_bytes {
        emit_progress(
            app,
            "download",
            68.0,
            calibration_measurement.mbps,
            format!(
                "Download stage used the profile guardrail at {} MB after calibration.",
                bytes_to_mebibytes(calibration_measurement.bytes_transferred),
            ),
        )?;

        return Ok(calibration_measurement);
    }

    emit_progress(
        app,
        "download",
        24.0,
        calibration_measurement.mbps,
        format!(
            "Refining download payload to ~{} MB for a stable 4-6s accuracy window...",
            adaptive_download_bytes / MEBIBYTE,
        ),
    )?;

    run_download_attempt(client, app, target, adaptive_download_bytes).await
}

async fn run_download_attempt(
    client: &Client,
    app: Option<&AppHandle>,
    target: SpeedTestTarget,
    download_bytes: usize,
) -> Result<ThroughputMeasurement, String> {
    let url = build_download_request_url(target, download_bytes);
    let response = client
        .get(&url)
        .timeout(download_stage_timeout(download_bytes))
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

        if app.is_some() && last_emit.elapsed() >= EMIT_INTERVAL {
            let elapsed = started.elapsed().as_secs_f64().max(0.001);
            let current_speed_mbps = bytes_to_mbps(total_bytes, elapsed);
            let percent =
                24.0 + ((total_bytes as f64 / download_bytes as f64).clamp(0.0, 1.0) * 44.0);
            if let Some(app_handle) = app {
                emit_progress(
                    Some(app_handle),
                    "download",
                    percent,
                    current_speed_mbps,
                    format!(
                        "Downloading test payload... {} / {} MB",
                        total_bytes / MEBIBYTE,
                        download_bytes / MEBIBYTE,
                    ),
                )?;
            }
            last_emit = Instant::now();
        }
    }

    ensure_bytes_transferred("Download test", total_bytes)?;
    let elapsed = started.elapsed();
    let measurement = ThroughputMeasurement {
        mbps: bytes_to_mbps(total_bytes, elapsed.as_secs_f64().max(0.001)),
        bytes_transferred: total_bytes,
        elapsed_ms: milliseconds_from_duration(elapsed),
    };

    if let Some(app_handle) = app {
        emit_progress(
            Some(app_handle),
            "download",
            68.0,
            measurement.mbps,
            format!(
                "Download stage complete using {} MB payload.",
                bytes_to_mebibytes(measurement.bytes_transferred),
            ),
        )?;
    }

    Ok(measurement)
}

fn create_upload_body_stream(
    upload_bytes: usize,
    started: Instant,
    app: Option<AppHandle>,
) -> reqwest::Body {
    let chunk_stream = stream::unfold(
        (0usize, Instant::now(), app),
        move |(sent_bytes, mut last_emit, app_handle)| async move {
            if sent_bytes >= upload_bytes {
                return None;
            }

            let next_chunk_len = (upload_bytes - sent_bytes).min(UPLOAD_CHUNK_BYTES);
            let next_sent = sent_bytes + next_chunk_len;

            if app_handle.is_some() && last_emit.elapsed() >= EMIT_INTERVAL {
                let elapsed = started.elapsed().as_secs_f64().max(0.001);
                let current_speed_mbps = bytes_to_mbps(next_sent, elapsed);
                let percent =
                    72.0 + ((next_sent as f64 / upload_bytes as f64).clamp(0.0, 1.0) * 24.0);
                let _ = emit_progress(
                    app_handle.as_ref(),
                    "upload",
                    percent,
                    current_speed_mbps,
                    format!(
                        "Uploading test payload... {} / {} MB",
                        next_sent / MEBIBYTE,
                        upload_bytes / MEBIBYTE,
                    ),
                );
                last_emit = Instant::now();
            }

            Some((
                Ok::<&'static [u8], std::io::Error>(&UPLOAD_CHUNK_BUFFER[..next_chunk_len]),
                (next_sent, last_emit, app_handle),
            ))
        },
    );

    reqwest::Body::wrap_stream(chunk_stream)
}

pub(super) async fn measure_upload(
    client: &Client,
    app: Option<&AppHandle>,
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

    let started = Instant::now();
    let body = create_upload_body_stream(upload_bytes, started, app.cloned());
    build_upload_request(
        client,
        target,
        body,
        upload_bytes,
        upload_stage_timeout(upload_bytes),
    )
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

pub(super) fn build_download_request_url(target: SpeedTestTarget, download_bytes: usize) -> String {
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
    body: reqwest::Body,
    payload_len: usize,
    stage_timeout: Duration,
) -> reqwest::RequestBuilder {
    match target.backend_kind {
        SpeedTestBackendKind::CloudflareAutoEdge => client
            .post(format!("{}?bytes={payload_len}", target.upload_url))
            .header(reqwest::header::CONTENT_LENGTH, payload_len.to_string())
            .timeout(stage_timeout)
            .body(body),
        SpeedTestBackendKind::LibreSpeedRegional => client
            .post(target.upload_url)
            .header(reqwest::header::CONTENT_LENGTH, payload_len.to_string())
            .timeout(stage_timeout)
            .body(body),
    }
}

pub(super) fn sanitize_download_mb(value: Option<u32>, target: SpeedTestTarget) -> u32 {
    value
        .unwrap_or(target.default_download_mb)
        .clamp(target.min_download_mb, target.max_download_mb)
}

fn mebibytes_to_bytes(mebibytes: u32) -> usize {
    mebibytes as usize * MEBIBYTE
}

pub(super) fn choose_calibration_download_bytes(target: SpeedTestTarget) -> usize {
    let calibration_mb = target
        .default_download_mb
        .min(target.min_download_mb.max(CALIBRATION_DOWNLOAD_FLOOR_MB));
    mebibytes_to_bytes(calibration_mb)
}

pub(super) fn is_transfer_duration_in_target_window(duration: Duration) -> bool {
    let elapsed_seconds = duration.as_secs_f64();
    elapsed_seconds >= TARGET_TRANSFER_DURATION_MIN_SECS
        && elapsed_seconds <= TARGET_TRANSFER_DURATION_MAX_SECS
}

pub(super) fn estimate_adaptive_download_bytes(
    target: SpeedTestTarget,
    measurement: &ThroughputMeasurement,
) -> usize {
    let min_download_bytes = mebibytes_to_bytes(target.min_download_mb);
    let max_download_bytes = mebibytes_to_bytes(target.max_download_mb);

    if measurement.mbps <= 0.0 || measurement.elapsed_ms == 0 {
        return measurement
            .bytes_transferred
            .max(min_download_bytes)
            .min(max_download_bytes);
    }

    let estimated_bytes = round_bytes_to_mebibytes(bytes_from_mbps_for_duration(
        measurement.mbps,
        TARGET_TRANSFER_DURATION_SECS,
    ));

    estimated_bytes.clamp(min_download_bytes, max_download_bytes)
}

pub(super) fn resolve_upload_bytes(target: SpeedTestTarget, download_bytes: usize) -> usize {
    (download_bytes / 2)
        .max(target.min_upload_bytes)
        .min(target.max_upload_bytes)
}

pub(super) fn download_stage_timeout(download_bytes: usize) -> Duration {
    transfer_stage_timeout(
        download_bytes,
        ASSUMED_MIN_DOWNLOAD_MBPS,
        MIN_DOWNLOAD_STAGE_TIMEOUT,
        MAX_DOWNLOAD_STAGE_TIMEOUT,
        6.0,
    )
}

pub(super) fn upload_stage_timeout(upload_bytes: usize) -> Duration {
    transfer_stage_timeout(
        upload_bytes,
        ASSUMED_MIN_UPLOAD_MBPS,
        MIN_UPLOAD_STAGE_TIMEOUT,
        MAX_UPLOAD_STAGE_TIMEOUT,
        5.0,
    )
}

fn transfer_stage_timeout(
    transfer_bytes: usize,
    assumed_min_mbps: f64,
    min_timeout: Duration,
    max_timeout: Duration,
    safety_margin_secs: f64,
) -> Duration {
    if transfer_bytes == 0 || assumed_min_mbps <= 0.0 {
        return min_timeout;
    }

    let estimated_seconds = ((transfer_bytes as f64 * 8.0) / 1_000_000.0 / assumed_min_mbps)
        + safety_margin_secs;

    Duration::from_secs_f64(
        estimated_seconds.clamp(min_timeout.as_secs_f64(), max_timeout.as_secs_f64()),
    )
}

pub(super) fn ensure_bytes_transferred(stage: &str, total_bytes: usize) -> Result<(), String> {
    if total_bytes == 0 {
        return Err(format!(
            "{stage} returned no payload bytes. Check connectivity and try again."
        ));
    }

    Ok(())
}

fn bytes_to_mbps(total_bytes: usize, elapsed_seconds: f64) -> f64 {
    if total_bytes == 0 || elapsed_seconds <= 0.0 {
        return 0.0;
    }
    (total_bytes as f64 * 8.0) / elapsed_seconds / 1_000_000.0
}

fn bytes_from_mbps_for_duration(mbps: f64, duration_seconds: f64) -> usize {
    if mbps <= 0.0 || duration_seconds <= 0.0 {
        return 0;
    }

    ((mbps * 1_000_000.0 / 8.0) * duration_seconds).round() as usize
}

fn milliseconds_from_duration(duration: Duration) -> u64 {
    duration.as_millis().min(u64::MAX as u128) as u64
}

fn bytes_to_mebibytes(total_bytes: usize) -> usize {
    (total_bytes / MEBIBYTE).max(1)
}

fn round_bytes_to_mebibytes(total_bytes: usize) -> usize {
    if total_bytes == 0 {
        return MEBIBYTE;
    }

    (((total_bytes as f64) / MEBIBYTE as f64).round() as usize).max(1) * MEBIBYTE
}
