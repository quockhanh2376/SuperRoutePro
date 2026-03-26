use crate::process_exec::{run_cmd_blocking, DEFAULT_CMD_TIMEOUT_SECS};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;

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
pub struct NetworkSnapshot {
    pub interfaces: Vec<NetworkInterface>,
    pub routes: Vec<RouteEntry>,
}

fn build_interface_index_lookup(adapters: &[crate::win32_net::NativeNic]) -> HashMap<String, String> {
    let mut lookup = HashMap::new();

    for nic in adapters {
        let interface_index = nic.interface_index.to_string();
        for ip_address in &nic.ip_addresses {
            lookup.insert(ip_address.clone(), interface_index.clone());
        }
    }

    lookup
}

fn parse_ipv4_route_print(
    output: &str,
    interface_index_lookup: &HashMap<String, String>,
) -> Vec<RouteEntry> {
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
            continue;
        }
        if !in_active_routes || trimmed.is_empty() {
            continue;
        }

        let parts: Vec<&str> = trimmed.split_whitespace().collect();
        if parts.len() < 5 {
            continue;
        }

        let interface_ip = parts[3];
        routes.push(RouteEntry {
            destination: parts[0].to_string(),
            netmask: parts[1].to_string(),
            gateway: parts[2].to_string(),
            metric: parts[4].to_string(),
            interface_index: interface_index_lookup
                .get(interface_ip)
                .cloned()
                .unwrap_or_default(),
        });
    }

    routes
}

fn is_blacklisted_nic(nic: &crate::win32_net::NativeNic) -> bool {
    let blacklist = [
        "virtual",
        "vmware",
        "vbox",
        "loopback",
        "wintun",
        "kernel",
        "miniport",
        "wi-fi direct",
        "tap-",
        "pseudo",
        "ethernet adapter v",
        "tailscale",
        "hyper-v",
        "vethernet",
        "default switch",
        "wsl",
        "wireguard",
    ];

    let desc_lower = nic.description.to_lowercase();
    let friendly_name_lower = nic.friendly_name.to_lowercase();

    blacklist
        .iter()
        .any(|token| desc_lower.contains(token) || friendly_name_lower.contains(token))
}

