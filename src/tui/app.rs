use std::collections::{BTreeMap, VecDeque};
use std::net::IpAddr;

use chrono::{DateTime, Local};
use ratatui::widgets::TableState;

use crate::config::Config;
use crate::net::device::Device;
use crate::net::ScanEvent;
use crate::tui::theme::Theme;

pub const MAX_ACTIVITY_LOG: usize = 500;

// ── View & Input modes ──────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViewMode {
    Table,
    Detail,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputMode {
    Normal,
    Search,
    PortScanConfirm,
    Help,
}

// ── Sort ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortColumn {
    IP,
    Name,
    Mac,
    Manufacturer,
    LastSeen,
}

impl SortColumn {
    pub fn next(self) -> Self {
        match self {
            SortColumn::IP => SortColumn::Name,
            SortColumn::Name => SortColumn::Mac,
            SortColumn::Mac => SortColumn::Manufacturer,
            SortColumn::Manufacturer => SortColumn::LastSeen,
            SortColumn::LastSeen => SortColumn::IP,
        }
    }

    #[allow(dead_code)]
    pub fn label(self) -> &'static str {
        match self {
            SortColumn::IP => "IP",
            SortColumn::Name => "Name",
            SortColumn::Mac => "MAC",
            SortColumn::Manufacturer => "Manufacturer",
            SortColumn::LastSeen => "Last Seen",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortOrder {
    Asc,
    Desc,
}

// ── Spinner ──────────────────────────────────────────────────────────

pub struct Spinner {
    frames: Vec<char>,
    frame_idx: usize,
    pub message: Option<String>,
}

impl Spinner {
    pub fn new() -> Self {
        Self {
            frames: vec!['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏'],
            frame_idx: 0,
            message: None,
        }
    }

    pub fn start(&mut self, message: &str) {
        self.message = Some(message.to_string());
        self.frame_idx = 0;
    }

    pub fn stop(&mut self) {
        self.message = None;
    }

    pub fn is_active(&self) -> bool {
        self.message.is_some()
    }

    pub fn tick(&mut self) {
        if self.message.is_some() {
            self.frame_idx = (self.frame_idx + 1) % self.frames.len();
        }
    }

    pub fn display(&self) -> Option<String> {
        self.message
            .as_ref()
            .map(|msg| format!("{} {}", self.frames[self.frame_idx], msg))
    }
}

// ── Activity log ─────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub enum ActivityKind {
    Discovery,
    PortScan,
    Error,
    Info,
}

#[derive(Debug, Clone)]
pub struct ActivityEvent {
    pub timestamp: DateTime<Local>,
    pub message: String,
    pub kind: ActivityKind,
}

// ── App state ────────────────────────────────────────────────────────

pub struct App {
    // Data
    pub devices: BTreeMap<IpAddr, Device>,
    pub filtered_devices: Vec<IpAddr>,

    // View state
    pub view_mode: ViewMode,
    pub input_mode: InputMode,
    pub table_state: TableState,
    pub detail_scroll: u16,

    // Sort / filter
    pub sort_column: SortColumn,
    pub sort_order: SortOrder,
    pub filter_input: String,

    // Scanning state
    pub scanning: bool,
    pub scan_count: usize,
    pub subnet: Option<String>,
    pub spinner: Spinner,
    pub port_scan_progress: Option<(usize, usize)>,
    pub port_scan_target: Option<IpAddr>,

    // Activity log
    pub activity_log: VecDeque<ActivityEvent>,
    pub log_scroll: usize,

