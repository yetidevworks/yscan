use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub network_interface: Option<String>,

    #[serde(default = "default_scan_interval")]
    pub scan_interval: u64,

    #[serde(default = "default_scan_timeout")]
    pub scan_timeout: u64,

    #[serde(default = "default_theme")]
    pub theme: String,

    #[serde(default)]
    pub scanners: ScannersConfig,

    #[serde(default)]
    pub port_scanner: PortScannerConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScannersConfig {
    #[serde(default = "default_true")]
    pub arp: bool,

    #[serde(default = "default_true")]
    pub mdns: bool,

    #[serde(default = "default_true")]
    pub ssdp: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortScannerConfig {
    #[serde(default = "default_port_timeout")]
    pub timeout_ms: u64,

    #[serde(default = "default_ports")]
    pub ports: Vec<u16>,
}

fn default_scan_interval() -> u64 {
    30
}
fn default_scan_timeout() -> u64 {
    10
}
fn default_theme() -> String {
    "dark".to_string()
}
fn default_true() -> bool {
    true
}
fn default_port_timeout() -> u64 {
    5000
}
fn default_ports() -> Vec<u16> {
    vec![
        21, 22, 23, 25, 53, 80, 110, 135, 139, 143, 389, 443, 445, 993, 995, 1433, 1521, 3306,
        3389, 5432, 5900, 8080, 8443, 9000, 9090, 9200, 9300, 10000, 27017,
    ]
}

impl Default for Config {
    fn default() -> Self {
        Self {
            network_interface: None,
            scan_interval: default_scan_interval(),
            scan_timeout: default_scan_timeout(),
            theme: default_theme(),
            scanners: ScannersConfig::default(),
            port_scanner: PortScannerConfig::default(),
        }
    }
}

impl Default for ScannersConfig {
    fn default() -> Self {
        Self {
            arp: true,
            mdns: true,
            ssdp: true,
        }
    }
}

impl Default for PortScannerConfig {
    fn default() -> Self {
        Self {
            timeout_ms: default_port_timeout(),
            ports: default_ports(),
        }
    }
}

impl Config {
    pub fn load() -> Result<Self> {
        let path = Self::config_path();
        if path.exists() {
            let contents = std::fs::read_to_string(&path)?;
            let config: Config = serde_yaml::from_str(&contents)?;
            Ok(config)
        } else {
            Ok(Config::default())
        }
    }

    pub fn save(&self) -> Result<()> {
        let path = Self::config_path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let contents = serde_yaml::to_string(self)?;
        std::fs::write(&path, contents)?;
        Ok(())
    }

    pub fn config_path() -> PathBuf {
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("yscan")
            .join("config.yaml")
    }
}
