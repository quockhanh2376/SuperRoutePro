use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::net::TcpStream;
use std::os::windows::process::CommandExt;
use std::process::Command;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

const CREATE_NO_WINDOW: u32 = 0x08000000;

// ======================== DATA TYPES ========================

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct NetworkInterface {
    pub index: String,
    pub ip: String,
    pub gateway: String,
    pub description: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RouteEntry {
    pub destination: String,
    pub netmask: String,
    pub gateway: String,
    pub metric: String,
    pub interface_index: String,
}

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

#[derive(Debug, Serialize, Deserialize)]
pub struct CommandResult {
    pub success: bool,
    pub output: String,
}

// ======================== HELPERS ========================

fn run_powershell(script: &str) -> Result<String, String> {
    let output = Command::new("powershell")
        .args(["-NoProfile", "-NonInteractive", "-Command", script])
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .map_err(|e| format!("Failed to run PowerShell: {}", e))?;

    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        // Some commands write to stdout even on "failure"
        if !stdout.is_empty() {
            Ok(stdout)
        } else {
            Err(stderr)
        }
    }
}

fn run_cmd(program: &str, args: &[&str]) -> Result<String, String> {
    let output = Command::new(program)
        .args(args)
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .map_err(|e| format!("Failed to run {}: {}", program, e))?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    if output.status.success() {
        Ok(stdout)
    } else if !stdout.is_empty() {
        Ok(format!("{}\n{}", stdout, stderr))
    } else {
        Err(stderr)
    }
}

fn prefix_to_mask(prefix: u32) -> String {
    if prefix > 32 {
        return "255.255.255.255".to_string();
    }
    let mask: u32 = if prefix == 0 {
        0
    } else {
        0xFFFFFFFF << (32 - prefix)
    };
    format!(
        "{}.{}.{}.{}",
        (mask >> 24) & 0xFF,
        (mask >> 16) & 0xFF,
        (mask >> 8) & 0xFF,
        mask & 0xFF
    )
}

fn parse_ping_latency(stdout: &str, elapsed_ms: u32) -> u32 {
    if stdout.contains("time=") {
        stdout
            .split("time=")
            .nth(1)
            .and_then(|s| s.split("ms").next())
            .and_then(|s| s.trim().parse::<u32>().ok())
            .unwrap_or(elapsed_ms)
    } else if stdout.contains("time<") {
        1
    } else {
        0
    }
}

fn ping_once_target(target: String, timeout_ms: &str) -> FpingHostResult {
    let start = Instant::now();
    let output = Command::new("ping")
        .args(["-n", "1", "-w", timeout_ms, &target])
        .creation_flags(CREATE_NO_WINDOW)
        .output();

    match output {
        Ok(out) => {
            let stdout = String::from_utf8_lossy(&out.stdout).to_string();
            let elapsed = start.elapsed().as_millis() as u32;
            let latency = parse_ping_latency(&stdout, elapsed);
            FpingHostResult {
                target,
                success: stdout.contains("Reply from") || stdout.contains("time="),
                latency_ms: latency,
                output: stdout,
            }
        }
        Err(e) => FpingHostResult {
            target,
            success: false,
            latency_ms: 0,
            output: format!("Ping failed: {}", e),
        },
    }
}

// ======================== TAURI COMMANDS ========================

/// Get list of active network interfaces (NICs)
#[tauri::command]
pub async fn get_network_interfaces(active_only: bool) -> Result<Vec<NetworkInterface>, String> {
    let ps_script = r#"
        Get-WmiObject Win32_NetworkAdapterConfiguration |
        Where-Object { $_.InterfaceIndex -ne $null } |
        Select-Object InterfaceIndex, Description, IPAddress, DefaultIPGateway |
        ConvertTo-Json -Compress
    "#;

    let output = run_powershell(ps_script)?;
    let data: serde_json::Value =
        serde_json::from_str(&output).map_err(|e| format!("JSON parse error: {}", e))?;

    let items = match &data {
        serde_json::Value::Array(arr) => arr.clone(),
        obj @ serde_json::Value::Object(_) => vec![obj.clone()],
        _ => return Ok(vec![]),
    };

    let blacklist = [
        "virtual", "vmware", "vbox", "loopback", "wintun", "kernel",
        "miniport", "wi-fi direct", "tap-", "pseudo", "ethernet adapter v",
    ];

    let mut interfaces: Vec<NetworkInterface> = Vec::new();

    for item in &items {
        let desc = item["Description"].as_str().unwrap_or("").to_string();
        let desc_lower = desc.to_lowercase();

        if blacklist.iter().any(|b| desc_lower.contains(b)) {
            continue;
        }

        let ip = match &item["IPAddress"] {
            serde_json::Value::Array(arr) => {
                arr.first()
                    .and_then(|v| v.as_str())
                    .unwrap_or("0.0.0.0")
                    .to_string()
            }
            serde_json::Value::String(s) => s.clone(),
            _ => "0.0.0.0".to_string(),
        };

        if active_only && (ip.is_empty() || ip == "0.0.0.0") {
            continue;
        }

        let gateway = match &item["DefaultIPGateway"] {
            serde_json::Value::Array(arr) => {
                arr.first()
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string()
            }
            serde_json::Value::String(s) => s.clone(),
            _ => String::new(),
        };

        let index = match &item["InterfaceIndex"] {
            serde_json::Value::Number(n) => n.to_string(),
            serde_json::Value::String(s) => s.clone(),
            _ => String::new(),
        };

        interfaces.push(NetworkInterface {
            index,
            ip,
            gateway,
            description: desc,
        });
    }

    Ok(interfaces)
}

