use crate::network;
use crate::repair_protocol::{
    AppxRemovalRequest, ProfileCleanupRequest, RepairCommandResult, RepairMachineAction,
    RepairSessionStatus,
};
use crate::repair_targets::{
    resolve_repair_target_by_sid, validate_target_sid, RepairTargetUser,
};
use std::collections::{HashMap, HashSet};
#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;
#[cfg(target_os = "windows")]
use std::process::{Command, Stdio};

#[cfg(target_os = "windows")]
const CREATE_NO_WINDOW: u32 = 0x08000000;

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
        output: "Repair Mode is locked. Unlock Repair Mode before running admin fixes."
            .to_string(),
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
async fn run_powershell_script(script: String) -> Result<String, String> {
    tauri::async_runtime::spawn_blocking(move || {
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
    })
    .await
    .map_err(|err| format!("PowerShell task join error: {err}"))?
}

#[cfg(not(target_os = "windows"))]
async fn run_powershell_script(_script: String) -> Result<String, String> {
    Err("Repair actions are only available on Windows.".to_string())
}

fn sanitize_cleanup_targets(targets: &[String]) -> Vec<String> {
    let mut seen = HashSet::new();
    let mut selected = Vec::new();

    for target in targets {
        let trimmed = target.trim().to_lowercase();
        if trimmed.is_empty() {
            continue;
        }

        let is_safe_token = trimmed
            .chars()
            .all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '_' || ch == '-');
        if !is_safe_token || !seen.insert(trimmed.clone()) {
            continue;
        }

        if cleanup_paths_for_target(
            &RepairTargetUser {
                sid: "S-1-5-21-0".to_string(),
                account_name: "placeholder".to_string(),
                profile_path: r"C:\Users\placeholder".to_string(),
                is_loaded: false,
            },
            &trimmed,
        )
        .is_some()
        {
            selected.push(trimmed);
        }
    }

    selected
}

fn cleanup_paths_for_target(target_user: &RepairTargetUser, target: &str) -> Option<Vec<String>> {
    let profile_root = target_user.profile_path.trim_end_matches(['\\', '/']);
    let local = format!(r"{profile_root}\AppData\Local");

    match target {
        "user_temp" => Some(vec![format!(r"{local}\Temp")]),
        "windows_temp" => Some(vec![r"C:\Windows\Temp".to_string()]),
        "windows_update_cache" => Some(vec![r"C:\Windows\SoftwareDistribution\Download".to_string()]),
        "prefetch" => Some(vec![r"C:\Windows\Prefetch".to_string()]),
        "explorer_cache" => Some(vec![
            format!(r"{local}\Microsoft\Windows\Explorer"),
        ]),
        "edge_cache" => Some(vec![
            format!(r"{local}\Microsoft\Edge\User Data\Default\Cache"),
            format!(r"{local}\Microsoft\Edge\User Data\Default\Code Cache"),
            format!(r"{local}\Microsoft\Edge\User Data\Default\GPUCache"),
        ]),
        "chrome_cache" => Some(vec![
            format!(r"{local}\Google\Chrome\User Data\Default\Cache"),
            format!(r"{local}\Google\Chrome\User Data\Default\Code Cache"),
            format!(r"{local}\Google\Chrome\User Data\Default\GPUCache"),
        ]),
        "firefox_cache" => Some(vec![
            format!(r"{local}\Mozilla\Firefox\Profiles"),
        ]),
        "inet_cache" => Some(vec![format!(r"{local}\Microsoft\Windows\INetCache")]),
        "web_cache" => Some(vec![format!(r"{local}\Microsoft\Windows\WebCache")]),
        "crash_dumps" => Some(vec![format!(r"{local}\CrashDumps")]),
        "wer_reports" => Some(vec![
            r"C:\ProgramData\Microsoft\Windows\WER".to_string(),
            format!(r"{local}\Microsoft\Windows\WER"),
        ]),
        "d3d_shader_cache" => Some(vec![format!(r"{local}\D3DSCache")]),
        _ => None,
    }
}

