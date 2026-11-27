use crate::network_snapshot::RouteEntry;
use crate::process_exec::run_hidden_output_blocking;
use crate::route_persist::{CustomRoute, NicIdentifier, PersistConfig, WanConfig};
use crate::win32_net::NativeNic;
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SavedDefaultRoute {
    pub gateway: String,
    pub metric: String,
    pub interface_index: String,
}

pub(crate) fn extract_default_routes(routes: &[RouteEntry]) -> Vec<SavedDefaultRoute> {
    routes
        .iter()
        .filter(|route| {
            route.destination == "0.0.0.0"
                && route.netmask == "0.0.0.0"
                && !route.gateway.trim().is_empty()
                && !route.gateway.eq_ignore_ascii_case("on-link")
                && route.gateway != "0.0.0.0"
        })
        .map(|route| SavedDefaultRoute {
            gateway: route.gateway.clone(),
            metric: if route.metric.trim().is_empty() {
                "1".to_string()
            } else {
                route.metric.clone()
            },
            interface_index: route.interface_index.clone(),
        })
        .collect()
}

pub(crate) fn has_default_route(
    routes: &[RouteEntry],
    gateway: &str,
    interface_index: &str,
) -> bool {
    routes.iter().any(|route| {
        route.destination == "0.0.0.0"
            && route.netmask == "0.0.0.0"
            && route.gateway.eq_ignore_ascii_case(gateway.trim())
            && (interface_index.trim().is_empty()
                || route.interface_index == interface_index.trim())
    })
}

pub(crate) fn apply_default_gateway_with_rollback<FRead, FClear, FAdd, FRestore>(
    gateway: &str,
    metric: &str,
    interface_index: &str,
    mut read_routes: FRead,
    mut clear_defaults: FClear,
    mut add_default: FAdd,
    mut restore_default: FRestore,
) -> Result<Vec<String>, String>
where
    FRead: FnMut() -> Result<Vec<RouteEntry>, String>,
    FClear: FnMut() -> Vec<String>,
    FAdd: FnMut(&str, &str, &str) -> Result<String, String>,
    FRestore: FnMut(&SavedDefaultRoute) -> Result<String, String>,
{
    let previous_defaults = read_routes()
        .map(|routes| extract_default_routes(&routes))
        .unwrap_or_default();

    let mut output_lines = clear_defaults();
    let add_result = add_default(gateway, metric, interface_index);

    let verified = match &add_result {
        Ok(_) => match read_routes() {
            Ok(routes) => has_default_route(&routes, gateway, interface_index),
            Err(_) => false,
        },
        Err(_) => false,
    };

    if let (Ok(add_output), true) = (&add_result, verified) {
        output_lines.push(add_output.clone());
        return Ok(output_lines);
    }

    let failure_reason = match add_result {
        Ok(add_output) => format!(
            "Default route via {gateway} (if {interface_index}) was not present in the routing table after route add. Output: {}",
            add_output.trim()
        ),
        Err(err) => err,
    };

    let mut rollback_notes = Vec::new();
    for prev in &previous_defaults {
        match restore_default(prev) {
            Ok(_) => rollback_notes.push(format!(
                "restored {} (if {})",
                prev.gateway, prev.interface_index
            )),
            Err(restore_err) => rollback_notes.push(format!(
                "failed to restore {} (if {}): {}",
                prev.gateway, prev.interface_index, restore_err
            )),
        }
    }

    if rollback_notes.is_empty() {
        Err(failure_reason)
    } else {
        Err(format!(
            "{failure_reason} (Rollback: {})",
            rollback_notes.join("; ")
        ))
    }
}

pub(crate) fn restore_saved_default_route(
    route: &SavedDefaultRoute,
    persistent: bool,
) -> Result<String, String> {
    let metric = if route.metric.trim().is_empty() {
        "1"
    } else {
        route.metric.trim()
    };
    let mut args = Vec::new();
    if persistent {
        args.push("-p");
    }
    args.extend([
        "add",
        "0.0.0.0",
        "mask",
        "0.0.0.0",
        &route.gateway,
        "metric",
        metric,
    ]);
    if !route.interface_index.trim().is_empty() {
        args.push("if");
        args.push(&route.interface_index);
    }
    run_route_command(&args)
}

#[derive(Debug, Clone)]
pub struct ApplyPersistReport {
    pub default_interface_index: String,
    pub custom_route_count: usize,
    pub output_lines: Vec<String>,
}

impl ApplyPersistReport {
    pub fn summary(&self) -> String {
        let mut lines = vec![format!(
            "Persist config applied on interface {} with {} custom route(s).",
            self.default_interface_index, self.custom_route_count
        )];
        lines.extend(self.output_lines.iter().cloned());
        lines.join("\n")
    }
}

