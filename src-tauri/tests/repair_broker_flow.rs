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

#[test]
fn repair_broker_survives_aborted_tcp_connection_and_serves_health() {
    use std::io::{Read, Write};
    use std::net::{TcpListener, TcpStream};
    use std::process::Command;
    use std::thread;
    use std::time::Duration;
    use super_route_pro_lib::repair_ipc::{decode_message, encode_message};
    use super_route_pro_lib::repair_protocol::{RepairIpcRequest, RepairIpcResponse};

    let probe_listener = TcpListener::bind(("127.0.0.1", 0)).expect("should reserve ephemeral port");
    let port = probe_listener
        .local_addr()
        .expect("should have local addr")
        .port();
    drop(probe_listener);

    let auth_token = "qc-broker-test-token";
    let mut broker = Command::new(env!("CARGO_BIN_EXE_SuperRouteRepairBroker"))
        .args([
            "--serve",
            &port.to_string(),
            auth_token,
            "qc-app",
            "qc-conn",
            &std::process::id().to_string(),
        ])
        .spawn()
        .expect("SuperRouteRepairBroker binary should spawn");

    // 1. Connect and immediately close without sending any bytes.
    let mut aborted = None;
    for _ in 0..40 {
        if let Ok(stream) = TcpStream::connect(("127.0.0.1", port)) {
            aborted = Some(stream);
            break;
        }
        thread::sleep(Duration::from_millis(50));
    }
    let aborted = aborted.expect("broker should accept initial TCP connection");
    drop(aborted);

    // 2. Follow up with a valid GetServiceHealth request.
    let send_ipc = |request: RepairServiceRequest| -> RepairServiceResponse {
        let mut client =
            TcpStream::connect(("127.0.0.1", port)).expect("broker should still accept connections");
        client
            .set_read_timeout(Some(Duration::from_secs(5)))
            .expect("should set read timeout");
        let encoded = encode_message(&RepairIpcRequest {
            auth_token: auth_token.to_string(),
            request,
        })
        .expect("request should encode");
        client.write_all(&encoded).expect("request should write");
        client.flush().expect("request should flush");

        let mut len_bytes = [0_u8; 4];
        client
            .read_exact(&mut len_bytes)
            .expect("response length should read");
        let payload_len = u32::from_le_bytes(len_bytes) as usize;
        let mut payload = vec![0_u8; payload_len];
        client
            .read_exact(&mut payload)
            .expect("response payload should read");
        let mut framed = len_bytes.to_vec();
        framed.extend(payload);
        let decoded: RepairIpcResponse = decode_message(&framed).expect("response should decode");
        decoded.response
    };

    match send_ipc(RepairServiceRequest::GetServiceHealth) {
        RepairServiceResponse::ServiceHealth(health) => {
            assert!(health.connected, "broker should remain healthy after aborted connection");
            assert!(!health.requires_unlock);
        }
        other => panic!("expected ServiceHealth response, got: {other:?}"),
    }

    // 3. Shut down broker cleanly.
    let _ = send_ipc(RepairServiceRequest::Shutdown);
    let status = broker.wait().expect("broker should exit cleanly");
    assert!(status.success(), "broker exited with {status}");
}
