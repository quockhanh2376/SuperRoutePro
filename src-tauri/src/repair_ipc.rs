use crate::repair_protocol::{RepairServiceHealth, RepairSessionStatus};

pub fn get_repair_service_health() -> RepairServiceHealth {
    RepairServiceHealth::service_unavailable()
}

pub fn get_repair_session_status() -> RepairSessionStatus {
    RepairSessionStatus::service_unavailable()
}
