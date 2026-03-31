use crate::{network, route_persist};
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
    UnlockRepairSession(UnlockRepairSessionRequest),
    LockRepairSession,
    Shutdown,
    RunMachineAction(RepairMachineAction),
    RunProfileCleanup(ProfileCleanupRequest),
    RunAppxRemoval(AppxRemovalRequest),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RepairServiceResponse {
    ServiceHealth(RepairServiceHealth),
    RepairSessionStatus(RepairSessionStatus),
    UnlockRepairSession(UnlockRepairSessionResponse),
    RepairAction(RepairCommandResult),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UnlockRepairSessionRequest {
    pub app_instance_id: String,
    pub connection_id: String,
    pub nonce: String,
    pub port: u16,
    pub parent_process_id: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UnlockRepairSessionResponse {
    pub unlocked: bool,
    pub detail: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RepairCommandResult {
    pub success: bool,
    pub output: String,
    pub requires_unlock: bool,
}

impl RepairCommandResult {
    pub fn success(output: impl Into<String>) -> Self {
        Self {
            success: true,
            output: output.into(),
            requires_unlock: false,
        }
    }

    pub fn failure(output: impl Into<String>) -> Self {
        Self {
            success: false,
            output: output.into(),
            requires_unlock: false,
        }
    }

    pub fn locked() -> Self {
        Self {
            success: false,
            output: "Repair Mode is locked. Unlock Repair Mode before running admin fixes."
                .to_string(),
            requires_unlock: true,
        }
    }
}

impl From<network::CommandResult> for RepairCommandResult {
    fn from(result: network::CommandResult) -> Self {
        Self {
            success: result.success,
            output: result.output,
            requires_unlock: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RepairIpcRequest {
    pub auth_token: String,
    pub request: RepairServiceRequest,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RepairIpcResponse {
    pub response: RepairServiceResponse,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AddRouteRequest {
    pub destination: String,
    pub mask: String,
    pub gateway: String,
    pub metric: String,
    pub interface_index: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeleteRouteRequest {
    pub destination: String,
    pub mask: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SetDefaultGatewayRequest {
    pub gateway: String,
    pub interface_index: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PersistConfigRequest {
    pub config: route_persist::PersistConfig,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProfileCleanupRequest {
    pub target_sid: String,
    pub targets: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AppxRemovalRequest {
    pub target_sid: String,
    pub packages: Vec<String>,
    pub remove_provisioned: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RepairMachineAction {
    AddRoute(AddRouteRequest),
    DeleteRoute(DeleteRouteRequest),
    FlushRoutes,
    SetDefaultGateway(SetDefaultGatewayRequest),
    ApplyPersistConfig(PersistConfigRequest),
    SavePersistConfig(PersistConfigRequest),
    ClearPersistConfig,
    FlushDns,
    RenewDhcpLease,
    ClearArpCache,
    ResetTcpIp,
    ResetWinsock,
    ResetFirewall,
    ResetWinHttpProxy,
    RestartActiveAdapters,
}
