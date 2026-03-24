use chrono::Utc;
use futures_util::StreamExt;
use reqwest::Client;
use serde::Serialize;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter};

const SPEED_TEST_PROGRESS_EVENT: &str = "speed-test://progress";
const PROVIDER_NAME: &str = "Cloudflare";
const SERVER_LABEL: &str = "Cloudflare Auto";
const DOWNLOAD_API_URL: &str = "https://speed.cloudflare.com/__down";
const UPLOAD_API_URL: &str = "https://speed.cloudflare.com/__up";
const TRACE_API_URL: &str = "https://speed.cloudflare.com/cdn-cgi/trace";
const DEFAULT_DOWNLOAD_MB: u32 = 24;
const MIN_DOWNLOAD_MB: u32 = 8;
const MAX_DOWNLOAD_MB: u32 = 32;
const DEFAULT_UPLOAD_BYTES: usize = 8 * 1024 * 1024;
const LATENCY_SAMPLES: usize = 6;
const REQUEST_TIMEOUT: Duration = Duration::from_secs(90);
const CONNECT_TIMEOUT: Duration = Duration::from_secs(8);
const EMIT_INTERVAL: Duration = Duration::from_millis(180);

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
    pub provider: String,
    pub server_label: String,
    pub download_mbps: f64,
    pub upload_mbps: f64,
    pub ping_ms: f64,
    pub jitter_ms: f64,
    pub ip: String,
    pub timestamp: String,
}

#[tauri::command]
pub async fn run_speed_test(
    app: AppHandle,
    download_mb: Option<u32>,
) -> Result<SpeedTestResult, String> {
    let download_bytes = sanitize_download_mb(download_mb) as usize * 1024 * 1024;
    let upload_bytes = DEFAULT_UPLOAD_BYTES.min((download_bytes / 2).max(2 * 1024 * 1024));
    let client = build_client()?;

    emit_progress(
        &app,
        "preflight",
        4.0,
        0.0,
        format!("Preparing native speed test via {PROVIDER_NAME}."),
    )?;

    let latency_points = measure_latency(&client).await?;
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

    let download_mbps = measure_download(&client, &app, download_bytes).await?;
    let upload_mbps = measure_upload(&client, &app, upload_bytes).await?;
    let ip = fetch_public_ip(&client)
        .await
        .unwrap_or_else(|_| "Unavailable".to_string());

    emit_progress(
        &app,
        "finalize",
        100.0,
        download_mbps.max(upload_mbps),
        "Speed test finished.".to_string(),
    )?;

    Ok(SpeedTestResult {
        provider: PROVIDER_NAME.to_string(),
        server_label: SERVER_LABEL.to_string(),
        download_mbps,
        upload_mbps,
        ping_ms,
        jitter_ms,
        ip,
        timestamp: Utc::now().to_rfc3339(),
    })
}

fn build_client() -> Result<Client, String> {
    Client::builder()
        .connect_timeout(CONNECT_TIMEOUT)
        .timeout(REQUEST_TIMEOUT)
        .user_agent("SuperRoutePro-SpeedTest/1.0")
        .build()
        .map_err(|error| format!("Could not initialize speed test client: {error}"))
}

async fn measure_latency(client: &Client) -> Result<Vec<f64>, String> {
    let mut points = Vec::with_capacity(LATENCY_SAMPLES);

    for sample in 0..LATENCY_SAMPLES {
        let url = format!("{DOWNLOAD_API_URL}?bytes=0&sample={sample}");
        let started = Instant::now();
        client
            .get(&url)
            .send()
            .await
            .map_err(|error| format!("Latency probe failed: {error}"))?
            .error_for_status()
            .map_err(|error| format!("Latency probe returned an error: {error}"))?;
        points.push(started.elapsed().as_secs_f64() * 1000.0);
    }

    if points.is_empty() {
        return Err("Latency probe returned no samples.".to_string());
    }

    Ok(points)
}

async fn measure_download(
    client: &Client,
    app: &AppHandle,
    download_bytes: usize,
) -> Result<f64, String> {
    emit_progress(
        app,
        "download",
        24.0,
        0.0,
        format!("Downloading ~{} MB test payload...", download_bytes / 1024 / 1024),
    )?;

    let url = format!("{DOWNLOAD_API_URL}?bytes={download_bytes}");
    let response = client
        .get(&url)
        .send()
        .await
        .map_err(|error| format!("Download test failed to start: {error}"))?
        .error_for_status()
        .map_err(|error| format!("Download test returned an error: {error}"))?;

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
            let percent = 24.0 + ((total_bytes as f64 / download_bytes as f64).clamp(0.0, 1.0) * 44.0);
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

    let elapsed = started.elapsed().as_secs_f64().max(0.001);
    Ok(bytes_to_mbps(total_bytes, elapsed))
}

async fn measure_upload(
    client: &Client,
    app: &AppHandle,
    upload_bytes: usize,
) -> Result<f64, String> {
    emit_progress(
        app,
        "upload",
        72.0,
        0.0,
        format!("Uploading ~{} MB validation payload...", upload_bytes / 1024 / 1024),
    )?;

    let payload = vec![0u8; upload_bytes];
    let started = Instant::now();
    client
        .post(format!("{UPLOAD_API_URL}?bytes={upload_bytes}"))
        .body(payload)
        .send()
        .await
        .map_err(|error| format!("Upload test failed to start: {error}"))?
        .error_for_status()
        .map_err(|error| format!("Upload test returned an error: {error}"))?;

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

async fn fetch_public_ip(client: &Client) -> Result<String, String> {
    let trace = client
        .get(TRACE_API_URL)
        .send()
        .await
        .map_err(|error| format!("Could not resolve public IP: {error}"))?
        .error_for_status()
        .map_err(|error| format!("Public IP lookup returned an error: {error}"))?
        .text()
        .await
        .map_err(|error| format!("Could not read public IP response: {error}"))?;

    Ok(parse_trace_ip(&trace).unwrap_or_else(|| "Unavailable".to_string()))
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

fn parse_trace_ip(trace: &str) -> Option<String> {
    trace.lines().find_map(|line| line.strip_prefix("ip=").map(str::to_string))
}

#[cfg(test)]
mod tests {
    use super::{calculate_jitter, parse_trace_ip, sanitize_download_mb};

    #[test]
    fn sanitize_download_mb_clamps_supported_range() {
        assert_eq!(sanitize_download_mb(None), 24);
        assert_eq!(sanitize_download_mb(Some(2)), 8);
        assert_eq!(sanitize_download_mb(Some(48)), 32);
        assert_eq!(sanitize_download_mb(Some(16)), 16);
    }

    #[test]
    fn parse_trace_ip_reads_cloudflare_trace_format() {
        let trace = "fl=29f64\nh=speed.cloudflare.com\nip=203.0.113.7\nts=1711111111.111\n";
        assert_eq!(parse_trace_ip(trace).as_deref(), Some("203.0.113.7"));
    }

    #[test]
    fn calculate_jitter_uses_average_gap_between_samples() {
        let points = [10.0, 14.0, 15.0, 21.0];
        let jitter = calculate_jitter(&points);
        assert!((jitter - 3.6666666667).abs() < 0.0001);
    }
}
