use anyhow::Result;
use mdns_sd::{ServiceDaemon, ServiceEvent};
use std::collections::HashSet;
use std::net::IpAddr;
use tokio::sync::mpsc;
use tokio::time::{sleep, Duration};

use super::device::{Device, DiscoverySource};
use super::oui;

/// Common mDNS service types to browse
const SERVICE_TYPES: &[&str] = &[
    "_http._tcp.local.",
    "_https._tcp.local.",
    "_ssh._tcp.local.",
    "_sftp-ssh._tcp.local.",
    "_smb._tcp.local.",
    "_afpovertcp._tcp.local.",
    "_nfs._tcp.local.",
    "_ftp._tcp.local.",
    "_airplay._tcp.local.",
    "_raop._tcp.local.",
    "_hap._tcp.local.",
    "_homekit._tcp.local.",
    "_googlecast._tcp.local.",
    "_spotify-connect._tcp.local.",
    "_sonos._tcp.local.",
    "_printer._tcp.local.",
    "_ipp._tcp.local.",
    "_ipps._tcp.local.",
    "_scanner._tcp.local.",
    "_companion-link._tcp.local.",
    "_sleep-proxy._udp.local.",
    "_device-info._tcp.local.",
    "_mqtt._tcp.local.",
    "_hue._tcp.local.",
];

/// Discover devices via mDNS service browsing, streaming results via channel
pub async fn discover(scan_timeout: Duration, tx: mpsc::UnboundedSender<Device>) -> Result<()> {
    let mdns = ServiceDaemon::new()?;
    let mut receivers = Vec::new();

    for service_type in SERVICE_TYPES {
        match mdns.browse(service_type) {
            Ok(receiver) => receivers.push((service_type.to_string(), receiver)),
            Err(_) => continue,
        }
    }

    // Track which IPs we've already sent to avoid excessive duplicates,
    // but we still send updates when new info is discovered
    let mut sent_ips: HashSet<IpAddr> = HashSet::new();

    let deadline = tokio::time::Instant::now() + scan_timeout;

    loop {
        if tokio::time::Instant::now() >= deadline {
            break;
        }

        let mut got_event = false;

        for (service_type, receiver) in &receivers {
            while let Ok(event) = receiver.try_recv() {
                got_event = true;
                if let ServiceEvent::ServiceResolved(info) = event {
                    for addr in info.get_addresses() {
                        // Skip IPv6 — we only want IPv4 unicast addresses
                        if !addr.is_ipv4() {
                            continue;
                        }
                        let ip = *addr;

                        let mut device = Device::new(ip);
                        device.sources.insert(DiscoverySource::Mdns.to_string());

                        // Extract service instance name from fullname
                        // e.g. "Tapo Plug C333._hap._tcp.local." → "Tapo Plug C333"
                        let fullname = info.get_fullname();
                        let instance_name = fullname
                            .strip_suffix(service_type)
                            .unwrap_or(fullname)
                            .trim_end_matches('.')
                            .to_string();

                        let properties = info.get_properties();

                        // Priority: instance name > hostname
                        // (TXT "md" field has cryptic model numbers like "P125", "AG035")
                        let display_name = if !instance_name.is_empty() {
                            Some(instance_name.clone())
                        } else {
                            None
                        }
                        .or_else(|| {
                            let hostname = info.get_hostname().trim_end_matches('.').to_string();
                            let clean = hostname
                                .strip_suffix(".local")
                                .unwrap_or(&hostname)
                                .to_string();
                            if !clean.is_empty() {
                                Some(clean)
                            } else {
                                None
                            }
                        });

                        if let Some(name) = display_name {
                            device.mdns_name = Some(name);
                        }

                        // Store service info
                        let svc_key =
                            format!("mdns_{}", service_type.trim_matches(&['.', '_'][..]));
                        device
                            .extra
                            .insert(svc_key, format!("{} (port {})", fullname, info.get_port()));

                        // Store TXT records
                        for property in properties.iter() {
                            let key = property.key();
                            let val = property.val_str();
                            device
                                .extra
                                .entry(format!("txt_{}", key))
                                .or_insert_with(|| val.to_string());
                        }

                        // Enrich with OUI if we have MAC
                        if let Some(ref mac) = device.mac {
                            if device.manufacturer.is_none() {
                                device.manufacturer = oui::lookup_manufacturer(mac);
                            }
                        }

                        // Always send — the engine will merge.
                        // Track for logging/debugging but don't gate on it.
                        sent_ips.insert(ip);
                        let _ = tx.send(device);
                    }
                }
            }
        }

        if !got_event {
            sleep(Duration::from_millis(100)).await;
        }
    }

    // Shutdown mDNS daemon
    let _ = mdns.shutdown();

    Ok(())
}
