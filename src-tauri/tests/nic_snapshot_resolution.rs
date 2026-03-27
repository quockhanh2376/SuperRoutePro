#![allow(dead_code)]

mod process_exec {
    pub use super_route_pro_lib::process_exec::*;
}

mod win32_net {
    pub use super_route_pro_lib::win32_net::*;
}

#[path = "../src/network_snapshot.rs"]
mod network_snapshot;
#[path = "../src/route_service_main.rs"]
mod route_service_main;

use super_route_pro_lib::route_persist::NicIdentifier;
use super_route_pro_lib::win32_net::NativeNic;

fn sample_enriched_adapters() -> Vec<NativeNic> {
    vec![
        NativeNic {
            interface_index: 22,
            description: "Broadcom NetXtreme Gigabit Ethernet".to_string(),
            mac_address: "AA-BB-CC-DD-EE-22".to_string(),
            friendly_name: "Ethernet 2".to_string(),
            ip_addresses: vec!["192.168.88.126".to_string()],
            gateways: vec!["192.168.88.1".to_string()],
            oper_status_up: true,
        },
        NativeNic {
            interface_index: 19,
            description: "Realtek PCIe GbE Family Controller".to_string(),
            mac_address: "AA-BB-CC-DD-EE-19".to_string(),
            friendly_name: "Ethernet 3".to_string(),
            ip_addresses: vec!["10.184.1.126".to_string()],
            gateways: vec![],
            oper_status_up: true,
        },
    ]
}

#[test]
fn network_snapshot_surfaces_enriched_description_for_active_nics() {
    let interfaces = network_snapshot::test_build_network_interfaces(&sample_enriched_adapters(), true);

    assert_eq!(interfaces.len(), 2);
    assert_eq!(interfaces[0].index, "22");
    assert_eq!(
        interfaces[0].description,
        "Broadcom NetXtreme Gigabit Ethernet"
    );
    assert_ne!(interfaces[0].description, "Ethernet 2");
}

#[test]
fn snapshot_description_roundtrips_into_route_service_nic_resolution() {
    let adapters = sample_enriched_adapters();
    let interfaces = network_snapshot::test_build_network_interfaces(&adapters, true);
    let persisted_nic = NicIdentifier {
        description: interfaces[0].description.clone(),
        mac_address: adapters[0].mac_address.replace('-', ":"),
    };

    assert_eq!(
        route_service_main::test_resolve_nic_interface_index_from_adapters(&persisted_nic, &adapters)
            .expect("snapshot-derived NIC identifier should resolve"),
        "22"
    );
}
