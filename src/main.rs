mod cli;
mod config;
mod net;
mod tui;

use anyhow::Result;
use clap::Parser;

use cli::{Cli, Command};
use config::Config;

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let mut config = Config::load().unwrap_or_default();

    // CLI overrides
    if let Some(ref iface) = cli.interface {
        config.network_interface = Some(iface.clone());
    }

    let theme = &cli.theme;

    match cli.command {
        Some(Command::Demo) => {
            tui::run_demo_tui(config, theme).await?;
        }
        Some(Command::Scan { json }) => {
            run_oneshot_scan(&config, json).await?;
        }
        None => {
            tui::run_tui(config, theme).await?;
        }
    }

    Ok(())
}

async fn run_oneshot_scan(config: &Config, json: bool) -> Result<()> {
    use std::time::Duration;
    use tokio::sync::mpsc;

    eprintln!("Scanning network...");

    let scan_timeout = Duration::from_secs(config.scan_timeout);
    let mut all_devices: std::collections::BTreeMap<std::net::IpAddr, net::device::Device> =
        std::collections::BTreeMap::new();

    // Detect local subnet
    let local_ip = net::util::local_ip()?;
    let subnet = net::util::local_subnet(local_ip)?;

    eprintln!("Local IP: {}, Subnet: {}", local_ip, subnet);

    // Create a channel for streaming results
    let (tx, mut rx) = mpsc::unbounded_channel::<net::device::Device>();

    // Spawn scanners
    if config.scanners.arp {
        let arp_subnet = subnet;
        let arp_tx = tx.clone();
        tokio::spawn(async move {
            let _ = net::arp::discover(&arp_subnet, scan_timeout, arp_tx).await;
        });
    }

    if config.scanners.mdns {
        let mdns_tx = tx.clone();
        tokio::spawn(async move {
            let _ = net::mdns::discover(scan_timeout, mdns_tx).await;
        });
    }

    if config.scanners.ssdp {
        let ssdp_tx = tx.clone();
        tokio::spawn(async move {
            let _ = net::ssdp::discover(scan_timeout, ssdp_tx).await;
        });
    }

    // Drop the original sender so rx closes when all tasks finish
    drop(tx);

    // Collect results as they stream in
    while let Some(device) = rx.recv().await {
        let ip = device.ip;
        let entry = all_devices
            .entry(ip)
            .or_insert_with(|| net::device::Device::new(ip));
        entry.merge(&device);
    }

    // Second pass: probe unnamed devices for hostname
    let unnamed: Vec<std::net::IpAddr> = all_devices
        .values()
        .filter(|d| d.display_name().is_empty())
        .map(|d| d.ip)
        .collect();

    if !unnamed.is_empty() {
        eprintln!("Probing {} unnamed devices for hostnames...", unnamed.len());
        let (probe_tx, mut probe_rx) = mpsc::unbounded_channel::<net::device::Device>();

        for ip in unnamed {
            let ptx = probe_tx.clone();
            tokio::spawn(async move {
                net::hostname::probe(ip, ptx).await;
            });
        }
        drop(probe_tx);

        while let Some(device) = probe_rx.recv().await {
            let ip = device.ip;
            let entry = all_devices
                .entry(ip)
                .or_insert_with(|| net::device::Device::new(ip));
            entry.merge(&device);
        }
    }

    if json {
        let devices: Vec<&net::device::Device> = all_devices.values().collect();
        println!("{}", serde_json::to_string_pretty(&devices)?);
    } else {
        // Print table
        println!(
            "{:<18} {:<24} {:<19} {:<16} Sources",
            "IP Address", "Name", "MAC Address", "Manufacturer"
        );
        println!("{}", "-".repeat(90));

        for device in all_devices.values() {
            let name = device.display_name();
            let name_display = if name.is_empty() { "-" } else { name };
            let mac = if device.mac_display().is_empty() {
                "-"
            } else {
                device.mac_display()
            };
            let mfr = if device.manufacturer_display().is_empty() {
                "-"
            } else {
                device.manufacturer_display()
            };
            let sources: Vec<&str> = device.sources.iter().map(|s| s.as_str()).collect();
            println!(
                "{:<18} {:<24} {:<19} {:<16} {}",
                device.ip,
                name_display,
                mac,
                mfr,
                sources.join(", ")
            );
        }

        eprintln!("\nFound {} devices", all_devices.len());
    }

    Ok(())
}
