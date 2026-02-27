use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Cell, Clear, Paragraph, Row, Table, Wrap};
use ratatui::Frame;

use super::app::{
    format_relative_time, ActivityKind, App, InputMode, SortColumn, SortOrder, ViewMode,
};
use super::theme::Theme;

// ── Main render entry ────────────────────────────────────────────────

pub fn render(f: &mut Frame, app: &mut App) {
    let theme = &app.theme;

    // Background fill
    let bg = Block::default().style(Style::default().bg(theme.bg));
    f.render_widget(bg, f.area());

    // Main layout: content | footer
    let main_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(0),    // Content
            Constraint::Length(1), // Footer
        ])
        .split(f.area());

    match app.view_mode {
        ViewMode::Table => render_table_view(f, app, main_chunks[0]),
        ViewMode::Detail => render_detail_view(f, app, main_chunks[0]),
    }

    render_footer(f, app, main_chunks[1]);

    // Modal overlays
    match app.input_mode {
        InputMode::Help => render_help_overlay(f, app),
        InputMode::PortScanConfirm => render_port_scan_modal(f, app),
        InputMode::Search => {} // Search bar is inline
        InputMode::Normal => {}
    }
}

// ── Table view ───────────────────────────────────────────────────────

fn render_table_view(f: &mut Frame, app: &mut App, area: Rect) {
    if app.show_activity {
        // Split: title bar | table | activity log
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Title bar
                Constraint::Min(0),    // Table
                Constraint::Length(8), // Activity log
            ])
            .split(area);

        render_title_bar(f, app, chunks[0]);
        render_device_table(f, app, chunks[1]);
        render_activity_log(f, app, chunks[2]);
    } else {
        // Split: title bar | table (no activity log)
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Title bar
                Constraint::Min(0),    // Table
            ])
            .split(area);

        render_title_bar(f, app, chunks[0]);
        render_device_table(f, app, chunks[1]);
    }
}

fn render_title_bar(f: &mut Frame, app: &App, area: Rect) {
    let theme = &app.theme;

    let device_count = app.filtered_devices.len();
    let total_count = app.devices.len();
    let subnet = app.subnet.as_deref().unwrap_or("unknown");

    let mut spans = vec![
        Span::styled(
            " YScan ",
            Style::default()
                .fg(theme.title_fg)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(" | ", Style::default().fg(theme.muted)),
        Span::styled(
            format!("{} devices", device_count),
            Style::default().fg(theme.fg),
        ),
    ];

    if device_count != total_count {
        spans.push(Span::styled(
            format!(" (of {})", total_count),
            Style::default().fg(theme.muted),
        ));
    }

    spans.push(Span::styled(" | ", Style::default().fg(theme.muted)));
    spans.push(Span::styled(
        format!("subnet {}", subnet),
        Style::default().fg(theme.muted),
    ));

    if app.is_elevated {
        spans.push(Span::styled(" | ", Style::default().fg(theme.muted)));
        spans.push(Span::styled("elevated", Style::default().fg(theme.warning)));
    }

    // Scanning status / spinner
    if let Some(spinner_text) = app.spinner.display() {
        spans.push(Span::styled(" | ", Style::default().fg(theme.muted)));
        spans.push(Span::styled(
            spinner_text,
            Style::default().fg(theme.accent),
        ));
    }

    // Search indicator
    if app.input_mode == InputMode::Search || !app.filter_input.is_empty() {
        spans.push(Span::styled(" | ", Style::default().fg(theme.muted)));
        spans.push(Span::styled("/", Style::default().fg(theme.accent)));
        spans.push(Span::styled(
            &app.filter_input,
            Style::default().fg(theme.fg),
        ));
        if app.input_mode == InputMode::Search {
            spans.push(Span::styled("_", Style::default().fg(theme.fg)));
        }
    }

    let title_line = Line::from(spans);
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme.border))
        .style(Style::default().bg(theme.bg));

    let paragraph = Paragraph::new(title_line).block(block);
    f.render_widget(paragraph, area);
}