pub fn apply_persist_config(config: &PersistConfig) -> Result<ApplyPersistReport, String> {
    let adapters = crate::win32_net::enumerate_adapters()?;
    apply_persist_config_with_adapters(config, &adapters)
}

pub fn apply_persist_config_with_adapters(
    config: &PersistConfig,
    adapters: &[NativeNic],
) -> Result<ApplyPersistReport, String> {
    let interface_index = resolve_nic_interface_index_from_adapters(&config.nic, adapters)?;
    let nic_index_lookup = build_nic_index_lookup(adapters);
    let mut output_lines = Vec::new();
    let mut errors = Vec::new();

    if let Some(wan) = &config.wan {
        let metric = if wan.metric.trim().is_empty() {
            "1"
        } else {
            wan.metric.trim()
        };
        match apply_default_gateway_with_rollback(
            &wan.gateway,
            metric,
            &interface_index,
            || crate::network_snapshot::get_routing_table_blocking_with_adapters(adapters),
            clear_default_routes,
            |gw, m, if_idx| {
                let target_wan = WanConfig {
                    gateway: gw.to_string(),
                    metric: m.to_string(),
                };
                apply_wan(&target_wan, if_idx)
            },
            |saved| restore_saved_default_route(saved, false),
        ) {
            Ok(lines) => output_lines.extend(lines),
            Err(err) => errors.push(format!(
                "Failed to apply persisted WAN {} on interface {}: {err}",
                wan.gateway, interface_index
            )),
        }
    }

    for route in &config.custom_routes {
        match resolve_custom_route_interface_index(route, &interface_index, &nic_index_lookup) {
            Ok(route_interface_index) => match apply_custom_route(route, &route_interface_index) {
                Ok(output) => output_lines.push(output),
                Err(err) => errors.push(format!(
                    "Failed to apply persisted route {}/{} via {}: {err}",
                    route.destination, route.mask, route.gateway
                )),
            },
            Err(err) => errors.push(format!(
                "Failed to resolve NIC for persisted route {}/{}: {err}",
                route.destination, route.mask
            )),
        }
    }

    if errors.is_empty() {
        Ok(ApplyPersistReport {
            default_interface_index: interface_index,
            custom_route_count: config.custom_routes.len(),
            output_lines,
        })
    } else {
        Err(errors.join("\n"))
    }
}

pub fn build_nic_index_lookup(adapters: &[NativeNic]) -> HashMap<String, String> {
    let mut lookup = HashMap::new();
    for adapter in adapters {
        lookup.insert(
            adapter.description.to_ascii_lowercase(),
            adapter.interface_index.to_string(),
        );
        lookup.insert(
            adapter.friendly_name.to_ascii_lowercase(),
            adapter.interface_index.to_string(),
        );
        if !adapter.mac_address.is_empty() {
            lookup.insert(
                adapter.mac_address.replace(':', "-").to_uppercase(),
                adapter.interface_index.to_string(),
            );
        }
    }
    lookup
}

pub fn resolve_nic_interface_index_from_adapters(
    nic: &NicIdentifier,
    adapters: &[NativeNic],
) -> Result<String, String> {
    let description = nic.description.to_ascii_lowercase();
    for adapter in adapters {
        if adapter.description.to_ascii_lowercase() == description {
            return Ok(adapter.interface_index.to_string());
        }
    }

    let normalized_mac = nic.mac_address.replace(':', "-").to_uppercase();
    for adapter in adapters {
        if adapter.mac_address.to_uppercase() == normalized_mac {
            return Ok(adapter.interface_index.to_string());
        }
    }

    Err(format!(
        "No adapter matching description='{}' or MAC='{}'",
        nic.description, nic.mac_address
    ))
}

pub fn resolve_custom_route_interface_index(
    route: &CustomRoute,
    default_interface_index: &str,
    nic_index_lookup: &HashMap<String, String>,
) -> Result<String, String> {
    match &route.nic {
        Some(nic) => {
            let description = nic.description.to_ascii_lowercase();
            if let Some(index) = nic_index_lookup.get(&description) {
                return Ok(index.clone());
            }

            let normalized_mac = nic.mac_address.replace(':', "-").to_uppercase();
            if let Some(index) = nic_index_lookup.get(&normalized_mac) {
                return Ok(index.clone());
            }

            Err(format!(
                "No adapter matching description='{}' or MAC='{}'",
                nic.description, nic.mac_address
            ))
        }
        None => Ok(default_interface_index.to_string()),
    }
}

