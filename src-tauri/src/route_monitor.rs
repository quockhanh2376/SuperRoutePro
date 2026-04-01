use crate::network_snapshot::{get_routing_table_blocking_with_adapters, RouteEntry};
use crate::route_apply;
use crate::route_persist::{CustomRoute, PersistConfig};
use crate::win32_net::NativeNic;
use std::time::Duration;

pub const ROUTE_MONITOR_POLL_INTERVAL: Duration = Duration::from_secs(8);
pub const ROUTE_MONITOR_CONFIRMATION_PASSES: u8 = 2;

pub fn inspect_config_drift(config: &PersistConfig) -> Result<Option<String>, String> {
    let adapters = crate::win32_net::enumerate_adapters()?;
    let routes = get_routing_table_blocking_with_adapters(&adapters)?;
    build_drift_signature(config, &routes, &adapters)
}

pub fn build_drift_signature(
    config: &PersistConfig,
    routes: &[RouteEntry],
    adapters: &[NativeNic],
) -> Result<Option<String>, String> {
    let default_interface_index =
        route_apply::resolve_nic_interface_index_from_adapters(&config.nic, adapters)?;

    if let Some(wan) = &config.wan {
        let default_routes: Vec<&RouteEntry> = routes
            .iter()
            .filter(|route| route.destination == "0.0.0.0" && route.netmask == "0.0.0.0")
            .collect();

        if default_routes.len() != 1 {
            return Ok(Some(format!("default-route-count:{}", default_routes.len())));
        }

        let active_default = default_routes[0];
        if active_default.gateway != wan.gateway
            || active_default.interface_index != default_interface_index
        {
            return Ok(Some(format!(
                "default-route-mismatch:{}:{}:{}:{}",
                active_default.gateway,
                active_default.interface_index,
                wan.gateway,
                default_interface_index
            )));
        }
    }

    let nic_index_lookup = route_apply::build_nic_index_lookup(adapters);
    for route in &config.custom_routes {
        let expected_interface_index = route_apply::resolve_custom_route_interface_index(
            route,
            &default_interface_index,
            &nic_index_lookup,
        )?;
        if !has_matching_custom_route(route, &expected_interface_index, routes) {
            return Ok(Some(format!(
                "missing-custom-route:{}/{}/{}:{}",
                route.destination, route.mask, route.gateway, expected_interface_index
            )));
        }
    }

    Ok(None)
}

fn has_matching_custom_route(
    expected_route: &CustomRoute,
    expected_interface_index: &str,
    routes: &[RouteEntry],
) -> bool {
    routes.iter().any(|route| {
        route.destination == expected_route.destination
            && route.netmask == expected_route.mask
            && route.gateway == expected_route.gateway
            && route.interface_index == expected_interface_index
    })
}

#[cfg(test)]
mod tests {
    use super::build_drift_signature;
    use crate::network_snapshot::RouteEntry;
    use crate::route_persist::{CustomRoute, NicIdentifier, PersistConfig, WanConfig};
    use crate::win32_net::NativeNic;

    fn sample_adapter() -> NativeNic {
        NativeNic {
            interface_index: 7,
            description: "Intel(R) Wi-Fi 6".to_string(),
            mac_address: "AA-BB-CC-DD-EE-FF".to_string(),
            friendly_name: "Wi-Fi".to_string(),
            ip_addresses: vec!["192.168.1.25".to_string()],
            gateways: vec!["192.168.1.1".to_string()],
            oper_status_up: true,
        }
    }

    fn sample_config() -> PersistConfig {
        PersistConfig {
            schema_version: 1,
            enabled: true,
            nic: NicIdentifier {
                description: "Intel(R) Wi-Fi 6".to_string(),
                mac_address: "AA-BB-CC-DD-EE-FF".to_string(),
            },
            wan: Some(WanConfig {
                gateway: "192.168.1.1".to_string(),
                metric: "1".to_string(),
            }),
            custom_routes: vec![CustomRoute {
                destination: "10.0.0.0".to_string(),
                mask: "255.0.0.0".to_string(),
                gateway: "192.168.1.1".to_string(),
                metric: "10".to_string(),
                nic: None,
            }],
            updated_at: None,
        }
    }

    #[test]
    fn drift_signature_is_empty_when_routes_match_config() {
        let routes = vec![
            RouteEntry {
                destination: "0.0.0.0".to_string(),
                netmask: "0.0.0.0".to_string(),
                gateway: "192.168.1.1".to_string(),
                metric: "25".to_string(),
                interface_index: "7".to_string(),
            },
            RouteEntry {
                destination: "10.0.0.0".to_string(),
                netmask: "255.0.0.0".to_string(),
                gateway: "192.168.1.1".to_string(),
                metric: "10".to_string(),
                interface_index: "7".to_string(),
            },
        ];

        assert_eq!(
            build_drift_signature(&sample_config(), &routes, &[sample_adapter()]).unwrap(),
            None
        );
    }

    #[test]
    fn drift_signature_detects_extra_default_route() {
        let routes = vec![
            RouteEntry {
                destination: "0.0.0.0".to_string(),
                netmask: "0.0.0.0".to_string(),
                gateway: "192.168.1.1".to_string(),
                metric: "25".to_string(),
                interface_index: "7".to_string(),
            },
            RouteEntry {
                destination: "0.0.0.0".to_string(),
                netmask: "0.0.0.0".to_string(),
                gateway: "10.8.0.1".to_string(),
                metric: "5".to_string(),
                interface_index: "77".to_string(),
            },
        ];

        assert_eq!(
            build_drift_signature(&sample_config(), &routes, &[sample_adapter()]).unwrap(),
            Some("default-route-count:2".to_string())
        );
    }

    #[test]
    fn drift_signature_detects_missing_custom_route() {
        let routes = vec![RouteEntry {
            destination: "0.0.0.0".to_string(),
            netmask: "0.0.0.0".to_string(),
            gateway: "192.168.1.1".to_string(),
            metric: "25".to_string(),
            interface_index: "7".to_string(),
        }];

        assert_eq!(
            build_drift_signature(&sample_config(), &routes, &[sample_adapter()]).unwrap(),
            Some("missing-custom-route:10.0.0.0/255.0.0.0/192.168.1.1:7".to_string())
        );
    }
}
