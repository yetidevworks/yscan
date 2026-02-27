use anyhow::Result;
use ipnet::Ipv4Net;
use std::collections::{HashMap, HashSet};
use std::net::{IpAddr, Ipv4Addr, SocketAddr, UdpSocket};
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

/// Sweep the subnet to populate the ARP cache, then read it.
/// Uses UDP (fast, no handshake) + TCP fallback to trigger ARP resolution,
/// while polling the native ARP cache for results.
pub async fn discover(
    subnet: &Ipv4Net,
    scan_timeout: Duration,
    tx: mpsc::UnboundedSender<Device>,
) -> Result<()> {
    let hosts: Vec<Ipv4Addr> = super::util::subnet_hosts(subnet);

    // Phase 1: blast UDP packets to trigger ARP — this is instant, no handshake
    let udp_hosts = hosts.clone();
    tokio::task::spawn_blocking(move || udp_sweep(&udp_hosts));

    // Phase 2: TCP probes for hosts that might not respond to UDP
    let semaphore = std::sync::Arc::new(Semaphore::new(256));
    let mut handles = Vec::new();
    for host in &hosts {
        let host = *host;
        let sem = semaphore.clone();
        let handle = tokio::spawn(async move {
            let _permit = sem.acquire().await.ok();
            let addr = SocketAddr::new(IpAddr::V4(host), 80);
            let _ = timeout(Duration::from_millis(100), TcpStream::connect(addr)).await;
        });
        handles.push(handle);
    }

    // Phase 3: poll the ARP cache while sweep is running, stream new devices
    let mut seen = HashSet::new();
    let deadline = Instant::now() + scan_timeout;
    let poll_interval = Duration::from_millis(200);

    loop {
        if let Ok(entries) = read_arp_cache().await {
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

        if Instant::now() >= deadline {
            break;
        }

        let all_done = handles.iter().all(|h| h.is_finished());
        if all_done {
            // Final cache read after all probes complete
            if let Ok(entries) = read_arp_cache().await {
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
            break;
        }

        tokio::time::sleep(poll_interval).await;
    }

    for handle in handles {
        handle.abort();
    }

    Ok(())
}

/// Blast a single UDP byte to port 9 (discard) on every host.
/// This triggers ARP resolution without needing a TCP handshake.
/// Runs on a blocking thread since std::net::UdpSocket is sync.
fn udp_sweep(hosts: &[Ipv4Addr]) {
    let Ok(socket) = UdpSocket::bind("0.0.0.0:0") else {
        return;
    };
    // Non-blocking so sends don't stall on unreachable hosts
    let _ = socket.set_nonblocking(true);

    for &host in hosts {
        let _ = socket.send_to(&[0u8], SocketAddr::new(IpAddr::V4(host), 9));
        // Also hit port 33434 (traceroute) as a second trigger
        let _ = socket.send_to(&[0u8], SocketAddr::new(IpAddr::V4(host), 33434));
    }
}

// ── Platform-specific ARP cache reading ─────────────────────────────

/// Read the ARP cache using the fastest available method for the platform
async fn read_arp_cache() -> Result<HashMap<Ipv4Addr, String>> {
    #[cfg(target_os = "macos")]
    {
        // Try native sysctl first, fall back to arp -a
        match tokio::task::spawn_blocking(read_arp_cache_macos_native).await? {
            Ok(entries) if !entries.is_empty() => Ok(entries),
            _ => read_arp_cache_command().await,
        }
    }

    #[cfg(target_os = "linux")]
    {
        match tokio::task::spawn_blocking(read_arp_cache_linux_proc).await? {
            Ok(entries) if !entries.is_empty() => Ok(entries),
            _ => read_arp_cache_command().await,
        }
    }

    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    {
        read_arp_cache_command().await
    }
}

/// macOS: read ARP cache via sysctl CTL_NET route info
#[cfg(target_os = "macos")]
fn read_arp_cache_macos_native() -> Result<HashMap<Ipv4Addr, String>> {
    use std::mem;
    use std::os::raw::c_int;

    // sysctl MIB for ARP table: CTL_NET, PF_ROUTE, 0, AF_INET, NET_RT_FLAGS, RTF_LLINFO
    const CTL_NET: c_int = 4;
    const PF_ROUTE: c_int = 17;
    const AF_INET: c_int = 2;
    const NET_RT_FLAGS: c_int = 2;
    const RTF_LLINFO: c_int = 0x400;

    let mib: [c_int; 6] = [CTL_NET, PF_ROUTE, 0, AF_INET, NET_RT_FLAGS, RTF_LLINFO];

    // First call: get buffer size
    let mut buf_len: libc::size_t = 0;
    let ret = unsafe {
        libc::sysctl(
            mib.as_ptr() as *mut c_int,
            6,
            std::ptr::null_mut(),
            &mut buf_len,
            std::ptr::null_mut(),
            0,
        )
    };
    if ret != 0 || buf_len == 0 {
        anyhow::bail!("sysctl size query failed");
    }

    // Second call: read data
    let mut buf = vec![0u8; buf_len];
    let ret = unsafe {
        libc::sysctl(
            mib.as_ptr() as *mut c_int,
            6,
            buf.as_mut_ptr() as *mut libc::c_void,
            &mut buf_len,
            std::ptr::null_mut(),
            0,
        )
    };
    if ret != 0 {
        anyhow::bail!("sysctl data query failed");
    }
    buf.truncate(buf_len);

    let mut entries = HashMap::new();
    let mut offset = 0;

    while offset + mem::size_of::<rt_msghdr>() <= buf.len() {
        let hdr = unsafe { &*(buf.as_ptr().add(offset) as *const rt_msghdr) };
        let msg_len = hdr.rtm_msglen as usize;
        if msg_len == 0 || offset + msg_len > buf.len() {
            break;
        }

        // Only process messages with LLINFO flag (ARP entries)
        if hdr.rtm_flags & RTF_LLINFO != 0 {
            let sa_start = offset + mem::size_of::<rt_msghdr>();
            if sa_start + mem::size_of::<libc::sockaddr_in>() <= offset + msg_len {
                // First sockaddr is the IP (sockaddr_in)
                let sa = unsafe { &*(buf.as_ptr().add(sa_start) as *const libc::sockaddr_in) };
                if sa.sin_family as c_int == AF_INET {
                    let ip_bytes = sa.sin_addr.s_addr.to_ne_bytes();
                    let ip = Ipv4Addr::new(ip_bytes[0], ip_bytes[1], ip_bytes[2], ip_bytes[3]);

                    // Second sockaddr is the link-layer address (sockaddr_dl)
                    let sa_len = sa.sin_len as usize;
                    let sdl_start = sa_start + roundup(sa_len);
                    if sdl_start + 8 <= offset + msg_len {
                        let sdl = unsafe { &*(buf.as_ptr().add(sdl_start) as *const sockaddr_dl) };
                        if sdl.sdl_alen == 6 {
                            let mac_offset = sdl_start + 8 + sdl.sdl_nlen as usize;
                            if mac_offset + 6 <= offset + msg_len {
                                let mac_bytes = &buf[mac_offset..mac_offset + 6];
                                let mac = format!(
                                    "{:02X}:{:02X}:{:02X}:{:02X}:{:02X}:{:02X}",
                                    mac_bytes[0],
                                    mac_bytes[1],
                                    mac_bytes[2],
                                    mac_bytes[3],
                                    mac_bytes[4],
                                    mac_bytes[5]
                                );
                                if mac != "00:00:00:00:00:00" && mac != "FF:FF:FF:FF:FF:FF" {
                                    entries.insert(ip, mac);
                                }
                            }
                        }
                    }
                }
            }
        }

        offset += msg_len;
    }

    Ok(entries)
}

#[cfg(target_os = "macos")]
fn roundup(len: usize) -> usize {
    if len > 0 {
        1 + ((len - 1) | (std::mem::size_of::<u64>() - 1))
    } else {
        std::mem::size_of::<u64>()
    }
}

#[cfg(target_os = "macos")]
#[repr(C)]
struct rt_msghdr {
    rtm_msglen: u16,
    rtm_version: u8,
    rtm_type: u8,
    rtm_index: u16,
    rtm_flags: i32,
    rtm_addrs: i32,
    rtm_pid: i32,
    rtm_seq: i32,
    rtm_errno: i32,
    rtm_use: i32,
    rtm_inits: u32,
    rtm_rmx: rt_metrics,
}

#[cfg(target_os = "macos")]
#[repr(C)]
struct rt_metrics {
    rmx_locks: u32,
    rmx_mtu: u32,
    rmx_hopcount: u32,
    rmx_expire: i32,
    rmx_recvpipe: u32,
    rmx_sendpipe: u32,
    rmx_ssthresh: u32,
    rmx_rtt: u32,
    rmx_rttvar: u32,
    rmx_pksent: u32,
    rmx_state: u32,
    rmx_filler: [u32; 3],
}

#[cfg(target_os = "macos")]
#[repr(C)]
struct sockaddr_dl {
    sdl_len: u8,
    sdl_family: u8,
    sdl_index: u16,
    sdl_type: u8,
    sdl_nlen: u8,
    sdl_alen: u8,
    sdl_slen: u8,
    // followed by interface name and link-layer address
}

/// Linux: read /proc/net/arp directly
#[cfg(target_os = "linux")]
fn read_arp_cache_linux_proc() -> Result<HashMap<Ipv4Addr, String>> {
    let contents = std::fs::read_to_string("/proc/net/arp")?;
    let mut entries = HashMap::new();

    for line in contents.lines().skip(1) {
        // Format: IP HW_TYPE FLAGS HW_ADDR MASK DEVICE
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() >= 4 {
            // Flags: 0x2 = ATF_COM (completed entry)
            let flags = u32::from_str_radix(parts[2].trim_start_matches("0x"), 16).unwrap_or(0);
            if flags & 0x2 == 0 {
                continue; // Skip incomplete entries
            }

            if let Ok(ip) = parts[0].parse::<Ipv4Addr>() {
                let mac = parts[3].to_uppercase();
                if mac != "00:00:00:00:00:00" && mac != "FF:FF:FF:FF:FF:FF" {
                    entries.insert(ip, mac);
                }
            }
        }
    }

    Ok(entries)
}

/// Fallback: parse `arp -a` output
async fn read_arp_cache_command() -> Result<HashMap<Ipv4Addr, String>> {
    let output = tokio::process::Command::new("arp")
        .arg("-a")
        .output()
        .await?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut entries = HashMap::new();

    for line in stdout.lines() {
        if let Some((ip, mac)) = parse_arp_line(line) {
            if mac != "(incomplete)" && mac != "FF:FF:FF:FF:FF:FF" {
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
    let ip_start = line.find('(')? + 1;
    let ip_end = line.find(')')?;
    let ip_str = &line[ip_start..ip_end];
    let ip: Ipv4Addr = ip_str.parse().ok()?;

    let at_idx = line.find(" at ")?;
    let after_at = &line[at_idx + 4..];
    let mac_end = after_at.find(' ').unwrap_or(after_at.len());
    let mac = &after_at[..mac_end];

    if mac == "(incomplete)" {
        return None;
    }

    Some((ip, normalize_mac(mac)))
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
