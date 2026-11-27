use super::describe_reqwest_error;
use crate::speed_test_targets::{SpeedTestBackendKind, SpeedTestTarget};
use reqwest::Client;
use std::time::{Duration, Instant};

const LATENCY_SAMPLES: usize = 8;
const LATENCY_WARMUP_SAMPLES: usize = 1;
const MIN_SUCCESSFUL_LATENCY_SAMPLES: usize = 5;
const LATENCY_OUTLIER_MAD_MULTIPLIER: f64 = 3.5;
pub(super) const LATENCY_STAGE_TIMEOUT: Duration = Duration::from_secs(6);

#[derive(Debug, Clone, PartialEq)]
pub(super) struct LatencyMeasurement {
    pub(super) attempted_samples: usize,
    pub(super) successful_samples: usize,
    pub(super) stable_samples: usize,
    pub(super) ping_ms: f64,
    pub(super) jitter_ms: f64,
}

pub(super) async fn measure_latency(
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
            .timeout(LATENCY_STAGE_TIMEOUT)
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

pub(super) fn build_latency_request_url(target: SpeedTestTarget, sample: usize) -> String {
    match target.backend_kind {
        SpeedTestBackendKind::CloudflareAutoEdge => {
            format!("{}?bytes=0&sample={sample}", target.download_url)
        }
        SpeedTestBackendKind::LibreSpeedRegional => format!("{}?r={sample}", target.latency_url),
    }
}

pub(super) fn describe_latency_probe_failure(
    sample_count: usize,
    last_error: Option<&str>,
) -> String {
    let mut message = format!(
        "Latency check collected only {sample_count} stable sample(s). At least {MIN_SUCCESSFUL_LATENCY_SAMPLES} successful probes are required."
    );

    if let Some(last_error) = last_error {
        message.push(' ');
        message.push_str(last_error);
    }

    message
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

pub(super) fn build_latency_baseline(points: &[f64]) -> Vec<f64> {
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

pub(super) fn calculate_jitter(points: &[f64]) -> f64 {
    if points.len() < 2 {
        return 0.0;
    }

    let deltas: Vec<f64> = points
        .windows(2)
        .map(|pair| (pair[1] - pair[0]).abs())
        .collect();

    median(&deltas)
}
