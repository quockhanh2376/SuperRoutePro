use super_route_pro_lib::repair_ipc::{
    decode_message,
    encode_message,
    get_repair_service_health,
    get_repair_session_status,
    handle_request,
};
use super_route_pro_lib::repair_protocol::{
    AddRouteRequest, RepairCommandResult, RepairMachineAction, RepairServiceHealth,
    RepairServiceRequest, RepairServiceResponse, RepairSessionStatus,
};
use super_route_pro_lib::repair_session::RepairSessionManager;

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

#[test]
fn service_health_round_trip_uses_stable_ipc_framing() {
    let request_frame = encode_message(&RepairServiceRequest::GetServiceHealth)
        .expect("request should encode");
    let request: RepairServiceRequest =
        decode_message(&request_frame).expect("request should decode");

    let mut session_manager = RepairSessionManager::new();
    let response = handle_request(&mut session_manager, request);
    let response_frame = encode_message(&response).expect("response should encode");
    let decoded: RepairServiceResponse =
        decode_message(&response_frame).expect("response should decode");

    match decoded {
        RepairServiceResponse::ServiceHealth(health) => {
            assert!(!health.connected, "service skeleton should still be unavailable");
            assert!(
                health.requires_unlock,
                "service skeleton should keep privileged actions gated"
            );
            assert_eq!(
                health.detail.as_deref(),
                Some("Repair service is not installed or reachable yet.")
            );
        }
        RepairServiceResponse::RepairSessionStatus(_) => {
            panic!("expected service health response");
        }
        RepairServiceResponse::UnlockRepairSession(_) => {
            panic!("expected service health response");
        }
        RepairServiceResponse::RepairAction(_) => {
            panic!("expected service health response");
        }
    }
}

#[test]
fn typed_actions_serialize_expected_requests() {
    let action = RepairMachineAction::AddRoute(AddRouteRequest {
        destination: "10.10.10.0".to_string(),
        mask: "255.255.255.0".to_string(),
        gateway: "10.10.10.1".to_string(),
        metric: "5".to_string(),
        interface_index: Some("12".to_string()),
    });

    let json = serde_json::to_value(&action).expect("typed action should serialize");
    assert_eq!(json["AddRoute"]["destination"], "10.10.10.0");
    assert_eq!(json["AddRoute"]["mask"], "255.255.255.0");
    assert_eq!(json["AddRoute"]["gateway"], "10.10.10.1");
    assert_eq!(json["AddRoute"]["metric"], "5");
    assert_eq!(json["AddRoute"]["interface_index"], "12");
}

#[test]
fn typed_actions_require_unlock_before_execution() {
    let mut session_manager = RepairSessionManager::new();
    let response = handle_request(
        &mut session_manager,
        RepairServiceRequest::RunMachineAction(RepairMachineAction::ResetWinsock),
    );

    match response {
        RepairServiceResponse::RepairAction(RepairCommandResult {
            success,
            requires_unlock,
            ..
        }) => {
            assert!(!success, "locked repair mode should reject machine actions");
            assert!(
                requires_unlock,
                "locked repair mode should tell the UI to unlock first"
            );
        }
        _ => panic!("expected machine action response"),
    }
}