fn cleanup_script_for_target(target_user: &RepairTargetUser, target: &str) -> Option<String> {
    let profile_root = ps_escape_single_quoted(&target_user.profile_path);

    let local_setup = format!(
        "$targetProfile = '{profile_root}'\n$targetLocalAppData = Join-Path $targetProfile 'AppData\\Local'\n"
    );

    let body = match target {
        "user_temp" => {
            "Remove-Item -Path (Join-Path $targetLocalAppData 'Temp\\*') -Recurse -Force -ErrorAction SilentlyContinue\nWrite-Output '[OK] User Temp cleaned.'"
        }
        "windows_temp" => {
            "Remove-Item -Path 'C:\\Windows\\Temp\\*' -Recurse -Force -ErrorAction SilentlyContinue\nWrite-Output '[OK] Windows Temp cleaned.'"
        }
        "windows_update_cache" => {
            "Stop-Service -Name wuauserv -Force -ErrorAction SilentlyContinue\nStop-Service -Name bits -Force -ErrorAction SilentlyContinue\nRemove-Item -Path 'C:\\Windows\\SoftwareDistribution\\Download\\*' -Recurse -Force -ErrorAction SilentlyContinue\nStart-Service -Name wuauserv -ErrorAction SilentlyContinue\nStart-Service -Name bits -ErrorAction SilentlyContinue\nWrite-Output '[OK] Windows Update cache cleaned.'"
        }
        "prefetch" => {
            "Remove-Item -Path 'C:\\Windows\\Prefetch\\*' -Recurse -Force -ErrorAction SilentlyContinue\nWrite-Output '[OK] Prefetch cleaned.'"
        }
        "explorer_cache" => {
            "Remove-Item -Path (Join-Path $targetLocalAppData 'Microsoft\\Windows\\Explorer\\thumbcache_*.db') -Force -ErrorAction SilentlyContinue\nRemove-Item -Path (Join-Path $targetLocalAppData 'Microsoft\\Windows\\Explorer\\iconcache_*.db') -Force -ErrorAction SilentlyContinue\nWrite-Output '[OK] Explorer thumbnail/icon cache cleaned.'"
        }
        "edge_cache" => {
            "$base = Join-Path $targetLocalAppData 'Microsoft\\Edge\\User Data\\Default'\nRemove-Item -Path (Join-Path $base 'Cache\\*') -Recurse -Force -ErrorAction SilentlyContinue\nRemove-Item -Path (Join-Path $base 'Code Cache\\*') -Recurse -Force -ErrorAction SilentlyContinue\nRemove-Item -Path (Join-Path $base 'GPUCache\\*') -Recurse -Force -ErrorAction SilentlyContinue\nWrite-Output '[OK] Microsoft Edge cache cleaned.'"
        }
        "chrome_cache" => {
            "$base = Join-Path $targetLocalAppData 'Google\\Chrome\\User Data\\Default'\nRemove-Item -Path (Join-Path $base 'Cache\\*') -Recurse -Force -ErrorAction SilentlyContinue\nRemove-Item -Path (Join-Path $base 'Code Cache\\*') -Recurse -Force -ErrorAction SilentlyContinue\nRemove-Item -Path (Join-Path $base 'GPUCache\\*') -Recurse -Force -ErrorAction SilentlyContinue\nWrite-Output '[OK] Google Chrome cache cleaned.'"
        }
        "firefox_cache" => {
            "$profiles = Join-Path $targetLocalAppData 'Mozilla\\Firefox\\Profiles'\nRemove-Item -Path \"$profiles\\*\\cache2\\*\" -Recurse -Force -ErrorAction SilentlyContinue\nWrite-Output '[OK] Mozilla Firefox cache cleaned.'"
        }
        "inet_cache" => {
            "Remove-Item -Path (Join-Path $targetLocalAppData 'Microsoft\\Windows\\INetCache\\*') -Recurse -Force -ErrorAction SilentlyContinue\nWrite-Output '[OK] INetCache cleaned.'"
        }
        "web_cache" => {
            "Remove-Item -Path (Join-Path $targetLocalAppData 'Microsoft\\Windows\\WebCache\\*') -Recurse -Force -ErrorAction SilentlyContinue\nWrite-Output '[OK] WebCache cleaned.'"
        }
        "crash_dumps" => {
            "Remove-Item -Path (Join-Path $targetLocalAppData 'CrashDumps\\*') -Recurse -Force -ErrorAction SilentlyContinue\nWrite-Output '[OK] Crash dumps cleaned.'"
        }
        "wer_reports" => {
            "Remove-Item -Path 'C:\\ProgramData\\Microsoft\\Windows\\WER\\*' -Recurse -Force -ErrorAction SilentlyContinue\nRemove-Item -Path (Join-Path $targetLocalAppData 'Microsoft\\Windows\\WER\\*') -Recurse -Force -ErrorAction SilentlyContinue\nWrite-Output '[OK] Windows Error Reporting (WER) cache cleaned.'"
        }
        "d3d_shader_cache" => {
            "Remove-Item -Path (Join-Path $targetLocalAppData 'D3DSCache\\*') -Recurse -Force -ErrorAction SilentlyContinue\nWrite-Output '[OK] DirectX Shader Cache cleaned.'"
        }
        _ => return None,
    };

    Some(format!(
        "$ErrorActionPreference='SilentlyContinue'\n{local_setup}{body}\n"
    ))
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
        if let Some(paths) = cleanup_paths_for_target(target_user, &target) {
            plan.extend(paths);
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

    let result =
        network::add_route(destination, mask, gateway, metric, interface_index).await?;
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
    if session_status.locked {
        return Ok(locked_result());
    }

    let result = match action {
        RepairMachineAction::AddRoute(request) => {
            network::add_route(
                request.destination,
                request.mask,
                request.gateway,
                request.metric,
                request.interface_index,
            )
            .await?
        }
        RepairMachineAction::DeleteRoute(request) => {
            network::delete_route(request.destination, request.mask).await?
        }
        RepairMachineAction::FlushRoutes => network::flush_routes().await?,
        RepairMachineAction::SetDefaultGateway(request) => {
            network::set_default_gateway(request.gateway, request.interface_index).await?
        }
        RepairMachineAction::SetWanPersistOnStartup(request) => {
            network::set_wan_persist_on_startup(request.interface_index, request.enabled).await?
        }
        RepairMachineAction::FlushDns => {
            network::run_network_command("ipconfig /flushdns".to_string()).await?
        }
        RepairMachineAction::RenewDhcpLease => {
            network::run_network_command("ipconfig /release && ipconfig /renew".to_string()).await?
        }
        RepairMachineAction::ClearArpCache => {
            network::run_network_command("netsh interface ip delete arpcache".to_string()).await?
        }
        RepairMachineAction::ResetTcpIp => {
            network::run_network_command("netsh int ip reset".to_string()).await?
        }
        RepairMachineAction::ResetWinsock => {
            network::run_network_command("netsh winsock reset".to_string()).await?
        }
        RepairMachineAction::ResetFirewall => {
            network::run_network_command("netsh advfirewall reset".to_string()).await?
        }
        RepairMachineAction::ResetWinHttpProxy => {
            network::run_network_command("netsh winhttp reset proxy".to_string()).await?
        }
        RepairMachineAction::RestartActiveAdapters => network::run_network_command(
            "powershell -NoProfile -Command Get-NetAdapter -Physical ^| Where-Object {$_.Status -eq 'Up'} ^| Restart-NetAdapter -Confirm:$false"
                .to_string(),
        )
        .await?,
    };

    Ok(from_network_result(result))
}

pub async fn clear_profile_caches(
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
        let Some(script) = cleanup_script_for_target(&target_user, &target) else {
            failed_count += 1;
            output_lines.push(format!("[FAIL] Unsupported cleanup target: {target}"));
            output_lines.push(String::new());
            continue;
        };

        match run_powershell_script(script).await {
            Ok(script_output) => {
                let clean_output = script_output.trim();
                if clean_output.is_empty() {
                    success_count += 1;
                    output_lines.push(format!("[OK] {target} cleaned."));
                } else {
                    output_lines.extend(clean_output.lines().map(|line| line.trim_end().to_string()));
                    if clean_output.contains("[FAIL]") {
                        failed_count += 1;
                    } else {
                        success_count += 1;
                    }
                }
            }
            Err(err) => {
                failed_count += 1;
                output_lines.push(format!("[FAIL] {target} cleanup error: {}", err.trim()));
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

pub async fn remove_appx_for_target(
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
        format!("Remove provisioned packages: {}", request.remove_provisioned),
        String::new(),
    ];
    let mut removed = 0u32;
    let mut skipped = 0u32;
    let mut failed = 0u32;

    for package_name in selected {
        let escaped_name = ps_escape_single_quoted(&package_name);
        let escaped_sid = ps_escape_single_quoted(&request.target_sid);
        let remove_provisioned = if request.remove_provisioned { "$true" } else { "$false" };
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

        match run_powershell_script(script).await {
            Ok(script_output) => {
                let clean_output = script_output.trim();
                if clean_output.is_empty() {
                    skipped += 1;
                    output_lines.push(format!("[SKIP] {package_name} no output returned"));
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

    Ok(RepairCommandResult {
        success: failed == 0,
        output: output_lines.join("\n"),
        requires_unlock: false,
    })
}
