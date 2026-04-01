use crate::repair_protocol::{
    AppxRemovalRequest, ProfileCleanupRequest, RepairCommandResult, RepairIpcRequest,
    RepairIpcResponse, RepairMachineAction, RepairServiceHealth, RepairServiceRequest,
    RepairServiceResponse, RepairSessionStatus, UnlockRepairSessionRequest,
    UnlockRepairSessionResponse,
};
use crate::repair_session::RepairSessionManager;
use serde::{de::DeserializeOwned, Serialize};
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

static REPAIR_SESSION_MANAGER: OnceLock<Mutex<RepairSessionManager>> = OnceLock::new();
static ACTIVE_REPAIR_HOST: OnceLock<Mutex<Option<RepairHostConnection>>> = OnceLock::new();

const REPAIR_HOST_TIMEOUT: Duration = Duration::from_secs(15);
const REPAIR_HOST_CONNECT_TIMEOUT: Duration = Duration::from_secs(2);
const REPAIR_HOST_UNLOCK_WAIT: Duration = Duration::from_secs(12);
const REPAIR_HOST_UNLOCK_POLL_INTERVAL: Duration = Duration::from_millis(250);

#[derive(Debug, Clone)]
struct RepairHostConnection {
    port: u16,
    auth_token: String,
    connection_id: String,
}

fn session_manager() -> &'static Mutex<RepairSessionManager> {
    REPAIR_SESSION_MANAGER.get_or_init(|| Mutex::new(RepairSessionManager::new()))
}

fn active_repair_host() -> &'static Mutex<Option<RepairHostConnection>> {
    ACTIVE_REPAIR_HOST.get_or_init(|| Mutex::new(None))
}

fn locked_result() -> RepairCommandResult {
    RepairCommandResult {
        success: false,
        output: "Repair Mode is locked. Unlock Repair Mode before running admin fixes.".to_string(),
        requires_unlock: true,
    }
}

fn mark_host_disconnected(connection_id: &str) {
    {
        let mut host = active_repair_host()
            .lock()
            .expect("repair host mutex should not be poisoned");
        *host = None;
    }

    let mut session = session_manager()
        .lock()
        .expect("repair session manager mutex should not be poisoned");
    session.on_disconnect(connection_id);
}

fn remember_host(request: &UnlockRepairSessionRequest) {
    let mut host = active_repair_host()
        .lock()
        .expect("repair host mutex should not be poisoned");
    *host = Some(RepairHostConnection {
        port: request.port,
        auth_token: request.nonce.clone(),
        connection_id: request.connection_id.clone(),
    });
}

fn current_host() -> Option<RepairHostConnection> {
    active_repair_host()
        .lock()
        .expect("repair host mutex should not be poisoned")
        .clone()
}

fn local_session_status() -> RepairSessionStatus {
    session_manager()
        .lock()
        .expect("repair session manager mutex should not be poisoned")
        .status()
}

fn local_session_can_run_privileged_actions_with(
    session_status: &RepairSessionStatus,
    process_is_elevated: bool,
) -> bool {
    process_is_elevated && !session_status.locked
}

fn local_session_can_run_privileged_actions(session_status: &RepairSessionStatus) -> bool {
    match crate::privilege::is_process_elevated() {
        Ok(process_is_elevated) => {
            local_session_can_run_privileged_actions_with(session_status, process_is_elevated)
        }
        Err(_) => false,
    }
}

fn auto_unlock_local_session_with_check<F>(
    app_instance_id: &str,
    connection_id: &str,
    process_is_elevated: F,
) -> Result<RepairSessionStatus, String>
where
    F: FnOnce() -> Result<bool, String>,
{
    if !process_is_elevated()? {
        return Ok(local_session_status());
    }

    let mut session = session_manager()
        .lock()
        .expect("repair session manager mutex should not be poisoned");
    if session.status().locked {
        session.unlock(app_instance_id.to_string(), connection_id.to_string());
    }
    Ok(session.status())
}

pub fn auto_unlock_local_session(
    app_instance_id: &str,
    connection_id: &str,
) -> Result<RepairSessionStatus, String> {
    auto_unlock_local_session_with_check(app_instance_id, connection_id, || {
        crate::privilege::is_process_elevated()
    })
}

fn reserve_repair_host_port() -> Result<u16, String> {
    let listener = TcpListener::bind(("127.0.0.1", 0))
        .map_err(|err| format!("Unable to reserve a local repair host port: {err}"))?;
    let port = listener
        .local_addr()
        .map_err(|err| format!("Unable to read the reserved repair host port: {err}"))?
        .port();
    drop(listener);
    Ok(port)
}

