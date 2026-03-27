//! Native network adapter enumeration using `netsh` and `getmac`.
//! Replaces PowerShell Get-NetAdapter and Get-WmiObject Win32_NetworkAdapterConfiguration.
//!
//! Uses netsh (always available on Windows) instead of wmic (deprecated/removed in Windows 11).

use crate::process_exec::run_hidden_stdout_blocking;
use serde::{Deserialize, Serialize};
use std::net::Ipv4Addr;
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

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

#[derive(Debug, Clone)]
struct CachedAdapterSnapshot {
    captured_at: Instant,
    adapters: Vec<NativeNic>,
    enriched_with_getmac: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct GetmacMetadata {
    connection_name: String,
    description: String,
    mac_address: String,
}

const ADAPTER_CACHE_TTL: Duration = Duration::from_secs(10);

fn adapter_cache() -> &'static Mutex<Option<CachedAdapterSnapshot>> {
    static CACHE: OnceLock<Mutex<Option<CachedAdapterSnapshot>>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(None))
}

pub fn is_valid_ipv4_address(value: &str) -> bool {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return false;
    }

    let Ok(address) = trimmed.parse::<Ipv4Addr>() else {
        return false;
    };
    let octets = address.octets();

    !address.is_unspecified() && !(octets[0] == 169 && octets[1] == 254)
}

fn cache_adapters(adapters: &[NativeNic], enriched_with_getmac: bool) {
    let mut cache = adapter_cache()
        .lock()
        .expect("adapter cache mutex should not be poisoned");
    *cache = Some(CachedAdapterSnapshot {
        captured_at: Instant::now(),
        adapters: adapters.to_vec(),
        enriched_with_getmac,
    });
}

fn recent_cached_adapters(require_getmac_enrichment: bool) -> Option<Vec<NativeNic>> {
    let mut cache = adapter_cache()
        .lock()
        .expect("adapter cache mutex should not be poisoned");
    let snapshot = cache.as_ref()?;
    if snapshot.captured_at.elapsed() > ADAPTER_CACHE_TTL {
        *cache = None;
        return None;
    }
    if require_getmac_enrichment && !snapshot.enriched_with_getmac {
        return None;
    }
    Some(snapshot.adapters.clone())
}

pub fn invalidate_adapter_cache() {
    let mut cache = adapter_cache()
        .lock()
        .expect("adapter cache mutex should not be poisoned");
    *cache = None;
}

fn parse_getmac_metadata(output: &str) -> Vec<GetmacMetadata> {
    output
        .lines()
        .filter_map(|line| {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                return None;
            }

            let fields = parse_csv_line(trimmed);
            if fields.len() < 3 {
                return None;
            }

            Some(GetmacMetadata {
                connection_name: fields[0].clone(),
                description: fields[1].clone(),
                mac_address: fields[2].clone(),
            })
        })
        .collect()
}

fn apply_getmac_metadata_to_adapters(adapters: &mut [NativeNic], metadata: &[GetmacMetadata]) {
    for entry in metadata {
        if let Some(nic) = adapters
            .iter_mut()
            .find(|adapter| adapter.friendly_name == entry.connection_name)
        {
            nic.mac_address = entry.mac_address.clone();
            if !entry.description.is_empty() && entry.description != "N/A" {
                nic.description = entry.description.clone();
            }
        }
    }
}

fn enrich_adapters_with_getmac(adapters: &mut [NativeNic]) {
    if let Ok(mac_text) = run_hidden_stdout_blocking("getmac", &["/fo", "csv", "/v", "/nh"]) {
        let metadata = parse_getmac_metadata(&mac_text);
        apply_getmac_metadata_to_adapters(adapters, &metadata);
    }
}

fn enumerate_adapters_inner(include_getmac_metadata: bool) -> Result<Vec<NativeNic>, String> {
    #[cfg(target_os = "windows")]
    {
        // Step 1: Get interface list with Idx, Name, State
        //   "Idx  Met  MTU  State  Name"
        let iface_text =
            run_hidden_stdout_blocking("netsh", &["interface", "ipv4", "show", "interfaces"])?;
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
        let addr_text =
            run_hidden_stdout_blocking("netsh", &["interface", "ipv4", "show", "addresses"])?;
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

        if include_getmac_metadata {
            // Reuse the fresh basic enumeration and only layer getmac metadata on top.
            // This keeps the startup snapshot fast while allowing later enrich flows to
            // avoid repeating the netsh passes.
            enrich_adapters_with_getmac(&mut adapters);
        }

        Ok(adapters)
    }

    #[cfg(not(target_os = "windows"))]
    {
        Err("NIC enumeration only supported on Windows".to_string())
    }
}

