use std::net::IpAddr;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::mpsc;
use tokio::time::{timeout, Duration};

use super::device::{Device, DiscoverySource};

/// HTTP ports to probe for banners (HTTP only, no TLS)
const HTTP_PORTS: &[u16] = &[80, 8080, 8006, 443, 8443];

/// Probe an IP for a hostname via reverse DNS and HTTP banners.
/// Sends a Device with the hostname set if found.
pub async fn probe(ip: IpAddr, tx: mpsc::UnboundedSender<Device>) {
    if let Some(name) = try_reverse_dns(ip).await {
        send_result(ip, name, &tx);
        return;
    }

    if let Some(name) = try_http_banner(ip).await {
        send_result(ip, name, &tx);
    }
}

fn send_result(ip: IpAddr, name: String, tx: &mpsc::UnboundedSender<Device>) {
    let mut device = Device::new(ip);
    device.hostname = Some(name);
    device.sources.insert(DiscoverySource::Hostname.to_string());
    let _ = tx.send(device);
}

/// Try reverse DNS lookup via PTR record
async fn try_reverse_dns(ip: IpAddr) -> Option<String> {
    let result = tokio::task::spawn_blocking(move || dns_lookup::lookup_addr(&ip))
        .await
        .ok()?
        .ok()?;

    // Filter out results that are just the IP address echoed back
    let trimmed = result.trim().to_string();
    if trimmed.is_empty() || trimmed == ip.to_string() {
        return None;
    }

    Some(trimmed)
}

/// Try HTTP banner detection on common ports
async fn try_http_banner(ip: IpAddr) -> Option<String> {
    let mut tasks = Vec::new();

    for &port in HTTP_PORTS {
        tasks.push(tokio::spawn(async move { probe_http_port(ip, port).await }));
    }

    for task in tasks {
        if let Ok(Some(name)) = task.await {
            return Some(name);
        }
    }

    None
}

/// Send a raw HTTP/1.0 request and parse the response for identifying info
async fn probe_http_port(ip: IpAddr, port: u16) -> Option<String> {
    let addr = format!("{}:{}", ip, port);
    let stream = timeout(
        Duration::from_secs(2),
        tokio::net::TcpStream::connect(&addr),
    )
    .await
    .ok()?
    .ok()?;

    let request = format!(
        "GET / HTTP/1.0\r\nHost: {}\r\nConnection: close\r\n\r\n",
        ip
    );

    let (mut reader, mut writer) = tokio::io::split(stream);
    writer.write_all(request.as_bytes()).await.ok()?;
    writer.shutdown().await.ok()?;

    let mut buf = vec![0u8; 4096];
    let n = timeout(Duration::from_secs(3), reader.read(&mut buf))
        .await
        .ok()?
        .ok()?;

    if n == 0 {
        return None;
    }

    let response = String::from_utf8_lossy(&buf[..n]);
    parse_response(&response)
}

/// Parse an HTTP response for Server header or <title> tag
fn parse_response(response: &str) -> Option<String> {
    // Try Server header first
    if let Some(name) = parse_server_header(response) {
        return Some(name);
    }

    // Try <title> tag as fallback
    parse_title_tag(response)
}

/// Extract and clean the Server header value
fn parse_server_header(response: &str) -> Option<String> {
    for line in response.lines() {
        if line.is_empty() {
            // End of headers
            break;
        }
        if let Some(value) = line
            .strip_prefix("Server: ")
            .or_else(|| line.strip_prefix("server: "))
        {
            let value = value.trim();
            if value.is_empty() {
                return None;
            }
            return Some(clean_server_name(value));
        }
    }
    None
}