fn send_request_to_host(
    host: &RepairHostConnection,
    request: RepairServiceRequest,
) -> Result<RepairServiceResponse, String> {
    let address = format!("127.0.0.1:{}", host.port);
    let socket_address = address
        .parse()
        .map_err(|err| format!("Invalid repair host address {address}: {err}"))?;
    let mut stream = TcpStream::connect_timeout(&socket_address, REPAIR_HOST_CONNECT_TIMEOUT)
        .map_err(|err| format!("Unable to connect to the repair host on {address}: {err}"))?;
    let _ = stream.set_read_timeout(Some(REPAIR_HOST_TIMEOUT));
    let _ = stream.set_write_timeout(Some(REPAIR_HOST_TIMEOUT));

    let envelope = RepairIpcRequest {
        auth_token: host.auth_token.clone(),
        request,
    };
    let encoded = encode_message(&envelope)?;
    stream
        .write_all(&encoded)
        .map_err(|err| format!("Unable to write repair host request: {err}"))?;
    stream
        .flush()
        .map_err(|err| format!("Unable to flush repair host request: {err}"))?;

    let response: RepairIpcResponse = read_framed_message(&mut stream)?;
    Ok(response.response)
}

fn request_with_active_host(
    request: RepairServiceRequest,
) -> Result<RepairServiceResponse, String> {
    let Some(host) = current_host() else {
        return Err("Repair Mode is locked.".to_string());
    };

    match send_request_to_host(&host, request) {
        Ok(response) => Ok(response),
        Err(err) => {
            mark_host_disconnected(&host.connection_id);
            Err(err)
        }
    }
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

fn read_framed_message<T: DeserializeOwned>(stream: &mut TcpStream) -> Result<T, String> {
    let mut len_bytes = [0_u8; 4];
    stream
        .read_exact(&mut len_bytes)
        .map_err(|err| format!("Unable to read repair host frame length: {err}"))?;
    let payload_len = u32::from_le_bytes(len_bytes) as usize;

    let mut payload = vec![0_u8; payload_len];
    stream
        .read_exact(&mut payload)
        .map_err(|err| format!("Unable to read repair host frame payload: {err}"))?;

    let mut framed = len_bytes.to_vec();
    framed.extend(payload);
    decode_message(&framed)
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
            RepairServiceResponse::UnlockRepairSession(
                session_manager.unlock_with_request(&request),
            )
        }
        RepairServiceRequest::LockRepairSession | RepairServiceRequest::Shutdown => {
            session_manager.lock();
            RepairServiceResponse::RepairSessionStatus(session_manager.status())
        }
        RepairServiceRequest::RunMachineAction(_)
        | RepairServiceRequest::RunProfileCleanup(_)
        | RepairServiceRequest::RunAppxRemoval(_) => {
            let status = session_manager.status();
            RepairServiceResponse::RepairAction(if status.locked {
                locked_result()
            } else {
                RepairCommandResult {
                    success: false,
                    output:
                        "Repair action dispatch is not connected to the elevated repair host yet."
                            .to_string(),
                    requires_unlock: false,
                }
            })
        }
    }
}

pub fn get_repair_service_health() -> RepairServiceHealth {
    match request_with_active_host(RepairServiceRequest::GetServiceHealth) {
        Ok(RepairServiceResponse::ServiceHealth(health)) => health,
        Ok(_) => RepairServiceHealth::service_unavailable(),
        Err(_) => {
            let session_status = local_session_status();
            if local_session_can_run_privileged_actions(&session_status) {
                RepairServiceHealth {
                    connected: true,
                    requires_unlock: false,
                    detail: Some(
                        "In-process elevated repair session is active for this app instance."
                            .to_string(),
                    ),
                }
            } else {
                RepairServiceHealth::service_unavailable()
            }
        }
    }
}

pub fn get_repair_session_status() -> RepairSessionStatus {
    match request_with_active_host(RepairServiceRequest::GetRepairSessionStatus) {
        Ok(RepairServiceResponse::RepairSessionStatus(status)) => status,
        Ok(_) => RepairSessionStatus::service_unavailable(),
        Err(_) => session_manager()
            .lock()
            .expect("repair session manager mutex should not be poisoned")
            .status(),
    }
}

pub fn issue_unlock_request(
    app_instance_id: &str,
    connection_id: &str,
) -> Result<UnlockRepairSessionRequest, String> {
    let port = reserve_repair_host_port()?;
    Ok(session_manager()
        .lock()
        .expect("repair session manager mutex should not be poisoned")
        .issue_unlock_request_for_port(
            app_instance_id.to_string(),
            connection_id.to_string(),
            port,
        ))
}

