use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet, VecDeque};
use std::fs;
use std::net::TcpStream;
use std::os::windows::process::CommandExt;
use std::process::{Command, Output, Stdio};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

const CREATE_NO_WINDOW: u32 = 0x08000000;
const DEFAULT_CMD_TIMEOUT_SECS: u64 = 30;
const DEFAULT_POWERSHELL_TIMEOUT_SECS: u64 = 45;
const NETWORK_COMMAND_TIMEOUT_SECS: u64 = 90;
const WAN_PERSIST_TASK_NAME: &str = "SuperRoutePro-PersistWAN";
const WAN_PERSIST_DIR: &str = r"C:\ProgramData\SuperRoutePro";
const WAN_PERSIST_SCRIPT_PATH: &str = r"C:\ProgramData\SuperRoutePro\persist-wan.cmd";
const BLOATWARE_CANDIDATES: [(&str, &str); 29] = [
    ("Clipchamp.Clipchamp", "Clipchamp"),
    ("Microsoft.BingNews", "Microsoft News"),
    ("Microsoft.BingWeather", "Microsoft Weather"),
    ("Microsoft.GetHelp", "Get Help"),
    ("Microsoft.Getstarted", "Get Started"),
    ("Microsoft.GamingApp", "Xbox"),
    ("Microsoft.Microsoft3DViewer", "3D Viewer"),
    (
        "Microsoft.MicrosoftOfficeHub",
        "Microsoft 365 (Office Hub)",
    ),
    (
        "Microsoft.MicrosoftSolitaireCollection",
        "Microsoft Solitaire Collection",
    ),
    ("Microsoft.MixedReality.Portal", "Mixed Reality Portal"),
    ("Microsoft.OutlookForWindows", "Outlook for Windows"),
    ("Microsoft.People", "People"),
    ("Microsoft.PowerAutomateDesktop", "Power Automate"),
    ("Microsoft.SkypeApp", "Skype"),
    ("Microsoft.Todos", "Microsoft To Do"),
    ("Microsoft.WindowsAlarms", "Clock"),
    ("microsoft.windowscommunicationsapps", "Mail and Calendar"),
    ("Microsoft.WindowsFeedbackHub", "Feedback Hub"),
    ("Microsoft.WindowsMaps", "Maps"),
    ("Microsoft.Xbox.TCUI", "Xbox TCUI"),
    ("Microsoft.XboxGameOverlay", "Xbox Game Bar Plugin"),
    ("Microsoft.XboxGamingOverlay", "Xbox Game Bar"),
    ("Microsoft.XboxIdentityProvider", "Xbox Identity Provider"),
    ("Microsoft.XboxSpeechToTextOverlay", "Xbox Speech To Text"),
    ("Microsoft.YourPhone", "Phone Link"),
    ("Microsoft.ZuneMusic", "Media Player (Legacy Music)"),
    ("Microsoft.ZuneVideo", "Movies & TV"),
    ("MicrosoftTeams", "Microsoft Teams"),
    ("MicrosoftCorporationII.MicrosoftFamily", "Microsoft Family"),
];

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

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BloatwareItem {
    pub package_name: String,
    pub label: String,
    pub installed: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BatteryReportResult {
    pub html: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BatterySummaryResult {
    pub present: bool,
    pub status: String,
    pub charge_percent: Option<u32>,
    pub design_capacity_mwh: Option<u32>,
    pub full_charge_capacity_mwh: Option<u32>,
    pub health_percent: Option<f32>,
    pub wear_percent: Option<f32>,
    pub cycle_count: Option<u32>,
    pub estimated_runtime_minutes: Option<u32>,
    pub estimated_runtime_full_minutes: Option<u32>,
    pub note: String,
}

// ======================== HELPERS ========================

fn run_process_blocking(
    program: &str,
    args: &[&str],
    timeout: Duration,
) -> Result<Output, String> {
    let mut child = Command::new(program)
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .creation_flags(CREATE_NO_WINDOW)
        .spawn()
        .map_err(|e| format!("Failed to spawn {}: {}", program, e))?;

    let started = Instant::now();
    loop {
        match child
            .try_wait()
            .map_err(|e| format!("Failed waiting for {}: {}", program, e))?
        {
            Some(_) => {
                return child
                    .wait_with_output()
                    .map_err(|e| format!("Failed to collect output for {}: {}", program, e));
            }
            None => {
                if started.elapsed() >= timeout {
                    let _ = child.kill();
                    let _ = child.wait();
                    return Err(format!(
                        "Command timed out after {}s: {} {}",
                        timeout.as_secs(),
                        program,
                        args.join(" ")
                    ));
                }
                thread::sleep(Duration::from_millis(80));
            }
        }
    }
}

fn run_powershell_blocking(script: &str, timeout: Duration) -> Result<String, String> {
    let output = run_process_blocking(
        "powershell",
        &["-NoProfile", "-NonInteractive", "-Command", script],
        timeout,
    )?;

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

fn run_cmd_blocking(program: &str, args: &[&str], timeout: Duration) -> Result<String, String> {
    let output = run_process_blocking(program, args, timeout)?;

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

async fn run_cmd(program: &str, args: &[&str]) -> Result<String, String> {
    let program_owned = program.to_string();
    let args_owned: Vec<String> = args.iter().map(|arg| (*arg).to_string()).collect();
    tauri::async_runtime::spawn_blocking(move || {
        let args_refs: Vec<&str> = args_owned.iter().map(String::as_str).collect();
        run_cmd_blocking(
            &program_owned,
            &args_refs,
            Duration::from_secs(DEFAULT_CMD_TIMEOUT_SECS),
        )
    })
    .await
    .map_err(|err| format!("Command task join error: {}", err))?
}

async fn run_powershell(script: &str) -> Result<String, String> {
    let script_owned = script.to_string();
    tauri::async_runtime::spawn_blocking(move || {
        run_powershell_blocking(
            &script_owned,
            Duration::from_secs(DEFAULT_POWERSHELL_TIMEOUT_SECS),
        )
    })
    .await
    .map_err(|err| format!("PowerShell task join error: {}", err))?
}

async fn run_process(
    program: &str,
    args: &[&str],
    timeout: Duration,
) -> Result<Output, String> {
    let program_owned = program.to_string();
    let args_owned: Vec<String> = args.iter().map(|arg| (*arg).to_string()).collect();
    tauri::async_runtime::spawn_blocking(move || {
        let args_refs: Vec<&str> = args_owned.iter().map(String::as_str).collect();
        run_process_blocking(&program_owned, &args_refs, timeout)
    })
    .await
    .map_err(|err| format!("Process task join error: {}", err))?
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

fn ps_escape_single_quoted(input: &str) -> String {
    input.replace('\'', "''")
}

fn is_task_not_found_error(text: &str) -> bool {
    let lower = text.to_ascii_lowercase();
    lower.contains("cannot find the file specified")
        || lower.contains("cannot find the task")
        || lower.contains("the system cannot find the file specified")
}

fn build_wan_persist_script(interface_index: u32) -> String {
    // Native .cmd batch script using only route.exe (no PowerShell).
    // 1. Parse 'route print -4' to find gateway for target interface index
    // 2. Delete all 0.0.0.0 default routes
    // 3. Add persistent route via target NIC gateway
    format!(r#"@echo off
setlocal enabledelayedexpansion
set "TARGET_IF={target_if}"
set "GATEWAY="

REM Parse route print to find gateway for our target interface
for /f "tokens=1-5" %%a in ('route print -4 ^| findstr /r "^  *0\.0\.0\.0"') do (
    if "%%e"=="!TARGET_IF!" (
        set "GATEWAY=%%c"
    )
)

if "!GATEWAY!"=="" (
    echo No gateway found for interface index %TARGET_IF%. Skipping startup WAN apply.
    exit /b 0
)

REM Delete all default routes then add persistent one for target NIC
route delete 0.0.0.0 >nul 2>&1
route -p add 0.0.0.0 mask 0.0.0.0 !GATEWAY! metric 1 if %TARGET_IF% >nul 2>&1

echo Startup WAN applied on interface %TARGET_IF% via gateway !GATEWAY!.
"#, target_if = interface_index)
}

fn ensure_wan_persist_script(interface_index: u32) -> Result<(), String> {
    fs::create_dir_all(WAN_PERSIST_DIR)
        .map_err(|e| format!("Failed to create {}: {}", WAN_PERSIST_DIR, e))?;
    fs::write(
        WAN_PERSIST_SCRIPT_PATH,
        build_wan_persist_script(interface_index),
    )
    .map_err(|e| format!("Failed to write {}: {}", WAN_PERSIST_SCRIPT_PATH, e))
}

/// Native cache cleanup for a target. Returns (label, success, detail).
fn native_cache_cleanup(target: &str) -> Option<(&'static str, bool, String)> {
    let local = std::env::var("LOCALAPPDATA").unwrap_or_else(|_| {
        let user_profile = std::env::var("USERPROFILE").unwrap_or_else(|_| r"C:\Users\Default".to_string());
        format!(r"{}\AppData\Local", user_profile)
    });

    match target {
        "user_temp" => {
            let path = std::path::Path::new(&local).join("Temp");
            let (del, fail) = clean_directory_contents(&path);
            Some(("User Temp", fail == 0, format!("[OK] User Temp cleaned ({del} items, {fail} failed).")))
        }
        "windows_temp" => {
            let path = std::path::Path::new(r"C:\Windows\Temp");
            let (del, fail) = clean_directory_contents(path);
            Some(("Windows Temp", fail == 0, format!("[OK] Windows Temp cleaned ({del} items, {fail} failed).")))
        }
        "windows_update_cache" => {
            #[cfg(target_os = "windows")]
            {
                let _ = std::process::Command::new("net").args(["stop", "wuauserv", "/y"])
                    .creation_flags(CREATE_NO_WINDOW).output();
                let _ = std::process::Command::new("net").args(["stop", "bits", "/y"])
                    .creation_flags(CREATE_NO_WINDOW).output();
            }
            let path = std::path::Path::new(r"C:\Windows\SoftwareDistribution\Download");
            let (del, fail) = clean_directory_contents(path);
            #[cfg(target_os = "windows")]
            {
                let _ = std::process::Command::new("net").args(["start", "wuauserv"])
                    .creation_flags(CREATE_NO_WINDOW).output();
                let _ = std::process::Command::new("net").args(["start", "bits"])
                    .creation_flags(CREATE_NO_WINDOW).output();
            }
            Some(("Windows Update Cache", fail == 0, format!("[OK] Windows Update cache cleaned ({del} items, {fail} failed).")))
        }
        "prefetch" => {
            let path = std::path::Path::new(r"C:\Windows\Prefetch");
            let (del, fail) = clean_directory_contents(path);
            Some(("Prefetch", fail == 0, format!("[OK] Prefetch cleaned ({del} items, {fail} failed).")))
        }
        "explorer_cache" => {
            let explorer_dir = std::path::Path::new(&local).join(r"Microsoft\Windows\Explorer");
            let (d1, f1) = clean_files_with_prefix(&explorer_dir, "thumbcache_", ".db");
            let (d2, f2) = clean_files_with_prefix(&explorer_dir, "iconcache_", ".db");
            let del = d1 + d2;
            let fail = f1 + f2;
            Some(("Explorer Cache (thumbnail/icon)", fail == 0, format!("[OK] Explorer cache cleaned ({del} items, {fail} failed).")))
        }
        "edge_cache" => {
            let base = std::path::Path::new(&local).join(r"Microsoft\Edge\User Data\Default");
            let mut del = 0u64; let mut fail = 0u64;
            for sub in ["Cache", "Code Cache", "GPUCache"] {
                let (d, f) = clean_directory_contents(&base.join(sub));
                del += d; fail += f;
            }
            Some(("Microsoft Edge Cache", fail == 0, format!("[OK] Edge cache cleaned ({del} items, {fail} failed).")))
        }
        "chrome_cache" => {
            let base = std::path::Path::new(&local).join(r"Google\Chrome\User Data\Default");
            let mut del = 0u64; let mut fail = 0u64;
            for sub in ["Cache", "Code Cache", "GPUCache"] {
                let (d, f) = clean_directory_contents(&base.join(sub));
                del += d; fail += f;
            }
            Some(("Google Chrome Cache", fail == 0, format!("[OK] Chrome cache cleaned ({del} items, {fail} failed).")))
        }
        "firefox_cache" => {
            let profiles_dir = std::path::Path::new(&local).join(r"Mozilla\Firefox\Profiles");
            let mut del = 0u64; let mut fail = 0u64;
            if let Ok(entries) = std::fs::read_dir(&profiles_dir) {
                for entry in entries.flatten() {
                    let cache2 = entry.path().join("cache2");
                    let (d, f) = clean_directory_contents(&cache2);
                    del += d; fail += f;
                }
            }
            Some(("Mozilla Firefox Cache", fail == 0, format!("[OK] Firefox cache cleaned ({del} items, {fail} failed).")))
        }
        "inet_cache" => {
            let path = std::path::Path::new(&local).join(r"Microsoft\Windows\INetCache");
            let (del, fail) = clean_directory_contents(&path);
            Some(("INetCache", fail == 0, format!("[OK] INetCache cleaned ({del} items, {fail} failed).")))
        }
        "web_cache" => {
            let path = std::path::Path::new(&local).join(r"Microsoft\Windows\WebCache");
            let (del, fail) = clean_directory_contents(&path);
            Some(("WebCache", fail == 0, format!("[OK] WebCache cleaned ({del} items, {fail} failed).")))
        }
        "crash_dumps" => {
            let path = std::path::Path::new(&local).join("CrashDumps");
            let (del, fail) = clean_directory_contents(&path);
            Some(("Crash Dumps", fail == 0, format!("[OK] Crash dumps cleaned ({del} items, {fail} failed).")))
        }
        "wer_reports" => {
            let (d1, f1) = clean_directory_contents(std::path::Path::new(r"C:\ProgramData\Microsoft\Windows\WER"));
            let (d2, f2) = clean_directory_contents(&std::path::Path::new(&local).join(r"Microsoft\Windows\WER"));
            let del = d1 + d2; let fail = f1 + f2;
            Some(("Windows Error Reporting (WER)", fail == 0, format!("[OK] WER cleaned ({del} items, {fail} failed).")))
        }
        "d3d_shader_cache" => {
            let path = std::path::Path::new(&local).join("D3DSCache");
            let (del, fail) = clean_directory_contents(&path);
            Some(("DirectX Shader Cache (D3DSCache)", fail == 0, format!("[OK] D3DSCache cleaned ({del} items, {fail} failed).")))
        }
        _ => None,
    }
}

/// Clean all files and subdirectories inside a directory, leaving the directory itself.
fn clean_directory_contents(path: &std::path::Path) -> (u64, u64) {
    let mut deleted = 0u64;
    let mut failed = 0u64;
    let Ok(entries) = std::fs::read_dir(path) else {
        return (0, 0);
    };
    for entry in entries.flatten() {
        let entry_path = entry.path();
        if entry_path.is_dir() {
            match std::fs::remove_dir_all(&entry_path) {
                Ok(_) => deleted += 1,
                Err(_) => failed += 1,
            }
        } else {
            match std::fs::remove_file(&entry_path) {
                Ok(_) => deleted += 1,
                Err(_) => failed += 1,
            }
        }
    }
    (deleted, failed)
}

/// Delete files matching a glob-like prefix in a directory (e.g. thumbcache_*.db)
fn clean_files_with_prefix(dir: &std::path::Path, prefix: &str, suffix: &str) -> (u64, u64) {
    let mut deleted = 0u64;
    let mut failed = 0u64;
    let Ok(entries) = std::fs::read_dir(dir) else {
        return (0, 0);
    };
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        if name.starts_with(prefix) && name.ends_with(suffix) {
            match std::fs::remove_file(entry.path()) {
                Ok(_) => deleted += 1,
                Err(_) => failed += 1,
            }
        }
    }
    (deleted, failed)
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
    let timeout_budget_ms = timeout_ms
        .parse::<u64>()
        .unwrap_or(1200)
        .saturating_add(1500);
    let output = run_process_blocking(
        "ping",
        &["-n", "1", "-w", timeout_ms, &target],
        Duration::from_millis(timeout_budget_ms),
    );

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
    let adapters = tauri::async_runtime::spawn_blocking(|| {
        crate::win32_net::enumerate_adapters()
    }).await.map_err(|e| format!("Task join error: {e}"))??;

    let blacklist = [
        "virtual", "vmware", "vbox", "loopback", "wintun", "kernel",
        "miniport", "wi-fi direct", "tap-", "pseudo", "ethernet adapter v",
    ];

    let mut interfaces: Vec<NetworkInterface> = Vec::new();

    for nic in &adapters {
        let desc_lower = nic.description.to_lowercase();
        if blacklist.iter().any(|b| desc_lower.contains(b)) {
            continue;
        }

        // Get first IPv4 address
        let ip = nic.ip_addresses.iter()
            .find(|a| a.contains('.'))
            .cloned()
            .unwrap_or_else(|| "0.0.0.0".to_string());

        if active_only && (ip.is_empty() || ip == "0.0.0.0") {
            continue;
        }

        let gateway = nic.gateways.iter()
            .find(|g| g.contains('.'))
            .cloned()
            .unwrap_or_default();

        interfaces.push(NetworkInterface {
            index: nic.interface_index.to_string(),
            ip,
            gateway,
            description: nic.description.clone(),
        });
    }

    Ok(interfaces)
}

/// Get IPv4 routing table
#[tauri::command]
pub async fn get_routing_table() -> Result<Vec<RouteEntry>, String> {
    // Use `route print -4` which always outputs IPv4 routes in a parseable table
    let output = run_cmd("route", &["print", "-4"]).await?;

    let mut routes: Vec<RouteEntry> = Vec::new();
    let mut in_active_routes = false;

    for line in output.lines() {
        let trimmed = line.trim();
        if trimmed.contains("Active Routes:") {
            in_active_routes = true;
            continue;
        }
        if trimmed.starts_with("Persistent Routes:") || trimmed.starts_with("=========") {
            if in_active_routes && trimmed.starts_with("Persistent") {
                break;
            }
            continue;
        }
        if trimmed.starts_with("Network Destination") {
            continue; // header
        }
        if !in_active_routes || trimmed.is_empty() {
            continue;
        }

        // Parse: "Network Destination   Netmask          Gateway       Interface  Metric"
        let parts: Vec<&str> = trimmed.split_whitespace().collect();
        if parts.len() >= 5 {
            routes.push(RouteEntry {
                destination: parts[0].to_string(),
                netmask: parts[1].to_string(),
                gateway: parts[2].to_string(),
                metric: parts[4].to_string(),
                interface_index: String::new(), // route print doesn't show interface index directly
            });
        }
    }

    Ok(routes)
}

pub fn add_route_blocking(
    destination: String,
    mask: String,
    gateway: String,
    metric: String,
    interface_index: Option<String>,
) -> Result<CommandResult, String> {
    let _ = run_cmd_blocking(
        "route",
        &["delete", &destination, "mask", &mask],
        Duration::from_secs(DEFAULT_CMD_TIMEOUT_SECS),
    );

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

    let result = run_cmd_blocking(
        args[0],
        &args[1..],
        Duration::from_secs(DEFAULT_CMD_TIMEOUT_SECS),
    )?;

    Ok(CommandResult {
        success: true,
        output: result,
    })
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
    tauri::async_runtime::spawn_blocking(move || {
        add_route_blocking(destination, mask, gateway, metric, interface_index)
    })
    .await
    .map_err(|err| format!("Route add task join error: {err}"))?
}

/// Delete a route
pub fn delete_route_blocking(destination: String, mask: String) -> Result<CommandResult, String> {
    let result = run_cmd_blocking(
        "route",
        &["delete", &destination, "mask", &mask],
        Duration::from_secs(DEFAULT_CMD_TIMEOUT_SECS),
    )?;
    Ok(CommandResult {
        success: true,
        output: result,
    })
}

#[tauri::command]
pub async fn delete_route(destination: String, mask: String) -> Result<CommandResult, String> {
    tauri::async_runtime::spawn_blocking(move || delete_route_blocking(destination, mask))
        .await
        .map_err(|err| format!("Route delete task join error: {err}"))?
}

/// Flush all routes
pub fn flush_routes_blocking() -> Result<CommandResult, String> {
    let result = run_cmd_blocking(
        "route",
        &["-f"],
        Duration::from_secs(DEFAULT_CMD_TIMEOUT_SECS),
    )?;
    Ok(CommandResult {
        success: true,
        output: result,
    })
}

#[tauri::command]
pub async fn flush_routes() -> Result<CommandResult, String> {
    tauri::async_runtime::spawn_blocking(flush_routes_blocking)
        .await
        .map_err(|err| format!("Route flush task join error: {err}"))?
}

pub fn set_default_gateway_blocking(
    gateway: String,
    interface_index: String,
) -> Result<CommandResult, String> {
    let target_interface_index = interface_index
        .trim()
        .parse::<u32>()
        .map_err(|_| "Invalid interface index".to_string())?;

    // Clean up stale default routes using native `route delete`
    let cleanup_output = run_cmd_blocking(
        "route",
        &["delete", "0.0.0.0"],
        Duration::from_secs(DEFAULT_CMD_TIMEOUT_SECS),
    ).unwrap_or_else(|_| "No stale routes to remove.".to_string());

    let _ = run_cmd_blocking(
        "route",
        &["delete", "0.0.0.0"],
        Duration::from_secs(DEFAULT_CMD_TIMEOUT_SECS),
    );

    let result = run_cmd_blocking(
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
        Duration::from_secs(DEFAULT_CMD_TIMEOUT_SECS),
    )?;

    Ok(CommandResult {
        success: true,
        output: format!("{}\n{}", cleanup_output.trim(), result.trim()),
    })
}

/// Set a NIC as default internet gateway
#[tauri::command]
pub async fn set_default_gateway(
    gateway: String,
    interface_index: String,
) -> Result<CommandResult, String> {
    tauri::async_runtime::spawn_blocking(move || {
        set_default_gateway_blocking(gateway, interface_index)
    })
    .await
    .map_err(|err| format!("Default gateway task join error: {err}"))?
}

pub fn set_wan_persist_on_startup_blocking(
    interface_index: String,
    enabled: bool,
) -> Result<CommandResult, String> {
    if enabled {
        let target_interface_index = interface_index
            .trim()
            .parse::<u32>()
            .map_err(|_| "Invalid interface index".to_string())?;

        ensure_wan_persist_script(target_interface_index)?;

        let task_command = format!(
            r#"cmd.exe /c "{}""#,
            WAN_PERSIST_SCRIPT_PATH
        );
        let create_output = run_cmd_blocking(
            "schtasks",
            &[
                "/Create",
                "/TN",
                WAN_PERSIST_TASK_NAME,
                "/SC",
                "ONSTART",
                "/RL",
                "HIGHEST",
                "/RU",
                "SYSTEM",
                "/TR",
                &task_command,
                "/F",
            ],
            Duration::from_secs(DEFAULT_CMD_TIMEOUT_SECS),
        )?;

        return Ok(CommandResult {
            success: true,
            output: format!(
                "Persist on startup enabled for interface {}.\n{}",
                target_interface_index,
                create_output.trim()
            ),
        });
    }

    let delete_output = match run_cmd_blocking(
        "schtasks",
        &["/Delete", "/TN", WAN_PERSIST_TASK_NAME, "/F"],
        Duration::from_secs(DEFAULT_CMD_TIMEOUT_SECS),
    ) {
        Ok(output) => output,
        Err(err) => {
            if is_task_not_found_error(&err) {
                "Startup task was already removed.".to_string()
            } else {
                return Err(err);
            }
        }
    };

    Ok(CommandResult {
        success: true,
        output: format!("Persist on startup disabled.\n{}", delete_output.trim()),
    })
}

/// Enable/disable WAN persist task that reapplies selected interface as default gateway on startup.
#[tauri::command]
pub async fn set_wan_persist_on_startup(
    interface_index: String,
    enabled: bool,
) -> Result<CommandResult, String> {
    tauri::async_runtime::spawn_blocking(move || {
        set_wan_persist_on_startup_blocking(interface_index, enabled)
    })
    .await
    .map_err(|err| format!("Persist-on-startup task join error: {err}"))?
}

/// Returns whether WAN persist startup task currently exists.
#[tauri::command]
pub async fn get_wan_persist_on_startup_status() -> Result<bool, String> {
    match run_cmd("schtasks", &["/Query", "/TN", WAN_PERSIST_TASK_NAME]).await {
        Ok(_) => Ok(true),
        Err(err) => {
            if is_task_not_found_error(&err) {
                Ok(false)
            } else {
                Err(err)
            }
        }
    }
}

pub fn run_network_command_blocking(command: String) -> Result<CommandResult, String> {
    let allowed_prefixes = [
        "ipconfig",
        "ipconfig /displaydns",
        "powercfg /batteryreport",
        "tracert",
        "nslookup",
        "netsh wlan show interface",
        "netsh winhttp reset proxy",
        "netsh int ip reset",
        "netsh winsock reset",
        "netsh interface ip delete arpcache",
        "netsh interface set interface",
        "netsh advfirewall reset",
    ];

    let cmd_lower = command.to_lowercase();
    if !allowed_prefixes
        .iter()
        .any(|prefix| cmd_lower.starts_with(prefix))
    {
        return Err("Command not allowed".to_string());
    }

    let output = run_process_blocking(
        "cmd",
        &["/C", &command],
        Duration::from_secs(NETWORK_COMMAND_TIMEOUT_SECS),
    )?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    Ok(CommandResult {
        success: output.status.success(),
        output: if stdout.is_empty() { stderr } else { stdout },
    })
}

/// Run a network fix command (flush DNS, renew IP, etc.)
#[tauri::command]
pub async fn run_network_command(command: String) -> Result<CommandResult, String> {
    tauri::async_runtime::spawn_blocking(move || run_network_command_blocking(command))
        .await
        .map_err(|err| format!("Network command task join error: {err}"))?
}

/// Ping a host and return latency
#[tauri::command]
pub async fn ping_host(target: String, count: Option<u32>) -> Result<PingResult, String> {
    let n = count.unwrap_or(1).to_string();

    let start = Instant::now();
    let output = run_process("ping", &["-n", &n, "-w", "2000", &target], Duration::from_secs(8))
        .await
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

/// Test TCP port connectivity (replaces PowerShell Test-NetConnection)
#[tauri::command]
pub async fn test_tcp_port(host: String, port: u16) -> Result<CommandResult, String> {
    use std::net::{TcpStream, ToSocketAddrs};

    let h = host.clone();
    let p = port;
    tauri::async_runtime::spawn_blocking(move || {
        let addr_str = format!("{}:{}", h, p);
        let start = Instant::now();

        // Resolve DNS
        let addrs: Vec<_> = match addr_str.to_socket_addrs() {
            Ok(a) => a.collect(),
            Err(e) => {
                return Ok(CommandResult {
                    success: false,
                    output: format!(
                        "ComputerName     : {}\nRemotePort       : {}\nTcpTestSucceeded : False\nError            : DNS resolution failed: {}",
                        h, p, e
                    ),
                });
            }
        };

        if addrs.is_empty() {
            return Ok(CommandResult {
                success: false,
                output: format!(
                    "ComputerName     : {}\nRemotePort       : {}\nTcpTestSucceeded : False\nError            : No addresses resolved",
                    h, p
                ),
            });
        }

        let target = addrs[0];
        let timeout = Duration::from_secs(5);
        let result = TcpStream::connect_timeout(&target, timeout);
        let elapsed = start.elapsed().as_millis();

        match result {
            Ok(_stream) => Ok(CommandResult {
                success: true,
                output: format!(
                    "ComputerName     : {}\nRemoteAddress    : {}\nRemotePort       : {}\nTcpTestSucceeded : True\nLatency(ms)      : {}",
                    h, target.ip(), p, elapsed
                ),
            }),
            Err(e) => Ok(CommandResult {
                success: false,
                output: format!(
                    "ComputerName     : {}\nRemoteAddress    : {}\nRemotePort       : {}\nTcpTestSucceeded : False\nLatency(ms)      : {}\nError            : {}",
                    h, target.ip(), p, elapsed, e
                ),
            }),
        }
    })
    .await
    .map_err(|e| format!("TCP port test join error: {e}"))?
}

/// Get bloatware candidates and installation status
#[tauri::command]
pub async fn get_bloatware_candidates() -> Result<Vec<BloatwareItem>, String> {
    let ps_script = r#"
        $names = @()
        try {
            $names += Get-AppxPackage -AllUsers -ErrorAction Stop | Select-Object -ExpandProperty Name
        } catch {
            $names += Get-AppxPackage -ErrorAction SilentlyContinue | Select-Object -ExpandProperty Name
        }
        try {
            $names += Get-AppxProvisionedPackage -Online -ErrorAction Stop | Select-Object -ExpandProperty DisplayName
        } catch {}
        $names |
            Where-Object { $_ -and $_.Trim().Length -gt 0 } |
            ForEach-Object { $_.ToLowerInvariant() } |
            Sort-Object -Unique |
            ConvertTo-Json -Compress
    "#;

    let output = run_powershell(ps_script).await?;
    let mut installed = HashSet::new();
    let parsed = serde_json::from_str::<serde_json::Value>(output.trim())
        .unwrap_or(serde_json::Value::Array(vec![]));

    match parsed {
        serde_json::Value::Array(arr) => {
            for item in arr {
                if let Some(name) = item.as_str() {
                    installed.insert(name.to_lowercase());
                }
            }
        }
        serde_json::Value::String(single_name) => {
            installed.insert(single_name.to_lowercase());
        }
        _ => {}
    }

    let mut items: Vec<BloatwareItem> = BLOATWARE_CANDIDATES
        .iter()
        .map(|(package_name, label)| BloatwareItem {
            package_name: (*package_name).to_string(),
            label: (*label).to_string(),
            installed: installed.contains(&package_name.to_lowercase()),
        })
        .collect();
    items.sort_by(|a, b| a.label.cmp(&b.label));
    Ok(items)
}

/// Remove selected bloatware packages
#[tauri::command]
pub async fn remove_bloatware(packages: Vec<String>) -> Result<CommandResult, String> {
    if packages.is_empty() {
        return Err("No packages selected".to_string());
    }

    let allowed: HashMap<String, &str> = BLOATWARE_CANDIDATES
        .iter()
        .map(|(package_name, _)| (package_name.to_lowercase(), *package_name))
        .collect();

    let mut selected = Vec::new();
    let mut seen = HashSet::new();
    for raw in packages {
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            continue;
        }

        let lower = trimmed.to_lowercase();
        let is_safe_token = lower
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || ch == '.' || ch == '_' || ch == '-');
        if !is_safe_token {
            continue;
        }

        if let Some(canonical) = allowed.get(&lower) {
            let canonical_name = (*canonical).to_string();
            if seen.insert(canonical_name.clone()) {
                selected.push(canonical_name);
            }
        }
    }

    if selected.is_empty() {
        return Err("No valid bloatware packages selected".to_string());
    }

    let mut output_lines = vec![
        format!("Requested removal for {} package(s).", selected.len()),
        "Administrative privileges may be required for removal.".to_string(),
        String::new(),
    ];
    let mut removed = 0u32;
    let mut skipped = 0u32;
    let mut failed = 0u32;

    for package_name in selected {
        let escaped_name = ps_escape_single_quoted(&package_name);
        let script = format!(
            r#"
$pkgName = '{escaped_name}'
$hasFailure = $false
$removedInstalled = 0
$removedProvisioned = 0

$installedMatches = Get-AppxPackage -AllUsers -ErrorAction SilentlyContinue | Where-Object {{ $_.Name -eq $pkgName }}
if (-not $installedMatches) {{
  $installedMatches = Get-AppxPackage -ErrorAction SilentlyContinue | Where-Object {{ $_.Name -eq $pkgName }}
}}
foreach ($pkg in $installedMatches) {{
  try {{
    Remove-AppxPackage -Package $pkg.PackageFullName -AllUsers -ErrorAction Stop | Out-Null
    $removedInstalled++
  }} catch {{
    $hasFailure = $true
    Write-Output "[FAIL] $pkgName installed remove error: $($_.Exception.Message)"
  }}
}}

try {{
  $provisionedMatches = Get-AppxProvisionedPackage -Online -ErrorAction Stop | Where-Object {{ $_.DisplayName -eq $pkgName }}
  foreach ($prov in $provisionedMatches) {{
    try {{
      Remove-AppxProvisionedPackage -Online -PackageName $prov.PackageName -ErrorAction Stop | Out-Null
      $removedProvisioned++
    }} catch {{
      $hasFailure = $true
      Write-Output "[FAIL] $pkgName provisioned remove error: $($_.Exception.Message)"
    }}
  }}
}} catch {{
  $hasFailure = $true
  Write-Output "[FAIL] $pkgName provisioned query error: $($_.Exception.Message)"
}}

if ($removedInstalled -gt 0 -or $removedProvisioned -gt 0) {{
  Write-Output "[OK] $pkgName removed installed=$removedInstalled provisioned=$removedProvisioned"
}} elseif ($hasFailure) {{
  Write-Output "[WARN] $pkgName no removal completed"
}} else {{
  Write-Output "[SKIP] $pkgName not installed"
}}
"#
        );

        match run_powershell(&script).await {
            Ok(script_output) => {
                let clean_output = script_output.trim();
                if clean_output.is_empty() {
                    skipped += 1;
                    output_lines.push(format!("[SKIP] {} no output returned", package_name));
                } else {
                    output_lines.extend(clean_output.lines().map(|line| line.trim_end().to_string()));
                    let has_fail = clean_output.contains("[FAIL]");
                    let has_ok = clean_output.contains("[OK]");
                    let has_skip = clean_output.contains("[SKIP]");
                    if has_fail {
                        failed += 1;
                    } else if has_ok {
                        removed += 1;
                    } else if has_skip {
                        skipped += 1;
                    } else {
                        skipped += 1;
                    }
                }
            }
            Err(err) => {
                failed += 1;
                output_lines.push(format!(
                    "[FAIL] {} command execution failed: {}",
                    package_name,
                    err.trim()
                ));
            }
        }

        output_lines.push(String::new());
    }

    output_lines.push(format!(
        "Summary: removed={} skipped={} failed={}",
        removed, skipped, failed
    ));

    Ok(CommandResult {
        success: failed == 0,
        output: output_lines.join("\n"),
    })
}

/// Generate and return battery report HTML for in-app preview
#[tauri::command]
pub async fn get_battery_report() -> Result<BatteryReportResult, String> {
    let report_path = std::env::temp_dir().join("SuperRoutePro-BatteryReport.html");
    let report_path_arg = report_path.to_string_lossy().to_string();

    let _ = run_cmd(
        "powercfg",
        &["/batteryreport", "/output", &report_path_arg],
    )
    .await?;

    let html = fs::read_to_string(&report_path).map_err(|e| {
        format!(
            "Failed to read battery report file: {} ({})",
            report_path_arg, e
        )
    })?;

    if html.trim().is_empty() {
        return Err("Battery report is empty".to_string());
    }

    Ok(BatteryReportResult { html })
}

/// Get battery health summary focused on wear level and estimated lifetime.
#[tauri::command]
pub async fn get_battery_summary() -> Result<BatterySummaryResult, String> {
    tauri::async_runtime::spawn_blocking(|| {
        #[cfg(target_os = "windows")]
        {
            use windows_sys::Win32::System::Power::*;

            let mut sps: SYSTEM_POWER_STATUS = unsafe { std::mem::zeroed() };
            let result = unsafe { GetSystemPowerStatus(&mut sps) };
            if result == 0 {
                return Err("GetSystemPowerStatus failed".to_string());
            }

            // Check if battery is present
            let no_battery = sps.BatteryFlag == 128 || sps.BatteryFlag == 255;
            if no_battery {
                return Ok(BatterySummaryResult {
                    present: false,
                    status: "No battery detected".to_string(),
                    charge_percent: None,
                    design_capacity_mwh: None,
                    full_charge_capacity_mwh: None,
                    health_percent: None,
                    wear_percent: None,
                    cycle_count: None,
                    estimated_runtime_minutes: None,
                    estimated_runtime_full_minutes: None,
                    note: "This machine may be desktop-only or battery telemetry is unavailable.".to_string(),
                });
            }

            let charge_percent = if sps.BatteryLifePercent <= 100 {
                Some(sps.BatteryLifePercent as u32)
            } else {
                None
            };

            let status = match (sps.ACLineStatus, sps.BatteryFlag) {
                (1, f) if f & 8 != 0 => "Charging".to_string(),
                (1, _) => "Connected to AC".to_string(),
                (0, f) if f & 4 != 0 => "Critical".to_string(),
                (0, f) if f & 2 != 0 => "Low".to_string(),
                (0, _) => "Discharging".to_string(),
                _ => format!("AC={} Flag={}", sps.ACLineStatus, sps.BatteryFlag),
            };

            let estimated_runtime_minutes = if sps.BatteryLifeTime != u32::MAX && sps.BatteryLifeTime > 0 {
                Some(sps.BatteryLifeTime / 60)
            } else {
                None
            };

            let estimated_runtime_full_minutes = if sps.BatteryFullLifeTime != u32::MAX && sps.BatteryFullLifeTime > 0 {
                Some(sps.BatteryFullLifeTime / 60)
            } else {
                match (estimated_runtime_minutes, charge_percent) {
                    (Some(rt), Some(cp)) if cp > 0 => Some((rt as f64 * 100.0 / cp as f64) as u32),
                    _ => None,
                }
            };

            // Try to get detailed battery info via DeviceIoControl IOCTL
            let ioctl_details = query_battery_details_ioctl();

            let (design_cap, full_cap, cycle, health_pct, wear_pct, note) = match ioctl_details {
                Some(details) => {
                    let health = if details.designed_capacity_mwh > 0 {
                        Some((details.full_charged_capacity_mwh as f32 / details.designed_capacity_mwh as f32) * 100.0)
                    } else {
                        None
                    };
                    let wear = health.map(|h| (100.0 - h).max(0.0));
                    let cc = if details.cycle_count > 0 { Some(details.cycle_count) } else { None };
                    (
                        Some(details.designed_capacity_mwh),
                        Some(details.full_charged_capacity_mwh),
                        cc,
                        health,
                        wear,
                        format!("Battery details from native IOCTL. Chemistry: {}", details.chemistry),
                    )
                }
                None => (
                    None, None, None, None, None,
                    "Battery data from Win32 GetSystemPowerStatus. IOCTL detail query unavailable.".to_string(),
                ),
            };

            Ok(BatterySummaryResult {
                present: true,
                status,
                charge_percent,
                design_capacity_mwh: design_cap,
                full_charge_capacity_mwh: full_cap,
                health_percent: health_pct,
                wear_percent: wear_pct,
                cycle_count: cycle,
                estimated_runtime_minutes,
                estimated_runtime_full_minutes,
                note,
            })
        }
        #[cfg(not(target_os = "windows"))]
        {
            Err("Battery summary only supported on Windows".to_string())
        }
    })
    .await
    .map_err(|e| format!("Battery task join error: {e}"))?
}

// ── Battery IOCTL helper ──

/// Battery detail info obtained from DeviceIoControl.
#[cfg(target_os = "windows")]
struct BatteryIoctlDetails {
    designed_capacity_mwh: u32,
    full_charged_capacity_mwh: u32,
    cycle_count: u32,
    chemistry: String,
}

/// Battery IOCTL constants (from batclass.h / WDK — not in windows-sys)
#[cfg(target_os = "windows")]
mod battery_ioctl {
    // CTL_CODE(FILE_DEVICE_BATTERY=0x29, function, METHOD_BUFFERED=0, FILE_READ_ACCESS=1)
    pub const IOCTL_BATTERY_QUERY_TAG: u32 = (0x29 << 16) | (1 << 14) | (0x10 << 2) | 0;
    pub const IOCTL_BATTERY_QUERY_INFORMATION: u32 = (0x29 << 16) | (1 << 14) | (0x11 << 2) | 0;

    // BATTERY_QUERY_INFORMATION_LEVEL
    pub const BATTERY_INFORMATION_LEVEL: u32 = 0;

    #[repr(C)]
    pub struct BatteryQueryInformation {
        pub battery_tag: u32,
        pub information_level: u32,
        pub at_rate: i32,
    }

    #[repr(C)]
    pub struct BatteryInformation {
        pub capabilities: u32,
        pub technology: u8,
        pub reserved: [u8; 3],
        pub chemistry: [u8; 4],
        pub designed_capacity: u32,
        pub full_charged_capacity: u32,
        pub default_alert1: u32,
        pub default_alert2: u32,
        pub critical_bias: u32,
        pub cycle_count: u32,
    }

    // GUID_DEVINTERFACE_BATTERY = {72631e54-78a4-11d0-bcf7-00aa00b7b32a}
    pub const BATTERY_GUID: windows_sys::core::GUID = windows_sys::core::GUID {
        data1: 0x72631e54,
        data2: 0x78a4,
        data3: 0x11d0,
        data4: [0xbc, 0xf7, 0x00, 0xaa, 0x00, 0xb7, 0xb3, 0x2a],
    };
}

/// Query battery details using SetupDi + DeviceIoControl.
/// All Win32 FFI is declared manually to avoid windows-sys handle type inconsistencies.
#[cfg(target_os = "windows")]
fn query_battery_details_ioctl() -> Option<BatteryIoctlDetails> {
    use battery_ioctl::*;
    use std::ffi::c_void;

    type HANDLE = *mut c_void;
    const INVALID_HANDLE: HANDLE = -1isize as HANDLE;

    // GUID struct for FFI
    #[repr(C)]
    struct GUID {
        data1: u32,
        data2: u16,
        data3: u16,
        data4: [u8; 8],
    }

    #[repr(C)]
    struct SP_DEVICE_INTERFACE_DATA {
        cb_size: u32,
        interface_class_guid: GUID,
        flags: u32,
        reserved: usize,
    }

    // SP_DEVICE_INTERFACE_DETAIL_DATA_W has a variable-length DevicePath at end
    #[repr(C)]
    struct SP_DEVINFO_DATA {
        cb_size: u32,
        class_guid: GUID,
        dev_inst: u32,
        reserved: usize,
    }

    const DIGCF_PRESENT: u32 = 0x2;
    const DIGCF_DEVICEINTERFACE: u32 = 0x10;
    const GENERIC_READ: u32 = 0x80000000;
    const GENERIC_WRITE: u32 = 0x40000000;
    const FILE_SHARE_READ: u32 = 1;
    const FILE_SHARE_WRITE: u32 = 2;
    const OPEN_EXISTING: u32 = 3;

    extern "system" {
        fn SetupDiGetClassDevsW(
            class_guid: *const GUID,
            enumerator: *const u16,
            hwnd_parent: HANDLE,
            flags: u32,
        ) -> HANDLE;

        fn SetupDiEnumDeviceInterfaces(
            dev_info: HANDLE,
            dev_info_data: *const c_void,
            interface_class_guid: *const GUID,
            member_index: u32,
            device_interface_data: *mut SP_DEVICE_INTERFACE_DATA,
        ) -> i32;

        fn SetupDiGetDeviceInterfaceDetailW(
            dev_info: HANDLE,
            device_interface_data: *mut SP_DEVICE_INTERFACE_DATA,
            device_interface_detail_data: *mut c_void,
            device_interface_detail_data_size: u32,
            required_size: *mut u32,
            device_info_data: *mut c_void,
        ) -> i32;

        fn SetupDiDestroyDeviceInfoList(dev_info: HANDLE) -> i32;

        fn CreateFileW(
            file_name: *const u16,
            desired_access: u32,
            share_mode: u32,
            security_attributes: *const c_void,
            creation_disposition: u32,
            flags_and_attributes: u32,
            template_file: HANDLE,
        ) -> HANDLE;

        fn DeviceIoControl(
            device: HANDLE,
            io_control_code: u32,
            in_buffer: *const c_void,
            in_buffer_size: u32,
            out_buffer: *mut c_void,
            out_buffer_size: u32,
            bytes_returned: *mut u32,
            overlapped: *mut c_void,
        ) -> i32;

        fn CloseHandle(handle: HANDLE) -> i32;
    }

    let battery_guid = GUID {
        data1: 0x72631e54,
        data2: 0x78a4,
        data3: 0x11d0,
        data4: [0xbc, 0xf7, 0x00, 0xaa, 0x00, 0xb7, 0xb3, 0x2a],
    };

    unsafe {
        // Find battery device
        let dev_info = SetupDiGetClassDevsW(
            &battery_guid,
            std::ptr::null(),
            std::ptr::null_mut(),
            DIGCF_PRESENT | DIGCF_DEVICEINTERFACE,
        );
        if dev_info == INVALID_HANDLE || dev_info.is_null() {
            return None;
        }

        // Enumerate first battery interface
        let mut iface_data: SP_DEVICE_INTERFACE_DATA = std::mem::zeroed();
        iface_data.cb_size = std::mem::size_of::<SP_DEVICE_INTERFACE_DATA>() as u32;

        if SetupDiEnumDeviceInterfaces(dev_info, std::ptr::null(), &battery_guid, 0, &mut iface_data) == 0 {
            SetupDiDestroyDeviceInfoList(dev_info);
            return None;
        }

        // Get required buffer size
        let mut required_size: u32 = 0;
        SetupDiGetDeviceInterfaceDetailW(
            dev_info, &mut iface_data, std::ptr::null_mut(), 0, &mut required_size, std::ptr::null_mut(),
        );
        if required_size == 0 {
            SetupDiDestroyDeviceInfoList(dev_info);
            return None;
        }

        // Allocate and get device detail
        // SP_DEVICE_INTERFACE_DETAIL_DATA_W: cbSize (u32) + DevicePath[1] (u16)
        // On 64-bit: cbSize = 8 (due to alignment)
        let mut detail_buf: Vec<u8> = vec![0u8; required_size as usize];
        let cb_size_ptr = detail_buf.as_mut_ptr() as *mut u32;
        *cb_size_ptr = 8; // sizeof SP_DEVICE_INTERFACE_DETAIL_DATA_W on 64-bit

        if SetupDiGetDeviceInterfaceDetailW(
            dev_info, &mut iface_data,
            detail_buf.as_mut_ptr() as *mut c_void,
            required_size, std::ptr::null_mut(), std::ptr::null_mut(),
        ) == 0 {
            SetupDiDestroyDeviceInfoList(dev_info);
            return None;
        }

        // Device path starts at offset 4 (after cbSize u32)
        let device_path = detail_buf.as_ptr().add(4) as *const u16;

        // Open battery device
        let handle = CreateFileW(
            device_path,
            GENERIC_READ | GENERIC_WRITE,
            FILE_SHARE_READ | FILE_SHARE_WRITE,
            std::ptr::null(),
            OPEN_EXISTING,
            0,
            std::ptr::null_mut(),
        );
        SetupDiDestroyDeviceInfoList(dev_info);

        if handle == INVALID_HANDLE || handle.is_null() {
            return None;
        }

        // Query battery tag
        let timeout: u32 = 0;
        let mut battery_tag: u32 = 0;
        let mut bytes_returned: u32 = 0;

        let ok = DeviceIoControl(
            handle,
            IOCTL_BATTERY_QUERY_TAG,
            &timeout as *const u32 as *const c_void,
            4,
            &mut battery_tag as *mut u32 as *mut c_void,
            4,
            &mut bytes_returned,
            std::ptr::null_mut(),
        );
        if ok == 0 || battery_tag == 0 {
            CloseHandle(handle);
            return None;
        }

        // Query battery information
        let query = BatteryQueryInformation {
            battery_tag,
            information_level: BATTERY_INFORMATION_LEVEL,
            at_rate: 0,
        };
        let mut info: BatteryInformation = std::mem::zeroed();
        bytes_returned = 0;

        let ok = DeviceIoControl(
            handle,
            IOCTL_BATTERY_QUERY_INFORMATION,
            &query as *const _ as *const c_void,
            std::mem::size_of::<BatteryQueryInformation>() as u32,
            &mut info as *mut _ as *mut c_void,
            std::mem::size_of::<BatteryInformation>() as u32,
            &mut bytes_returned,
            std::ptr::null_mut(),
        );
        CloseHandle(handle);

        if ok == 0 {
            return None;
        }

        let chemistry = String::from_utf8_lossy(&info.chemistry).trim().to_string();

        Some(BatteryIoctlDetails {
            designed_capacity_mwh: info.designed_capacity,
            full_charged_capacity_mwh: info.full_charged_capacity,
            cycle_count: info.cycle_count,
            chemistry,
        })
    }
}

#[cfg(not(target_os = "windows"))]
fn query_battery_details_ioctl() -> Option<()> {
    None
}

/// Clear selected system/browser cache targets
#[tauri::command]
pub async fn clear_cache_targets(targets: Vec<String>) -> Result<CommandResult, String> {
    if targets.is_empty() {
        return Err("No cache targets selected".to_string());
    }

    let result = tauri::async_runtime::spawn_blocking(move || {
        let mut seen = HashSet::new();
        let mut output_lines = Vec::new();
        let mut success_count = 0u32;
        let mut failed_count = 0u32;
        let mut valid_count = 0u32;

        for target in &targets {
            let trimmed = target.trim().to_lowercase();
            if trimmed.is_empty() { continue; }
            let is_safe_token = trimmed.chars().all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '_' || ch == '-');
            if !is_safe_token || seen.contains(&trimmed) { continue; }
            seen.insert(trimmed.clone());

            match native_cache_cleanup(&trimmed) {
                Some((label, success, detail)) => {
                    valid_count += 1;
                    output_lines.push(format!("[TARGET] {}", label));
                    output_lines.push(detail);
                    if success { success_count += 1; } else { failed_count += 1; }
                }
                None => {
                    // Skip unknown targets silently
                }
            }
            output_lines.push(String::new());
        }

        if valid_count == 0 {
            return Err("No valid cache targets selected".to_string());
        }

        output_lines.insert(0, format!("Requested cleanup for {} cache target(s).", valid_count));
        output_lines.insert(1, "Administrative privileges may be required for some targets.".to_string());
        output_lines.insert(2, String::new());
        output_lines.push(format!("Summary: success={} failed={}", success_count, failed_count));

        Ok(CommandResult {
            success: failed_count == 0,
            output: output_lines.join("\n"),
        })
    }).await.map_err(|e| format!("Cache cleanup task join error: {e}"))??;

    Ok(result)
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
        .take(512)
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
