use crate::cache_cleanup::{
    cleanup_paths_for_profile_root, run_cleanup_for_profile_root, sanitize_cleanup_targets,
};
use crate::network;
use crate::persist_startup;
use crate::repair_protocol::{
    AppxRemovalRequest, ProfileCleanupRequest, RepairCommandResult, RepairMachineAction,
    RepairSessionStatus,
};
use crate::repair_targets::{resolve_repair_target_by_sid, validate_target_sid, RepairTargetUser};
#[cfg(target_os = "windows")]
use crate::win32_consts::CREATE_NO_WINDOW;
use std::collections::{HashMap, HashSet};
#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;
use std::path::Path;
#[cfg(target_os = "windows")]
use std::process::{Command, Stdio};

const ALLOWED_BLOATWARE_PACKAGES: [&str; 29] = [
    "Clipchamp.Clipchamp",
    "Microsoft.BingNews",
    "Microsoft.BingWeather",
    "Microsoft.GetHelp",
    "Microsoft.Getstarted",
    "Microsoft.GamingApp",
    "Microsoft.Microsoft3DViewer",
    "Microsoft.MicrosoftOfficeHub",
    "Microsoft.MicrosoftSolitaireCollection",
    "Microsoft.MixedReality.Portal",
    "Microsoft.OutlookForWindows",
    "Microsoft.People",
    "Microsoft.PowerAutomateDesktop",
    "Microsoft.SkypeApp",
    "Microsoft.Todos",
    "Microsoft.WindowsAlarms",
    "microsoft.windowscommunicationsapps",
    "Microsoft.WindowsFeedbackHub",
    "Microsoft.WindowsMaps",
    "Microsoft.Xbox.TCUI",
    "Microsoft.XboxGameOverlay",
    "Microsoft.XboxGamingOverlay",
    "Microsoft.XboxIdentityProvider",
    "Microsoft.XboxSpeechToTextOverlay",
    "Microsoft.YourPhone",
    "Microsoft.ZuneMusic",
    "Microsoft.ZuneVideo",
    "MicrosoftTeams",
    "MicrosoftCorporationII.MicrosoftFamily",
];

fn locked_result() -> RepairCommandResult {
    RepairCommandResult {
        success: false,
        output: "Repair Mode is locked. Unlock Repair Mode before running admin fixes.".to_string(),
        requires_unlock: true,
    }
}

fn from_network_result(result: network::CommandResult) -> RepairCommandResult {
    RepairCommandResult {
        success: result.success,
        output: result.output,
        requires_unlock: false,
    }
}

fn ps_escape_single_quoted(input: &str) -> String {
    input.replace('\'', "''")
}

#[cfg(target_os = "windows")]
fn run_powershell_script_blocking(script: String) -> Result<String, String> {
    let output = Command::new("powershell")
        .args(["-NoProfile", "-NonInteractive", "-Command", script.as_str()])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .map_err(|err| format!("Failed to run PowerShell cleanup command: {err}"))?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    if output.status.success() {
        Ok(stdout)
    } else if !stdout.trim().is_empty() {
        Ok(stdout)
    } else {
        Err(stderr)
    }
}

#[cfg(not(target_os = "windows"))]
fn run_powershell_script_blocking(_script: String) -> Result<String, String> {
    Err("Repair actions are only available on Windows.".to_string())
}

pub fn validate_profile_cleanup_request(request: &ProfileCleanupRequest) -> Result<(), String> {
    if !validate_target_sid(&request.target_sid) {
        return Err("Missing or invalid target SID for profile cleanup.".to_string());
    }

    if sanitize_cleanup_targets(&request.targets).is_empty() {
        return Err("No valid cleanup targets selected.".to_string());
    }

    Ok(())
}

pub fn build_profile_cleanup_plan_for_target(
    target_user: &RepairTargetUser,
    request: &ProfileCleanupRequest,
) -> Result<Vec<String>, String> {
    validate_profile_cleanup_request(request)?;

    if target_user.sid != request.target_sid {
        return Err("Cleanup request target SID does not match resolved target user.".to_string());
    }

    let mut plan = Vec::new();
    for target in sanitize_cleanup_targets(&request.targets) {
        if let Some(paths) =
            cleanup_paths_for_profile_root(Path::new(&target_user.profile_path), &target)
        {
            plan.extend(
                paths
                    .into_iter()
                    .map(|path| path.to_string_lossy().to_string()),
            );
        }
    }

    if plan.is_empty() {
        return Err("No profile cleanup paths resolved for the selected target.".to_string());
    }

    Ok(plan)
}