/// Enumerate network adapters using netsh only.
/// Faster than the full variant because it skips the expensive `getmac` enrichment pass.
pub fn enumerate_adapters_basic() -> Result<Vec<NativeNic>, String> {
    if let Some(cached) = recent_cached_adapters(false) {
        return Ok(cached);
    }

    let adapters = enumerate_adapters_inner(false)?;
    cache_adapters(&adapters, false);
    Ok(adapters)
}

/// Enumerate adapters for UI snapshots.
/// Prefers a fresh enriched cache when available so the UI does not regress from
/// stable adapter descriptions back to friendly aliases like "Ethernet 2".
pub fn enumerate_adapters_for_snapshot() -> Result<Vec<NativeNic>, String> {
    if let Some(cached) = recent_cached_adapters(true) {
        return Ok(cached);
    }

    enumerate_adapters_basic()
}

/// Enumerate network adapters using netsh plus getmac enrichment (works on all Windows 10/11).
pub fn enumerate_adapters() -> Result<Vec<NativeNic>, String> {
    if let Some(cached) = recent_cached_adapters(true) {
        return Ok(cached);
    }

    let mut adapters = if let Some(cached_basic) = recent_cached_adapters(false) {
        cached_basic
    } else {
        enumerate_adapters_basic()?
    };
    enrich_adapters_with_getmac(&mut adapters);
    cache_adapters(&adapters, true);
    Ok(adapters)
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

#[cfg(test)]
mod tests {
    use super::{
        apply_getmac_metadata_to_adapters, cache_adapters, invalidate_adapter_cache,
        is_valid_ipv4_address, parse_getmac_metadata, recent_cached_adapters, NativeNic,
    };

    #[test]
    fn valid_ipv4_filter_rejects_unspecified_link_local_and_non_ipv4_values() {
        assert!(is_valid_ipv4_address("192.168.1.25"));
        assert!(!is_valid_ipv4_address(""));
        assert!(!is_valid_ipv4_address("0.0.0.0"));
        assert!(!is_valid_ipv4_address("169.254.10.20"));
        assert!(!is_valid_ipv4_address("fe80::1"));
    }

    #[test]
    fn parse_getmac_metadata_reads_connection_name_description_and_mac() {
        let metadata = parse_getmac_metadata(
            r#""Wi-Fi","Intel(R) Wi-Fi 6 AX201 160MHz","AA-BB-CC-DD-EE-FF","\Device\Tcpip_{GUID}""#,
        );

        assert_eq!(metadata.len(), 1);
        assert_eq!(metadata[0].connection_name, "Wi-Fi");
        assert_eq!(metadata[0].description, "Intel(R) Wi-Fi 6 AX201 160MHz");
        assert_eq!(metadata[0].mac_address, "AA-BB-CC-DD-EE-FF");
    }

    #[test]
    fn apply_getmac_metadata_updates_cached_basic_adapters_without_reenumerating_netsh() {
        let mut adapters = vec![NativeNic {
            interface_index: 7,
            description: "Wi-Fi".to_string(),
            mac_address: String::new(),
            friendly_name: "Wi-Fi".to_string(),
            ip_addresses: vec!["192.168.1.25".to_string()],
            gateways: vec!["192.168.1.1".to_string()],
            oper_status_up: true,
        }];
        let metadata = parse_getmac_metadata(
            r#""Wi-Fi","Intel(R) Wi-Fi 6 AX201 160MHz","AA-BB-CC-DD-EE-FF","\Device\Tcpip_{GUID}""#,
        );

        apply_getmac_metadata_to_adapters(&mut adapters, &metadata);

        assert_eq!(adapters[0].description, "Intel(R) Wi-Fi 6 AX201 160MHz");
        assert_eq!(adapters[0].mac_address, "AA-BB-CC-DD-EE-FF");
    }

    #[test]
    fn invalidate_adapter_cache_clears_recent_snapshot() {
        let adapters = vec![NativeNic {
            interface_index: 7,
            description: "Intel(R) Wi-Fi".to_string(),
            mac_address: "AA-BB-CC-DD-EE-FF".to_string(),
            friendly_name: "Wi-Fi".to_string(),
            ip_addresses: vec!["192.168.1.25".to_string()],
            gateways: vec!["192.168.1.1".to_string()],
            oper_status_up: true,
        }];

        cache_adapters(&adapters, true);
        assert!(recent_cached_adapters(true).is_some());

        invalidate_adapter_cache();

        assert!(recent_cached_adapters(false).is_none());
    }
}
