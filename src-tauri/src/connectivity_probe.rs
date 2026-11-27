use reqwest::{redirect::Policy, Client, StatusCode};
use std::sync::OnceLock;
use std::time::Duration;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ConnectivityProbeKind {
    MicrosoftConnectTest,
    CloudflareTrace,
}

#[derive(Debug, Clone, Copy)]
struct ConnectivityProbeTarget {
    url: &'static str,
    kind: ConnectivityProbeKind,
}

const CONNECTIVITY_PROBE_TIMEOUT_SECS: u64 = 2;
const CONNECTIVITY_PROBE_MAX_BODY_BYTES: usize = 4 * 1024;
static CONNECTIVITY_PROBE_CLIENT: OnceLock<Client> = OnceLock::new();
const CONNECTIVITY_PROBE_USER_AGENT: &str =
    concat!("SuperRoutePro/", env!("CARGO_PKG_VERSION"), " ConnectivityProbe");
const CONNECTIVITY_PROBE_TARGETS: [ConnectivityProbeTarget; 2] = [
    ConnectivityProbeTarget {
        url: "http://www.msftconnecttest.com/connecttest.txt",
        kind: ConnectivityProbeKind::MicrosoftConnectTest,
    },
    ConnectivityProbeTarget {
        url: "https://speed.cloudflare.com/cdn-cgi/trace",
        kind: ConnectivityProbeKind::CloudflareTrace,
    },
];

fn connectivity_probe_status_matches(kind: ConnectivityProbeKind, status: StatusCode) -> bool {
    match kind {
        ConnectivityProbeKind::MicrosoftConnectTest | ConnectivityProbeKind::CloudflareTrace => {
            status == StatusCode::OK
        }
    }
}

fn connectivity_probe_body_matches(kind: ConnectivityProbeKind, body: &str) -> bool {
    match kind {
        ConnectivityProbeKind::MicrosoftConnectTest => body.trim() == "Microsoft Connect Test",
        ConnectivityProbeKind::CloudflareTrace => {
            let mut has_host = false;
            let mut has_ip = false;
            for line in body.lines() {
                let trimmed = line.trim();
                if trimmed.eq_ignore_ascii_case("h=speed.cloudflare.com") {
                    has_host = true;
                }
                if trimmed.starts_with("ip=") {
                    has_ip = true;
                }
            }
            has_host && has_ip
        }
    }
}

fn build_connectivity_probe_client() -> Result<Client, String> {
    Client::builder()
        .user_agent(CONNECTIVITY_PROBE_USER_AGENT)
        .redirect(Policy::none())
        .timeout(Duration::from_secs(CONNECTIVITY_PROBE_TIMEOUT_SECS))
        .build()
        .map_err(|error| format!("Failed to build internet probe client: {error}"))
}

fn shared_connectivity_probe_client() -> Result<&'static Client, String> {
    if let Some(client) = CONNECTIVITY_PROBE_CLIENT.get() {
        return Ok(client);
    }
    let client = build_connectivity_probe_client()?;
    Ok(CONNECTIVITY_PROBE_CLIENT.get_or_init(|| client))
}

fn append_bounded_chunk(buffer: &mut Vec<u8>, chunk: &[u8], max_bytes: usize) -> bool {
    let remaining = max_bytes.saturating_sub(buffer.len());
    if remaining == 0 {
        return true;
    }
    let to_copy = remaining.min(chunk.len());
    buffer.extend_from_slice(&chunk[..to_copy]);
    buffer.len() >= max_bytes
}

async fn read_bounded_response_body(
    mut response: reqwest::Response,
    target_url: &str,
) -> Result<String, String> {
    let mut buffer = Vec::with_capacity(CONNECTIVITY_PROBE_MAX_BODY_BYTES);
    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|error| format!("Connectivity probe body read failed for {target_url}: {error}"))?
    {
        if append_bounded_chunk(&mut buffer, &chunk, CONNECTIVITY_PROBE_MAX_BODY_BYTES) {
            break;
        }
    }
    Ok(String::from_utf8_lossy(&buffer).into_owned())
}

async fn probe_connectivity_target(
    client: &Client,
    target: ConnectivityProbeTarget,
) -> Result<bool, String> {
    let response = client
        .get(target.url)
        .send()
        .await
        .map_err(|error| format!("Connectivity probe request failed for {}: {error}", target.url))?;
    let status = response.status();
    if !connectivity_probe_status_matches(target.kind, status) {
        return Ok(false);
    }
    let body = read_bounded_response_body(response, target.url).await?;

    Ok(connectivity_probe_body_matches(target.kind, &body))
}

