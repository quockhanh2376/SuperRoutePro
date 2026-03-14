use crate::repair_protocol::RepairSessionStatus;

#[derive(Debug, Default)]
pub struct RepairSessionManager {
    app_instance_id: Option<String>,
    connection_id: Option<String>,
    target_sid: Option<String>,
}

impl RepairSessionManager {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn unlock(&mut self, app_instance_id: impl Into<String>, connection_id: impl Into<String>) {
        self.app_instance_id = Some(app_instance_id.into());
        self.connection_id = Some(connection_id.into());
    }

    pub fn lock(&mut self) {
        self.app_instance_id = None;
        self.connection_id = None;
        self.target_sid = None;
    }

    pub fn on_disconnect(&mut self, connection_id: &str) {
        if self.connection_id.as_deref() == Some(connection_id) {
            self.lock();
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