fn render_device_table(f: &mut Frame, app: &mut App, area: Rect) {
    let theme = &app.theme;

    let sort_indicator = match app.sort_order {
        SortOrder::Asc => " \u{25b2}",
        SortOrder::Desc => " \u{25bc}",
    };

    let col_header = |name: &str, col: SortColumn| -> String {
        if app.sort_column == col {
            format!("{}{}", name, sort_indicator)
        } else {
            name.to_string()
        }
    };

    let header = Row::new(vec![
        Cell::from(col_header("IP Address", SortColumn::IP)),
        Cell::from(col_header("Name", SortColumn::Name)),
        Cell::from(col_header("MAC Address", SortColumn::Mac)),
        Cell::from(col_header("Manufacturer", SortColumn::Manufacturer)),
        Cell::from(col_header("Last Seen", SortColumn::LastSeen)),
    ])
    .style(
        Style::default()
            .fg(theme.header_fg)
            .add_modifier(Modifier::BOLD),
    )
    .height(1);

    let rows: Vec<Row> = app
        .filtered_devices
        .iter()
        .map(|ip| {
            let device = app.devices.get(ip).unwrap();

            let ip_style = if device.is_self {
                Style::default().fg(theme.accent)
            } else {
                Style::default().fg(theme.fg)
            };

            let name = device.display_name();
            let name_style = if name.is_empty() {
                Style::default().fg(theme.muted)
            } else {
                Style::default().fg(theme.primary)
            };

            let display_name = if name.is_empty() {
                "-".to_string()
            } else {
                name.to_string()
            };

            let mac = device.mac_display();
            let mac_display = if mac.is_empty() { "-" } else { mac };

            let mfr = device.manufacturer_display();
            let mfr_display = if mfr.is_empty() { "-" } else { mfr };

            let last_seen = format_relative_time(&device.last_seen);

            let ports_indicator = if !device.open_ports.is_empty() {
                format!(" [{}]", device.open_ports.len())
            } else {
                String::new()
            };

            Row::new(vec![
                Cell::from(Line::from(vec![
                    Span::styled(ip.to_string(), ip_style),
                    Span::styled(ports_indicator, Style::default().fg(theme.success)),
                ])),
                Cell::from(Span::styled(display_name, name_style)),
                Cell::from(Span::styled(
                    mac_display.to_string(),
                    Style::default().fg(theme.fg),
                )),
                Cell::from(Span::styled(
                    mfr_display.to_string(),
                    Style::default().fg(theme.secondary),
                )),
                Cell::from(Span::styled(last_seen, Style::default().fg(theme.muted))),
            ])
        })
        .collect();

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme.border))
        .title(Span::styled(
            " Devices ",
            Style::default().fg(theme.title_fg),
        ))
        .style(Style::default().bg(theme.bg));

    let widths = [
        Constraint::Length(18), // IP Address
        Constraint::Length(24), // Name
        Constraint::Length(19), // MAC Address
        Constraint::Min(20),    // Manufacturer (flexible, gets remaining space)
        Constraint::Length(12), // Last Seen
    ];

    let table = Table::new(rows, widths)
        .header(header)
        .block(block)
        .row_highlight_style(
            Style::default()
                .bg(theme.selection_bg)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("> ");

    f.render_stateful_widget(table, area, &mut app.table_state);
}

