#[path = "../src/route_service_main.rs"]
mod route_service_main;

use super_route_pro_lib::route_persist::{CustomRoute, NicIdentifier};
use super_route_pro_lib::win32_net::NativeNic;

fn sample_adapters() -> Vec<NativeNic> {
    vec![
        NativeNic {
            interface_index: 19,
            description: "Realtek PCIe GbE Family Controller".to_string(),
            mac_address: "E4-54-E8-E3-3A-1A".to_string(),
            friendly_name: "Ethernet".to_string(),
            ip_addresses: vec!["10.184.1.44".to_string()],
            gateways: vec!["10.184.1.1".to_string()],
            oper_status_up: true,
        },
        NativeNic {
            interface_index: 22,
            description: "Intel(R) Wi-Fi 6 AX201 160MHz".to_string(),
            mac_address: "D8-9E-F3-11-22-33".to_string(),
            friendly_name: "Wi-Fi".to_string(),
            ip_addresses: vec!["192.168.88.10".to_string()],
            gateways: vec!["192.168.88.1".to_string()],
            oper_status_up: true,
        },
    ]
}

#[test]
fn route_service_lookup_prefers_description_then_mac_and_friendly_name() {
    let adapters = sample_adapters();
    let lookup = route_service_main::test_build_nic_index_lookup(&adapters);

    assert_eq!(lookup.get("realtek pcie gbe family controller").map(String::as_str), Some("19"));
    assert_eq!(lookup.get("ethernet").map(String::as_str), Some("19"));
    assert_eq!(lookup.get("E4-54-E8-E3-3A-1A").map(String::as_str), Some("19"));
    assert_eq!(lookup.get("wi-fi").map(String::as_str), Some("22"));

    let nic = NicIdentifier {
        description: "Intel(R) Wi-Fi 6 AX201 160MHz".to_string(),
        mac_address: "00:00:00:00:00:00".to_string(),
    };
    assert_eq!(
        route_service_main::test_resolve_nic_interface_index_from_adapters(&nic, &adapters)
            .expect("description match should resolve"),
        "22"
    );

    let mac_only = NicIdentifier {
        description: "Unknown Adapter".to_string(),
        mac_address: "E4:54:E8:E3:3A:1A".to_string(),
    };
    assert_eq!(
        route_service_main::test_resolve_nic_interface_index_from_adapters(&mac_only, &adapters)
            .expect("MAC fallback should resolve"),
        "19"
    );
}

#[test]
fn route_service_custom_routes_resolve_to_the_selected_or_matched_interface() {
    let adapters = sample_adapters();
    let lookup = route_service_main::test_build_nic_index_lookup(&adapters);

    let route_with_match = CustomRoute {
        destination: "10.184.0.0".to_string(),
        mask: "255.255.255.0".to_string(),
        gateway: "10.184.1.1".to_string(),
        metric: "10".to_string(),
        nic: Some(NicIdentifier {
            description: "Ethernet".to_string(),
            mac_address: "E4-54-E8-E3-3A-1A".to_string(),
        }),
    };
    let route_without_match = CustomRoute {
        destination: "172.16.0.0".to_string(),
        mask: "255.240.0.0".to_string(),
        gateway: "172.16.1.1".to_string(),
        metric: "20".to_string(),
        nic: None,
    };

    assert_eq!(
        route_service_main::test_resolve_custom_route_interface_index(
            &route_with_match,
            "19",
            &lookup,
        )
        .expect("matching route NIC should resolve"),
        "19"
    );
    assert_eq!(
        route_service_main::test_resolve_custom_route_interface_index(
            &route_without_match,
            "22",
            &lookup,
        )
        .expect("missing route NIC should fall back to selected interface"),
        "22"
    );
}
