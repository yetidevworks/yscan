use anyhow::{Context, Result};
use ipnet::Ipv4Net;
use std::net::{IpAddr, Ipv4Addr};

/// Detect the local IPv4 address
pub fn local_ip() -> Result<Ipv4Addr> {
    let ip = local_ip_address::local_ip().context("Failed to detect local IP address")?;
    match ip {
        IpAddr::V4(v4) => Ok(v4),
        IpAddr::V6(_) => anyhow::bail!("Only IPv4 is supported"),
    }
}

/// Get the subnet CIDR for the local network (assumes /24)
pub fn local_subnet(ip: Ipv4Addr) -> Result<Ipv4Net> {
    let net = Ipv4Net::new(ip, 24).context("Failed to create subnet")?;
    Ok(net)
}

/// Generate all host IPs in a /24 subnet (excluding network and broadcast)
pub fn subnet_hosts(subnet: &Ipv4Net) -> Vec<Ipv4Addr> {
    subnet.hosts().collect()
}

/// Check if running with elevated privileges
pub fn is_elevated() -> bool {
    nix::unistd::geteuid().is_root()
}
