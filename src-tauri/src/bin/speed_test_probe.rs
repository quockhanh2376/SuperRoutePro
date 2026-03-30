use serde::Serialize;
use std::{env, process::ExitCode};
use super_route_pro_lib::{run_speed_test_snapshot, SpeedTestResult};

const DEFAULT_TARGETS: [&str; 4] = ["auto_asia", "auto_au", "jp_kr", "eu"];

#[derive(Debug, Serialize)]
struct ProbeReport {
    target_id: String,
    ok: bool,
    result: Option<SpeedTestResult>,
    error: Option<String>,
}

fn main() -> ExitCode {
    let target_ids = collect_target_ids();
    let mut reports = Vec::with_capacity(target_ids.len());
    let mut had_failure = false;

    for target_id in target_ids {
        eprintln!("Running native speed test probe for {target_id}...");
        match tauri::async_runtime::block_on(run_speed_test_snapshot(None, Some(target_id.as_str()))) {
            Ok(result) => {
                eprintln!(
                    "  ok: {:.1} Mbps down / {:.1} Mbps up, ping {:.1} ms, route {}, edge {}",
                    result.download_mbps,
                    result.upload_mbps,
                    result.ping_ms,
                    result.route_fit,
                    result.resolved_colo.as_deref().unwrap_or("n/a"),
                );
                reports.push(ProbeReport {
                    target_id,
                    ok: true,
                    result: Some(result),
                    error: None,
                });
            }
            Err(error) => {
                eprintln!("  failed: {error}");
                had_failure = true;
                reports.push(ProbeReport {
                    target_id,
                    ok: false,
                    result: None,
                    error: Some(error),
                });
            }
        }
    }

    match serde_json::to_string_pretty(&reports) {
        Ok(output) => println!("{output}"),
        Err(error) => {
            eprintln!("Failed to serialize probe output: {error}");
            return ExitCode::from(1);
        }
    }

    if had_failure {
        ExitCode::from(1)
    } else {
        ExitCode::SUCCESS
    }
}

fn collect_target_ids() -> Vec<String> {
    let cli_targets: Vec<String> = env::args().skip(1).collect();
    if cli_targets.is_empty() {
        return DEFAULT_TARGETS.iter().map(|target_id| (*target_id).to_string()).collect();
    }

    cli_targets
}