fn build_network_interfaces(
    adapters: &[crate::win32_net::NativeNic],
    active_only: bool,
) -> Vec<NetworkInterface> {
    let mut interfaces: Vec<NetworkInterface> = Vec::new();

    for nic in adapters {
        if is_blacklisted_nic(nic) {
            continue;
        }

        let first_ipv4 = nic.ip_addresses.iter().find(|a| a.contains('.')).cloned();
        let valid_ipv4 = nic
            .ip_addresses
            .iter()
            .find(|ip| crate::win32_net::is_valid_ipv4_address(ip))
            .cloned();

        if active_only && (!nic.oper_status_up || valid_ipv4.is_none()) {
            continue;
        }

        let ip = if active_only {
            valid_ipv4.unwrap_or_else(|| "0.0.0.0".to_string())
        } else {
            valid_ipv4
                .or(first_ipv4)
                .unwrap_or_else(|| "0.0.0.0".to_string())
        };

        let gateway = nic
            .gateways
            .iter()
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

    interfaces
}

fn read_ipv4_route_table_blocking() -> Result<String, String> {
    run_cmd_blocking(
        "route",
        &["print", "-4"],
        Duration::from_secs(DEFAULT_CMD_TIMEOUT_SECS),
    )
}

#[tauri::command]
pub async fn get_network_interfaces(active_only: bool) -> Result<Vec<NetworkInterface>, String> {
    let adapters =
        tauri::async_runtime::spawn_blocking(crate::win32_net::enumerate_adapters_for_snapshot)
            .await
            .map_err(|error| format!("Task join error: {error}"))??;
    Ok(build_network_interfaces(&adapters, active_only))
}

#[tauri::command]
pub async fn get_network_snapshot(active_only: bool) -> Result<NetworkSnapshot, String> {
    let adapters_task =
        tauri::async_runtime::spawn_blocking(crate::win32_net::enumerate_adapters_for_snapshot);
    let route_task = tauri::async_runtime::spawn_blocking(read_ipv4_route_table_blocking);

    let adapters = adapters_task
        .await
        .map_err(|error| format!("Network adapter task join error: {error}"))??;
    let route_output = route_task
        .await
        .map_err(|error| format!("Route snapshot task join error: {error}"))??;
    let interface_index_lookup = build_interface_index_lookup(&adapters);

    Ok(NetworkSnapshot {
        interfaces: build_network_interfaces(&adapters, active_only),
        routes: parse_ipv4_route_print(&route_output, &interface_index_lookup),
    })
}

#[tauri::command]
pub async fn get_routing_table() -> Result<Vec<RouteEntry>, String> {
    let route_task = tauri::async_runtime::spawn_blocking(read_ipv4_route_table_blocking);
    let adapters_task =
        tauri::async_runtime::spawn_blocking(crate::win32_net::enumerate_adapters_for_snapshot);

    let output = route_task
        .await
        .map_err(|error| format!("Route print task join error: {error}"))??;
    let adapters = adapters_task
        .await
        .map_err(|error| format!("Route adapter lookup task join error: {error}"))?
        .unwrap_or_default();

    Ok(parse_ipv4_route_print(
        &output,
        &build_interface_index_lookup(&adapters),
    ))
}

#[cfg(test)]
mod tests {
    use super::{
        build_interface_index_lookup, build_network_interfaces, is_blacklisted_nic,
        parse_ipv4_route_print,
    };
    use crate::win32_net::NativeNic;

    #[test]
    fn route_print_parser_maps_interface_ip_back_to_interface_index() {
        let output = r#"
===========================================================================
Interface List
  7...aa bb cc dd ee ff ...... Wi-Fi
===========================================================================
IPv4 Route Table
===========================================================================
Active Routes:
Network Destination        Netmask          Gateway       Interface  Metric
          0.0.0.0          0.0.0.0      192.168.1.1   192.168.1.10     25
      192.168.1.0    255.255.255.0         On-link    192.168.1.10    281
===========================================================================
Persistent Routes:
  None
"#;

        let adapters = vec![NativeNic {
            interface_index: 7,
            description: "Intel(R) Wi-Fi".to_string(),
            mac_address: "AA-BB-CC-DD-EE-FF".to_string(),
            friendly_name: "Wi-Fi".to_string(),
            ip_addresses: vec!["192.168.1.10".to_string()],
            gateways: vec!["192.168.1.1".to_string()],
            oper_status_up: true,
        }];

        let routes = parse_ipv4_route_print(output, &build_interface_index_lookup(&adapters));

        assert_eq!(routes.len(), 2);
        assert_eq!(routes[0].interface_index, "7");
        assert_eq!(routes[1].interface_index, "7");
        assert_eq!(routes[0].gateway, "192.168.1.1");
        assert_eq!(routes[1].gateway, "On-link");
    }

    #[test]
    fn build_network_interfaces_active_only_requires_up_nics_with_real_ipv4() {
        let adapters = vec![
            NativeNic {
                interface_index: 7,
                description: "Intel(R) Wi-Fi".to_string(),
                mac_address: "AA-BB-CC-DD-EE-FF".to_string(),
                friendly_name: "Wi-Fi".to_string(),
                ip_addresses: vec!["192.168.1.10".to_string()],
                gateways: vec!["192.168.1.1".to_string()],
                oper_status_up: true,
            },
            NativeNic {
                interface_index: 11,
                description: "USB Ethernet".to_string(),
                mac_address: "11-22-33-44-55-66".to_string(),
                friendly_name: "Ethernet 2".to_string(),
                ip_addresses: vec!["169.254.10.20".to_string()],
                gateways: vec![],
                oper_status_up: true,
            },
            NativeNic {
                interface_index: 12,
                description: "Dock Ethernet".to_string(),
                mac_address: "22-33-44-55-66-77".to_string(),
                friendly_name: "Ethernet 3".to_string(),
                ip_addresses: vec!["192.168.50.10".to_string()],
                gateways: vec!["192.168.50.1".to_string()],
                oper_status_up: false,
            },
        ];

        let interfaces = build_network_interfaces(&adapters, true);

        assert_eq!(interfaces.len(), 1);
        assert_eq!(interfaces[0].index, "7");
        assert_eq!(interfaces[0].ip, "192.168.1.10");
    }

    #[test]
    fn blacklisted_nic_detection_checks_friendly_name_and_virtual_tokens() {
        let tailscale = NativeNic {
            interface_index: 30,
            description: "Tunnel".to_string(),
            mac_address: "00-11-22-33-44-88".to_string(),
            friendly_name: "Tailscale Tunnel".to_string(),
            ip_addresses: vec!["100.115.14.41".to_string()],
            gateways: vec![],
            oper_status_up: true,
        };
        let hyper_v = NativeNic {
            interface_index: 31,
            description: "vEthernet (Default Switch)".to_string(),
            mac_address: "00-11-22-33-44-99".to_string(),
            friendly_name: "vEthernet (Default Switch)".to_string(),
            ip_addresses: vec!["172.18.224.1".to_string()],
            gateways: vec![],
            oper_status_up: true,
        };

        assert!(is_blacklisted_nic(&tailscale));
        assert!(is_blacklisted_nic(&hyper_v));
        assert!(build_network_interfaces(&[tailscale, hyper_v], true).is_empty());
    }
}
