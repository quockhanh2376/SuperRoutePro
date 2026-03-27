use crate::cache_cleanup::{
    label_for_target, run_cleanup_for_current_user, sanitize_cleanup_targets,
};
use crate::bloatware_catalog::{canonical_bloatware_package, BLOATWARE_CANDIDATES};
use crate::connectivity_probe;
use crate::process_exec::{
    run_cmd_blocking, run_powershell, run_process_blocking, DEFAULT_CMD_TIMEOUT_SECS,
    NETWORK_COMMAND_TIMEOUT_SECS,
};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::process::Output;
use std::time::{Duration, Instant};

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

fn collect_process_output(output: &Output) -> String {
    [
        String::from_utf8_lossy(&output.stdout).trim().to_string(),
        String::from_utf8_lossy(&output.stderr).trim().to_string(),
    ]
    .into_iter()
    .filter(|entry| !entry.is_empty())
    .collect::<Vec<_>>()
    .join("\n")
}

fn command_result_from_outputs(
    outputs: &[&Output],
    success: bool,
    success_fallback: &str,
    failure_fallback: &str,
) -> CommandResult {
    let combined_output = outputs
        .iter()
        .map(|output| collect_process_output(output))
        .filter(|output| !output.is_empty())
        .collect::<Vec<_>>()
        .join("\n");

    CommandResult {
        success,
        output: if combined_output.is_empty() {
            if success {
                success_fallback.to_string()
            } else {
                failure_fallback.to_string()
            }
        } else {
            combined_output
        },
    }
}

fn remaining_dhcp_timeout(started: Instant, total_timeout: Duration) -> Result<Duration, String> {
    total_timeout
        .checked_sub(started.elapsed())
        .filter(|timeout| !timeout.is_zero())
        .ok_or_else(|| {
            format!(
                "Command timed out after {}s: ipconfig /release && ipconfig /renew",
                total_timeout.as_secs()
            )
        })
}

fn normalize_dhcp_timeout_error(err: String, total_timeout: Duration) -> String {
    if err.starts_with("Command timed out after") {
        format!(
            "Command timed out after {}s: ipconfig /release && ipconfig /renew",
            total_timeout.as_secs()
        )
    } else {
        err
    }
}

