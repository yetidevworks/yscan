use anyhow::Result;
use ssdp_client::URN;
use std::net::IpAddr;
use tokio::sync::mpsc;
use tokio::time::Duration;

use super::device::{Device, DiscoverySource};
use super::oui;

/// Discover devices via SSDP/UPnP, streaming results via channel
pub async fn discover(scan_timeout: Duration, tx: mpsc::UnboundedSender<Device>) -> Result<()> {
    let search_target = URN::device("schemas-upnp-org", "Basic", 1);

    // Search for all root devices
    if let Ok(mut responses) =
        ssdp_client::search(&search_target.into(), scan_timeout, 3, None).await
    {
        use futures_util::StreamExt;
        while let Some(Ok(response)) = responses.next().await {
            let st = response.search_target().to_string();
            let location = response.location().to_string();
            let server = response.server().to_string();

            if let Some(addr) = extract_ip_from_url(&location) {
                let mut device = Device::new(addr);
                device.sources.insert(DiscoverySource::Ssdp.to_string());
                device.extra.insert("upnp_location".to_string(), location);
                device.extra.entry("upnp_st".to_string()).or_insert(st);

                if !server.is_empty() {
                    device
                        .extra
                        .entry("upnp_server".to_string())
                        .or_insert_with(|| server.clone());

                    if device.hostname.is_none() {
                        let name = server
                            .split('/')
                            .next()
                            .unwrap_or(&server)
                            .trim()
                            .to_string();
                        if !name.is_empty() {
                            device.hostname = Some(name);
                        }
                    }
                }

                if let Some(ref mac) = device.mac {
                    if device.manufacturer.is_none() {
                        device.manufacturer = oui::lookup_manufacturer(mac);
                    }
                }

                let _ = tx.send(device);
            }
        }
    }

    // Also try ssdp:all for broader discovery
    if let Ok(mut responses) =
        ssdp_client::search(&"ssdp:all".parse().unwrap(), scan_timeout, 2, None).await
    {
        use futures_util::StreamExt;
        while let Some(Ok(response)) = responses.next().await {
            let location = response.location().to_string();
            let server = response.server().to_string();

            if let Some(addr) = extract_ip_from_url(&location) {
                let mut device = Device::new(addr);
                device.sources.insert(DiscoverySource::Ssdp.to_string());
                device.extra.insert("upnp_location".to_string(), location);

                if !server.is_empty() {
                    device
                        .extra
                        .entry("upnp_server".to_string())
                        .or_insert_with(|| server.clone());

                    if device.hostname.is_none() {
                        let name = server
                            .split('/')
                            .next()
                            .unwrap_or(&server)
                            .trim()
                            .to_string();
                        if !name.is_empty() {
                            device.hostname = Some(name);
                        }
                    }
                }

                let _ = tx.send(device);
            }
        }
    }

    Ok(())
}

/// Extract an IP address from a URL string like "http://192.168.1.1:8080/path"
fn extract_ip_from_url(url_str: &str) -> Option<IpAddr> {
    let url = url::Url::parse(url_str).ok()?;
    let host = url.host_str()?;
    host.parse::<IpAddr>().ok()
}