pub fn validate_appx_removal_request(request: &AppxRemovalRequest) -> Result<(), String> {
    if !validate_target_sid(&request.target_sid) {
        return Err("Missing or invalid target SID for Appx removal.".to_string());
    }

    let allowed: HashMap<String, &str> = ALLOWED_BLOATWARE_PACKAGES
        .iter()
        .map(|package_name| (package_name.to_lowercase(), *package_name))
        .collect();

    let valid_count = request
        .packages
        .iter()
        .filter(|package| allowed.contains_key(&package.trim().to_lowercase()))
        .count();

    if valid_count == 0 {
        return Err("No valid Appx packages selected.".to_string());
    }

    Ok(())
}

pub fn run_machine_action_blocking(
    session_status: &RepairSessionStatus,
    action: RepairMachineAction,
) -> Result<RepairCommandResult, String> {
    if session_status.locked {
        return Ok(locked_result());
    }

    let result = match action {
        RepairMachineAction::AddRoute(request) => network::add_route_blocking(
            request.destination,
            request.mask,
            request.gateway,
            request.metric,
            request.interface_index,
        )?,
        RepairMachineAction::DeleteRoute(request) => {
            network::delete_route_blocking(request.destination, request.mask)?
        }
        RepairMachineAction::FlushRoutes => network::flush_routes_blocking()?,
        RepairMachineAction::SetDefaultGateway(request) => {
            network::set_default_gateway_blocking(request.gateway, request.interface_index)?
        }
        RepairMachineAction::SetWanPersistOnStartup(request) => {
            network::set_wan_persist_on_startup_blocking(request.interface_index, request.enabled)?
        }
        RepairMachineAction::SavePersistConfig(request) => {
            persist_startup::save_enabled_config(&request.config)?;
            network::CommandResult {
                success: true,
                output: format!(
                    "Persist startup config saved for '{}'.",
                    request.config.nic.description
                ),
            }
        }
        RepairMachineAction::ClearPersistConfig => {
            persist_startup::clear_persisted_startup_state()?;
            network::CommandResult {
                success: true,
                output: "Persist startup config cleared.".to_string(),
            }
        }
        RepairMachineAction::FlushDns => {
            network::run_network_command_blocking("ipconfig /flushdns".to_string())?
        }
        RepairMachineAction::RenewDhcpLease => network::renew_dhcp_lease_blocking()?,
        RepairMachineAction::ClearArpCache => {
            network::run_network_command_blocking("netsh interface ip delete arpcache".to_string())?
        }
        RepairMachineAction::ResetTcpIp => {
            network::run_network_command_blocking("netsh int ip reset".to_string())?
        }
        RepairMachineAction::ResetWinsock => {
            network::run_network_command_blocking("netsh winsock reset".to_string())?
        }
        RepairMachineAction::ResetFirewall => {
            network::run_network_command_blocking("netsh advfirewall reset".to_string())?
        }
        RepairMachineAction::ResetWinHttpProxy => {
            network::run_network_command_blocking("netsh winhttp reset proxy".to_string())?
        }
        RepairMachineAction::RestartActiveAdapters => {
            // Enumerate physical adapters that are up, then disable+enable each via netsh
            match crate::win32_net::enumerate_adapters_basic() {
                Ok(adapters) => {
                    let mut restarted = 0;
                    let mut errors = Vec::new();
                    for nic in adapters
                        .iter()
                        .filter(|a| a.oper_status_up && !a.friendly_name.is_empty())
                    {
                        let name = &nic.friendly_name;
                        let disable = network::run_network_command_blocking(format!(
                            "netsh interface set interface \"{}\" disable",
                            name
                        ));
                        let enable = network::run_network_command_blocking(format!(
                            "netsh interface set interface \"{}\" enable",
                            name
                        ));
                        match (disable, enable) {
                            (Ok(_), Ok(_)) => restarted += 1,
                            _ => errors.push(format!("Failed to restart adapter: {}", name)),
                        }
                    }
                    if errors.is_empty() {
                        network::CommandResult {
                            success: true,
                            output: format!("Restarted {} active adapter(s).", restarted),
                        }
                    } else {
                        network::CommandResult {
                            success: false,
                            output: format!(
                                "Restarted {} adapter(s). Errors: {}",
                                restarted,
                                errors.join("; ")
                            ),
                        }
                    }
                }
                Err(e) => network::CommandResult {
                    success: false,
                    output: format!("Failed to enumerate adapters: {}", e),
                },
            }
        }
    };

    Ok(from_network_result(result))
}

