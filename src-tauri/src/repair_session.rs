use crate::repair_protocol::{
    RepairSessionStatus, UnlockRepairSessionRequest, UnlockRepairSessionResponse,
};
#[cfg(not(target_os = "windows"))]
use std::sync::atomic::{AtomicU64, Ordering};

#[cfg(not(target_os = "windows"))]
static NEXT_UNLOCK_NONCE: AtomicU64 = AtomicU64::new(1);

#[cfg(target_os = "windows")]
fn generate_unlock_nonce() -> String {
    use std::fmt::Write;
    use windows_sys::Win32::Security::Cryptography::{
        BCryptGenRandom, BCRYPT_USE_SYSTEM_PREFERRED_RNG,
    };

    let mut bytes = [0u8; 32];
    let status = unsafe {
        BCryptGenRandom(
            std::ptr::null_mut(),
            bytes.as_mut_ptr(),
            bytes.len() as u32,
            BCRYPT_USE_SYSTEM_PREFERRED_RNG,
        )
    };
    assert_eq!(
        status, 0,
        "BCryptGenRandom failed while generating repair unlock token (NTSTATUS 0x{status:08x})"
    );

    let mut hex = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        let _ = write!(&mut hex, "{byte:02x}");
    }
    hex
}

#[cfg(not(target_os = "windows"))]
fn generate_unlock_nonce() -> String {
    format!(
        "unlock-{}",
        NEXT_UNLOCK_NONCE.fetch_add(1, Ordering::Relaxed)
    )
}

#[derive(Debug, Default)]
pub struct RepairSessionManager {
    app_instance_id: Option<String>,
    connection_id: Option<String>,
    target_sid: Option<String>,
    pending_app_instance_id: Option<String>,
    pending_connection_id: Option<String>,
    pending_unlock_nonce: Option<String>,
}

impl RepairSessionManager {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn unlock(&mut self, app_instance_id: impl Into<String>, connection_id: impl Into<String>) {
        self.app_instance_id = Some(app_instance_id.into());
        self.connection_id = Some(connection_id.into());
        self.pending_app_instance_id = None;
        self.pending_connection_id = None;
        self.pending_unlock_nonce = None;
    }

    pub fn lock(&mut self) {
        self.app_instance_id = None;
        self.connection_id = None;
        self.target_sid = None;
        self.pending_app_instance_id = None;
        self.pending_connection_id = None;
        self.pending_unlock_nonce = None;
    }

    pub fn on_disconnect(&mut self, connection_id: &str) {
        if self.connection_id.as_deref() == Some(connection_id)
            || self.pending_connection_id.as_deref() == Some(connection_id)
        {
            self.lock();
        }
    }

    pub fn issue_unlock_request(
        &mut self,
        app_instance_id: impl Into<String>,
        connection_id: impl Into<String>,
    ) -> UnlockRepairSessionRequest {
        self.issue_unlock_request_for_port(app_instance_id, connection_id, 0)
    }

    pub fn issue_unlock_request_for_port(
        &mut self,
        app_instance_id: impl Into<String>,
        connection_id: impl Into<String>,
        port: u16,
    ) -> UnlockRepairSessionRequest {
        let app_instance_id = app_instance_id.into();
        let connection_id = connection_id.into();
        let nonce = generate_unlock_nonce();

        self.pending_app_instance_id = Some(app_instance_id.clone());
        self.pending_connection_id = Some(connection_id.clone());
        self.pending_unlock_nonce = Some(nonce.clone());

        UnlockRepairSessionRequest {
            app_instance_id,
            connection_id,
            nonce,
            port,
            parent_process_id: std::process::id(),
        }
    }

    pub fn unlock_with_request(
        &mut self,
        request: &UnlockRepairSessionRequest,
    ) -> UnlockRepairSessionResponse {
        let request_matches = self.pending_app_instance_id.as_deref()
            == Some(&request.app_instance_id)
            && self.pending_connection_id.as_deref() == Some(&request.connection_id)
            && self.pending_unlock_nonce.as_deref() == Some(&request.nonce);

        if request_matches {
            self.unlock(
                request.app_instance_id.clone(),
                request.connection_id.clone(),
            );
            UnlockRepairSessionResponse {
                unlocked: true,
                detail: None,
            }
        } else {
            UnlockRepairSessionResponse {
                unlocked: false,
                detail: Some(
                    "Unlock request did not match the pending repair session.".to_string(),
                ),
            }
        }
    }

    pub fn status(&self) -> RepairSessionStatus {
        let locked = self.app_instance_id.is_none();

        RepairSessionStatus {
            locked,
            connected: self.connection_id.is_some(),
            target_sid: self.target_sid.clone(),
            requires_unlock: locked,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::RepairSessionManager;

    #[cfg(target_os = "windows")]
    #[test]
    fn issue_unlock_request_generates_64_char_random_hex_nonce() {
        let mut manager = RepairSessionManager::new();
        let first = manager.issue_unlock_request("app-1", "conn-1");
        let second = manager.issue_unlock_request("app-1", "conn-2");

        assert_eq!(first.nonce.len(), 64);
        assert!(first.nonce.bytes().all(|b| b.is_ascii_hexdigit()));
        assert_eq!(second.nonce.len(), 64);
        assert!(second.nonce.bytes().all(|b| b.is_ascii_hexdigit()));
        assert_ne!(first.nonce, second.nonce);
    }
}
