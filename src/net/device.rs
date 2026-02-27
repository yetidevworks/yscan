use chrono::{DateTime, Local};
use serde::Serialize;
use std::collections::{BTreeSet, HashMap};
use std::net::IpAddr;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize)]
pub enum DiscoverySource {
    ArpCache,
    Mdns,
    Ssdp,
    Hostname,
}

impl std::fmt::Display for DiscoverySource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DiscoverySource::ArpCache => write!(f, "ARP"),
            DiscoverySource::Mdns => write!(f, "mDNS"),
            DiscoverySource::Ssdp => write!(f, "SSDP"),
            DiscoverySource::Hostname => write!(f, "DNS"),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct PortInfo {
    pub port: u16,
    pub service: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct Device {
    pub ip: IpAddr,
    pub mac: Option<String>,
    pub hostname: Option<String>,
    pub mdns_name: Option<String>,
    pub manufacturer: Option<String>,
    pub sources: BTreeSet<String>,
    pub first_seen: DateTime<Local>,
    pub last_seen: DateTime<Local>,
    pub open_ports: Vec<PortInfo>,
    pub extra: HashMap<String, String>,
    pub is_self: bool,
}

impl Device {
    pub fn new(ip: IpAddr) -> Self {
        let now = Local::now();
        Self {
            ip,
            mac: None,
            hostname: None,
            mdns_name: None,
            manufacturer: None,
            sources: BTreeSet::new(),
            first_seen: now,
            last_seen: now,
            open_ports: Vec::new(),
            extra: HashMap::new(),
            is_self: false,
        }
    }

    pub fn display_name(&self) -> &str {
        if let Some(ref name) = self.hostname {
            name
        } else if let Some(ref name) = self.mdns_name {
            name
        } else {
            ""
        }
    }

    pub fn mac_display(&self) -> &str {
        self.mac.as_deref().unwrap_or("")
    }

    pub fn manufacturer_display(&self) -> &str {
        self.manufacturer.as_deref().unwrap_or("")
    }

    /// Merge data from another device discovery into this one
    pub fn merge(&mut self, other: &Device) {
        self.last_seen = Local::now();

        if self.mac.is_none() && other.mac.is_some() {
            self.mac.clone_from(&other.mac);
        }
        if self.hostname.is_none() && other.hostname.is_some() {
            self.hostname.clone_from(&other.hostname);
        }
        if self.mdns_name.is_none() && other.mdns_name.is_some() {
            self.mdns_name.clone_from(&other.mdns_name);
        }
        if self.manufacturer.is_none() && other.manufacturer.is_some() {
            self.manufacturer.clone_from(&other.manufacturer);
        }
        for source in &other.sources {
            self.sources.insert(source.clone());
        }
        for (k, v) in &other.extra {
            self.extra.entry(k.clone()).or_insert_with(|| v.clone());
        }
    }
}

/// Well-known port to service name mapping
pub fn port_service_name(port: u16) -> &'static str {
    match port {
        20 => "ftp-data",
        21 => "ftp",
        22 => "ssh",
        23 => "telnet",
        25 => "smtp",
        53 => "dns",
        67 => "dhcp",
        68 => "dhcp",
        80 => "http",
        110 => "pop3",
        123 => "ntp",
        135 => "msrpc",
        137 => "netbios",
        138 => "netbios",
        139 => "netbios",
        143 => "imap",
        161 => "snmp",
        389 => "ldap",
        443 => "https",
        445 => "smb",
        465 => "smtps",
        514 => "syslog",
        587 => "submission",
        631 => "ipp",
        636 => "ldaps",
        993 => "imaps",
        995 => "pop3s",
        1433 => "mssql",
        1521 => "oracle",
        1883 => "mqtt",
        2049 => "nfs",
        3306 => "mysql",
        3389 => "rdp",
        5432 => "postgres",
        5900 => "vnc",
        5353 => "mdns",
        6379 => "redis",
        8080 => "http-alt",
        8443 => "https-alt",
        8883 => "mqtt-tls",
        9000 => "portainer",
        9090 => "prometheus",
        9200 => "elasticsearch",
        9300 => "es-transport",
        10000 => "webmin",
        27017 => "mongodb",
        _ => "unknown",
    }
}