fn render_activity_log(f: &mut Frame, app: &App, area: Rect) {
    let theme = &app.theme;

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme.border))
        .title(Span::styled(
            " Activity ",
            Style::default().fg(theme.title_fg),
        ))
        .style(Style::default().bg(theme.bg));

    let inner = block.inner(area);
    f.render_widget(block, area);

    let visible_height = inner.height as usize;
    let total = app.activity_log.len();
    let skip = if total > visible_height + app.log_scroll {
        total - visible_height - app.log_scroll
    } else {
        0
    };

    let lines: Vec<Line> = app
        .activity_log
        .iter()
        .skip(skip)
        .take(visible_height)
        .map(|event| {
            let time_str = event.timestamp.format("%H:%M:%S").to_string();
            let kind_color = match event.kind {
                ActivityKind::Discovery => theme.success,
                ActivityKind::PortScan => theme.primary,
                ActivityKind::Error => theme.error,
                ActivityKind::Info => theme.muted,
            };

            Line::from(vec![
                Span::styled(format!(" {} ", time_str), Style::default().fg(theme.muted)),
                Span::styled(&event.message, Style::default().fg(kind_color)),
            ])
        })
        .collect();

    let paragraph = Paragraph::new(lines);
    f.render_widget(paragraph, inner);
}

// ── Detail view ──────────────────────────────────────────────────────

fn render_detail_view(f: &mut Frame, app: &mut App, area: Rect) {
    let theme = &app.theme;

    let device = match app.selected_device() {
        Some(d) => d.clone(),
        None => {
            let block = Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(theme.border))
                .title(" No Device Selected ")
                .style(Style::default().bg(theme.bg));
            f.render_widget(block, area);
            return;
        }
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme.border_focused))
        .title(Span::styled(
            format!(" Device: {} ", device.ip),
            Style::default()
                .fg(theme.title_fg)
                .add_modifier(Modifier::BOLD),
        ))
        .style(Style::default().bg(theme.bg));

    let inner = block.inner(area);
    f.render_widget(block, area);

    let mut lines: Vec<Line> = Vec::new();

    // Section: Basic Info
    lines.push(Line::from(Span::styled(
        " Basic Information",
        Style::default()
            .fg(theme.primary)
            .add_modifier(Modifier::BOLD),
    )));
    lines.push(Line::from(""));

    let field = |label: &str, value: &str, theme: &Theme| -> Line {
        Line::from(vec![
            Span::styled(
                format!("   {:<16}", label),
                Style::default().fg(theme.header_fg),
            ),
            Span::styled(value.to_string(), Style::default().fg(theme.fg)),
        ])
    };

    lines.push(field("IP Address", &device.ip.to_string(), theme));

    let name = device.display_name();
    if !name.is_empty() {
        lines.push(field("Display Name", name, theme));
    }
    if let Some(ref hostname) = device.hostname {
        lines.push(field("Hostname", hostname, theme));
    }
    if let Some(ref mdns) = device.mdns_name {
        lines.push(field("mDNS Name", mdns, theme));
    }
    lines.push(field(
        "MAC Address",
        if device.mac_display().is_empty() {
            "-"
        } else {
            device.mac_display()
        },
        theme,
    ));
    lines.push(field(
        "Manufacturer",
        if device.manufacturer_display().is_empty() {
            "-"
        } else {
            device.manufacturer_display()
        },
        theme,
    ));

    if device.is_self {
        lines.push(field("Note", "This is your device", theme));
    }

    lines.push(field(
        "First Seen",
        &device.first_seen.format("%Y-%m-%d %H:%M:%S").to_string(),
        theme,
    ));
    lines.push(field(
        "Last Seen",
        &format!(
            "{} ({})",
            device.last_seen.format("%Y-%m-%d %H:%M:%S"),
            format_relative_time(&device.last_seen)
        ),
        theme,
    ));

    // Section: Sources
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        " Discovery Sources",
        Style::default()
            .fg(theme.primary)
            .add_modifier(Modifier::BOLD),
    )));
    lines.push(Line::from(""));

    for source in &device.sources {
        lines.push(Line::from(vec![
            Span::styled("   \u{2022} ", Style::default().fg(theme.success)),
            Span::styled(source.to_string(), Style::default().fg(theme.fg)),
        ]));
    }

    // Section: Open Ports
    if !device.open_ports.is_empty() {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            format!(" Open Ports ({})", device.open_ports.len()),
            Style::default()
                .fg(theme.primary)
                .add_modifier(Modifier::BOLD),
        )));
        lines.push(Line::from(""));

        for port in &device.open_ports {
            lines.push(Line::from(vec![
                Span::styled("   ", Style::default()),
                Span::styled(
                    format!("{:<8}", port.port),
                    Style::default().fg(theme.accent),
                ),
                Span::styled(&port.service, Style::default().fg(theme.fg)),
            ]));
        }
    }

    // Section: Extra Info
    let mut extras: Vec<_> = device.extra.iter().collect();
    if !extras.is_empty() {
        extras.sort_by_key(|(k, _)| (*k).clone());
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            " Additional Information",
            Style::default()
                .fg(theme.primary)
                .add_modifier(Modifier::BOLD),
        )));
        lines.push(Line::from(""));

        for (key, value) in extras {
            let display_key = key
                .replace("upnp_", "UPnP ")
                .replace("mdns_", "mDNS ")
                .replace("txt_", "TXT ");
            lines.push(Line::from(vec![
                Span::styled(
                    format!("   {:<20}", display_key),
                    Style::default().fg(theme.header_fg),
                ),
                Span::styled(value.to_string(), Style::default().fg(theme.muted)),
            ]));
        }
    }

    let paragraph = Paragraph::new(lines)
        .scroll((app.detail_scroll, 0))
        .wrap(Wrap { trim: false });

    f.render_widget(paragraph, inner);
}

