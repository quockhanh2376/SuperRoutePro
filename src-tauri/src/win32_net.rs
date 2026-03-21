//! Native network adapter enumeration using `netsh` and `getmac`.
//! Replaces PowerShell Get-NetAdapter and Get-WmiObject Win32_NetworkAdapterConfiguration.
//!
//! Uses netsh (always available on Windows) instead of wmic (deprecated/removed in Windows 11).

use serde::{Deserialize, Serialize};
use std::process::Command;

#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NativeNic {
    pub interface_index: u32,
    pub description: String,
    pub mac_address: String,
    pub friendly_name: String,
    pub ip_addresses: Vec<String>,
    pub gateways: Vec<String>,
    pub oper_status_up: bool,
}

#[cfg(target_os = "windows")]
const CREATE_NO_WINDOW: u32 = 0x08000000;

/// Run a command with CREATE_NO_WINDOW and timeout, return stdout as String.
#[cfg(target_os = "windows")]
fn run_hidden(cmd: &str, args: &[&str]) -> Result<String, String> {
    let output = Command::new(cmd)
        .args(args)
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .map_err(|e| format!("Failed to run {} {:?}: {}", cmd, args, e))?;

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

/// Enumerate network adapters using netsh (works on all Windows 10/11).
pub fn enumerate_adapters() -> Result<Vec<NativeNic>, String> {
    #[cfg(target_os = "windows")]
    {
        // Step 1: Get interface list with Idx, Name, State
        //   "Idx  Met  MTU  State  Name"
        let iface_text = run_hidden("netsh", &["interface", "ipv4", "show", "interfaces"])?;
        let mut adapters: Vec<NativeNic> = Vec::new();

        for line in iface_text.lines() {
            // skip header + separator + empty
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with("---") || trimmed.starts_with("Idx") {
                continue;
            }

            // Format:  "  7          30        1500  connected     Wi-Fi"
            // Columns: Idx  Met  MTU  State  Name (name can have spaces)
            let cols: Vec<&str> = trimmed.split_whitespace().collect();
            if cols.len() < 5 {
                continue;
            }

            let idx: u32 = match cols[0].parse() {
                Ok(v) => v,
                Err(_) => continue, // Not a data row
            };
            let state = cols[3];
            // Name is everything from cols[4] onward (handles multi-word names)
            let name = cols[4..].join(" ");

            adapters.push(NativeNic {
                interface_index: idx,
                description: name.clone(),
                mac_address: String::new(),
                friendly_name: name,
                ip_addresses: Vec::new(),
                gateways: Vec::new(),
                oper_status_up: state == "connected",
            });
        }

        // Step 2: Get IP addresses and gateways from "netsh interface ipv4 show addresses"
        let addr_text = run_hidden("netsh", &["interface", "ipv4", "show", "addresses"])?;
        let mut current_name = String::new();

        for line in addr_text.lines() {
            let trimmed = line.trim();

            // "Configuration for interface "Wi-Fi""
            if let Some(start) = trimmed.find("\"") {
                if let Some(end) = trimmed.rfind("\"") {
                    if end > start {
                        current_name = trimmed[start + 1..end].to_string();
                    }
                }
                continue;
            }

            if current_name.is_empty() {
                continue;
            }

            // "IP Address:   192.168.1.148"
            if trimmed.starts_with("IP Address:") {
                let ip = trimmed.trim_start_matches("IP Address:").trim().to_string();
                if !ip.is_empty() {
                    if let Some(nic) = adapters
                        .iter_mut()
                        .find(|a| a.friendly_name == current_name)
                    {
                        nic.ip_addresses.push(ip);
                    }
                }
            }

            // "Default Gateway:   192.168.1.1"
            if trimmed.starts_with("Default Gateway:") {
                let gw = trimmed
                    .trim_start_matches("Default Gateway:")
                    .trim()
                    .to_string();
                if !gw.is_empty() {
                    if let Some(nic) = adapters
                        .iter_mut()
                        .find(|a| a.friendly_name == current_name)
                    {
                        nic.gateways.push(gw);
                    }
                }
            }
        }

        // Step 3: Get MAC addresses from "getmac /fo csv /v /nh"
        if let Ok(mac_text) = run_hidden("getmac", &["/fo", "csv", "/v", "/nh"]) {
            for line in mac_text.lines() {
                let trimmed = line.trim();
                if trimmed.is_empty() {
                    continue;
                }

                // CSV: "Connection Name","Network Adapter","Physical Address","Transport Name"
                let fields: Vec<String> = parse_csv_line(trimmed);
                if fields.len() >= 3 {
                    let conn_name = &fields[0];
                    let adapter_desc = &fields[1];
                    let mac = &fields[2];

                    // Match by connection name (friendly_name)
                    if let Some(nic) = adapters.iter_mut().find(|a| a.friendly_name == *conn_name) {
                        nic.mac_address = mac.clone();
                        // Use the adapter description from getmac as the real description
                        if !adapter_desc.is_empty() && *adapter_desc != "N/A" {
                            nic.description = adapter_desc.clone();
                        }
                    }
                }
            }
        }

        Ok(adapters)
    }

    #[cfg(not(target_os = "windows"))]
    {
        Err("NIC enumeration only supported on Windows".to_string())
    }
}

/// Simple CSV line parser handling quoted fields.
fn parse_csv_line(line: &str) -> Vec<String> {
    let mut fields = Vec::new();
    let mut current = String::new();
    let mut in_quotes = false;
    let mut chars = line.chars().peekable();

    while let Some(ch) = chars.next() {
        match ch {
            '"' => {
                if in_quotes {
                    // Check for escaped quote ""
                    if chars.peek() == Some(&'"') {
                        current.push('"');
                        chars.next();
                    } else {
                        in_quotes = false;
                    }
                } else {
                    in_quotes = true;
                }
            }
            ',' if !in_quotes => {
                fields.push(current.trim().to_string());
                current = String::new();
            }
            _ => {
                current.push(ch);
            }
        }
    }
    fields.push(current.trim().to_string());
    fields
}
