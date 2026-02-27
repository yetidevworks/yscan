use anyhow::Result;
use std::net::{IpAddr, SocketAddr};
use tokio::net::TcpStream;
use tokio::sync::{mpsc, Semaphore};
use tokio::time::{timeout, Duration};

use super::device::{port_service_name, PortInfo};

/// Progress update during port scanning
pub struct PortScanProgress {
    pub scanned: usize,
    pub total: usize,
    pub ip: IpAddr,
}

/// Scan a set of ports on a target IP, reporting progress
pub async fn scan_ports(
    ip: IpAddr,
    ports: &[u16],
    timeout_ms: u64,
    progress_tx: mpsc::UnboundedSender<PortScanProgress>,
) -> Result<Vec<PortInfo>> {
    let semaphore = std::sync::Arc::new(Semaphore::new(64));
    let total = ports.len();
    let scanned = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let connect_timeout = Duration::from_millis(timeout_ms);

    let mut handles = Vec::with_capacity(total);

    for &port in ports {
        let sem = semaphore.clone();
        let scanned = scanned.clone();
        let progress_tx = progress_tx.clone();

        let handle = tokio::spawn(async move {
            let _permit = sem.acquire().await.ok();
            let addr = SocketAddr::new(ip, port);
            let result = timeout(connect_timeout, TcpStream::connect(addr)).await;

            let count = scanned.fetch_add(1, std::sync::atomic::Ordering::Relaxed) + 1;
            let _ = progress_tx.send(PortScanProgress {
                scanned: count,
                total,
                ip,
            });

            match result {
                Ok(Ok(_)) => Some(PortInfo {
                    port,
                    service: port_service_name(port).to_string(),
                }),
                _ => None,
            }
        });
        handles.push(handle);
    }

    let mut open_ports = Vec::new();
    for handle in handles {
        if let Ok(Some(port_info)) = handle.await {
            open_ports.push(port_info);
        }
    }

    open_ports.sort_by_key(|p| p.port);
    Ok(open_ports)
}