pub fn complete_unlock_request(request: UnlockRepairSessionRequest) -> UnlockRepairSessionResponse {
    let deadline = Instant::now() + REPAIR_HOST_UNLOCK_WAIT;
    let candidate_host = RepairHostConnection {
        port: request.port,
        auth_token: request.nonce.clone(),
        connection_id: request.connection_id.clone(),
    };
    let mut last_error: Option<String> = None;

    while Instant::now() < deadline {
        match send_request_to_host(&candidate_host, RepairServiceRequest::GetServiceHealth) {
            Ok(RepairServiceResponse::ServiceHealth(health)) if health.connected => {
                let response = {
                    let mut manager = session_manager()
                        .lock()
                        .expect("repair session manager mutex should not be poisoned");
                    manager.unlock_with_request(&request)
                };

                if response.unlocked {
                    remember_host(&request);
                }
                return response;
            }
            Ok(RepairServiceResponse::ServiceHealth(_))
            | Ok(RepairServiceResponse::RepairSessionStatus(_))
            | Ok(RepairServiceResponse::UnlockRepairSession(_))
            | Ok(RepairServiceResponse::RepairAction(_)) => {
                last_error =
                    Some("Repair host returned an unexpected unlock response.".to_string());
            }
            Err(err) => {
                last_error = Some(err);
            }
        }

        std::thread::sleep(REPAIR_HOST_UNLOCK_POLL_INTERVAL);
    }

    UnlockRepairSessionResponse {
        unlocked: false,
        detail: Some(last_error.unwrap_or_else(|| {
            "Timed out waiting for the elevated repair host to accept the unlock request."
                .to_string()
        })),
    }
}

pub fn lock_repair_mode() -> RepairSessionStatus {
    if let Some(host) = current_host() {
        let _ = send_request_to_host(&host, RepairServiceRequest::LockRepairSession);
        let _ = send_request_to_host(&host, RepairServiceRequest::Shutdown);
        mark_host_disconnected(&host.connection_id);
    } else {
        session_manager()
            .lock()
            .expect("repair session manager mutex should not be poisoned")
            .lock();
    }

    session_manager()
        .lock()
        .expect("repair session manager mutex should not be poisoned")
        .status()
}

pub fn run_machine_action(action: RepairMachineAction) -> Result<RepairCommandResult, String> {
    match request_with_active_host(RepairServiceRequest::RunMachineAction(action.clone())) {
        Ok(RepairServiceResponse::RepairAction(result)) => Ok(result),
        Ok(_) => Err("Repair host returned an unexpected machine-action response.".to_string()),
        Err(_) => {
            let session_status = local_session_status();
            if local_session_can_run_privileged_actions(&session_status) {
                crate::repair_actions::run_machine_action_blocking(&session_status, action)
            } else {
                Ok(locked_result())
            }
        }
    }
}

pub fn run_profile_cleanup(request: ProfileCleanupRequest) -> Result<RepairCommandResult, String> {
    match request_with_active_host(RepairServiceRequest::RunProfileCleanup(request.clone())) {
        Ok(RepairServiceResponse::RepairAction(result)) => Ok(result),
        Ok(_) => Err("Repair host returned an unexpected cleanup response.".to_string()),
        Err(_) => {
            let session_status = local_session_status();
            if local_session_can_run_privileged_actions(&session_status) {
                crate::repair_actions::clear_profile_caches_blocking(&session_status, request)
            } else {
                Ok(locked_result())
            }
        }
    }
}

pub fn run_appx_removal(request: AppxRemovalRequest) -> Result<RepairCommandResult, String> {
    match request_with_active_host(RepairServiceRequest::RunAppxRemoval(request.clone())) {
        Ok(RepairServiceResponse::RepairAction(result)) => Ok(result),
        Ok(_) => Err("Repair host returned an unexpected Appx response.".to_string()),
        Err(_) => {
            let session_status = local_session_status();
            if local_session_can_run_privileged_actions(&session_status) {
                crate::repair_actions::remove_appx_for_target_blocking(&session_status, request)
            } else {
                Ok(locked_result())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        auto_unlock_local_session_with_check, local_session_can_run_privileged_actions_with,
    };
    use crate::repair_protocol::RepairSessionStatus;

    #[test]
    fn local_session_requires_elevation_and_unlocked_state() {
        let unlocked = RepairSessionStatus {
            locked: false,
            connected: true,
            target_sid: None,
            requires_unlock: false,
        };
        let locked = RepairSessionStatus {
            locked: true,
            connected: false,
            target_sid: None,
            requires_unlock: true,
        };

        assert!(local_session_can_run_privileged_actions_with(&unlocked, true));
        assert!(!local_session_can_run_privileged_actions_with(&unlocked, false));
        assert!(!local_session_can_run_privileged_actions_with(&locked, true));
    }

    #[test]
    fn auto_unlock_local_session_unlocks_when_process_is_elevated() {
        let before = crate::repair_ipc::lock_repair_mode();
        assert!(before.locked, "test should start from a locked repair session");

        let status =
            auto_unlock_local_session_with_check("app-elevated", "conn-elevated", || Ok(true))
                .expect("elevated process should auto-unlock");

        assert!(!status.locked, "repair mode should auto-unlock");
        assert!(status.connected, "local elevated session should be connected");

        let after = crate::repair_ipc::lock_repair_mode();
        assert!(after.locked, "test should restore the locked repair session");
    }

    #[test]
    fn auto_unlock_local_session_stays_locked_when_process_is_not_elevated() {
        let before = crate::repair_ipc::lock_repair_mode();
        assert!(before.locked, "test should start from a locked repair session");

        let status =
            auto_unlock_local_session_with_check("app-standard", "conn-standard", || Ok(false))
                .expect("standard process should return a status");

        assert!(status.locked, "non-elevated sessions should stay locked");
    }
}