fn ps_escape_single_quoted(input: &str) -> String {
    input.replace('\'', "''")
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

fn run_typed_network_command(
    program: &str,
    args: &[&str],
    timeout: Duration,
    success_fallback: &str,
    failure_fallback: &str,
) -> Result<CommandResult, String> {
    let output = run_process_blocking(program, args, timeout)?;
    Ok(command_result_from_outputs(
        &[&output],
        output.status.success(),
        success_fallback,
        failure_fallback,
    ))
}

pub fn flush_dns_blocking() -> Result<CommandResult, String> {
    run_typed_network_command(
        "ipconfig",
        &["/flushdns"],
        Duration::from_secs(NETWORK_COMMAND_TIMEOUT_SECS),
        "DNS cache flushed.",
        "DNS flush failed.",
    )
}

pub fn clear_arp_cache_blocking() -> Result<CommandResult, String> {
    run_typed_network_command(
        "netsh",
        &["interface", "ip", "delete", "arpcache"],
        Duration::from_secs(NETWORK_COMMAND_TIMEOUT_SECS),
        "ARP cache cleared.",
        "ARP cache clear failed.",
    )
}

pub fn reset_tcp_ip_blocking() -> Result<CommandResult, String> {
    run_typed_network_command(
        "netsh",
        &["int", "ip", "reset"],
        Duration::from_secs(NETWORK_COMMAND_TIMEOUT_SECS),
        "TCP/IP stack reset.",
        "TCP/IP reset failed.",
    )
}

pub fn reset_winsock_blocking() -> Result<CommandResult, String> {
    run_typed_network_command(
        "netsh",
        &["winsock", "reset"],
        Duration::from_secs(NETWORK_COMMAND_TIMEOUT_SECS),
        "Winsock reset completed.",
        "Winsock reset failed.",
    )
}

pub fn reset_firewall_blocking() -> Result<CommandResult, String> {
    run_typed_network_command(
        "netsh",
        &["advfirewall", "reset"],
        Duration::from_secs(NETWORK_COMMAND_TIMEOUT_SECS),
        "Firewall reset completed.",
        "Firewall reset failed.",
    )
}

pub fn reset_winhttp_proxy_blocking() -> Result<CommandResult, String> {
    run_typed_network_command(
        "netsh",
        &["winhttp", "reset", "proxy"],
        Duration::from_secs(NETWORK_COMMAND_TIMEOUT_SECS),
        "WinHTTP proxy reset completed.",
        "WinHTTP proxy reset failed.",
    )
}

pub fn set_adapter_enabled_blocking(
    interface_name: &str,
    enabled: bool,
) -> Result<CommandResult, String> {
    let name_arg = format!(r#"name="{}""#, interface_name);
    let admin_arg = if enabled {
        "admin=enabled"
    } else {
        "admin=disabled"
    };

    run_typed_network_command(
        "netsh",
        &["interface", "set", "interface", &name_arg, admin_arg],
        Duration::from_secs(NETWORK_COMMAND_TIMEOUT_SECS),
        if enabled {
            "Adapter enabled."
        } else {
            "Adapter disabled."
        },
        if enabled {
            "Adapter enable failed."
        } else {
            "Adapter disable failed."
        },
    )
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
    let has_shell_metacharacters = contains_disallowed_shell_metacharacters(trimmed);

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

fn contains_disallowed_shell_metacharacters(command: &str) -> bool {
    let mut in_double_quotes = false;

    for ch in command.chars() {
        match ch {
            '"' => in_double_quotes = !in_double_quotes,
            '&' | '|' | '>' | '<' | '^' | '%' | '\r' | '\n' => return true,
            '(' | ')' if !in_double_quotes => return true,
            _ => {}
        }
    }

    in_double_quotes
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

pub fn renew_dhcp_lease_blocking() -> Result<CommandResult, String> {
    let total_timeout = Duration::from_secs(NETWORK_COMMAND_TIMEOUT_SECS);
    let started = Instant::now();

    let release_output = run_process_blocking("ipconfig", &["/release"], total_timeout)
        .map_err(|err| normalize_dhcp_timeout_error(err, total_timeout))?;

    if !release_output.status.success() {
        return Ok(command_result_from_outputs(
            &[&release_output],
            false,
            "DHCP lease renewed.",
            "DHCP lease release failed.",
        ));
    }

    let renew_timeout = remaining_dhcp_timeout(started, total_timeout)?;
    let renew_output = run_process_blocking("ipconfig", &["/renew"], renew_timeout)
        .map_err(|err| normalize_dhcp_timeout_error(err, total_timeout))?;

    Ok(command_result_from_outputs(
        &[&release_output, &renew_output],
        renew_output.status.success(),
        "DHCP lease renewed.",
        "DHCP lease renew failed.",
    ))
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

        if let Some(canonical) = canonical_bloatware_package(trimmed) {
            let canonical_name = canonical.to_string();
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
            format!(
                "Requested cleanup for {} cache target(s).",
                selected_targets.len()
            ),
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
    connectivity_probe::check_connectivity().await
}

#[cfg(test)]
mod tests {
    use super::{
        collect_process_output, command_result_from_outputs,
        contains_disallowed_shell_metacharacters, normalize_dhcp_timeout_error,
        validate_network_command,
    };
    use std::os::windows::process::ExitStatusExt;
    use std::process::{ExitStatus, Output};
    use std::time::Duration;

    fn fake_output(code: u32, stdout: &str, stderr: &str) -> Output {
        Output {
            status: ExitStatus::from_raw(code),
            stdout: stdout.as_bytes().to_vec(),
            stderr: stderr.as_bytes().to_vec(),
        }
    }

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

    #[test]
    fn validate_network_command_allows_quoted_parentheses_for_adapter_names() {
        assert!(validate_network_command(
            r#"netsh interface set interface "Ethernet (Corp)" disable"#
        )
        .is_ok());
        assert!(!contains_disallowed_shell_metacharacters(
            r#"netsh interface set interface "Ethernet (Corp)" disable"#
        ));
    }

    #[test]
    fn validate_network_command_rejects_unquoted_parentheses_and_unbalanced_quotes() {
        assert!(
            validate_network_command("netsh interface set interface Ethernet (Corp) disable")
                .is_err()
        );
        assert!(
            validate_network_command(r#"netsh interface set interface "Ethernet disable"#).is_err()
        );
    }

    #[test]
    fn collect_process_output_combines_stdout_and_stderr() {
        let output = fake_output(1, "line one\n", "line two\n");

        assert_eq!(collect_process_output(&output), "line one\nline two");
    }

    #[test]
    fn command_result_from_outputs_marks_failure_when_any_step_fails() {
        let release_output = fake_output(0, "released", "");
        let renew_output = fake_output(1, "", "renew failed");

        let result = command_result_from_outputs(
            &[&release_output, &renew_output],
            false,
            "DHCP lease renewed.",
            "DHCP lease renew failed.",
        );

        assert!(!result.success);
        assert_eq!(result.output, "released\nrenew failed");
    }

    #[test]
    fn command_result_from_outputs_uses_failure_fallback_without_output() {
        let renew_output = fake_output(1, "", "");

        let result = command_result_from_outputs(
            &[&renew_output],
            false,
            "DHCP lease renewed.",
            "DHCP lease renew failed.",
        );

        assert!(!result.success);
        assert_eq!(result.output, "DHCP lease renew failed.");
    }

    #[test]
    fn normalize_dhcp_timeout_error_rewrites_step_timeout_to_overall_budget() {
        let normalized = normalize_dhcp_timeout_error(
            "Command timed out after 12s: ipconfig /renew".to_string(),
            Duration::from_secs(90),
        );

        assert_eq!(
            normalized,
            "Command timed out after 90s: ipconfig /release && ipconfig /renew"
        );
    }

}