pub async fn add_route(
    session_status: &RepairSessionStatus,
    destination: String,
    mask: String,
    gateway: String,
    metric: String,
    interface_index: Option<String>,
) -> Result<RepairCommandResult, String> {
    if session_status.locked {
        return Ok(locked_result());
    }

    let result = network::add_route(destination, mask, gateway, metric, interface_index).await?;
    Ok(from_network_result(result))
}

pub async fn delete_route(
    session_status: &RepairSessionStatus,
    destination: String,
    mask: String,
) -> Result<RepairCommandResult, String> {
    if session_status.locked {
        return Ok(locked_result());
    }

    let result = network::delete_route(destination, mask).await?;
    Ok(from_network_result(result))
}

pub async fn flush_routes(
    session_status: &RepairSessionStatus,
) -> Result<RepairCommandResult, String> {
    if session_status.locked {
        return Ok(locked_result());
    }

    let result = network::flush_routes().await?;
    Ok(from_network_result(result))
}

pub async fn set_default_gateway(
    session_status: &RepairSessionStatus,
    gateway: String,
    interface_index: String,
) -> Result<RepairCommandResult, String> {
    if session_status.locked {
        return Ok(locked_result());
    }

    let result = network::set_default_gateway(gateway, interface_index).await?;
    Ok(from_network_result(result))
}

pub async fn set_wan_persist_on_startup(
    session_status: &RepairSessionStatus,
    interface_index: String,
    enabled: bool,
) -> Result<RepairCommandResult, String> {
    if session_status.locked {
        return Ok(locked_result());
    }

    let result = network::set_wan_persist_on_startup(interface_index, enabled).await?;
    Ok(from_network_result(result))
}

pub async fn run_machine_action(
    session_status: &RepairSessionStatus,
    action: RepairMachineAction,
) -> Result<RepairCommandResult, String> {
    let session = session_status.clone();
    tauri::async_runtime::spawn_blocking(move || run_machine_action_blocking(&session, action))
        .await
        .map_err(|err| format!("Repair machine action task join error: {err}"))?
}

pub fn clear_profile_caches_blocking(
    session_status: &RepairSessionStatus,
    request: ProfileCleanupRequest,
) -> Result<RepairCommandResult, String> {
    if session_status.locked {
        return Ok(locked_result());
    }

    validate_profile_cleanup_request(&request)?;
    let target_user = resolve_repair_target_by_sid(&request.target_sid)?;
    let selected_targets = sanitize_cleanup_targets(&request.targets);

    let mut output_lines = vec![
        format!(
            "Requested cleanup for {} target(s) on {} ({})",
            selected_targets.len(),
            target_user.account_name,
            target_user.sid
        ),
        format!("Resolved profile root: {}", target_user.profile_path),
        String::new(),
    ];
    let mut success_count = 0u32;
    let mut failed_count = 0u32;

    for target in selected_targets {
        output_lines.push(format!("[TARGET] {target}"));
        match run_cleanup_for_profile_root(Path::new(&target_user.profile_path), &target) {
            Some((success, detail)) => {
                output_lines.push(detail);
                if success {
                    success_count += 1;
                } else {
                    failed_count += 1;
                }
            }
            None => {
                failed_count += 1;
                output_lines.push(format!("[FAIL] Unsupported cleanup target: {target}"));
            }
        }
        output_lines.push(String::new());
    }

    output_lines.push(format!(
        "Summary: success={} failed={}",
        success_count, failed_count
    ));

    Ok(RepairCommandResult {
        success: failed_count == 0,
        output: output_lines.join("\n"),
        requires_unlock: false,
    })
}