// ── Footer ───────────────────────────────────────────────────────────

fn render_footer(f: &mut Frame, app: &App, area: Rect) {
    let theme = &app.theme;

    let bindings = match (&app.view_mode, &app.input_mode) {
        (_, InputMode::Search) => vec![("Esc", "cancel"), ("Enter", "apply"), ("type", "filter")],
        (_, InputMode::PortScanConfirm) => vec![("Enter", "start scan"), ("Esc", "cancel")],
        (_, InputMode::Help) => vec![("Esc/q/?", "close")],
        (ViewMode::Table, InputMode::Normal) => vec![
            ("j/k", "navigate"),
            ("Enter", "details"),
            ("p", "port scan"),
            ("/", "search"),
            ("s/S", "sort"),
            ("r", "rescan"),
            ("a", "activity"),
            ("y/Y", "copy IP/MAC"),
            ("?", "help"),
            ("q", "quit"),
        ],
        (ViewMode::Detail, InputMode::Normal) => vec![
            ("Esc/q", "back"),
            ("j/k", "scroll"),
            ("p", "port scan"),
            ("y/Y", "copy IP/MAC"),
        ],
    };

    let mut spans = Vec::new();
    for (i, (key, desc)) in bindings.iter().enumerate() {
        if i > 0 {
            spans.push(Span::styled(" | ", Style::default().fg(theme.muted)));
        }
        spans.push(Span::styled(
            format!(" {} ", key),
            Style::default()
                .fg(theme.bg)
                .bg(theme.primary)
                .add_modifier(Modifier::BOLD),
        ));
        spans.push(Span::styled(
            format!(" {}", desc),
            Style::default().fg(theme.muted),
        ));
    }

    let line = Line::from(spans);
    let footer = Paragraph::new(line).style(Style::default().bg(theme.bg));
    f.render_widget(footer, area);
}

// ── Port scan modal ──────────────────────────────────────────────────

