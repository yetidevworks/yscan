# YScan

```
_____.___. _________                     
\__  |   |/   _____/ ____ _____    ____  
 /   |   |\_____  \_/ ___\\__  \  /    \ 
 \____   |/        \  \___ / __ \|   |  \
 / ______/_______  /\___  >____  /___|  /
 \/              \/     \/     \/     \/ 
      Network discovery made easy!
```

A TUI-first network scanner that discovers devices on your local network using ARP, mDNS, SSDP, and hostname probing. See every device — named and identified — in a live, sortable, searchable dashboard.

**Supported Platforms:** macOS and Linux

## Features

- **TUI Dashboard** — Interactive table of all discovered devices with live updates
- **Multi-protocol discovery** — ARP sweep, mDNS service browsing, SSDP/UPnP, reverse DNS, and HTTP banner detection
- **Automatic identification** — Resolves hostnames, manufacturer names (via OUI), and service types
- **Hostname probing** — Unnamed devices get probed via reverse DNS and HTTP banners (identifies Proxmox, TrueNAS, Pi-hole, UniFi, and more)
- **Port scanning** — On-demand port scan for any device with 29 common ports
- **Sort & filter** — Sort by any column, search across IP, name, MAC, and manufacturer
- **Clipboard support** — Copy IP or MAC address with a single keystroke
- **8 color themes** — Dark, Light, Dracula, Nord, One Dark, Monokai Pro, Tokyo Night, Synthwave — press `t` to cycle live, your choice is saved automatically
- **Oneshot mode** — Single scan with table or JSON output for scripting
- **Periodic rescans** — Continuous background scanning at configurable intervals

## Installation

### Homebrew (recommended)

```bash
brew install yetidevworks/yscan/yscan
```

### From crates.io

```bash
cargo install yscan
```

### From source

```bash
git clone https://github.com/yetidevworks/yscan
cd yscan
cargo install --path .
```

### Pre-built binaries

