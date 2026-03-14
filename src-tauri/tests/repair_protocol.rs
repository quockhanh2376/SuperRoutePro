use super_route_pro_lib::repair_ipc::{
    get_repair_service_health,
    get_repair_session_status,
};
use super_route_pro_lib::repair_protocol::{RepairServiceHealth, RepairSessionStatus};

#[test]
fn repair_protocol_types_serialize_expected_status_fields() {
    let session = RepairSessionStatus {
        locked: true,
        connected: false,
        target_sid: Some("S-1-5-21-1000".to_string()),
        requires_unlock: true,
    };
    let session_json = serde_json::to_value(&session).expect("session should serialize");
    assert_eq!(session_json["locked"], true);
    assert_eq!(session_json["connected"], false);
    assert_eq!(session_json["target_sid"], "S-1-5-21-1000");
    assert_eq!(session_json["requires_unlock"], true);

    let health = RepairServiceHealth {
        connected: false,
        requires_unlock: true,
        detail: Some("service unavailable".to_string()),
    };
    let health_json = serde_json::to_value(&health).expect("health should serialize");
    assert_eq!(health_json["connected"], false);
    assert_eq!(health_json["requires_unlock"], true);
    assert_eq!(health_json["detail"], "service unavailable");
}

#[test]
fn repair_protocol_placeholder_ipc_returns_service_unavailable_state() {
    let session = get_repair_session_status();
    assert!(session.locked, "placeholder session should stay locked");
    assert!(
        !session.connected,
        "placeholder session should report service disconnected until the service exists"
    );
    assert_eq!(session.target_sid, None);
    assert!(
        session.requires_unlock,
        "placeholder session should keep privileged actions gated"
    );

    let health = get_repair_service_health();
    assert!(
        !health.connected,
        "placeholder health should report the repair service as unavailable"
    );
    assert!(
        health.requires_unlock,
        "placeholder health should tell the UI that privileged actions remain locked"
    );
    assert_eq!(
        health.detail.as_deref(),
        Some("Repair service is not installed or reachable yet.")
    );
}