    // UI
    pub theme_name: String,
    pub theme: Theme,
    pub should_quit: bool,
    pub status_message: Option<String>,
    pub config: Config,
    #[allow(dead_code)]
    pub is_demo: bool,
    pub is_elevated: bool,
    /// Whether the user has manually navigated the table (j/k/g/G/arrows).
    /// When false, selection stays at position 0 as devices stream in.
    pub user_navigated: bool,
    /// Whether the activity log panel is visible
    pub show_activity: bool,
}

impl App {
    pub fn new(config: Config, theme_name: &str, theme: Theme, is_demo: bool) -> Self {
        let is_elevated = crate::net::util::is_elevated();
        let mut app = Self {
            devices: BTreeMap::new(),
            filtered_devices: Vec::new(),
            view_mode: ViewMode::Table,
            input_mode: InputMode::Normal,
            table_state: TableState::default(),
            detail_scroll: 0,
            sort_column: SortColumn::IP,
            sort_order: SortOrder::Asc,
            filter_input: String::new(),
            scanning: false,
            scan_count: 0,
            subnet: None,
            spinner: Spinner::new(),
            port_scan_progress: None,
            port_scan_target: None,
            activity_log: VecDeque::new(),
            log_scroll: 0,
            theme_name: theme_name.to_string(),
            theme,
            should_quit: false,
            status_message: None,
            config,
            is_demo,
            is_elevated,
            user_navigated: false,
            show_activity: false,
        };
        // Detect subnet for display
        if let Ok(ip) = crate::net::util::local_ip() {
            if let Ok(subnet) = crate::net::util::local_subnet(ip) {
                app.subnet = Some(subnet.to_string());
            }
        }
        app
    }

    // ── Theme ─────────────────────────────────────────────────────────

    pub fn cycle_theme(&mut self) {
        let next = Theme::next_name(&self.theme_name);
        self.theme_name = next.to_string();
        self.theme = Theme::by_name(next);
    }

    // ── Device management ────────────────────────────────────────────

    pub fn add_or_update_device(&mut self, device: Device) {
        let ip = device.ip;
        let existing = self.devices.entry(ip).or_insert_with(|| Device::new(ip));
        existing.merge(&device);
        self.rebuild_filtered();
    }

    pub fn selected_device(&self) -> Option<&Device> {
        let idx = self.table_state.selected()?;
        let ip = self.filtered_devices.get(idx)?;
        self.devices.get(ip)
    }

    pub fn selected_ip(&self) -> Option<IpAddr> {
        let idx = self.table_state.selected()?;
        self.filtered_devices.get(idx).copied()
    }

    // ── Sort & filter ────────────────────────────────────────────────

    pub fn rebuild_filtered(&mut self) {
        let selected_ip = self.selected_ip();

        let query = self.filter_input.to_lowercase();

        let mut ips: Vec<IpAddr> = self
            .devices
            .iter()
            .filter(|(ip, dev)| {
                if query.is_empty() {
                    return true;
                }
                let ip_str = ip.to_string().to_lowercase();
                let name = dev.display_name().to_lowercase();
                let mac = dev.mac_display().to_lowercase();
                let mfr = dev.manufacturer_display().to_lowercase();
                ip_str.contains(&query)
                    || name.contains(&query)
                    || mac.contains(&query)
                    || mfr.contains(&query)
            })
            .map(|(ip, _)| *ip)
            .collect();

        // Sort
        let devices = &self.devices;
        let sort_col = self.sort_column;
        let sort_order = self.sort_order;

        ips.sort_by(|a, b| {
            let da = devices.get(a).unwrap();
            let db = devices.get(b).unwrap();
            let cmp = match sort_col {
                SortColumn::IP => a.cmp(b),
                SortColumn::Name => da
                    .display_name()
                    .to_lowercase()
                    .cmp(&db.display_name().to_lowercase()),
                SortColumn::Mac => da.mac_display().cmp(db.mac_display()),
                SortColumn::Manufacturer => da
                    .manufacturer_display()
                    .to_lowercase()
                    .cmp(&db.manufacturer_display().to_lowercase()),
                SortColumn::LastSeen => da.last_seen.cmp(&db.last_seen),
            };
            let primary = match sort_order {
                SortOrder::Asc => cmp,
                SortOrder::Desc => cmp.reverse(),
            };
            primary.then_with(|| a.cmp(b))
        });

        self.filtered_devices = ips;

        // Restore selection — if user hasn't navigated, pin to first row
        if !self.user_navigated {
            if !self.filtered_devices.is_empty() {
                self.table_state.select(Some(0));
            } else {
                self.table_state.select(None);
            }
            return;
        }

        if let Some(ip) = selected_ip {
            if let Some(pos) = self.filtered_devices.iter().position(|i| *i == ip) {
                self.table_state.select(Some(pos));
                return;
            }
        }
        if !self.filtered_devices.is_empty() {
            self.table_state.select(Some(0));
        } else {
            self.table_state.select(None);
        }
    }