/// Get IPv4 routing table
#[tauri::command]
pub async fn get_routing_table() -> Result<Vec<RouteEntry>, String> {
    let ps_script = r#"
        Get-NetRoute -AddressFamily IPv4 |
        Select-Object DestinationPrefix, NextHop, RouteMetric, InterfaceIndex |
        ConvertTo-Json -Compress
    "#;

    let output = run_powershell(ps_script)?;
    let data: serde_json::Value =
        serde_json::from_str(&output).map_err(|e| format!("JSON parse error: {}", e))?;

    let items = match &data {
        serde_json::Value::Array(arr) => arr.clone(),
        obj @ serde_json::Value::Object(_) => vec![obj.clone()],
        _ => return Ok(vec![]),
    };

    let mut routes: Vec<RouteEntry> = Vec::new();

    for item in &items {
        let prefix = item["DestinationPrefix"]
            .as_str()
            .unwrap_or("")
            .to_string();
        let next_hop = item["NextHop"].as_str().unwrap_or("").to_string();

        let metric = match &item["RouteMetric"] {
            serde_json::Value::Number(n) => n.to_string(),
            serde_json::Value::String(s) => s.clone(),
            _ => "0".to_string(),
        };

        let if_index = match &item["InterfaceIndex"] {
            serde_json::Value::Number(n) => n.to_string(),
            serde_json::Value::String(s) => s.clone(),
            _ => "0".to_string(),
        };

        // Split prefix into destination and mask
        let (dest, mask) = if let Some(pos) = prefix.find('/') {
            let ip = prefix[..pos].to_string();
            let prefix_len: u32 = prefix[pos + 1..].parse().unwrap_or(32);
            (ip, prefix_to_mask(prefix_len))
        } else {
            (prefix, "255.255.255.255".to_string())
        };

        routes.push(RouteEntry {
            destination: dest,
            netmask: mask,
            gateway: next_hop,
            metric,
            interface_index: if_index,
        });
    }

    Ok(routes)
}

/// Add a persistent route
#[tauri::command]
pub async fn add_route(
    destination: String,
    mask: String,
    gateway: String,
    metric: String,
    interface_index: Option<String>,
) -> Result<CommandResult, String> {
    // First try to delete existing route
    let _ = run_cmd("route", &["delete", &destination, "mask", &mask]);

    let mut args = vec![
        "route",
        "-p",
        "add",
        &destination,
        "mask",
        &mask,
        &gateway,
        "metric",
        &metric,
    ];

    let if_idx;
    if let Some(ref idx) = interface_index {
        if !idx.is_empty() {
            if_idx = idx.clone();
            args.push("if");
            args.push(&if_idx);
        }
    }

    let result = run_cmd(args[0], &args[1..])?;

    Ok(CommandResult {
        success: true,
        output: result,
    })
}

/// Delete a route
#[tauri::command]
pub async fn delete_route(destination: String, mask: String) -> Result<CommandResult, String> {
    let result = run_cmd("route", &["delete", &destination, "mask", &mask])?;
    Ok(CommandResult {
        success: true,
        output: result,
    })
}

/// Flush all routes
#[tauri::command]
pub async fn flush_routes() -> Result<CommandResult, String> {
    let result = run_cmd("route", &["-f"])?;
    Ok(CommandResult {
        success: true,
        output: result,
    })
}

/// Set a NIC as default internet gateway
#[tauri::command]
pub async fn set_default_gateway(
    gateway: String,
    interface_index: String,
) -> Result<CommandResult, String> {
    // Raise metric of all existing default routes
    let _ = run_powershell("Set-NetRoute -DestinationPrefix 0.0.0.0/0 -RouteMetric 500");

    // Delete existing default route
    let _ = run_cmd("route", &["delete", "0.0.0.0"]);

    // Add new default route with low metric
    let result = run_cmd(
        "route",
        &[
            "-p",
            "add",
            "0.0.0.0",
            "mask",
            "0.0.0.0",
            &gateway,
            "metric",
            "1",
            "if",
            &interface_index,
        ],
    )?;

    Ok(CommandResult {
        success: true,
        output: result,
    })
}

