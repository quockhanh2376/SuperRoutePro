#[cfg(target_os = "windows")]
use crate::win32_consts::CREATE_NO_WINDOW;
use crate::cache_cleanup::{label_for_target, run_cleanup_for_current_user, sanitize_cleanup_targets};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::net::TcpStream;
use std::os::windows::process::CommandExt;
use std::process::{Command, Output, Stdio};
use std::thread;
use std::time::{Duration, Instant};

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
    ("Microsoft.MicrosoftOfficeHub", "Microsoft 365 (Office Hub)"),
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

// ======================== HELPERS ========================

fn run_process_blocking(program: &str, args: &[&str], timeout: Duration) -> Result<Output, String> {
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
    format!(
        r#"@echo off
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
for /l %%i in (1,1,6) do route delete 0.0.0.0 >nul 2>&1
route -p add 0.0.0.0 mask 0.0.0.0 !GATEWAY! metric 1 if %TARGET_IF% >nul 2>&1

echo Startup WAN applied on interface %TARGET_IF% via gateway !GATEWAY!.
"#,
        target_if = interface_index
    )
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

fn cleanup_default_routes_blocking() -> String {
    let mut outputs: Vec<String> = Vec::new();

    for _ in 0..6 {
        match run_cmd_blocking(
            "route",
            &["delete", "0.0.0.0"],
            Duration::from_secs(DEFAULT_CMD_TIMEOUT_SECS),
        ) {
            Ok(output) => {
                let trimmed = output.trim();
                if trimmed.is_empty() {
                    continue;
                }
                outputs.push(trimmed.to_string());
            }
            Err(err) => {
                if outputs.is_empty() {
                    return "No stale routes to remove.".to_string();
                }
                let trimmed = err.trim();
                if !trimmed.is_empty() {
                    outputs.push(trimmed.to_string());
                }
                break;
            }
        }
    }

    if outputs.is_empty() {
        "No stale routes to remove.".to_string()
    } else {
        outputs.join("\n")
    }
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
    let interface_index = target_interface_index.to_string();

    // Clean up stale default routes using native `route delete`
    let cleanup_output = cleanup_default_routes_blocking();

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

        let task_command = format!(r#"cmd.exe /c "{}""#, WAN_PERSIST_SCRIPT_PATH);
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

fn validate_network_command(command: &str) -> Result<(), String> {
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

    let trimmed = command.trim();
    let cmd_lower = trimmed.to_ascii_lowercase();
    let has_shell_metacharacters = trimmed.chars().any(|ch| {
        matches!(
            ch,
            '&' | '|' | '>' | '<' | '^' | '%' | '(' | ')' | '\r' | '\n'
        )
    });

    if trimmed.is_empty() || has_shell_metacharacters {
        return Err("Command not allowed".to_string());
    }

    if allowed_prefixes
        .iter()
        .any(|prefix| cmd_lower.starts_with(prefix))
    {
        Ok(())
    } else {
        Err("Command not allowed".to_string())
    }
}

pub fn run_network_command_blocking(command: String) -> Result<CommandResult, String> {
    validate_network_command(&command)?;
    let trimmed_command = command.trim().to_string();

    let output = run_process_blocking(
        "cmd",
        &["/C", &trimmed_command],
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
                    output_lines
                        .extend(clean_output.lines().map(|line| line.trim_end().to_string()));
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

/// Clear selected system/browser cache targets
#[tauri::command]
pub async fn clear_cache_targets(targets: Vec<String>) -> Result<CommandResult, String> {
    if targets.is_empty() {
        return Err("No cache targets selected".to_string());
    }

    let result = tauri::async_runtime::spawn_blocking(move || {
        let mut output_lines = Vec::new();
        let mut success_count = 0u32;
        let mut failed_count = 0u32;
        let selected_targets = sanitize_cleanup_targets(&targets);

        for target in &selected_targets {
            output_lines.push(format!(
                "[TARGET] {}",
                label_for_target(target).unwrap_or(target.as_str())
            ));
            match run_cleanup_for_current_user(target) {
                Some((_, success, detail)) => {
                    output_lines.push(detail);
                    if success {
                        success_count += 1;
                    } else {
                        failed_count += 1;
                    }
                }
                None => output_lines.push(format!("[FAIL] Unsupported cleanup target: {target}")),
            }
            output_lines.push(String::new());
        }

        if selected_targets.is_empty() {
            return Err("No valid cache targets selected".to_string());
        }

        output_lines.insert(
            0,
            format!("Requested cleanup for {} cache target(s).", selected_targets.len()),
        );
        output_lines.insert(
            1,
            "Administrative privileges may be required for some targets.".to_string(),
        );
        output_lines.insert(2, String::new());
        output_lines.push(format!(
            "Summary: success={} failed={}",
            success_count, failed_count
        ));

        Ok(CommandResult {
            success: failed_count == 0,
            output: output_lines.join("\n"),
        })
    })
    .await
    .map_err(|e| format!("Cache cleanup task join error: {e}"))??;

    Ok(result)
}

/// Check internet connectivity
#[tauri::command]
pub async fn check_internet() -> Result<bool, String> {
    tauri::async_runtime::spawn_blocking(|| {
        match TcpStream::connect_timeout(&"8.8.8.8:53".parse().unwrap(), Duration::from_secs(3)) {
            Ok(_) => Ok(true),
            Err(_) => Ok(false),
        }
    })
    .await
    .map_err(|err| format!("Internet check task join error: {err}"))?
}

#[cfg(test)]
mod tests {
    use super::validate_network_command;

    #[test]
    fn validate_network_command_allows_expected_diagnostic_commands() {
        assert!(validate_network_command("tracert -d 8.8.8.8").is_ok());
        assert!(validate_network_command("nslookup example.com 8.8.8.8").is_ok());
    }

    #[test]
    fn validate_network_command_rejects_shell_chaining_after_allowed_prefix() {
        assert!(validate_network_command("tracert -d 8.8.8.8 && whoami").is_err());
        assert!(validate_network_command("nslookup example.com | more").is_err());
    }
}
