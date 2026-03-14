use crate::network;
use crate::repair_protocol::{
    RepairCommandResult, RepairMachineAction, RepairSessionStatus,
};

fn locked_result() -> RepairCommandResult {
    RepairCommandResult {
        success: false,
        output: "Repair Mode is locked. Unlock Repair Mode before running admin fixes."
            .to_string(),
        requires_unlock: true,
    }
}

fn from_network_result(result: network::CommandResult) -> RepairCommandResult {
    RepairCommandResult {
        success: result.success,
        output: result.output,
        requires_unlock: false,
    }
}

pub async fn add_route(
    session_status: &RepairSessionStatus,
    destination: String,
    mask: String,
    gateway: String,
    metric: String,
    interface_index: Option<String>,
) -> Result<RepairCommandResult, String> {
    if session_status.locked {
        return Ok(locked_result());
    }

    let result =
        network::add_route(destination, mask, gateway, metric, interface_index).await?;
    Ok(from_network_result(result))
}

pub async fn delete_route(
    session_status: &RepairSessionStatus,
    destination: String,
    mask: String,
) -> Result<RepairCommandResult, String> {
    if session_status.locked {
        return Ok(locked_result());
    }

    let result = network::delete_route(destination, mask).await?;
    Ok(from_network_result(result))
}

pub async fn flush_routes(
    session_status: &RepairSessionStatus,
) -> Result<RepairCommandResult, String> {
    if session_status.locked {
        return Ok(locked_result());
    }

    let result = network::flush_routes().await?;
    Ok(from_network_result(result))
}

pub async fn set_default_gateway(
    session_status: &RepairSessionStatus,
    gateway: String,
    interface_index: String,
) -> Result<RepairCommandResult, String> {
    if session_status.locked {
        return Ok(locked_result());
    }

    let result = network::set_default_gateway(gateway, interface_index).await?;
    Ok(from_network_result(result))
}

pub async fn set_wan_persist_on_startup(
    session_status: &RepairSessionStatus,
    interface_index: String,
    enabled: bool,
) -> Result<RepairCommandResult, String> {
    if session_status.locked {
        return Ok(locked_result());
    }

    let result = network::set_wan_persist_on_startup(interface_index, enabled).await?;
    Ok(from_network_result(result))
}

pub async fn run_machine_action(
    session_status: &RepairSessionStatus,
    action: RepairMachineAction,
) -> Result<RepairCommandResult, String> {
    if session_status.locked {
        return Ok(locked_result());
    }

    let result = match action {
        RepairMachineAction::AddRoute(request) => {
            network::add_route(
                request.destination,
                request.mask,
                request.gateway,
                request.metric,
                request.interface_index,
            )
            .await?
        }
        RepairMachineAction::DeleteRoute(request) => {
            network::delete_route(request.destination, request.mask).await?
        }
        RepairMachineAction::FlushRoutes => network::flush_routes().await?,
        RepairMachineAction::SetDefaultGateway(request) => {
            network::set_default_gateway(request.gateway, request.interface_index).await?
        }
        RepairMachineAction::SetWanPersistOnStartup(request) => {
            network::set_wan_persist_on_startup(request.interface_index, request.enabled).await?
        }
        RepairMachineAction::FlushDns => {
            network::run_network_command("ipconfig /flushdns".to_string()).await?
        }
        RepairMachineAction::RenewDhcpLease => {
            network::run_network_command("ipconfig /release && ipconfig /renew".to_string()).await?
        }
        RepairMachineAction::ClearArpCache => {
            network::run_network_command("netsh interface ip delete arpcache".to_string()).await?
        }
        RepairMachineAction::ResetTcpIp => {
            network::run_network_command("netsh int ip reset".to_string()).await?
        }
        RepairMachineAction::ResetWinsock => {
            network::run_network_command("netsh winsock reset".to_string()).await?
        }
        RepairMachineAction::ResetFirewall => {
            network::run_network_command("netsh advfirewall reset".to_string()).await?
        }
        RepairMachineAction::ResetWinHttpProxy => {
            network::run_network_command("netsh winhttp reset proxy".to_string()).await?
        }
        RepairMachineAction::RestartActiveAdapters => network::run_network_command(
            "powershell -NoProfile -Command Get-NetAdapter -Physical ^| Where-Object {$_.Status -eq 'Up'} ^| Restart-NetAdapter -Confirm:$false"
                .to_string(),
        )
        .await?,
    };

    Ok(from_network_result(result))
}