Download from [GitHub Releases](https://github.com/yetidevworks/yscan/releases).

## Quick Start

```bash
# Open the TUI dashboard (recommended: run with sudo for ARP scanning)
sudo yscan

# One-shot scan to stdout
sudo yscan scan

# JSON output for scripting
sudo yscan scan --json

# Use a specific network interface
sudo yscan -i en0

# Start with a specific theme, or press 't' in the TUI to cycle (auto-saved)
yscan --theme tokyo-night
```

> **Note:** `sudo` is recommended for ARP-based discovery. Without elevated privileges, ARP scanning is limited to devices already in the system's ARP cache. mDNS, SSDP, and hostname probing work without sudo.

## TUI Dashboard

Run `yscan` with no arguments to open the interactive dashboard:

```
┌─ YScan | 12 devices | 192.168.1.0/24 | elevated | ⠹ Scanning network... ───┐
│                                                                            │
│  IP Address       Name                    MAC Address        Manufacturer  │
│  ───────────────────────────────────────────────────────────────────────── │
│  192.168.1.1      router.local            aa:bb:cc:dd:ee:ff  Ubiquiti      │
│▸ 192.168.1.10     NAS.local               11:22:33:44:55:66  Synology      │
│  192.168.1.20     Proxmox VE              77:88:99:aa:bb:cc  Intel         │
│  192.168.1.30     Living Room TV          dd:ee:ff:00:11:22  Samsung       │
│  192.168.1.45     pi-hole.local           33:44:55:66:77:88  Raspberry Pi  │
│  192.168.1.100    -                       99:aa:bb:cc:dd:ee  Apple         │
│                                                                            │
├─ Activity ─────────────────────────────────────────────────────────────────┤
│  10:30:15  Discovered router.local (192.168.1.1)                           │
│  10:30:16  Discovered NAS.local (192.168.1.10)                             │
│  10:30:17  Scan complete - 12 devices                                      │
├────────────────────────────────────────────────────────────────────────────┤
│  [/]search [s]ort [p]ort scan [y]ank IP [a]ctivity [r]escan [?]help [q]uit │
└────────────────────────────────────────────────────────────────────────────┘
```

### Keyboard Shortcuts

| Key | Action |
|-----|--------|
| `j` / `k` / `↑` / `↓` | Navigate device list |
| `g` / `G` | Jump to first / last device |
| `PageUp` / `PageDown` | Scroll by page |
| `Enter` | Open device detail view |
| `/` | Search / filter devices |
| `s` | Cycle sort column (IP → Name → MAC → Manufacturer → Last Seen) |
| `S` | Toggle sort order (ascending / descending) |
| `p` | Port scan selected device |
| `y` | Copy IP address to clipboard |
| `Y` | Copy MAC address to clipboard |
| `a` | Toggle activity log panel |
| `t` | Cycle color theme (saved automatically) |
| `r` | Trigger manual rescan |
| `?` | Show help overlay |
| `q` / `Esc` | Quit (or clear search filter) |
| `Ctrl+C` | Force quit |
| `Ctrl+Z` | Suspend to background (Unix) |

### Detail View

Press `Enter` on any device to see full details including:
- IP address and MAC address
- Hostname and mDNS name
- Manufacturer (via IEEE OUI database)
- Discovery sources (ARP, mDNS, SSDP, DNS)
- Open ports with service names
- First seen and last seen timestamps
- Extra metadata from mDNS/SSDP

## How It Works

YScan uses multiple discovery protocols in parallel, then merges results by IP address:

```
┌────────────────────────────────────────────────────────────────┐
│                      Discovery Engine                          │
│                                                                │
│  ┌─────────┐  ┌─────────┐  ┌─────────┐  ┌───────────────────┐  │
│  │   ARP   │  │  mDNS   │  │  SSDP   │  │  Hostname Probe   │  │
│  │  sweep  │  │ browse  │  │ search  │  │  (reverse DNS +   │  │
│  │         │  │         │  │         │  │   HTTP banners)   │  │
│  └────┬────┘  └────┬────┘  └────┬────┘  └────────┬──────────┘  │
│       │            │            │                │             │
│       └────────────┴────────────┴────────────────┘             │
│                            │                                   │
│                    Merge by IP address                         │
│                            │                                   │
│                    ┌───────▼───────┐                           │
│                    │  Device Map   │                           │
│                    └───────┬───────┘                           │
│                            │                                   │
└────────────────────────────┼───────────────────────────────────┘
                             │
                     ┌───────▼───────┐
                     │   TUI / CLI   │
                     └───────────────┘
```

### Discovery Protocols

| Protocol | What it finds | Requires sudo? |
|----------|---------------|----------------|
| **ARP** | All devices on the subnet (IP + MAC + manufacturer via OUI) | Yes (for active sweep) |
| **mDNS** | Devices advertising services (printers, smart home, NAS, etc.) | No |
| **SSDP** | UPnP devices (routers, media servers, smart TVs) | No |
| **Hostname** | Reverse DNS names + HTTP banner identification for unnamed devices | No |

### Hostname Probing

Devices that aren't discovered by mDNS or SSDP (like Proxmox servers, bare-metal hosts, or network appliances) often have no display name. YScan automatically probes these unnamed devices using:

1. **Reverse DNS (PTR lookup)** — Fast, returns hostnames like `pve.local` or `nas.home`
2. **HTTP banner detection** — Probes ports 80, 8080, 8006, 443, 8443 with a raw HTTP request and identifies the device from `Server` headers and `<title>` tags

Known server signatures are mapped to friendly names:

| Server Banner | Identified As |
|---------------|---------------|
| `pve-api-daemon/3.0` | Proxmox VE |
| `TrueNAS` | TrueNAS |
| `Synology` | Synology DSM |
| `UniFi` | UniFi Controller |
| `Pi-hole` | Pi-hole |
| `Home Assistant` | Home Assistant |
| `OPNsense` / `pfSense` | OPNsense / pfSense |

## Configuration

YScan works out of the box with no configuration. Optionally create a config file to customize behavior.

**Config file location:**
- macOS: `~/Library/Application Support/yscan/config.yaml`
- Linux: `~/.config/yscan/config.yaml`

```yaml
# Network interface (auto-detected if omitted)
network_interface: en0

# Seconds between automatic rescans (default: 30)
scan_interval: 30

# Timeout per scan cycle in seconds (default: 10)
scan_timeout: 10

# Color theme: dark, light, dracula, nord, onedark, monokai-pro, tokyo-night, synthwave
theme: dark

# Toggle individual scanners
scanners:
  arp: true
  mdns: true
  ssdp: true

# Port scanner settings
port_scanner:
  timeout_ms: 5000
  ports:
    - 22
    - 80
    - 443
    - 8080
    # ... add or remove ports as needed
```

### Default Ports

The port scanner checks these 29 ports by default:

`21` `22` `23` `25` `53` `80` `110` `135` `139` `143` `389` `443` `445` `993` `995` `1433` `1521` `3306` `3389` `5432` `5900` `8080` `8443` `9000` `9090` `9200` `9300` `10000` `27017`

## CLI Reference

```
yscan [OPTIONS] [COMMAND]

Commands:
  scan    One-shot scan, print results to stdout
  demo    Launch with synthetic demo data

Options:
  -i, --interface <INTERFACE>  Network interface (e.g., en0)
      --theme <THEME>          Color theme [default: dark]
  -h, --help                   Print help
  -V, --version                Print version
```

### Examples

```bash
# Interactive TUI (default)
sudo yscan

# One-shot scan with table output
sudo yscan scan

# JSON output for piping to jq, scripts, etc.
sudo yscan scan --json

# Scan a specific interface
sudo yscan -i eth0

# Try the TUI with demo data (no network access needed)
yscan demo

# Synthwave theme
yscan --theme synthwave
```

## License

MIT
