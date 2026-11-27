use crate::bloatware_catalog::canonical_bloatware_package;
use crate::cache_cleanup::{
    cleanup_paths_for_profile_root, run_cleanup_for_profile_root, sanitize_cleanup_targets,
};
use crate::network;
use crate::persist_startup;
use crate::process_exec::{
    ps_escape_single_quoted, run_powershell_blocking, DEFAULT_POWERSHELL_TIMEOUT_SECS,
};
use crate::repair_protocol::{
    AppxRemovalRequest, ProfileCleanupRequest, RepairCommandResult, RepairMachineAction,
    RepairSessionStatus,
};
use crate::repair_targets::{resolve_repair_target_by_sid, validate_target_sid, RepairTargetUser};
use std::collections::HashSet;
use std::path::Path;
use std::time::Duration;

fn locked_result_if_needed(session_status: &RepairSessionStatus) -> Option<RepairCommandResult> {
    session_status.locked.then(RepairCommandResult::locked)
}

fn selected_cleanup_targets(request: &ProfileCleanupRequest) -> Vec<String> {
    sanitize_cleanup_targets(&request.targets)
}

fn cleanup_plan_for_profile_root(profile_root: &Path, targets: &[String]) -> Vec<String> {
    targets
        .iter()
        .filter_map(|target| cleanup_paths_for_profile_root(profile_root, target))
        .flat_map(|paths| paths.into_iter().map(|path| path.to_string_lossy().to_string()))
        .collect()
}

fn selected_appx_packages(request: &AppxRemovalRequest) -> Vec<String> {
    let mut seen = HashSet::new();

    request
        .packages
        .iter()
        .filter_map(|package| {
            let canonical_name = canonical_bloatware_package(package)?.to_string();
            seen.insert(canonical_name.clone())
                .then_some(canonical_name)
        })
        .collect()
}

fn validated_cleanup_targets(request: &ProfileCleanupRequest) -> Result<Vec<String>, String> {
    if !validate_target_sid(&request.target_sid) {
        return Err("Missing or invalid target SID for profile cleanup.".to_string());
    }

    let selected = selected_cleanup_targets(request);
    if selected.is_empty() {
        return Err("No valid cleanup targets selected.".to_string());
    }

    Ok(selected)
}

fn validated_appx_packages(request: &AppxRemovalRequest) -> Result<Vec<String>, String> {
    if !validate_target_sid(&request.target_sid) {
        return Err("Missing or invalid target SID for Appx removal.".to_string());
    }

    let selected = selected_appx_packages(request);
    if selected.is_empty() {
        return Err("No valid Appx packages selected.".to_string());
    }

    Ok(selected)
}

pub fn validate_profile_cleanup_request(request: &ProfileCleanupRequest) -> Result<(), String> {
    validated_cleanup_targets(request).map(|_| ())
}

pub fn build_profile_cleanup_plan_for_target(
    target_user: &RepairTargetUser,
    request: &ProfileCleanupRequest,
) -> Result<Vec<String>, String> {
    let selected_targets = validated_cleanup_targets(request)?;

    if target_user.sid != request.target_sid {
        return Err("Cleanup request target SID does not match resolved target user.".to_string());
    }

    let plan = cleanup_plan_for_profile_root(Path::new(&target_user.profile_path), &selected_targets);

    if plan.is_empty() {
        return Err("No profile cleanup paths resolved for the selected target.".to_string());
    }

    Ok(plan)
}

pub fn validate_appx_removal_request(request: &AppxRemovalRequest) -> Result<(), String> {
    validated_appx_packages(request).map(|_| ())
}

