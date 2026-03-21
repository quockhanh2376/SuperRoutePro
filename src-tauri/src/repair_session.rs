use crate::repair_protocol::{
    RepairSessionStatus, UnlockRepairSessionRequest, UnlockRepairSessionResponse,
};
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_UNLOCK_NONCE: AtomicU64 = AtomicU64::new(1);

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
        let nonce = format!(
            "unlock-{}",
            NEXT_UNLOCK_NONCE.fetch_add(1, Ordering::Relaxed)
        );

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
