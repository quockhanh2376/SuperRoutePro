#[path = "../src/repair_broker_main.rs"]
mod repair_broker_main;

use super_route_pro_lib::repair_protocol::{
    RepairCommandResult, RepairServiceHealth, RepairServiceRequest, RepairServiceResponse,
    RepairSessionStatus,
};
use super_route_pro_lib::repair_session::RepairSessionManager;

#[test]
fn repair_broker_unauthorized_responses_keep_privileged_calls_gated() {
    match repair_broker_main::test_unauthorized_response(&RepairServiceRequest::GetServiceHealth) {
        RepairServiceResponse::ServiceHealth(RepairServiceHealth {
            connected,
            requires_unlock,
            detail,
        }) => {
            assert!(!connected);
            assert!(requires_unlock);
            assert_eq!(detail.as_deref(), Some("Unauthorized repair host request."));
        }
        other => panic!("unexpected response: {other:?}"),
    }

    match repair_broker_main::test_unauthorized_response(&RepairServiceRequest::RunMachineAction(
        super_route_pro_lib::repair_protocol::RepairMachineAction::FlushRoutes,
    )) {
        RepairServiceResponse::RepairAction(RepairCommandResult {
            success,
            requires_unlock,
            output,
        }) => {
            assert!(!success);
            assert!(requires_unlock);
            assert_eq!(output, "Unauthorized repair host request.");
        }
        other => panic!("unexpected response: {other:?}"),
    }
}

#[test]
fn repair_broker_request_flow_reports_health_and_shutdowns_cleanly() {
    let mut session_manager = RepairSessionManager::new();
    session_manager.unlock("app-1", "conn-1");

    let mut should_exit = false;
    match repair_broker_main::test_handle_request(
        &mut session_manager,
        44561,
        RepairServiceRequest::GetServiceHealth,
        &mut should_exit,
    ) {
        RepairServiceResponse::ServiceHealth(health) => {
            assert!(health.connected);
            assert!(!health.requires_unlock);
            assert_eq!(
                health.detail.as_deref(),
                Some("Elevated repair host is active on 127.0.0.1:44561.")
            );
        }
        other => panic!("unexpected response: {other:?}"),
    }
    assert!(!should_exit);

    match repair_broker_main::test_handle_request(
        &mut session_manager,
        44561,
        RepairServiceRequest::GetRepairSessionStatus,
        &mut should_exit,
    ) {
        RepairServiceResponse::RepairSessionStatus(RepairSessionStatus {
            locked,
            connected,
            target_sid,
            requires_unlock,
        }) => {
            assert!(!locked);
            assert!(connected);
            assert_eq!(target_sid, None);
            assert!(!requires_unlock);
        }
        other => panic!("unexpected response: {other:?}"),
    }

    match repair_broker_main::test_handle_request(
        &mut session_manager,
        44561,
        RepairServiceRequest::Shutdown,
        &mut should_exit,
    ) {
        RepairServiceResponse::RepairSessionStatus(status) => {
            assert!(status.locked);
            assert!(!status.connected);
            assert!(status.requires_unlock);
        }
        other => panic!("unexpected response: {other:?}"),
    }
    assert!(should_exit);
}
