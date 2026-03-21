#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::io::ErrorKind;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::time::Duration;

use super_route_pro_lib::repair_actions::{
    clear_profile_caches_blocking, remove_appx_for_target_blocking, run_machine_action_blocking,
};
use super_route_pro_lib::repair_ipc::{decode_message, encode_message};
use super_route_pro_lib::repair_protocol::{
    RepairCommandResult, RepairIpcRequest, RepairIpcResponse, RepairServiceHealth,
    RepairServiceRequest, RepairServiceResponse,
};
use super_route_pro_lib::repair_session::RepairSessionManager;

fn main() {
    if let Err(err) = run() {
        eprintln!("{err}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let mut args = std::env::args().skip(1);
    let Some(mode) = args.next() else {
        return Err("Usage: SuperRouteRepairBroker --serve <port> <auth-token> <app-instance-id> <connection-id> <parent-process-id>".to_string());
    };

    if mode != "--serve" {
        return Err("Repair broker only supports --serve mode.".to_string());
    }

    let port = args
        .next()
        .ok_or_else(|| "Missing repair host port.".to_string())?
        .parse::<u16>()
        .map_err(|_| "Repair host port must be a valid TCP port.".to_string())?;
    let auth_token = args
        .next()
        .ok_or_else(|| "Missing repair host auth token.".to_string())?;
    let app_instance_id = args
        .next()
        .ok_or_else(|| "Missing repair app instance id.".to_string())?;
    let connection_id = args
        .next()
        .ok_or_else(|| "Missing repair connection id.".to_string())?;
    let parent_process_id = args
        .next()
        .ok_or_else(|| "Missing launching app process id.".to_string())?
        .parse::<u32>()
        .map_err(|_| "Launching app process id must be a valid unsigned integer.".to_string())?;

    let listener = TcpListener::bind(("127.0.0.1", port))
        .map_err(|err| format!("Unable to bind elevated repair host on 127.0.0.1:{port}: {err}"))?;
    listener
        .set_nonblocking(true)
        .map_err(|err| format!("Unable to set repair host listener to non-blocking mode: {err}"))?;
    let parent_process = ParentProcessMonitor::attach(parent_process_id)?;

    let mut session_manager = RepairSessionManager::new();
    session_manager.unlock(app_instance_id, connection_id);

    let mut should_exit = false;
    while !should_exit {
        let (mut stream, _) = match listener.accept() {
            Ok(connection) => connection,
            Err(err) if err.kind() == ErrorKind::WouldBlock => {
                if !parent_process.is_running() {
                    session_manager.lock();
                    break;
                }
                std::thread::sleep(Duration::from_millis(200));
                continue;
            }
            Err(err) => {
                return Err(format!("Repair host accept failed: {err}"));
            }
        };

        let request = read_request(&mut stream)?;
        let response = if request.auth_token != auth_token {
            unauthorized_response(&request.request)
        } else {
            handle_request(
                &mut session_manager,
                port,
                request.request,
                &mut should_exit,
            )
        };

        let framed = encode_message(&RepairIpcResponse { response })?;
        stream
            .write_all(&framed)
            .map_err(|err| format!("Repair host response write failed: {err}"))?;
        stream
            .flush()
            .map_err(|err| format!("Repair host response flush failed: {err}"))?;
    }

    Ok(())
}

fn read_request(stream: &mut std::net::TcpStream) -> Result<RepairIpcRequest, String> {
    let mut len_bytes = [0_u8; 4];
    stream
        .read_exact(&mut len_bytes)
        .map_err(|err| format!("Repair host request length read failed: {err}"))?;
    let payload_len = u32::from_le_bytes(len_bytes) as usize;

    let mut payload = vec![0_u8; payload_len];
    stream
        .read_exact(&mut payload)
        .map_err(|err| format!("Repair host request payload read failed: {err}"))?;

    let mut framed = len_bytes.to_vec();
    framed.extend(payload);
    decode_message(&framed)
}

fn unauthorized_response(request: &RepairServiceRequest) -> RepairServiceResponse {
    match request {
        RepairServiceRequest::GetServiceHealth => {
            RepairServiceResponse::ServiceHealth(RepairServiceHealth {
                connected: false,
                requires_unlock: true,
                detail: Some("Unauthorized repair host request.".to_string()),
            })
        }
        RepairServiceRequest::GetRepairSessionStatus
        | RepairServiceRequest::UnlockRepairSession(_)
        | RepairServiceRequest::LockRepairSession
        | RepairServiceRequest::Shutdown
        | RepairServiceRequest::RunMachineAction(_)
        | RepairServiceRequest::RunProfileCleanup(_)
        | RepairServiceRequest::RunAppxRemoval(_) => {
            RepairServiceResponse::RepairAction(RepairCommandResult {
                success: false,
                output: "Unauthorized repair host request.".to_string(),
                requires_unlock: true,
            })
        }
    }
}

fn handle_request(
    session_manager: &mut RepairSessionManager,
    port: u16,
    request: RepairServiceRequest,
    should_exit: &mut bool,
) -> RepairServiceResponse {
    match request {
        RepairServiceRequest::GetServiceHealth => {
            RepairServiceResponse::ServiceHealth(RepairServiceHealth {
                connected: true,
                requires_unlock: session_manager.status().locked,
                detail: Some(format!(
                    "Elevated repair host is active on 127.0.0.1:{port}."
                )),
            })
        }
        RepairServiceRequest::GetRepairSessionStatus => {
            RepairServiceResponse::RepairSessionStatus(session_manager.status())
        }
        RepairServiceRequest::UnlockRepairSession(request) => {
            RepairServiceResponse::UnlockRepairSession(
                session_manager.unlock_with_request(&request),
            )
        }
        RepairServiceRequest::LockRepairSession => {
            session_manager.lock();
            RepairServiceResponse::RepairSessionStatus(session_manager.status())
        }
        RepairServiceRequest::Shutdown => {
            session_manager.lock();
            *should_exit = true;
            RepairServiceResponse::RepairSessionStatus(session_manager.status())
        }
        RepairServiceRequest::RunMachineAction(action) => RepairServiceResponse::RepairAction(
            run_machine_action_blocking(&session_manager.status(), action)
                .unwrap_or_else(repair_action_error),
        ),
        RepairServiceRequest::RunProfileCleanup(request) => RepairServiceResponse::RepairAction(
            clear_profile_caches_blocking(&session_manager.status(), request)
                .unwrap_or_else(repair_action_error),
        ),
        RepairServiceRequest::RunAppxRemoval(request) => RepairServiceResponse::RepairAction(
            remove_appx_for_target_blocking(&session_manager.status(), request)
                .unwrap_or_else(repair_action_error),
        ),
    }
}

fn repair_action_error(err: String) -> RepairCommandResult {
    RepairCommandResult {
        success: false,
        output: err,
        requires_unlock: false,
    }
}

#[cfg(target_os = "windows")]
struct ParentProcessMonitor {
    handle: windows_sys::Win32::Foundation::HANDLE,
}

#[cfg(target_os = "windows")]
impl ParentProcessMonitor {
    fn attach(parent_process_id: u32) -> Result<Self, String> {
        use windows_sys::Win32::Storage::FileSystem::SYNCHRONIZE;
        use windows_sys::Win32::System::Threading::OpenProcess;

        let handle = unsafe { OpenProcess(SYNCHRONIZE, 0, parent_process_id) };
        if handle.is_null() {
            return Err(format!(
                "Unable to monitor launching app process {parent_process_id}."
            ));
        }

        Ok(Self { handle })
    }

    fn is_running(&self) -> bool {
        use windows_sys::Win32::Foundation::WAIT_TIMEOUT;
        use windows_sys::Win32::System::Threading::WaitForSingleObject;

        unsafe { WaitForSingleObject(self.handle, 0) == WAIT_TIMEOUT }
    }
}

#[cfg(target_os = "windows")]
impl Drop for ParentProcessMonitor {
    fn drop(&mut self) {
        unsafe {
            let _ = windows_sys::Win32::Foundation::CloseHandle(self.handle);
        }
    }
}

#[cfg(not(target_os = "windows"))]
struct ParentProcessMonitor;

#[cfg(not(target_os = "windows"))]
impl ParentProcessMonitor {
    fn attach(_parent_process_id: u32) -> Result<Self, String> {
        Ok(Self)
    }

    fn is_running(&self) -> bool {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::ParentProcessMonitor;
    use std::process::Command;

    #[test]
    fn parent_process_monitor_treats_the_current_process_as_alive() {
        let monitor =
            ParentProcessMonitor::attach(std::process::id()).expect("current process should open");
        assert!(
            monitor.is_running(),
            "the broker should keep serving while the launching app is still alive"
        );
    }

    #[test]
    fn parent_process_monitor_detects_when_a_child_process_has_exited() {
        let mut child = Command::new("cmd")
            .args(["/C", "exit", "0"])
            .spawn()
            .expect("test child should spawn");
        let child_pid = child.id();
        let monitor = ParentProcessMonitor::attach(child_pid)
            .expect("exited child should still be observable via its process handle");
        let status = child.wait().expect("child should exit cleanly");
        assert!(status.success(), "test child should exit successfully");
        assert!(
            !monitor.is_running(),
            "the broker should self-terminate once the launching app process has exited"
        );
    }
}