pub fn run_machine_action_blocking(
    session_status: &RepairSessionStatus,
    action: RepairMachineAction,
) -> Result<RepairCommandResult, String> {
    if let Some(result) = locked_result_if_needed(session_status) {
        return Ok(result);
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
        RepairMachineAction::ApplyPersistConfig(request) => {
            let report = crate::route_apply::apply_persist_config(&request.config)?;
            network::CommandResult {
                success: true,
                output: report.summary(),
            }
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
        RepairMachineAction::FlushDns => network::flush_dns_blocking()?,
        RepairMachineAction::RenewDhcpLease => network::renew_dhcp_lease_blocking()?,
        RepairMachineAction::ClearArpCache => network::clear_arp_cache_blocking()?,
        RepairMachineAction::ResetTcpIp => network::reset_tcp_ip_blocking()?,
        RepairMachineAction::ResetWinsock => network::reset_winsock_blocking()?,
        RepairMachineAction::ResetFirewall => network::reset_firewall_blocking()?,
        RepairMachineAction::ResetWinHttpProxy => network::reset_winhttp_proxy_blocking()?,
        RepairMachineAction::RestartActiveAdapters => {
            match crate::win32_net::enumerate_adapters_basic() {
                Ok(adapters) => {
                    let mut restarted = 0;
                    let mut errors = Vec::new();
                    for nic in adapters
                        .iter()
                        .filter(|a| a.oper_status_up && !a.friendly_name.is_empty())
                    {
                        let name = &nic.friendly_name;
                        let disable = network::set_adapter_enabled_blocking(name, false);
                        let enable = disable
                            .as_ref()
                            .ok()
                            .and_then(|result| result.success.then_some(()))
                            .map(|_| network::set_adapter_enabled_blocking(name, true));
                        match (disable, enable) {
                            (Ok(disable_result), Some(Ok(enable_result)))
                                if disable_result.success && enable_result.success =>
                            {
                                restarted += 1
                            }
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

    Ok(result.into())
}

pub async fn add_route(
    session_status: &RepairSessionStatus,
    destination: String,
    mask: String,
    gateway: String,
    metric: String,
    interface_index: Option<String>,
) -> Result<RepairCommandResult, String> {
    if let Some(result) = locked_result_if_needed(session_status) {
        return Ok(result);
    }

    let result = network::add_route(destination, mask, gateway, metric, interface_index).await?;
    Ok(result.into())
}

pub async fn delete_route(
    session_status: &RepairSessionStatus,
    destination: String,
    mask: String,
) -> Result<RepairCommandResult, String> {
    if let Some(result) = locked_result_if_needed(session_status) {
        return Ok(result);
    }

    let result = network::delete_route(destination, mask).await?;
    Ok(result.into())
}

pub async fn flush_routes(
    session_status: &RepairSessionStatus,
) -> Result<RepairCommandResult, String> {
    if let Some(result) = locked_result_if_needed(session_status) {
        return Ok(result);
    }

    let result = network::flush_routes().await?;
    Ok(result.into())
}

pub async fn set_default_gateway(
    session_status: &RepairSessionStatus,
    gateway: String,
    interface_index: String,
) -> Result<RepairCommandResult, String> {
    if let Some(result) = locked_result_if_needed(session_status) {
        return Ok(result);
    }

    let result = network::set_default_gateway(gateway, interface_index).await?;
    Ok(result.into())
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
    if let Some(result) = locked_result_if_needed(session_status) {
        return Ok(result);
    }

    let selected_targets = validated_cleanup_targets(&request)?;
    let target_user = resolve_repair_target_by_sid(&request.target_sid)?;

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
    if let Some(result) = locked_result_if_needed(session_status) {
        return Ok(result);
    }

    let selected = validated_appx_packages(&request)?;
    let target_user = resolve_repair_target_by_sid(&request.target_sid)?;

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
    let script = build_batch_appx_removal_script(
        &selected,
        &request.target_sid,
        request.remove_provisioned,
    );
    let timeout = appx_removal_timeout(selected.len());

    let (removed, skipped, failed) = match run_powershell_blocking(&script, timeout) {
        Ok(script_output) => {
            let clean_output = script_output.trim();
            if clean_output.is_empty() {
                for package_name in &selected {
                    output_lines.push(format!("[SKIP] {package_name} no output returned"));
                }
                (0, selected.len() as u32, 0)
            } else {
                output_lines
                    .extend(clean_output.lines().map(|line| line.trim_end().to_string()));
                summarize_batch_appx_output(&selected, clean_output)
            }
        }
        Err(err) => {
            for package_name in &selected {
                output_lines.push(format!(
                    "[FAIL] {} command execution failed: {}",
                    package_name,
                    err.trim()
                ));
            }
            (0, 0, selected.len() as u32)
        }
    };

    output_lines.push(String::new());
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

const APPX_REMOVAL_TIMEOUT_BASE_SECS: u64 = 30;
const APPX_REMOVAL_TIMEOUT_PER_PACKAGE_SECS: u64 = 20;
const APPX_REMOVAL_TIMEOUT_MAX_SECS: u64 = 280;

fn appx_removal_timeout(package_count: usize) -> Duration {
    let scaled = APPX_REMOVAL_TIMEOUT_BASE_SECS
        .saturating_add((package_count as u64).saturating_mul(APPX_REMOVAL_TIMEOUT_PER_PACKAGE_SECS));
    Duration::from_secs(scaled.clamp(DEFAULT_POWERSHELL_TIMEOUT_SECS, APPX_REMOVAL_TIMEOUT_MAX_SECS))
}

fn summarize_batch_appx_output(selected: &[String], script_output: &str) -> (u32, u32, u32) {
    let mut removed = 0u32;
    let mut skipped = 0u32;
    let mut failed = 0u32;

    for package_name in selected {
        let fail_prefix = format!("[FAIL] {package_name} ");
        let ok_prefix = format!("[OK] {package_name} ");
        let skip_prefix = format!("[SKIP] {package_name} ");

        let mut pkg_failed = false;
        let mut pkg_ok = false;
        let mut pkg_skip = false;

        for line in script_output.lines().map(str::trim) {
            if line.starts_with(&fail_prefix) {
                pkg_failed = true;
            } else if line.starts_with(&ok_prefix) {
                pkg_ok = true;
            } else if line.starts_with(&skip_prefix) {
                pkg_skip = true;
            }
        }

        if pkg_failed {
            failed += 1;
        } else if pkg_ok {
            removed += 1;
        } else if pkg_skip {
            skipped += 1;
        } else {
            skipped += 1;
        }
    }

    (removed, skipped, failed)
}

fn build_batch_appx_removal_script(
    selected: &[String],
    target_sid: &str,
    remove_provisioned: bool,
) -> String {
    let escaped_sid = ps_escape_single_quoted(target_sid);
    let remove_provisioned_literal = if remove_provisioned { "$true" } else { "$false" };
    let package_literals = selected
        .iter()
        .map(|name| format!("'{}'", ps_escape_single_quoted(name)))
        .collect::<Vec<_>>()
        .join(", ");

    format!(
        r#"
$targetSid = '{escaped_sid}'
$removeProvisioned = {remove_provisioned_literal}
$packages = @({package_literals})

$allInstalled = @(Get-AppxPackage -User $targetSid -ErrorAction SilentlyContinue)
$allProvisioned = @()
$provisionedQueryError = $null

if ($removeProvisioned) {{
  try {{
    $allProvisioned = @(Get-AppxProvisionedPackage -Online -ErrorAction Stop)
  }} catch {{
    $provisionedQueryError = $_.Exception.Message
  }}
}}

foreach ($pkgName in $packages) {{
  $hasFailure = $false
  $removedInstalled = 0
  $removedProvisioned = 0

  $installedMatches = @($allInstalled | Where-Object {{ $_.Name -eq $pkgName }})
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
    if ($null -ne $provisionedQueryError) {{
      $hasFailure = $true
      Write-Output "[FAIL] $pkgName provisioned query error: $provisionedQueryError"
    }} else {{
      $provisionedMatches = @($allProvisioned | Where-Object {{ $_.DisplayName -eq $pkgName }})
      foreach ($prov in $provisionedMatches) {{
        try {{
          Remove-AppxProvisionedPackage -Online -PackageName $prov.PackageName -ErrorAction Stop | Out-Null
          $removedProvisioned++
        }} catch {{
          $hasFailure = $true
          Write-Output "[FAIL] $pkgName provisioned remove error: $($_.Exception.Message)"
        }}
      }}
    }}
  }}

  if ($removedInstalled -gt 0 -or $removedProvisioned -gt 0) {{
    Write-Output "[OK] $pkgName removed installed=$removedInstalled provisioned=$removedProvisioned"
  }} elseif ($hasFailure) {{
    Write-Output "[WARN] $pkgName no removal completed"
  }} else {{
    Write-Output "[SKIP] $pkgName not installed for target user"
  }}
}}
"#
    )
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

#[cfg(test)]
mod tests {
    use super::{
        build_profile_cleanup_plan_for_target, selected_appx_packages, selected_cleanup_targets,
        validate_appx_removal_request, validate_profile_cleanup_request,
    };
    use crate::repair_protocol::{AppxRemovalRequest, ProfileCleanupRequest};
    use crate::repair_targets::RepairTargetUser;

    fn sample_target_user() -> RepairTargetUser {
        RepairTargetUser {
            sid: "S-1-5-21-123456789-234567890-345678901-1001".to_string(),
            account_name: "demo".to_string(),
            profile_path: r"C:\Users\demo".to_string(),
            is_loaded: true,
        }
    }

    #[test]
    fn profile_cleanup_request_requires_valid_sid_and_known_targets() {
        let invalid_sid = ProfileCleanupRequest {
            target_sid: "demo".to_string(),
            targets: vec!["user_temp".to_string()],
        };
        assert_eq!(
            validate_profile_cleanup_request(&invalid_sid).unwrap_err(),
            "Missing or invalid target SID for profile cleanup."
        );

        let no_valid_targets = ProfileCleanupRequest {
            target_sid: sample_target_user().sid,
            targets: vec!["../oops".to_string()],
        };
        assert_eq!(
            validate_profile_cleanup_request(&no_valid_targets).unwrap_err(),
            "No valid cleanup targets selected."
        );
    }

    #[test]
    fn profile_cleanup_plan_expands_known_targets_under_target_profile() {
        let request = ProfileCleanupRequest {
            target_sid: sample_target_user().sid.clone(),
            targets: vec!["user_temp".to_string(), "chrome_cache".to_string()],
        };

        let plan = build_profile_cleanup_plan_for_target(&sample_target_user(), &request)
            .expect("known cleanup targets should resolve");

        assert_eq!(plan.len(), 4);
        assert!(plan
            .iter()
            .any(|path| path.ends_with(r"demo\AppData\Local\Temp")));
        assert!(plan
            .iter()
            .any(|path| path.ends_with(r"Google\Chrome\User Data\Default\Cache")));
        assert!(plan
            .iter()
            .any(|path| path.ends_with(r"Google\Chrome\User Data\Default\Code Cache")));
        assert!(plan
            .iter()
            .any(|path| path.ends_with(r"Google\Chrome\User Data\Default\GPUCache")));
    }

    #[test]
    fn profile_cleanup_plan_rejects_mismatched_target_sid() {
        let request = ProfileCleanupRequest {
            target_sid: "S-1-5-21-000-111-222-1001".to_string(),
            targets: vec!["user_temp".to_string()],
        };

        assert_eq!(
            build_profile_cleanup_plan_for_target(&sample_target_user(), &request).unwrap_err(),
            "Cleanup request target SID does not match resolved target user."
        );
    }

    #[test]
    fn selected_cleanup_targets_deduplicates_and_filters_invalid_tokens() {
        let request = ProfileCleanupRequest {
            target_sid: sample_target_user().sid,
            targets: vec![
                " user_temp ".to_string(),
                "USER_TEMP".to_string(),
                "../oops".to_string(),
                "chrome_cache".to_string(),
            ],
        };

        let selected = selected_cleanup_targets(&request);
        assert_eq!(selected, vec!["user_temp", "chrome_cache"]);
    }

    #[test]
    fn appx_removal_request_accepts_known_packages_case_insensitively() {
        let request = AppxRemovalRequest {
            target_sid: sample_target_user().sid,
            packages: vec!["microsoft.skypeapp".to_string()],
            remove_provisioned: false,
        };

        validate_appx_removal_request(&request)
            .expect("known package names should validate case-insensitively");
    }

    #[test]
    fn appx_removal_request_rejects_empty_or_unknown_package_selection() {
        let request = AppxRemovalRequest {
            target_sid: sample_target_user().sid,
            packages: vec!["Contoso.UnknownApp".to_string()],
            remove_provisioned: false,
        };

        assert_eq!(
            validate_appx_removal_request(&request).unwrap_err(),
            "No valid Appx packages selected."
        );
    }

    #[test]
    fn selected_appx_packages_canonicalizes_case_and_deduplicates() {
        let request = AppxRemovalRequest {
            target_sid: sample_target_user().sid,
            packages: vec![
                "microsoft.skypeapp".to_string(),
                "Microsoft.SkypeApp".to_string(),
                "Contoso.UnknownApp".to_string(),
            ],
            remove_provisioned: false,
        };

        let selected = selected_appx_packages(&request);
        assert_eq!(selected, vec!["Microsoft.SkypeApp"]);
    }

    #[test]
    fn appx_removal_timeout_scales_with_package_count_within_ipc_budget() {
        use std::time::Duration;
        assert_eq!(super::appx_removal_timeout(1), Duration::from_secs(50));
        assert_eq!(super::appx_removal_timeout(5), Duration::from_secs(130));
        assert_eq!(super::appx_removal_timeout(20), Duration::from_secs(280));
        assert!(super::appx_removal_timeout(100) < Duration::from_secs(300));
    }

    #[test]
    fn summarize_batch_appx_output_counts_ok_skip_and_fail_per_package() {
        let selected = vec![
            "Microsoft.BingNews".to_string(),
            "Microsoft.SkypeApp".to_string(),
            "Microsoft.GamingApp".to_string(),
        ];
        let output = "\
[OK] Microsoft.BingNews removed installed=1 provisioned=1
[SKIP] Microsoft.SkypeApp not installed for target user
[FAIL] Microsoft.GamingApp installed remove error: Access denied
[WARN] Microsoft.GamingApp no removal completed";

        assert_eq!(super::summarize_batch_appx_output(&selected, output), (1, 1, 1));
    }
}