pub(crate) fn clear_default_routes() -> Vec<String> {
    let mut outputs = Vec::new();
    for _ in 0..6 {
        match run_route_command(&["delete", "0.0.0.0"]) {
            Ok(output) => {
                if !output.is_empty() {
                    outputs.push(output);
                }
            }
            Err(err) => {
                if outputs.is_empty() {
                    outputs.push(format!("Default route cleanup stopped: {err}"));
                }
                break;
            }
        }
    }
    outputs
}

fn apply_wan(wan: &WanConfig, interface_index: &str) -> Result<String, String> {
    let metric = if wan.metric.is_empty() {
        "1"
    } else {
        &wan.metric
    };

    let output = run_route_command(&[
        "add",
        "0.0.0.0",
        "mask",
        "0.0.0.0",
        &wan.gateway,
        "metric",
        metric,
        "if",
        interface_index,
    ])?;

    Ok(format!(
        "Persisted WAN gateway {} applied on interface {}. {}",
        wan.gateway, interface_index, output
    ))
}

fn apply_custom_route(route: &CustomRoute, interface_index: &str) -> Result<String, String> {
    let metric = if route.metric.is_empty() {
        "10"
    } else {
        &route.metric
    };

    let result = crate::network::add_route_blocking_with_persistence(
        route.destination.clone(),
        route.mask.clone(),
        route.gateway.clone(),
        metric.to_string(),
        Some(interface_index.to_string()),
        false,
    )?;
    if !result.success {
        return Err(result.output);
    }

    Ok(format!(
        "Persisted route {}/{} via {} applied on interface {}. {}",
        route.destination, route.mask, route.gateway, interface_index, result.output
    ))
}

fn run_route_command(args: &[&str]) -> Result<String, String> {
    let output = run_hidden_output_blocking("route", args)?;
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();

    if output.status.success() {
        Ok(stdout)
    } else if stderr.is_empty() {
        Err(stdout)
    } else if stdout.is_empty() {
        Err(stderr)
    } else {
        Err(format!("{stderr} {stdout}"))
    }
}

#[cfg(test)]
mod tests {
    use super::{apply_default_gateway_with_rollback, SavedDefaultRoute};
    use crate::network_snapshot::RouteEntry;

    fn sample_default_route(gateway: &str, interface_index: &str, metric: &str) -> RouteEntry {
        RouteEntry {
            destination: "0.0.0.0".to_string(),
            netmask: "0.0.0.0".to_string(),
            gateway: gateway.to_string(),
            interface_index: interface_index.to_string(),
            metric: metric.to_string(),
        }
    }

    #[test]
    fn apply_default_gateway_rolls_back_previous_route_when_add_fails() {
        let mut restored: Vec<SavedDefaultRoute> = Vec::new();
        let err = apply_default_gateway_with_rollback(
            "10.0.0.1",
            "1",
            "12",
            || Ok(vec![sample_default_route("192.168.1.1", "7", "25")]),
            || vec!["cleared".to_string()],
            |_, _, _| Err("route add failed".to_string()),
            |saved| {
                restored.push(saved.clone());
                Ok("OK".to_string())
            },
        )
        .expect_err("failed add should return Err after rollback");

        assert!(err.contains("route add failed"));
        assert!(err.contains("restored 192.168.1.1"));
        assert_eq!(
            restored,
            vec![SavedDefaultRoute {
                gateway: "192.168.1.1".to_string(),
                metric: "25".to_string(),
                interface_index: "7".to_string(),
            }]
        );
    }

    #[test]
    fn apply_default_gateway_rolls_back_when_route_add_exits_zero_but_route_missing() {
        let mut read_calls = 0usize;
        let mut restored: Vec<SavedDefaultRoute> = Vec::new();
        let err = apply_default_gateway_with_rollback(
            "10.0.0.1",
            "1",
            "12",
            || {
                read_calls += 1;
                if read_calls == 1 {
                    Ok(vec![sample_default_route("192.168.1.1", "7", "25")])
                } else {
                    // Simulate route.exe returning exit 0 while failing to actually install 10.0.0.1
                    Ok(Vec::new())
                }
            },
            || vec!["cleared".to_string()],
            |_, _, _| Ok("OK!".to_string()),
            |saved| {
                restored.push(saved.clone());
                Ok("OK".to_string())
            },
        )
        .expect_err("missing verified route must trigger rollback and return Err");

        assert!(err.contains("not present in the routing table"));
        assert_eq!(restored.len(), 1);
        assert_eq!(restored[0].gateway, "192.168.1.1");
    }
}
