use crate::repair_protocol::{
    RepairCommandResult, RepairServiceHealth, RepairServiceRequest, RepairServiceResponse,
    RepairSessionStatus,
    UnlockRepairSessionRequest, UnlockRepairSessionResponse,
};
use crate::repair_session::RepairSessionManager;
use serde::{de::DeserializeOwned, Serialize};
use std::sync::{Mutex, OnceLock};

static REPAIR_SESSION_MANAGER: OnceLock<Mutex<RepairSessionManager>> = OnceLock::new();

fn session_manager() -> &'static Mutex<RepairSessionManager> {
    REPAIR_SESSION_MANAGER.get_or_init(|| Mutex::new(RepairSessionManager::new()))
}

pub fn encode_message<T: Serialize>(message: &T) -> Result<Vec<u8>, String> {
    let payload = serde_json::to_vec(message).map_err(|err| err.to_string())?;
    let payload_len = u32::try_from(payload.len()).map_err(|_| "payload too large".to_string())?;

    let mut framed = payload_len.to_le_bytes().to_vec();
    framed.extend(payload);
    Ok(framed)
}

pub fn decode_message<T: DeserializeOwned>(bytes: &[u8]) -> Result<T, String> {
    if bytes.len() < 4 {
        return Err("message frame missing length prefix".to_string());
    }

    let mut len_bytes = [0_u8; 4];
    len_bytes.copy_from_slice(&bytes[..4]);
    let expected_len = u32::from_le_bytes(len_bytes) as usize;
    let payload = &bytes[4..];

    if payload.len() != expected_len {
        return Err("message frame length did not match payload".to_string());
    }

    serde_json::from_slice(payload).map_err(|err| err.to_string())
}

pub fn handle_request(
    session_manager: &mut RepairSessionManager,
    request: RepairServiceRequest,
) -> RepairServiceResponse {
    match request {
        RepairServiceRequest::GetServiceHealth => {
            RepairServiceResponse::ServiceHealth(RepairServiceHealth::service_unavailable())
        }
        RepairServiceRequest::GetRepairSessionStatus => {
            RepairServiceResponse::RepairSessionStatus(session_manager.status())
        }
        RepairServiceRequest::UnlockRepairSession(request) => {
            RepairServiceResponse::UnlockRepairSession(session_manager.unlock_with_request(&request))
        }
        RepairServiceRequest::RunMachineAction(_) => {
            let status = session_manager.status();
            RepairServiceResponse::RepairAction(RepairCommandResult {
                success: false,
                output: if status.locked {
                    "Repair Mode is locked. Unlock Repair Mode before running admin fixes."
                        .to_string()
                } else {
                    "Machine action dispatch is not connected to the repair service yet."
                        .to_string()
                },
                requires_unlock: status.locked,
            })
        }
    }
}

pub fn get_repair_service_health() -> RepairServiceHealth {
    let mut session_manager = session_manager()
        .lock()
        .expect("repair session manager mutex should not be poisoned");
    match handle_request(&mut session_manager, RepairServiceRequest::GetServiceHealth) {
        RepairServiceResponse::ServiceHealth(health) => health,
        RepairServiceResponse::RepairSessionStatus(_)
        | RepairServiceResponse::UnlockRepairSession(_)
        | RepairServiceResponse::RepairAction(_) => RepairServiceHealth::service_unavailable(),
    }
}

pub fn get_repair_session_status() -> RepairSessionStatus {
    let mut session_manager = session_manager()
        .lock()
        .expect("repair session manager mutex should not be poisoned");
    match handle_request(&mut session_manager, RepairServiceRequest::GetRepairSessionStatus) {
        RepairServiceResponse::RepairSessionStatus(status) => status,
        RepairServiceResponse::ServiceHealth(_)
        | RepairServiceResponse::UnlockRepairSession(_) => RepairSessionStatus::service_unavailable(),
        RepairServiceResponse::RepairAction(_) => RepairSessionStatus::service_unavailable(),
    }
}

pub fn issue_unlock_request(
    app_instance_id: &str,
    connection_id: &str,
) -> UnlockRepairSessionRequest {
    session_manager()
        .lock()
        .expect("repair session manager mutex should not be poisoned")
        .issue_unlock_request(app_instance_id.to_string(), connection_id.to_string())
}

pub fn complete_unlock_request(
    request: UnlockRepairSessionRequest,
) -> UnlockRepairSessionResponse {
    let mut session_manager = session_manager()
        .lock()
        .expect("repair session manager mutex should not be poisoned");

    match handle_request(
        &mut session_manager,
        RepairServiceRequest::UnlockRepairSession(request),
    ) {
        RepairServiceResponse::UnlockRepairSession(response) => response,
        RepairServiceResponse::ServiceHealth(_)
        | RepairServiceResponse::RepairSessionStatus(_)
        | RepairServiceResponse::RepairAction(_) => {
            UnlockRepairSessionResponse {
                unlocked: false,
                detail: Some("Repair service returned an unexpected unlock response.".to_string()),
            }
        }
    }
}

pub fn lock_repair_mode() -> RepairSessionStatus {
    let mut session_manager = session_manager()
        .lock()
        .expect("repair session manager mutex should not be poisoned");
    session_manager.lock();
    session_manager.status()
}
