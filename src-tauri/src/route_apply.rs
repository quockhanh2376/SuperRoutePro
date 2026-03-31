use crate::process_exec::run_hidden_output_blocking;
use crate::route_persist::{CustomRoute, NicIdentifier, PersistConfig, WanConfig};
use crate::win32_net::NativeNic;
use std::collections::HashMap;

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
        output_lines.extend(clear_default_routes());
        match apply_wan(wan, &interface_index) {
            Ok(output) => output_lines.push(output),
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

fn clear_default_routes() -> Vec<String> {
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

    let output = run_route_command(&[
        "add",
        &route.destination,
        "mask",
        &route.mask,
        &route.gateway,
        "metric",
        metric,
        "if",
        interface_index,
    ])?;

    Ok(format!(
        "Persisted route {}/{} via {} applied on interface {}. {}",
        route.destination, route.mask, route.gateway, interface_index, output
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