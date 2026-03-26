use crate::process_exec::run_cmd;
use serde::{Deserialize, Serialize};
use std::fs;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BatteryReportResult {
    pub html: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BatterySummaryResult {
    pub present: bool,
    pub status: String,
    pub charge_percent: Option<u32>,
    pub design_capacity_mwh: Option<u32>,
    pub full_charge_capacity_mwh: Option<u32>,
    pub health_percent: Option<f32>,
    pub wear_percent: Option<f32>,
    pub cycle_count: Option<u32>,
    pub estimated_runtime_minutes: Option<u32>,
    pub estimated_runtime_full_minutes: Option<u32>,
    pub note: String,
}

#[tauri::command]
pub async fn get_battery_report() -> Result<BatteryReportResult, String> {
    let report_path = std::env::temp_dir().join("SuperRoutePro-BatteryReport.html");
    let report_path_arg = report_path.to_string_lossy().to_string();

    let _ = run_cmd("powercfg", &["/batteryreport", "/output", &report_path_arg]).await?;

    let html = fs::read_to_string(&report_path).map_err(|e| {
        format!(
            "Failed to read battery report file: {} ({})",
            report_path_arg, e
        )
    })?;

    if html.trim().is_empty() {
        return Err("Battery report is empty".to_string());
    }

    Ok(BatteryReportResult { html })
}

#[tauri::command]
pub async fn get_battery_summary() -> Result<BatterySummaryResult, String> {
    tauri::async_runtime::spawn_blocking(|| {
        #[cfg(target_os = "windows")]
        {
            use windows_sys::Win32::System::Power::*;

            let mut sps: SYSTEM_POWER_STATUS = unsafe { std::mem::zeroed() };
            let result = unsafe { GetSystemPowerStatus(&mut sps) };
            if result == 0 {
                return Err("GetSystemPowerStatus failed".to_string());
            }

            let no_battery = sps.BatteryFlag == 128 || sps.BatteryFlag == 255;
            if no_battery {
                return Ok(BatterySummaryResult {
                    present: false,
                    status: "No battery detected".to_string(),
                    charge_percent: None,
                    design_capacity_mwh: None,
                    full_charge_capacity_mwh: None,
                    health_percent: None,
                    wear_percent: None,
                    cycle_count: None,
                    estimated_runtime_minutes: None,
                    estimated_runtime_full_minutes: None,
                    note: "This machine may be desktop-only or battery telemetry is unavailable."
                        .to_string(),
                });
            }

            let charge_percent = if sps.BatteryLifePercent <= 100 {
                Some(sps.BatteryLifePercent as u32)
            } else {
                None
            };

            let status = match (sps.ACLineStatus, sps.BatteryFlag) {
                (1, f) if f & 8 != 0 => "Charging".to_string(),
                (1, _) => "Connected to AC".to_string(),
                (0, f) if f & 4 != 0 => "Critical".to_string(),
                (0, f) if f & 2 != 0 => "Low".to_string(),
                (0, _) => "Discharging".to_string(),
                _ => format!("AC={} Flag={}", sps.ACLineStatus, sps.BatteryFlag),
            };

            let estimated_runtime_minutes =
                if sps.BatteryLifeTime != u32::MAX && sps.BatteryLifeTime > 0 {
                    Some(sps.BatteryLifeTime / 60)
                } else {
                    None
                };

            let estimated_runtime_full_minutes = if sps.BatteryFullLifeTime != u32::MAX
                && sps.BatteryFullLifeTime > 0
            {
                Some(sps.BatteryFullLifeTime / 60)
            } else {
                match (estimated_runtime_minutes, charge_percent) {
                    (Some(rt), Some(cp)) if cp > 0 => Some((rt as f64 * 100.0 / cp as f64) as u32),
                    _ => None,
                }
            };

            let ioctl_details = query_battery_details_ioctl();

            let (design_cap, full_cap, cycle, health_pct, wear_pct, note) = match ioctl_details {
                Some(details) => {
                    let health = if details.designed_capacity_mwh > 0 {
                        Some(
                            (details.full_charged_capacity_mwh as f32
                                / details.designed_capacity_mwh as f32)
                                * 100.0,
                        )
                    } else {
                        None
                    };
                    let wear = health.map(|h| (100.0 - h).max(0.0));
                    let cc = if details.cycle_count > 0 {
                        Some(details.cycle_count)
                    } else {
                        None
                    };
                    (
                        Some(details.designed_capacity_mwh),
                        Some(details.full_charged_capacity_mwh),
                        cc,
                        health,
                        wear,
                        format!(
                            "Battery details from native IOCTL. Chemistry: {}",
                            details.chemistry
                        ),
                    )
                }
                None => (
                    None,
                    None,
                    None,
                    None,
                    None,
                    "Battery data from Win32 GetSystemPowerStatus. IOCTL detail query unavailable."
                        .to_string(),
                ),
            };

            Ok(BatterySummaryResult {
                present: true,
                status,
                charge_percent,
                design_capacity_mwh: design_cap,
                full_charge_capacity_mwh: full_cap,
                health_percent: health_pct,
                wear_percent: wear_pct,
                cycle_count: cycle,
                estimated_runtime_minutes,
                estimated_runtime_full_minutes,
                note,
            })
        }
        #[cfg(not(target_os = "windows"))]
        {
            Err("Battery summary only supported on Windows".to_string())
        }
    })
    .await
    .map_err(|e| format!("Battery task join error: {e}"))?
}

#[cfg(target_os = "windows")]
struct BatteryIoctlDetails {
    designed_capacity_mwh: u32,
    full_charged_capacity_mwh: u32,
    cycle_count: u32,
    chemistry: String,
}

#[cfg(target_os = "windows")]
mod battery_ioctl {
    pub const IOCTL_BATTERY_QUERY_TAG: u32 = (0x29 << 16) | (1 << 14) | (0x10 << 2);
    pub const IOCTL_BATTERY_QUERY_INFORMATION: u32 = (0x29 << 16) | (1 << 14) | (0x11 << 2);
    pub const BATTERY_INFORMATION_LEVEL: u32 = 0;

    #[repr(C)]
    pub struct BatteryQueryInformation {
        pub battery_tag: u32,
        pub information_level: u32,
        pub at_rate: i32,
    }

    #[repr(C)]
    pub struct BatteryInformation {
        pub capabilities: u32,
        pub technology: u8,
        pub reserved: [u8; 3],
        pub chemistry: [u8; 4],
        pub designed_capacity: u32,
        pub full_charged_capacity: u32,
        pub default_alert1: u32,
        pub default_alert2: u32,
        pub critical_bias: u32,
        pub cycle_count: u32,
    }
}

#[cfg(target_os = "windows")]
fn query_battery_details_ioctl() -> Option<BatteryIoctlDetails> {
    use battery_ioctl::*;
    use std::ffi::c_void;

    type Handle = *mut c_void;
    const INVALID_HANDLE: Handle = -1isize as Handle;

    #[repr(C)]
    struct Guid {
        data1: u32,
        data2: u16,
        data3: u16,
        data4: [u8; 8],
    }

    #[repr(C)]
    struct SpDeviceInterfaceData {
        cb_size: u32,
        interface_class_guid: Guid,
        flags: u32,
        reserved: usize,
    }

    const DIGCF_PRESENT: u32 = 0x2;
    const DIGCF_DEVICEINTERFACE: u32 = 0x10;
    const GENERIC_READ: u32 = 0x80000000;
    const GENERIC_WRITE: u32 = 0x40000000;
    const FILE_SHARE_READ: u32 = 1;
    const FILE_SHARE_WRITE: u32 = 2;
    const OPEN_EXISTING: u32 = 3;

    extern "system" {
        fn SetupDiGetClassDevsW(
            class_guid: *const Guid,
            enumerator: *const u16,
            hwnd_parent: Handle,
            flags: u32,
        ) -> Handle;

        fn SetupDiEnumDeviceInterfaces(
            dev_info: Handle,
            dev_info_data: *const c_void,
            interface_class_guid: *const Guid,
            member_index: u32,
            device_interface_data: *mut SpDeviceInterfaceData,
        ) -> i32;

        fn SetupDiGetDeviceInterfaceDetailW(
            dev_info: Handle,
            device_interface_data: *mut SpDeviceInterfaceData,
            device_interface_detail_data: *mut c_void,
            device_interface_detail_data_size: u32,
            required_size: *mut u32,
            device_info_data: *mut c_void,
        ) -> i32;

        fn SetupDiDestroyDeviceInfoList(dev_info: Handle) -> i32;

        fn CreateFileW(
            file_name: *const u16,
            desired_access: u32,
            share_mode: u32,
            security_attributes: *const c_void,
            creation_disposition: u32,
            flags_and_attributes: u32,
            template_file: Handle,
        ) -> Handle;

        fn DeviceIoControl(
            device: Handle,
            io_control_code: u32,
            in_buffer: *const c_void,
            in_buffer_size: u32,
            out_buffer: *mut c_void,
            out_buffer_size: u32,
            bytes_returned: *mut u32,
            overlapped: *mut c_void,
        ) -> i32;

        fn CloseHandle(handle: Handle) -> i32;
    }

    let battery_guid = Guid {
        data1: 0x72631e54,
        data2: 0x78a4,
        data3: 0x11d0,
        data4: [0xbc, 0xf7, 0x00, 0xaa, 0x00, 0xb7, 0xb3, 0x2a],
    };

    unsafe {
        let dev_info = SetupDiGetClassDevsW(
            &battery_guid,
            std::ptr::null(),
            std::ptr::null_mut(),
            DIGCF_PRESENT | DIGCF_DEVICEINTERFACE,
        );
        if dev_info == INVALID_HANDLE || dev_info.is_null() {
            return None;
        }

        let mut iface_data: SpDeviceInterfaceData = std::mem::zeroed();
        iface_data.cb_size = std::mem::size_of::<SpDeviceInterfaceData>() as u32;

        if SetupDiEnumDeviceInterfaces(
            dev_info,
            std::ptr::null(),
            &battery_guid,
            0,
            &mut iface_data,
        ) == 0
        {
            SetupDiDestroyDeviceInfoList(dev_info);
            return None;
        }

        let mut required_size: u32 = 0;
        SetupDiGetDeviceInterfaceDetailW(
            dev_info,
            &mut iface_data,
            std::ptr::null_mut(),
            0,
            &mut required_size,
            std::ptr::null_mut(),
        );
        if required_size == 0 {
            SetupDiDestroyDeviceInfoList(dev_info);
            return None;
        }

        let mut detail_buf: Vec<u8> = vec![0u8; required_size as usize];
        let cb_size_ptr = detail_buf.as_mut_ptr() as *mut u32;
        *cb_size_ptr = 8;

        if SetupDiGetDeviceInterfaceDetailW(
            dev_info,
            &mut iface_data,
            detail_buf.as_mut_ptr() as *mut c_void,
            required_size,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
        ) == 0
        {
            SetupDiDestroyDeviceInfoList(dev_info);
            return None;
        }

        let device_path = detail_buf.as_ptr().add(4) as *const u16;
        let handle = CreateFileW(
            device_path,
            GENERIC_READ | GENERIC_WRITE,
            FILE_SHARE_READ | FILE_SHARE_WRITE,
            std::ptr::null(),
            OPEN_EXISTING,
            0,
            std::ptr::null_mut(),
        );
        SetupDiDestroyDeviceInfoList(dev_info);

        if handle == INVALID_HANDLE || handle.is_null() {
            return None;
        }

        let timeout: u32 = 0;
        let mut battery_tag: u32 = 0;
        let mut bytes_returned: u32 = 0;

        let ok = DeviceIoControl(
            handle,
            IOCTL_BATTERY_QUERY_TAG,
            &timeout as *const u32 as *const c_void,
            4,
            &mut battery_tag as *mut u32 as *mut c_void,
            4,
            &mut bytes_returned,
            std::ptr::null_mut(),
        );
        if ok == 0 || battery_tag == 0 {
            CloseHandle(handle);
            return None;
        }

        let query = BatteryQueryInformation {
            battery_tag,
            information_level: BATTERY_INFORMATION_LEVEL,
            at_rate: 0,
        };
        let mut info: BatteryInformation = std::mem::zeroed();
        bytes_returned = 0;

        let ok = DeviceIoControl(
            handle,
            IOCTL_BATTERY_QUERY_INFORMATION,
            &query as *const _ as *const c_void,
            std::mem::size_of::<BatteryQueryInformation>() as u32,
            &mut info as *mut _ as *mut c_void,
            std::mem::size_of::<BatteryInformation>() as u32,
            &mut bytes_returned,
            std::ptr::null_mut(),
        );
        CloseHandle(handle);

        if ok == 0 {
            return None;
        }

        let chemistry = String::from_utf8_lossy(&info.chemistry).trim().to_string();

        Some(BatteryIoctlDetails {
            designed_capacity_mwh: info.designed_capacity,
            full_charged_capacity_mwh: info.full_charged_capacity,
            cycle_count: info.cycle_count,
            chemistry,
        })
    }
}

#[cfg(not(target_os = "windows"))]
fn query_battery_details_ioctl() -> Option<()> {
    None
}