/// Map known server banner patterns to friendly names
fn clean_server_name(server: &str) -> String {
    let lower = server.to_lowercase();

    if lower.contains("pve-api-daemon") {
        return "Proxmox VE".to_string();
    }
    if lower.contains("proxmox") {
        return "Proxmox".to_string();
    }
    if lower.contains("esxi") || lower.contains("vmware") {
        return "VMware ESXi".to_string();
    }
    if lower.contains("unifi") {
        return "UniFi Controller".to_string();
    }
    if lower.contains("pihole") {
        return "Pi-hole".to_string();
    }
    if lower.contains("home assistant") || lower.contains("homeassistant") {
        return "Home Assistant".to_string();
    }
    if lower.contains("opnsense") {
        return "OPNsense".to_string();
    }
    if lower.contains("pfsense") {
        return "pfSense".to_string();
    }
    if lower.contains("truenas") {
        return "TrueNAS".to_string();
    }
    if lower.contains("synology") {
        return "Synology DSM".to_string();
    }

    // Return the raw value with version stripped for common servers
    if let Some(base) = server.split('/').next() {
        base.to_string()
    } else {
        server.to_string()
    }
}

/// Extract text from the first <title>...</title> tag
fn parse_title_tag(response: &str) -> Option<String> {
    let lower = response.to_lowercase();
    let start = lower.find("<title>")? + 7;
    let end = lower[start..].find("</title>")? + start;

    let title = response[start..end].trim().to_string();
    if title.is_empty() {
        return None;
    }

    // Clean up known title patterns
    clean_title(&title)
}

/// Map known title patterns to friendly names
fn clean_title(title: &str) -> Option<String> {
    let lower = title.to_lowercase();

    // Skip generic/unhelpful titles
    if lower == "301 moved permanently"
        || lower == "302 found"
        || lower == "redirect"
        || lower == "loading..."
        || lower == "error"
    {
        return None;
    }

    if lower.contains("proxmox virtual environment") || lower.starts_with("pve") {
        return Some("Proxmox VE".to_string());
    }
    if lower.contains("truenas") {
        return Some("TrueNAS".to_string());
    }
    if lower.contains("synology") {
        return Some("Synology DSM".to_string());
    }
    if lower.contains("unifi") {
        return Some("UniFi Controller".to_string());
    }
    if lower.contains("pi-hole") || lower.contains("pihole") {
        return Some("Pi-hole".to_string());
    }
    if lower.contains("home assistant") {
        return Some("Home Assistant".to_string());
    }
    if lower.contains("opnsense") {
        return Some("OPNsense".to_string());
    }
    if lower.contains("pfsense") {
        return Some("pfSense".to_string());
    }

    // Use the title as-is if it's short enough to be a name
    if title.len() <= 40 {
        Some(title.to_string())
    } else {
        // Truncate long titles
        Some(format!("{}...", &title[..37]))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clean_server_name() {
        assert_eq!(clean_server_name("pve-api-daemon/3.0"), "Proxmox VE");
        assert_eq!(clean_server_name("nginx/1.18.0"), "nginx");
        assert_eq!(clean_server_name("Apache/2.4.41"), "Apache");
        assert_eq!(clean_server_name("lighttpd"), "lighttpd");
    }

    #[test]
    fn test_parse_server_header() {
        let resp = "HTTP/1.1 200 OK\r\nServer: pve-api-daemon/3.0\r\n\r\n<html>";
        assert_eq!(parse_server_header(resp), Some("Proxmox VE".to_string()));

        let resp = "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\n\r\n";
        assert_eq!(parse_server_header(resp), None);
    }

    #[test]
    fn test_parse_title_tag() {
        let resp = "HTTP/1.1 200 OK\r\n\r\n<html><head><title>pve - Proxmox Virtual Environment</title></head></html>";
        assert_eq!(parse_title_tag(resp), Some("Proxmox VE".to_string()));

        let resp = "HTTP/1.1 200 OK\r\n\r\n<html><head><title>My NAS</title></head></html>";
        assert_eq!(parse_title_tag(resp), Some("My NAS".to_string()));
    }

    #[test]
    fn test_parse_title_skips_redirects() {
        let resp = "HTTP/1.1 301 Moved\r\n\r\n<html><head><title>301 Moved Permanently</title></head></html>";
        assert_eq!(parse_title_tag(resp), None);
    }
}
