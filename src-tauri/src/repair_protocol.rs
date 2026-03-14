use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RepairSessionStatus {
    pub locked: bool,
    pub connected: bool,
    pub target_sid: Option<String>,
    pub requires_unlock: bool,
}

impl RepairSessionStatus {
    pub fn service_unavailable() -> Self {
        Self {
            locked: true,
            connected: false,
            target_sid: None,
            requires_unlock: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RepairServiceHealth {
    pub connected: bool,
    pub requires_unlock: bool,
    pub detail: Option<String>,
}

impl RepairServiceHealth {
    pub fn service_unavailable() -> Self {
        Self {
            connected: false,
            requires_unlock: true,
            detail: Some("Repair service is not installed or reachable yet.".to_string()),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RepairServiceRequest {
    GetServiceHealth,
    GetRepairSessionStatus,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RepairServiceResponse {
    ServiceHealth(RepairServiceHealth),
    RepairSessionStatus(RepairSessionStatus),
}