/// Run a network fix command (flush DNS, renew IP, etc.)
#[tauri::command]
pub async fn run_network_command(command: String) -> Result<CommandResult, String> {
    // Whitelist of allowed commands for security
    let allowed_prefixes = [
        "ipconfig",
        "ipconfig /displaydns",
        "tracert",
        "nslookup",
        "netsh wlan show interface",
        "netsh winhttp reset proxy",
        "netsh int ip reset",
        "netsh winsock reset",
        "netsh interface ip delete arpcache",
        "netsh advfirewall reset",
        "powershell -noprofile -command get-netadapter",
        "powershell -noprofile -command test-netconnection",
    ];

    let cmd_lower = command.to_lowercase();
    if !allowed_prefixes
        .iter()
        .any(|prefix| cmd_lower.starts_with(prefix))
    {
        return Err("Command not allowed".to_string());
    }

    let output = Command::new("cmd")
        .args(["/C", &command])
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .map_err(|e| format!("Failed to run command: {}", e))?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    Ok(CommandResult {
        success: output.status.success(),
        output: if stdout.is_empty() { stderr } else { stdout },
    })
}

/// Ping a host and return latency
#[tauri::command]
pub async fn ping_host(target: String, count: Option<u32>) -> Result<PingResult, String> {
    let n = count.unwrap_or(1).to_string();

    let start = Instant::now();
    let output = Command::new("ping")
        .args(["-n", &n, "-w", "2000", &target])
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .map_err(|e| format!("Ping failed: {}", e))?;

    let elapsed = start.elapsed().as_millis() as u32;
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();

    let latency = parse_ping_latency(&stdout, elapsed);

    Ok(PingResult {
        success: stdout.contains("Reply from") || stdout.contains("time="),
        latency_ms: latency,
        output: stdout,
    })
}

/// fping-like scan over multiple targets (parallel ping once per host)
#[tauri::command]
pub async fn fping_scan(
    targets: Vec<String>,
    timeout_ms: Option<u32>,
) -> Result<FpingScanResult, String> {
    let timeout = timeout_ms.unwrap_or(1200).clamp(200, 10_000).to_string();

    let clean_targets: Vec<String> = targets
        .into_iter()
        .map(|t| t.trim().to_string())
        .filter(|t| !t.is_empty())
        .take(128)
        .collect();

    if clean_targets.is_empty() {
        return Err("No targets provided".to_string());
    }

    let cpu_workers = thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(4);
    let max_parallel = 24usize;
    let worker_count = clean_targets
        .len()
        .min(max_parallel)
        .min(cpu_workers.max(1));

    let queue: Arc<Mutex<VecDeque<(usize, String)>>> = Arc::new(Mutex::new(
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
        let timeout_clone = timeout.clone();
        workers.push(thread::spawn(move || loop {
            let next_job = {
                let mut guard = match queue_ref.lock() {
                    Ok(g) => g,
                    Err(poisoned) => poisoned.into_inner(),
                };
                guard.pop_front()
            };

            let (index, target) = match next_job {
                Some(job) => job,
                None => break,
            };

            let result = ping_once_target(target, &timeout_clone);
            let mut out_guard = match results_ref.lock() {
                Ok(g) => g,
                Err(poisoned) => poisoned.into_inner(),
            };
            out_guard.push((index, result));
        }));
    }

    for worker in workers {
        let _ = worker.join();
    }

    let mut ordered_results = {
        let guard = match results.lock() {
            Ok(g) => g,
            Err(poisoned) => poisoned.into_inner(),
        };
        guard.clone()
    };
    ordered_results.sort_by_key(|(index, _)| *index);
    let hosts: Vec<FpingHostResult> = ordered_results
        .into_iter()
        .map(|(_, host_result)| host_result)
        .collect();

    let sent = hosts.len() as u32;
    let received = hosts.iter().filter(|h| h.success).count() as u32;
    let loss_percent = if sent == 0 {
        100.0
    } else {
        ((sent - received) as f32 / sent as f32) * 100.0
    };

    let mut min_ms = 0;
    let mut max_ms = 0;
    let mut avg_ms = 0;
    let alive_latencies: Vec<u32> = hosts
        .iter()
        .filter(|h| h.success)
        .map(|h| h.latency_ms)
        .collect();

    if !alive_latencies.is_empty() {
        min_ms = *alive_latencies.iter().min().unwrap_or(&0);
        max_ms = *alive_latencies.iter().max().unwrap_or(&0);
        avg_ms = alive_latencies.iter().sum::<u32>() / alive_latencies.len() as u32;
    }

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

/// Check internet connectivity
#[tauri::command]
pub async fn check_internet() -> Result<bool, String> {
    match TcpStream::connect_timeout(
        &"8.8.8.8:53".parse().unwrap(),
        Duration::from_secs(3),
    ) {
        Ok(_) => Ok(true),
        Err(_) => Ok(false),
    }
}
