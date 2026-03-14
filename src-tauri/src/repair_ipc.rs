use crate::repair_protocol::{
    RepairServiceHealth, RepairServiceRequest, RepairServiceResponse, RepairSessionStatus,
};
use crate::repair_session::RepairSessionManager;
use serde::{de::DeserializeOwned, Serialize};

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
    session_manager: &RepairSessionManager,
    request: RepairServiceRequest,
) -> RepairServiceResponse {
    match request {
        RepairServiceRequest::GetServiceHealth => {
            RepairServiceResponse::ServiceHealth(RepairServiceHealth::service_unavailable())
        }
        RepairServiceRequest::GetRepairSessionStatus => {
            RepairServiceResponse::RepairSessionStatus(session_manager.status())
        }
    }
}

pub fn get_repair_service_health() -> RepairServiceHealth {
    match handle_request(
        &RepairSessionManager::new(),
        RepairServiceRequest::GetServiceHealth,
    ) {
        RepairServiceResponse::ServiceHealth(health) => health,
        RepairServiceResponse::RepairSessionStatus(_) => RepairServiceHealth::service_unavailable(),
    }
}

pub fn get_repair_session_status() -> RepairSessionStatus {
    match handle_request(
        &RepairSessionManager::new(),
        RepairServiceRequest::GetRepairSessionStatus,
    ) {
        RepairServiceResponse::RepairSessionStatus(status) => status,
        RepairServiceResponse::ServiceHealth(_) => RepairSessionStatus::service_unavailable(),
    }
}
