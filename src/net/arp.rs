use anyhow::Result;
use futures_util::future::join_all;
use ipnet::Ipv4Net;
use std::collections::{HashMap, HashSet};
use std::net::{IpAddr, Ipv4Addr};
use tokio::net::TcpStream;
use tokio::sync::{mpsc, Semaphore};
use tokio::time::{timeout, Duration, Instant};

use super::device::{Device, DiscoverySource};
use super::oui;

/// Check if an IPv4 address is a usable unicast address (not broadcast/multicast/loopback/network)
fn is_usable_unicast(ip: Ipv4Addr) -> bool {
    let octets = ip.octets();
    !ip.is_broadcast()
        && !ip.is_multicast()
        && !ip.is_loopback()
        && !ip.is_unspecified()
        && octets[3] != 255 // subnet broadcast
        && octets[3] != 0 // network address
}

/// Sweep the subnet with TCP connection attempts to populate the ARP cache,
/// streaming discovered devices via the channel as they appear.
pub async fn discover(
    subnet: &Ipv4Net,
    scan_timeout: Duration,
    tx: mpsc::UnboundedSender<Device>,
) -> Result<()> {
    let hosts: Vec<Ipv4Addr> = super::util::subnet_hosts(subnet);
    let semaphore = std::sync::Arc::new(Semaphore::new(256));

    // Spawn all TCP probes as background tasks — all ports probed concurrently per host
    let mut handles = Vec::new();
    for host in hosts {
        let sem = semaphore.clone();
        let handle = tokio::spawn(async move {
            let _permit = sem.acquire().await.ok();
            // Probe all ports concurrently (we just need one SYN to trigger ARP)
            let probes: Vec<_> = [80, 443, 22, 445]
                .iter()
                .map(|&port| {
                    let addr = std::net::SocketAddr::new(IpAddr::V4(host), port);
                    timeout(Duration::from_millis(100), TcpStream::connect(addr))
                })
                .collect();
            join_all(probes).await;
        });
        handles.push(handle);
    }

    // While sweep is running, periodically read ARP cache and stream new devices
    let mut seen = HashSet::new();
    let deadline = Instant::now() + scan_timeout;
    let poll_interval = Duration::from_millis(250);

    loop {
        // Read ARP cache and send any new devices
        if let Ok(entries) = parse_arp_cache().await {
            for (ip, mac) in entries {
                if is_usable_unicast(ip) && seen.insert(ip) {
                    let mut device = Device::new(IpAddr::V4(ip));
                    device.mac = Some(mac.clone());
                    device.sources.insert(DiscoverySource::ArpCache.to_string());
                    device.manufacturer = oui::lookup_manufacturer(&mac);
                    let _ = tx.send(device);
                }
            }
        }

        // Check if we've exceeded the timeout
        if Instant::now() >= deadline {
            break;
        }

        // Check if all probes are done
        let all_done = handles.iter().all(|h| h.is_finished());
        if all_done {
            // Do one final ARP cache read
            if let Ok(entries) = parse_arp_cache().await {
                for (ip, mac) in entries {
                    if seen.insert(ip) {
                        let mut device = Device::new(IpAddr::V4(ip));
                        device.mac = Some(mac.clone());
                        device.sources.insert(DiscoverySource::ArpCache.to_string());
                        device.manufacturer = oui::lookup_manufacturer(&mac);
                        let _ = tx.send(device);
                    }
                }
            }
            break;
        }

        tokio::time::sleep(poll_interval).await;
    }

    // Clean up any remaining handles
    for handle in handles {
        handle.abort();
    }

    Ok(())
}

/// Parse the system ARP cache
async fn parse_arp_cache() -> Result<HashMap<Ipv4Addr, String>> {
    let output = tokio::process::Command::new("arp")
        .arg("-a")
        .output()
        .await?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut entries = HashMap::new();

    for line in stdout.lines() {
        if let Some((ip, mac)) = parse_arp_line(line) {
            // Skip incomplete entries
            if mac != "(incomplete)" && mac != "ff:ff:ff:ff:ff:ff" {
                entries.insert(ip, mac);
            }
        }
    }

    Ok(entries)
}

/// Parse a single line from `arp -a` output
/// macOS format: hostname (ip) at mac on iface [ifscope] ...
/// Linux format: hostname (ip) at mac [ether] on iface
fn parse_arp_line(line: &str) -> Option<(Ipv4Addr, String)> {
    // Extract IP from parentheses
    let ip_start = line.find('(')? + 1;
    let ip_end = line.find(')')?;
    let ip_str = &line[ip_start..ip_end];
    let ip: Ipv4Addr = ip_str.parse().ok()?;

    // Extract MAC after " at "
    let at_idx = line.find(" at ")?;
    let after_at = &line[at_idx + 4..];
    let mac_end = after_at.find(' ').unwrap_or(after_at.len());
    let mac = &after_at[..mac_end];

    if mac == "(incomplete)" {
        return None;
    }

    // Normalize MAC: pad single-digit octets, uppercase
    let normalized = normalize_mac(mac);

    Some((ip, normalized))
}

/// Normalize a MAC address to XX:XX:XX:XX:XX:XX format
fn normalize_mac(mac: &str) -> String {
    let parts: Vec<&str> = mac.split(':').collect();
    if parts.len() == 6 {
        parts
            .iter()
            .map(|p| format!("{:0>2}", p))
            .collect::<Vec<_>>()
            .join(":")
            .to_uppercase()
    } else {
        mac.to_uppercase()
    }
}
