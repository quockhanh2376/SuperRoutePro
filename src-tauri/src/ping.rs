use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::net::{Ipv4Addr, SocketAddr, ToSocketAddrs};
use std::sync::{Arc, Mutex};
use std::thread;

#[cfg(not(target_os = "windows"))]
use crate::process_exec::{run_process_blocking, DEFAULT_CMD_TIMEOUT_SECS};
#[cfg(not(target_os = "windows"))]
use std::time::Duration;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PingResult {
    pub success: bool,
    pub latency_ms: u32,
    pub output: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct FpingHostResult {
    pub target: String,
    pub success: bool,
    pub latency_ms: u32,
    pub output: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct FpingScanResult {
    pub sent: u32,
    pub received: u32,
    pub loss_percent: f32,
    pub min_ms: u32,
    pub avg_ms: u32,
    pub max_ms: u32,
    pub hosts: Vec<FpingHostResult>,
}

fn resolve_ipv4_target(target: &str) -> Result<Ipv4Addr, String> {
    let trimmed = target.trim();
    if trimmed.is_empty() {
        return Err("Target is empty".to_string());
    }

    if let Ok(ip) = trimmed.parse::<Ipv4Addr>() {
        return Ok(ip);
    }

    let addr_text = format!("{trimmed}:0");
    let addresses = addr_text
        .to_socket_addrs()
        .map_err(|error| format!("DNS resolution failed: {error}"))?;

    addresses
        .filter_map(|address| match address {
            SocketAddr::V4(v4) => Some(*v4.ip()),
            SocketAddr::V6(_) => None,
        })
        .next()
        .ok_or_else(|| "No IPv4 address resolved for target.".to_string())
}

#[cfg(target_os = "windows")]
mod native_icmp {
    use std::ffi::c_void;
    use std::mem;
    use std::ptr;

    use windows_sys::Win32::Foundation::{HANDLE, INVALID_HANDLE_VALUE};

    #[repr(C)]
    struct IpOptionInformation {
        ttl: u8,
        tos: u8,
        flags: u8,
        options_size: u8,
        options_data: *mut u8,
    }

    #[repr(C)]
    struct IcmpEchoReply {
        address: u32,
        status: u32,
        round_trip_time: u32,
        data_size: u16,
        reserved: u16,
        data: *mut c_void,
        options: IpOptionInformation,
    }

    #[link(name = "iphlpapi")]
    extern "system" {
        fn IcmpCreateFile() -> HANDLE;
        fn IcmpCloseHandle(handle: HANDLE) -> i32;
        fn IcmpSendEcho(
            handle: HANDLE,
            destination_address: u32,
            request_data: *const c_void,
            request_size: u16,
            request_options: *const c_void,
            reply_buffer: *mut c_void,
            reply_size: u32,
            timeout: u32,
        ) -> u32;
    }

    pub fn send_echo(ip: std::net::Ipv4Addr, timeout_ms: u32) -> Result<(u32, u8), String> {
        let handle = unsafe { IcmpCreateFile() };
        if handle.is_null() || handle == INVALID_HANDLE_VALUE {
            return Err("IcmpCreateFile failed".to_string());
        }

        let payload = b"superroute-pro";
        let mut reply_buffer = vec![0u8; mem::size_of::<IcmpEchoReply>() + payload.len() + 16];
        let ip_addr = u32::from_be_bytes(ip.octets());

        let reply_count = unsafe {
            IcmpSendEcho(
                handle,
                ip_addr,
                payload.as_ptr() as *const c_void,
                payload.len() as u16,
                ptr::null(),
                reply_buffer.as_mut_ptr() as *mut c_void,
                reply_buffer.len() as u32,
                timeout_ms,
            )
        };

        unsafe {
            let _ = IcmpCloseHandle(handle);
        }

        if reply_count == 0 {
            return Err("Request timed out".to_string());
        }

        let reply = unsafe { &*(reply_buffer.as_ptr() as *const IcmpEchoReply) };
        if reply.status != 0 {
            return Err(format!("ICMP error status {}", reply.status));
        }

        Ok((reply.round_trip_time, reply.options.ttl))
    }
}

#[cfg(target_os = "windows")]
fn ping_once_blocking(target: &str, timeout_ms: u32) -> Result<(u32, String), String> {
    let ip = resolve_ipv4_target(target)?;
    match native_icmp::send_echo(ip, timeout_ms) {
        Ok((latency_ms, ttl)) => Ok((
            latency_ms,
            format!(
                "Reply from {}: bytes=32 time={}ms TTL={}",
                ip, latency_ms, ttl
            ),
        )),
        Err(error) => Err(error),
    }
}

#[cfg(not(target_os = "windows"))]
fn ping_once_blocking(target: &str, timeout_ms: u32) -> Result<(u32, String), String> {
    let output = run_process_blocking(
        "ping",
        &["-c", "1", "-W", &(timeout_ms / 1000).max(1).to_string(), target],
        Duration::from_secs(DEFAULT_CMD_TIMEOUT_SECS),
    )?;
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    if output.status.success() {
        Ok((1, stdout))
    } else {
        Err(stdout)
    }
}

fn run_ping_series(target: &str, count: u32, timeout_ms: u32) -> PingResult {
    let mut lines = vec![format!("Pinging {target} with native ICMP...")];
    let mut successes = 0u32;
    let mut total_latency = 0u32;
    let mut last_latency = 0u32;

    for _ in 0..count.max(1) {
        match ping_once_blocking(target, timeout_ms) {
            Ok((latency_ms, line)) => {
                successes += 1;
                total_latency += latency_ms;
                last_latency = latency_ms;
                lines.push(line);
            }
            Err(error) => {
                lines.push(format!("Request failed for {target}: {error}"));
            }
        }
    }

    let latency_ms = if successes > 0 {
        total_latency / successes
    } else {
        last_latency
    };

    lines.push(String::new());
    lines.push(format!(
        "Summary: sent={} received={} lost={}",
        count.max(1),
        successes,
        count.max(1).saturating_sub(successes)
    ));

    PingResult {
        success: successes > 0,
        latency_ms,
        output: lines.join("\n"),
    }
}

fn ping_once_target(target: String, timeout_ms: u32) -> FpingHostResult {
    match ping_once_blocking(&target, timeout_ms) {
        Ok((latency_ms, output)) => FpingHostResult {
            target,
            success: true,
            latency_ms,
            output,
        },
        Err(error) => FpingHostResult {
            target,
            success: false,
            latency_ms: 0,
            output: format!("Ping failed: {error}"),
        },
    }
}

#[tauri::command]
pub async fn ping_host(target: String, count: Option<u32>) -> Result<PingResult, String> {
    let target_owned = target.trim().to_string();
    let count = count.unwrap_or(1).clamp(1, 8);
    tauri::async_runtime::spawn_blocking(move || run_ping_series(&target_owned, count, 2_000))
        .await
        .map_err(|error| format!("Ping task join error: {error}"))
}

#[tauri::command]
pub async fn fping_scan(
    targets: Vec<String>,
    timeout_ms: Option<u32>,
) -> Result<FpingScanResult, String> {
    let timeout = timeout_ms.unwrap_or(1200).clamp(200, 10_000);

    let clean_targets: Vec<String> = targets
        .into_iter()
        .map(|target| target.trim().to_string())
        .filter(|target| !target.is_empty())
        .take(512)
        .collect();

    if clean_targets.is_empty() {
        return Err("No targets provided".to_string());
    }

    let cpu_workers = thread::available_parallelism()
        .map(|count| count.get())
        .unwrap_or(4);
    let worker_count = clean_targets.len().min(64).min(cpu_workers.max(1));

    let queue = Arc::new(Mutex::new(
        clean_targets
            .iter()
            .cloned()
            .enumerate()
            .collect::<VecDeque<(usize, String)>>(),
    ));
    let results: Arc<Mutex<Vec<(usize, FpingHostResult)>>> =
        Arc::new(Mutex::new(Vec::with_capacity(clean_targets.len())));

    let mut workers = Vec::with_capacity(worker_count);
    for _ in 0..worker_count {
        let queue_ref = Arc::clone(&queue);
        let results_ref = Arc::clone(&results);
        workers.push(thread::spawn(move || loop {
            let job = {
                let mut guard = match queue_ref.lock() {
                    Ok(guard) => guard,
                    Err(poisoned) => poisoned.into_inner(),
                };
                guard.pop_front()
            };

            let (index, target) = match job {
                Some(job) => job,
                None => break,
            };

            let result = ping_once_target(target, timeout);
            let mut guard = match results_ref.lock() {
                Ok(guard) => guard,
                Err(poisoned) => poisoned.into_inner(),
            };
            guard.push((index, result));
        }));
    }

    for worker in workers {
        let _ = worker.join();
    }

    let mut ordered = match results.lock() {
        Ok(guard) => guard.clone(),
        Err(poisoned) => poisoned.into_inner().clone(),
    };
    ordered.sort_by_key(|(index, _)| *index);
    let hosts: Vec<FpingHostResult> = ordered
        .into_iter()
        .map(|(_, result)| result)
        .collect();

    let sent = hosts.len() as u32;
    let received = hosts.iter().filter(|host| host.success).count() as u32;
    let loss_percent = if sent == 0 {
        100.0
    } else {
        ((sent - received) as f32 / sent as f32) * 100.0
    };

    let live_latencies: Vec<u32> = hosts
        .iter()
        .filter(|host| host.success)
        .map(|host| host.latency_ms)
        .collect();
    let min_ms = live_latencies.iter().min().copied().unwrap_or(0);
    let max_ms = live_latencies.iter().max().copied().unwrap_or(0);
    let avg_ms = if live_latencies.is_empty() {
        0
    } else {
        live_latencies.iter().sum::<u32>() / live_latencies.len() as u32
    };

    Ok(FpingScanResult {
        sent,
        received,
        loss_percent,
        min_ms,
        avg_ms,
        max_ms,
        hosts,
    })
}

#[cfg(test)]
mod tests {
    use super::resolve_ipv4_target;

    #[test]
    fn resolve_ipv4_target_accepts_literal_ipv4() {
        let target = resolve_ipv4_target("8.8.8.8").expect("literal IPv4 should resolve");
        assert_eq!(target.octets(), [8, 8, 8, 8]);
    }
}