pub(crate) async fn check_connectivity() -> Result<bool, String> {
    let client = shared_connectivity_probe_client()?;

    for target in CONNECTIVITY_PROBE_TARGETS {
        match probe_connectivity_target(client, target).await {
            Ok(true) => return Ok(true),
            Ok(false) | Err(_) => continue,
        }
    }

    Ok(false)
}

#[cfg(test)]
mod tests {
    use super::{
        connectivity_probe_body_matches, connectivity_probe_status_matches,
        CONNECTIVITY_PROBE_USER_AGENT, ConnectivityProbeKind,
    };
    use reqwest::StatusCode;

    #[test]
    fn connectivity_probe_accepts_microsoft_connect_test_payload() {
        assert!(connectivity_probe_status_matches(
            ConnectivityProbeKind::MicrosoftConnectTest,
            StatusCode::OK
        ));
        assert!(connectivity_probe_body_matches(
            ConnectivityProbeKind::MicrosoftConnectTest,
            "Microsoft Connect Test"
        ));
    }

    #[test]
    fn connectivity_probe_rejects_non_ok_status() {
        assert!(!connectivity_probe_status_matches(
            ConnectivityProbeKind::MicrosoftConnectTest,
            StatusCode::FOUND
        ));
        assert!(!connectivity_probe_status_matches(
            ConnectivityProbeKind::CloudflareTrace,
            StatusCode::NO_CONTENT
        ));
    }

    #[test]
    fn connectivity_probe_accepts_trimmed_microsoft_body() {
        assert!(connectivity_probe_body_matches(
            ConnectivityProbeKind::MicrosoftConnectTest,
            "  Microsoft Connect Test\r\n"
        ));
    }

    #[test]
    fn connectivity_probe_rejects_unexpected_microsoft_probe_body() {
        assert!(!connectivity_probe_body_matches(
            ConnectivityProbeKind::MicrosoftConnectTest,
            "<html>Captive portal</html>"
        ));
    }

    #[test]
    fn connectivity_probe_accepts_cloudflare_trace_payload() {
        let body = "fl=961f82\nh=speed.cloudflare.com\nip=115.79.58.84\nvisit_scheme=https\n";
        assert!(connectivity_probe_status_matches(
            ConnectivityProbeKind::CloudflareTrace,
            StatusCode::OK
        ));
        assert!(connectivity_probe_body_matches(
            ConnectivityProbeKind::CloudflareTrace,
            body
        ));
    }

    #[test]
    fn connectivity_probe_accepts_cloudflare_trace_with_whitespace_and_case_variation() {
        let body =
            "colo=SGN\n  H=speed.cloudflare.com  \nvisit_scheme=https\nip=115.79.58.84\n";
        assert!(connectivity_probe_body_matches(
            ConnectivityProbeKind::CloudflareTrace,
            body
        ));
    }

    #[test]
    fn connectivity_probe_rejects_non_trace_cloudflare_body() {
        assert!(!connectivity_probe_body_matches(
            ConnectivityProbeKind::CloudflareTrace,
            "ok"
        ));
    }

    #[test]
    fn connectivity_probe_user_agent_tracks_package_version() {
        assert!(CONNECTIVITY_PROBE_USER_AGENT.starts_with("SuperRoutePro/"));
        assert!(CONNECTIVITY_PROBE_USER_AGENT.contains(env!("CARGO_PKG_VERSION")));
        assert!(CONNECTIVITY_PROBE_USER_AGENT.ends_with(" ConnectivityProbe"));
    }

    #[test]
    fn connectivity_probe_caps_response_body_at_4kb() {
        let mut buf = Vec::new();
        let chunk = vec![b'x'; 3000];
        assert!(!super::append_bounded_chunk(
            &mut buf,
            &chunk,
            super::CONNECTIVITY_PROBE_MAX_BODY_BYTES
        ));
        assert!(super::append_bounded_chunk(
            &mut buf,
            &chunk,
            super::CONNECTIVITY_PROBE_MAX_BODY_BYTES
        ));
        assert_eq!(buf.len(), 4096);
    }

    #[test]
    fn connectivity_probe_reuses_static_http_client() {
        let first = super::shared_connectivity_probe_client().expect("client should initialize");
        let second = super::shared_connectivity_probe_client().expect("client should reuse");
        assert!(std::ptr::eq(first, second));
    }
}