fn render_port_scan_modal(f: &mut Frame, app: &App) {
    let theme = &app.theme;
    let area = centered_rect(50, 30, f.area());
    f.render_widget(Clear, area);

    let ip = app
        .selected_ip()
        .map(|ip| ip.to_string())
        .unwrap_or_else(|| "?".to_string());

    let port_count = app.config.port_scanner.ports.len();
    let timeout = app.config.port_scanner.timeout_ms;

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme.border_focused))
        .title(Span::styled(
            " Port Scan ",
            Style::default()
                .fg(theme.title_fg)
                .add_modifier(Modifier::BOLD),
        ))
        .style(Style::default().bg(theme.bg));

    let inner = block.inner(area);
    f.render_widget(block, area);

    let lines = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled("  Target:  ", Style::default().fg(theme.header_fg)),
            Span::styled(&ip, Style::default().fg(theme.accent)),
        ]),
        Line::from(vec![
            Span::styled("  Ports:   ", Style::default().fg(theme.header_fg)),
            Span::styled(
                format!("{} common ports", port_count),
                Style::default().fg(theme.fg),
            ),
        ]),
        Line::from(vec![
            Span::styled("  Timeout: ", Style::default().fg(theme.header_fg)),
            Span::styled(
                format!("{}ms per port", timeout),
                Style::default().fg(theme.fg),
            ),
        ]),
        Line::from(""),
        Line::from(""),
        Line::from(vec![
            Span::styled("  ", Style::default()),
            Span::styled(
                " Enter ",
                Style::default()
                    .fg(theme.bg)
                    .bg(theme.success)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(" Start Scan  ", Style::default().fg(theme.fg)),
            Span::styled(
                " Esc ",
                Style::default()
                    .fg(theme.bg)
                    .bg(theme.error)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(" Cancel", Style::default().fg(theme.fg)),
        ]),
    ];

    let paragraph = Paragraph::new(lines);
    f.render_widget(paragraph, inner);
}

// ── Help overlay ─────────────────────────────────────────────────────

fn render_help_overlay(f: &mut Frame, app: &App) {
    let theme = &app.theme;
    let area = centered_rect(60, 70, f.area());
    f.render_widget(Clear, area);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme.border_focused))
        .title(Span::styled(
            " Help ",
            Style::default()
                .fg(theme.title_fg)
                .add_modifier(Modifier::BOLD),
        ))
        .style(Style::default().bg(theme.bg));

    let inner = block.inner(area);
    f.render_widget(block, area);

    let section = |title: &str| {
        Line::from(Span::styled(
            format!("  {}", title),
            Style::default()
                .fg(theme.primary)
                .add_modifier(Modifier::BOLD),
        ))
    };

    let binding = |key: &str, desc: &str, theme: &Theme| {
        Line::from(vec![
            Span::styled(
                format!("    {:<14}", key),
                Style::default().fg(theme.accent),
            ),
            Span::styled(desc.to_string(), Style::default().fg(theme.fg)),
        ])
    };

    let lines = vec![
        Line::from(""),
        section("Navigation"),
        Line::from(""),
        binding("j / Down", "Move down", theme),
        binding("k / Up", "Move up", theme),
        binding("g", "Go to top", theme),
        binding("G", "Go to bottom", theme),
        binding("Enter", "View device details", theme),
        binding("Esc / q", "Back / Quit", theme),
        Line::from(""),
        section("Actions"),
        Line::from(""),
        binding("p", "Port scan selected device", theme),
        binding("y", "Copy IP to clipboard", theme),
        binding("Y", "Copy MAC to clipboard", theme),
        binding("r", "Force network rescan", theme),
        Line::from(""),
        section("Search & Sort"),
        Line::from(""),
        binding("/", "Search / filter devices", theme),
        binding("s", "Cycle sort column", theme),
        binding("S", "Toggle sort order", theme),
        Line::from(""),
        section("Other"),
        Line::from(""),
        binding("?", "Toggle this help", theme),
        binding("Ctrl+C", "Quit immediately", theme),
        Line::from(""),
        Line::from(Span::styled(
            "  Press Esc or ? to close",
            Style::default().fg(theme.muted),
        )),
    ];

    let paragraph = Paragraph::new(lines).alignment(Alignment::Left);
    f.render_widget(paragraph, inner);
}

// ── Helpers ──────────────────────────────────────────────────────────

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}
