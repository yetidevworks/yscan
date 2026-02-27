use chrono::Local;
use std::collections::BTreeSet;
use std::net::IpAddr;

use crate::net::device::{Device, PortInfo};

/// Generate synthetic demo devices for testing the UI
pub fn generate_demo_devices() -> Vec<Device> {
    let now = Local::now();
    let mut devices = Vec::new();

    // Router / Gateway
    let mut d = Device::new("192.168.1.1".parse::<IpAddr>().unwrap());
    d.hostname = Some("router.local".to_string());
    d.mac = Some("A4:2B:8C:12:34:56".to_string());
    d.manufacturer = Some("Netgear".to_string());
    d.sources = BTreeSet::from(["ARP".to_string(), "SSDP".to_string()]);
    d.open_ports = vec![
        PortInfo {
            port: 80,
            service: "http".to_string(),
        },
        PortInfo {
            port: 443,
            service: "https".to_string(),
        },
        PortInfo {
            port: 53,
            service: "dns".to_string(),
        },
    ];
    d.extra.insert(
        "upnp_server".to_string(),
        "Linux/3.4 UPnP/1.1 Netgear/R7000".to_string(),
    );
    d.first_seen = now - chrono::Duration::hours(48);
    d.last_seen = now;
    devices.push(d);

    // MacBook Pro (self)
    let mut d = Device::new("192.168.1.10".parse::<IpAddr>().unwrap());
    d.hostname = Some("rhuk-mbp.local".to_string());
    d.mac = Some("3C:22:FB:AB:CD:EF".to_string());
    d.manufacturer = Some("Apple".to_string());
    d.sources = BTreeSet::from(["ARP".to_string(), "mDNS".to_string()]);
    d.is_self = true;
    d.open_ports = vec![PortInfo {
        port: 22,
        service: "ssh".to_string(),
    }];
    d.first_seen = now - chrono::Duration::hours(24);
    d.last_seen = now;
    devices.push(d);

    // iPhone
    let mut d = Device::new("192.168.1.15".parse::<IpAddr>().unwrap());
    d.mdns_name = Some("iPhone".to_string());
    d.mac = Some("F0:DB:F8:11:22:33".to_string());
    d.manufacturer = Some("Apple".to_string());
    d.sources = BTreeSet::from(["ARP".to_string(), "mDNS".to_string()]);
    d.first_seen = now - chrono::Duration::hours(6);
    d.last_seen = now;
    devices.push(d);

    // Sonos Speaker
    let mut d = Device::new("192.168.1.20".parse::<IpAddr>().unwrap());
    d.hostname = Some("Living Room".to_string());
    d.mdns_name = Some("Sonos-Living-Room".to_string());
    d.mac = Some("B8:E9:37:44:55:66".to_string());
    d.manufacturer = Some("Sonos".to_string());
    d.sources = BTreeSet::from(["ARP".to_string(), "mDNS".to_string(), "SSDP".to_string()]);
    d.open_ports = vec![PortInfo {
        port: 1443,
        service: "https".to_string(),
    }];
    d.first_seen = now - chrono::Duration::hours(72);
    d.last_seen = now;
    devices.push(d);

    // NAS
    let mut d = Device::new("192.168.1.50".parse::<IpAddr>().unwrap());
    d.hostname = Some("synology-nas".to_string());
    d.mac = Some("00:11:32:AA:BB:CC".to_string());
    d.manufacturer = Some("Synology".to_string());
    d.sources = BTreeSet::from(["ARP".to_string(), "mDNS".to_string(), "SSDP".to_string()]);
    d.open_ports = vec![
        PortInfo {
            port: 22,
            service: "ssh".to_string(),
        },
        PortInfo {
            port: 80,
            service: "http".to_string(),
        },
        PortInfo {
            port: 443,
            service: "https".to_string(),
        },
        PortInfo {
            port: 445,
            service: "smb".to_string(),
        },
        PortInfo {
            port: 5000,
            service: "upnp".to_string(),
        },
        PortInfo {
            port: 5001,
            service: "upnp-tls".to_string(),
        },
    ];
    d.first_seen = now - chrono::Duration::hours(168);
    d.last_seen = now;
    devices.push(d);

    // Raspberry Pi
    let mut d = Device::new("192.168.1.100".parse::<IpAddr>().unwrap());
    d.hostname = Some("pihole".to_string());
    d.mac = Some("DC:A6:32:DD:EE:FF".to_string());
    d.manufacturer = Some("Raspberry Pi".to_string());
    d.sources = BTreeSet::from(["ARP".to_string(), "mDNS".to_string()]);
    d.open_ports = vec![
        PortInfo {
            port: 22,
            service: "ssh".to_string(),
        },
        PortInfo {
            port: 53,
            service: "dns".to_string(),
        },
        PortInfo {
            port: 80,
            service: "http".to_string(),
        },
    ];
    d.first_seen = now - chrono::Duration::hours(720);
    d.last_seen = now;
    devices.push(d);

    // Smart TV
    let mut d = Device::new("192.168.1.30".parse::<IpAddr>().unwrap());
    d.hostname = Some("Samsung-TV".to_string());
    d.mac = Some("8C:71:F8:77:88:99".to_string());
    d.manufacturer = Some("Samsung".to_string());
    d.sources = BTreeSet::from(["ARP".to_string(), "SSDP".to_string()]);
    d.extra
        .insert("upnp_server".to_string(), "Samsung TV UPnP/1.0".to_string());
    d.extra.insert(
        "upnp_location".to_string(),
        "http://192.168.1.30:9197/dmr".to_string(),
    );
    d.first_seen = now - chrono::Duration::hours(48);
    d.last_seen = now - chrono::Duration::minutes(15);
    devices.push(d);

    // Google Home
    let mut d = Device::new("192.168.1.35".parse::<IpAddr>().unwrap());
    d.mdns_name = Some("Google-Home-Kitchen".to_string());
    d.mac = Some("F4:F5:D8:AA:BB:CC".to_string());
    d.manufacturer = Some("Google".to_string());
    d.sources = BTreeSet::from(["ARP".to_string(), "mDNS".to_string()]);
    d.extra
        .insert("txt_md".to_string(), "Google Home Mini".to_string());
    d.first_seen = now - chrono::Duration::hours(96);
    d.last_seen = now;
    devices.push(d);

    // Printer
    let mut d = Device::new("192.168.1.200".parse::<IpAddr>().unwrap());
    d.hostname = Some("HP-LaserJet".to_string());
    d.mac = Some("3C:D9:2B:11:22:33".to_string());
    d.manufacturer = Some("HP".to_string());
    d.sources = BTreeSet::from(["ARP".to_string(), "mDNS".to_string()]);
    d.open_ports = vec![
        PortInfo {
            port: 80,
            service: "http".to_string(),
        },
        PortInfo {
            port: 443,
            service: "https".to_string(),
        },
        PortInfo {
            port: 631,
            service: "ipp".to_string(),
        },
        PortInfo {
            port: 9100,
            service: "jetdirect".to_string(),
        },
    ];
    d.first_seen = now - chrono::Duration::hours(336);
    d.last_seen = now - chrono::Duration::minutes(5);
    devices.push(d);

    // Unknown device (no hostname)
    let mut d = Device::new("192.168.1.42".parse::<IpAddr>().unwrap());
    d.mac = Some("E4:C3:2A:99:88:77".to_string());
    d.manufacturer = Some("TP-Link".to_string());
    d.sources = BTreeSet::from(["ARP".to_string()]);
    d.first_seen = now - chrono::Duration::minutes(30);
    d.last_seen = now - chrono::Duration::minutes(10);
    devices.push(d);

    // Ubiquiti AP
    let mut d = Device::new("192.168.1.2".parse::<IpAddr>().unwrap());
    d.hostname = Some("UAP-AC-Pro".to_string());
    d.mac = Some("FC:EC:DA:44:55:66".to_string());
    d.manufacturer = Some("Ubiquiti".to_string());
    d.sources = BTreeSet::from(["ARP".to_string(), "mDNS".to_string()]);
    d.open_ports = vec![
        PortInfo {
            port: 22,
            service: "ssh".to_string(),
        },
        PortInfo {
            port: 443,
            service: "https".to_string(),
        },
    ];
    d.first_seen = now - chrono::Duration::hours(720);
    d.last_seen = now;
    devices.push(d);

    // Amazon Echo
    let mut d = Device::new("192.168.1.45".parse::<IpAddr>().unwrap());
    d.mdns_name = Some("Echo-Bedroom".to_string());
    d.mac = Some("68:54:FD:DD:EE:FF".to_string());
    d.manufacturer = Some("Amazon".to_string());
    d.sources = BTreeSet::from(["ARP".to_string(), "mDNS".to_string()]);
    d.first_seen = now - chrono::Duration::hours(200);
    d.last_seen = now - chrono::Duration::minutes(2);
    devices.push(d);

    devices
}
