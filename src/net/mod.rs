pub mod arp;
pub mod device;
pub mod hostname;
pub mod mdns;
pub mod oui;
pub mod port_scanner;
pub mod ssdp;
pub mod util;

use anyhow::Result;
use device::{Device, PortInfo};
use std::collections::{BTreeMap, HashSet};
use std::net::IpAddr;
use tokio::sync::mpsc;
use tokio::task::JoinSet;
use tokio::time::Duration;

use crate::config::Config;

/// Events sent from the discovery engine to the TUI
#[derive(Debug)]
pub enum ScanEvent {
    /// A new device was discovered or an existing one updated
    DeviceDiscovered(Box<Device>),
    /// Port scan completed for a device
    PortScanResult { ip: IpAddr, ports: Vec<PortInfo> },
    /// Port scan progress update
    PortScanProgress {
        #[allow(dead_code)]
        ip: IpAddr,
        scanned: usize,
        total: usize,
    },
    /// An error occurred during scanning
    Error(String),
    /// A scan cycle has started
    ScanStarted,
    /// A scan cycle has completed
    ScanCompleted { device_count: usize },
}

/// Commands sent from the TUI to the discovery engine
#[derive(Debug)]
pub enum ScanCommand {
    /// Scan ports on a specific device
    ScanPorts(IpAddr),
    /// Force a new scan cycle
    Rescan,
    /// Shut down the engine
    Shutdown,
}

/// The discovery engine runs background scans and reports results via channels
pub struct DiscoveryEngine {
    config: Config,
    event_tx: mpsc::UnboundedSender<ScanEvent>,
    cmd_rx: mpsc::UnboundedReceiver<ScanCommand>,
    devices: BTreeMap<IpAddr, Device>,
    local_ip: Option<std::net::Ipv4Addr>,
    /// Internal channel for scanners to send discovered devices
    scanner_tx: mpsc::UnboundedSender<Device>,
    scanner_rx: mpsc::UnboundedReceiver<Device>,
    /// Tracks running scanner tasks for the current scan cycle
    scan_tasks: JoinSet<()>,
    /// IPs already probed for hostname in the current scan cycle
    probed_ips: HashSet<IpAddr>,
}

impl DiscoveryEngine {
    pub fn new(
        config: Config,
        event_tx: mpsc::UnboundedSender<ScanEvent>,
        cmd_rx: mpsc::UnboundedReceiver<ScanCommand>,
    ) -> Self {
        let (scanner_tx, scanner_rx) = mpsc::unbounded_channel();
        Self {
            config,
            event_tx,
            cmd_rx,
            devices: BTreeMap::new(),
            local_ip: None,
            scanner_tx,
            scanner_rx,
            scan_tasks: JoinSet::new(),
            probed_ips: HashSet::new(),
        }
    }

    /// Run the engine loop
    pub async fn run(mut self) -> Result<()> {
        // Detect local IP
        match util::local_ip() {
            Ok(ip) => {
                self.local_ip = Some(ip);
            }
            Err(e) => {
                let _ = self.event_tx.send(ScanEvent::Error(format!(
                    "Failed to detect local IP: {}",
                    e
                )));
            }
        }

        // Start initial scan
        self.spawn_scan_tasks();

        let scan_interval = Duration::from_secs(self.config.scan_interval);
        let mut interval = tokio::time::interval(scan_interval);
        interval.tick().await; // Consume the first immediate tick

        loop {
            tokio::select! {
                // Drain incoming scanner results — devices appear in real time
                Some(device) = self.scanner_rx.recv() => {
                    self.merge_and_emit(device);
                }

                // A scanner task finished
                Some(_result) = self.scan_tasks.join_next() => {
                    // If all scanner tasks are done, emit ScanCompleted
                    if self.scan_tasks.is_empty() {
                        let _ = self.event_tx.send(ScanEvent::ScanCompleted {
                            device_count: self.devices.len(),
                        });
                    }
                }

                // Periodic rescan
                _ = interval.tick() => {
                    self.spawn_scan_tasks();
                }

                // Handle commands
                cmd = self.cmd_rx.recv() => {
                    match cmd {
                        Some(ScanCommand::Rescan) => {
                            self.spawn_scan_tasks();
                        }
                        Some(ScanCommand::ScanPorts(ip)) => {
                            self.run_port_scan(ip).await;
                        }
                        Some(ScanCommand::Shutdown) | None => {
                            break;
                        }
                    }
                }
            }
        }

        Ok(())
    }

    /// Spawn all enabled scanner tasks. Each gets a clone of the internal sender.
    fn spawn_scan_tasks(&mut self) {
        let _ = self.event_tx.send(ScanEvent::ScanStarted);
        self.probed_ips.clear();

        let scan_timeout = Duration::from_secs(self.config.scan_timeout);

        if self.config.scanners.arp {
            if let Some(local_ip) = self.local_ip {
                if let Ok(subnet) = util::local_subnet(local_ip) {
                    let tx = self.scanner_tx.clone();
                    self.scan_tasks.spawn(async move {
                        let _ = arp::discover(&subnet, scan_timeout, tx).await;
                    });
                }
            }
        }

        if self.config.scanners.mdns {
            let tx = self.scanner_tx.clone();
            self.scan_tasks.spawn(async move {
                let _ = mdns::discover(scan_timeout, tx).await;
            });
        }

        if self.config.scanners.ssdp {
            let tx = self.scanner_tx.clone();
            self.scan_tasks.spawn(async move {
                let _ = ssdp::discover(scan_timeout, tx).await;
            });
        }
    }

    /// Merge a discovered device into the device map and emit an event to the TUI
    fn merge_and_emit(&mut self, discovered: Device) {
        let ip = discovered.ip;
        let device = self.devices.entry(ip).or_insert_with(|| Device::new(ip));
        device.merge(&discovered);

        // Mark self
        if let Some(local_ip) = self.local_ip {
            if ip == IpAddr::V4(local_ip) {
                device.is_self = true;
            }
        }

        let _ = self
            .event_tx
            .send(ScanEvent::DeviceDiscovered(Box::new(device.clone())));

        // Probe unnamed devices for hostname via reverse DNS / HTTP banner
        if device.display_name().is_empty() && self.probed_ips.insert(ip) {
            let tx = self.scanner_tx.clone();
            tokio::spawn(async move {
                hostname::probe(ip, tx).await;
            });
        }
    }

    async fn run_port_scan(&self, ip: IpAddr) {
        let ports = self.config.port_scanner.ports.clone();
        let timeout_ms = self.config.port_scanner.timeout_ms;
        let event_tx = self.event_tx.clone();

        let (progress_tx, mut progress_rx) =
            mpsc::unbounded_channel::<port_scanner::PortScanProgress>();

        // Forward progress events
        let event_tx_progress = event_tx.clone();
        tokio::spawn(async move {
            while let Some(progress) = progress_rx.recv().await {
                let _ = event_tx_progress.send(ScanEvent::PortScanProgress {
                    ip: progress.ip,
                    scanned: progress.scanned,
                    total: progress.total,
                });
            }
        });

        match port_scanner::scan_ports(ip, &ports, timeout_ms, progress_tx).await {
            Ok(open_ports) => {
                let _ = event_tx.send(ScanEvent::PortScanResult {
                    ip,
                    ports: open_ports,
                });
            }
            Err(e) => {
                let _ = event_tx.send(ScanEvent::Error(format!(
                    "Port scan failed for {}: {}",
                    ip, e
                )));
            }
        }
    }
}