pub async fn clear_profile_caches(
    session_status: &RepairSessionStatus,
    request: ProfileCleanupRequest,
) -> Result<RepairCommandResult, String> {
    let session = session_status.clone();
    tauri::async_runtime::spawn_blocking(move || clear_profile_caches_blocking(&session, request))
        .await
        .map_err(|err| format!("Profile cleanup task join error: {err}"))?
}

pub fn remove_appx_for_target_blocking(
    session_status: &RepairSessionStatus,
    request: AppxRemovalRequest,
) -> Result<RepairCommandResult, String> {
    if session_status.locked {
        return Ok(locked_result());
    }

    validate_appx_removal_request(&request)?;
    let target_user = resolve_repair_target_by_sid(&request.target_sid)?;

    let allowed: HashMap<String, &str> = ALLOWED_BLOATWARE_PACKAGES
        .iter()
        .map(|package_name| (package_name.to_lowercase(), *package_name))
        .collect();
    let mut seen = HashSet::new();
    let selected: Vec<String> = request
        .packages
        .iter()
        .filter_map(|package| {
            let canonical = allowed.get(&package.trim().to_lowercase())?;
            let canonical_name = (*canonical).to_string();
            if seen.insert(canonical_name.clone()) {
                Some(canonical_name)
            } else {
                None
            }
        })
        .collect();

    let mut output_lines = vec![
        format!(
            "Requested Appx removal for {} package(s) on {} ({})",
            selected.len(),
            target_user.account_name,
            target_user.sid
        ),
        format!(
            "Remove provisioned packages: {}",
            request.remove_provisioned
        ),
        String::new(),
    ];
    let mut removed = 0u32;
    let mut skipped = 0u32;
    let mut failed = 0u32;

    for package_name in selected {
        let escaped_name = ps_escape_single_quoted(&package_name);
        let escaped_sid = ps_escape_single_quoted(&request.target_sid);
        let remove_provisioned = if request.remove_provisioned {
            "$true"
        } else {
            "$false"
        };
        let script = format!(
            r#"
$pkgName = '{escaped_name}'
$targetSid = '{escaped_sid}'
$removeProvisioned = {remove_provisioned}
$hasFailure = $false
$removedInstalled = 0
$removedProvisioned = 0

$installedMatches = Get-AppxPackage -User $targetSid -ErrorAction SilentlyContinue | Where-Object {{ $_.Name -eq $pkgName }}
foreach ($pkg in $installedMatches) {{
  try {{
    Remove-AppxPackage -Package $pkg.PackageFullName -ErrorAction Stop | Out-Null
    $removedInstalled++
  }} catch {{
    $hasFailure = $true
    Write-Output "[FAIL] $pkgName installed remove error: $($_.Exception.Message)"
  }}
}}

if ($removeProvisioned) {{
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
}}

if ($removedInstalled -gt 0 -or $removedProvisioned -gt 0) {{
  Write-Output "[OK] $pkgName removed installed=$removedInstalled provisioned=$removedProvisioned"
}} elseif ($hasFailure) {{
  Write-Output "[WARN] $pkgName no removal completed"
}} else {{
  Write-Output "[SKIP] $pkgName not installed for target user"
}}
"#
        );

        match run_powershell_script_blocking(script) {
            Ok(script_output) => {
                let clean_output = script_output.trim();
                if clean_output.is_empty() {
                    skipped += 1;
                    output_lines.push(format!("[SKIP] {package_name} no output returned"));
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

    Ok(RepairCommandResult {
        success: failed == 0,
        output: output_lines.join("\n"),
        requires_unlock: false,
    })
}

pub async fn remove_appx_for_target(
    session_status: &RepairSessionStatus,
    request: AppxRemovalRequest,
) -> Result<RepairCommandResult, String> {
    let session = session_status.clone();
    tauri::async_runtime::spawn_blocking(move || remove_appx_for_target_blocking(&session, request))
        .await
        .map_err(|err| format!("Appx removal task join error: {err}"))?
}