    // ── Navigation ───────────────────────────────────────────────────

    pub fn move_selection(&mut self, delta: i32) {
        self.user_navigated = true;
        let len = self.filtered_devices.len();
        if len == 0 {
            return;
        }
        let current = self.table_state.selected().unwrap_or(0);
        let new = if delta > 0 {
            (current + delta as usize).min(len - 1)
        } else {
            current.saturating_sub((-delta) as usize)
        };
        self.table_state.select(Some(new));
    }

    pub fn select_first(&mut self) {
        self.user_navigated = true;
        if !self.filtered_devices.is_empty() {
            self.table_state.select(Some(0));
        }
    }

    pub fn select_last(&mut self) {
        self.user_navigated = true;
        if !self.filtered_devices.is_empty() {
            self.table_state
                .select(Some(self.filtered_devices.len() - 1));
        }
    }

    // ── Activity log ─────────────────────────────────────────────────

    pub fn push_activity(&mut self, kind: ActivityKind, message: String) {
        if self.activity_log.len() >= MAX_ACTIVITY_LOG {
            self.activity_log.pop_front();
        }
        self.activity_log.push_back(ActivityEvent {
            timestamp: Local::now(),
            message,
            kind,
        });
        // Auto-scroll to bottom
        if self.log_scroll <= 1 {
            self.log_scroll = 0;
        }
    }

    // ── Event handling ───────────────────────────────────────────────

    pub fn handle_scan_event(&mut self, event: ScanEvent) {
        match event {
            ScanEvent::DeviceDiscovered(boxed) => {
                let device = *boxed;
                let ip = device.ip;
                let name = if !device.display_name().is_empty() {
                    format!("{} ({})", device.display_name(), ip)
                } else {
                    ip.to_string()
                };
                self.add_or_update_device(device);
                self.push_activity(ActivityKind::Discovery, format!("Discovered {}", name));
            }
            ScanEvent::PortScanResult { ip, ports } => {
                if let Some(device) = self.devices.get_mut(&ip) {
                    device.open_ports = ports.clone();
                }
                let count = ports.len();
                self.push_activity(
                    ActivityKind::PortScan,
                    format!("Port scan complete for {} - {} open ports", ip, count),
                );
                self.port_scan_progress = None;
                self.port_scan_target = None;
                self.spinner.stop();
            }
            ScanEvent::PortScanProgress {
                ip: _,
                scanned,
                total,
            } => {
                self.port_scan_progress = Some((scanned, total));
                self.spinner
                    .start(&format!("Scanning ports... {}/{}", scanned, total));
            }
            ScanEvent::Error(msg) => {
                self.push_activity(ActivityKind::Error, msg);
            }
            ScanEvent::ScanStarted => {
                self.scanning = true;
                self.spinner.start("Scanning network...");
                self.push_activity(ActivityKind::Info, "Scan cycle started".to_string());
            }
            ScanEvent::ScanCompleted { device_count } => {
                self.scanning = false;
                self.scan_count += 1;
                self.spinner.stop();
                self.push_activity(
                    ActivityKind::Info,
                    format!("Scan complete - {} devices", device_count),
                );
            }
        }
    }
}

/// Format a duration relative to now (e.g., "just now", "5m ago", "2h ago")
pub fn format_relative_time(dt: &DateTime<Local>) -> String {
    let now = Local::now();
    let diff = now.signed_duration_since(*dt);

    if diff.num_seconds() < 60 {
        "just now".to_string()
    } else if diff.num_minutes() < 60 {
        format!("{}m ago", diff.num_minutes())
    } else if diff.num_hours() < 24 {
        format!("{}h ago", diff.num_hours())
    } else {
        format!("{}d ago", diff.num_days())
    }
}
