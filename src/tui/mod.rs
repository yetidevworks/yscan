pub mod app;
pub mod demo;
pub mod theme;
pub mod ui;

use anyhow::Result;
use crossterm::event::{
    self, DisableBracketedPaste, EnableBracketedPaste, Event, KeyCode, KeyEventKind, KeyModifiers,
};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;
use std::io;
use tokio::sync::mpsc;
use tokio::time::Duration;

use crate::config::Config;
use crate::net::{DiscoveryEngine, ScanCommand, ScanEvent};

use app::{ActivityKind, App, InputMode, ViewMode};
use theme::Theme;

/// Run the TUI with live network scanning
pub async fn run_tui(config: Config, theme_name: &str) -> Result<()> {
    let theme = Theme::by_name(theme_name);
    let mut app = App::new(config.clone(), theme, false);

    // Set up channels
    let (event_tx, event_rx) = mpsc::unbounded_channel::<ScanEvent>();
    let (cmd_tx, cmd_rx) = mpsc::unbounded_channel::<ScanCommand>();

    // Start discovery engine
    let engine = DiscoveryEngine::new(config, event_tx, cmd_rx);
    tokio::spawn(async move {
        if let Err(e) = engine.run().await {
            eprintln!("Discovery engine error: {}", e);
        }
    });

    let result = run_event_loop(&mut app, event_rx, cmd_tx).await;

    result
}

/// Run the TUI with synthetic demo data (no network scanning)
pub async fn run_demo_tui(config: Config, theme_name: &str) -> Result<()> {
    let theme = Theme::by_name(theme_name);
    let mut app = App::new(config, theme, true);

    // Load demo devices
    let demo_devices = demo::generate_demo_devices();
    for device in demo_devices {
        app.add_or_update_device(device);
    }
    app.push_activity(
        ActivityKind::Info,
        "Demo mode - showing synthetic data".to_string(),
    );

    // No engine needed for demo mode
    let (_event_tx, event_rx) = mpsc::unbounded_channel::<ScanEvent>();
    let (cmd_tx, _cmd_rx) = mpsc::unbounded_channel::<ScanCommand>();

    run_event_loop(&mut app, event_rx, cmd_tx).await
}

/// Main event loop shared between live and demo modes
async fn run_event_loop(
    app: &mut App,
    mut event_rx: mpsc::UnboundedReceiver<ScanEvent>,
    cmd_tx: mpsc::UnboundedSender<ScanCommand>,
) -> Result<()> {
    // Terminal setup
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableBracketedPaste)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let result = run_app_loop(&mut terminal, app, &mut event_rx, &cmd_tx).await;

    // Terminal teardown
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableBracketedPaste
    )?;
    terminal.show_cursor()?;

    // Signal engine shutdown
    let _ = cmd_tx.send(ScanCommand::Shutdown);

    result
}

async fn run_app_loop(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut App,
    event_rx: &mut mpsc::UnboundedReceiver<ScanEvent>,
    cmd_tx: &mpsc::UnboundedSender<ScanCommand>,
) -> Result<()> {
    loop {
        // Draw
        terminal.draw(|f| ui::render(f, app))?;

        // Drain scan events (non-blocking)
        while let Ok(scan_event) = event_rx.try_recv() {
            app.handle_scan_event(scan_event);
        }

        // Tick spinner
        app.spinner.tick();

        // Poll timeout
        let poll_timeout = if app.spinner.is_active() {
            Duration::from_millis(80)
        } else {
            Duration::from_millis(250)
        };

        if event::poll(poll_timeout)? {
            let ev = event::read()?;

            if let Event::Key(key) = ev {
                if key.kind == KeyEventKind::Release {
                    continue;
                }

                // Global Ctrl+C
                if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
                    app.should_quit = true;
                    continue;
                }

                // Ctrl+Z suspend
                #[cfg(unix)]
                if key.code == KeyCode::Char('z') && key.modifiers.contains(KeyModifiers::CONTROL) {
                    disable_raw_mode()?;
                    execute!(
                        terminal.backend_mut(),
                        LeaveAlternateScreen,
                        DisableBracketedPaste
                    )?;
                    terminal.show_cursor()?;
                    unsafe {
                        libc::raise(libc::SIGTSTP);
                    }
                    enable_raw_mode()?;
                    execute!(
                        terminal.backend_mut(),
                        EnterAlternateScreen,
                        EnableBracketedPaste
                    )?;
                    terminal.clear()?;
                    continue;
                }

                // Mode-specific key handling
                match app.input_mode {
                    InputMode::Search => handle_search_keys(app, key.code),
                    InputMode::PortScanConfirm => {
                        handle_port_scan_confirm_keys(app, key.code, cmd_tx)
                    }
                    InputMode::Help => handle_help_keys(app, key.code),
                    InputMode::Normal => match app.view_mode {
                        ViewMode::Table => handle_table_keys(app, key.code, cmd_tx),
                        ViewMode::Detail => handle_detail_keys(app, key.code, cmd_tx),
                    },
                }
            }
        }

        if app.should_quit {
            break;
        }
    }

    Ok(())
}

// ── Key handlers ─────────────────────────────────────────────────────

fn handle_table_keys(app: &mut App, key: KeyCode, cmd_tx: &mpsc::UnboundedSender<ScanCommand>) {
    match key {
        KeyCode::Char('q') | KeyCode::Esc => {
            if !app.filter_input.is_empty() {
                app.filter_input.clear();
                app.rebuild_filtered();
            } else {
                app.should_quit = true;
            }
        }
        KeyCode::Char('j') | KeyCode::Down => app.move_selection(1),
        KeyCode::Char('k') | KeyCode::Up => app.move_selection(-1),
        KeyCode::PageDown => app.move_selection(20),
        KeyCode::PageUp => app.move_selection(-20),
        KeyCode::Char('g') => app.select_first(),
        KeyCode::Char('G') => app.select_last(),
        KeyCode::Enter => {
            if app.selected_device().is_some() {
                app.view_mode = ViewMode::Detail;
                app.detail_scroll = 0;
            }
        }
        KeyCode::Char('p') => {
            if app.selected_device().is_some() {
                app.input_mode = InputMode::PortScanConfirm;
            }
        }
        KeyCode::Char('/') => {
            app.input_mode = InputMode::Search;
        }
        KeyCode::Char('s') => {
            app.sort_column = app.sort_column.next();
            app.rebuild_filtered();
        }
        KeyCode::Char('S') => {
            app.sort_order = match app.sort_order {
                app::SortOrder::Asc => app::SortOrder::Desc,
                app::SortOrder::Desc => app::SortOrder::Asc,
            };
            app.rebuild_filtered();
        }
        KeyCode::Char('r') => {
            let _ = cmd_tx.send(ScanCommand::Rescan);
            app.push_activity(ActivityKind::Info, "Manual rescan triggered".to_string());
        }
        KeyCode::Char('y') => copy_ip(app),
        KeyCode::Char('Y') => copy_mac(app),
        KeyCode::Char('a') => {
            app.show_activity = !app.show_activity;
        }
        KeyCode::Char('?') => {
            app.input_mode = InputMode::Help;
        }
        _ => {}
    }
}

fn handle_detail_keys(app: &mut App, key: KeyCode, cmd_tx: &mpsc::UnboundedSender<ScanCommand>) {
    match key {
        KeyCode::Esc | KeyCode::Char('q') => {
            app.view_mode = ViewMode::Table;
        }
        KeyCode::Char('j') | KeyCode::Down => {
            app.detail_scroll = app.detail_scroll.saturating_add(1);
        }
        KeyCode::Char('k') | KeyCode::Up => {
            app.detail_scroll = app.detail_scroll.saturating_sub(1);
        }
        KeyCode::PageDown => {
            app.detail_scroll = app.detail_scroll.saturating_add(20);
        }
        KeyCode::PageUp => {
            app.detail_scroll = app.detail_scroll.saturating_sub(20);
        }
        KeyCode::Char('p') => {
            if app.selected_device().is_some() {
                app.input_mode = InputMode::PortScanConfirm;
            }
        }
        KeyCode::Char('y') => copy_ip(app),
        KeyCode::Char('Y') => copy_mac(app),
        KeyCode::Char('r') => {
            let _ = cmd_tx.send(ScanCommand::Rescan);
        }
        _ => {}
    }
}

fn handle_search_keys(app: &mut App, key: KeyCode) {
    match key {
        KeyCode::Esc => {
            app.input_mode = InputMode::Normal;
            app.filter_input.clear();
            app.rebuild_filtered();
        }
        KeyCode::Enter => {
            app.input_mode = InputMode::Normal;
        }
        KeyCode::Backspace => {
            app.filter_input.pop();
            app.rebuild_filtered();
        }
        KeyCode::Char(c) => {
            app.filter_input.push(c);
            app.rebuild_filtered();
        }
        _ => {}
    }
}

fn handle_port_scan_confirm_keys(
    app: &mut App,
    key: KeyCode,
    cmd_tx: &mpsc::UnboundedSender<ScanCommand>,
) {
    match key {
        KeyCode::Enter => {
            if let Some(ip) = app.selected_ip() {
                let _ = cmd_tx.send(ScanCommand::ScanPorts(ip));
                app.port_scan_target = Some(ip);
                app.push_activity(
                    ActivityKind::PortScan,
                    format!("Starting port scan on {}", ip),
                );
                app.spinner.start(&format!("Scanning ports on {}...", ip));
            }
            app.input_mode = InputMode::Normal;
        }
        KeyCode::Esc | KeyCode::Char('q') => {
            app.input_mode = InputMode::Normal;
        }
        _ => {}
    }
}

fn handle_help_keys(app: &mut App, key: KeyCode) {
    match key {
        KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('?') => {
            app.input_mode = InputMode::Normal;
        }
        _ => {}
    }
}

// ── Clipboard helpers ────────────────────────────────────────────────

fn copy_ip(app: &mut App) {
    if let Some(device) = app.selected_device() {
        let text = device.ip.to_string();
        match arboard::Clipboard::new().and_then(|mut cb| cb.set_text(&text)) {
            Ok(_) => {
                app.status_message = Some(format!("Copied IP: {}", text));
                app.push_activity(ActivityKind::Info, format!("Copied IP: {}", text));
            }
            Err(_) => {
                app.status_message = Some("Failed to copy to clipboard".to_string());
            }
        }
    }
}

fn copy_mac(app: &mut App) {
    if let Some(device) = app.selected_device() {
        if let Some(ref mac) = device.mac {
            let text = mac.clone();
            match arboard::Clipboard::new().and_then(|mut cb| cb.set_text(&text)) {
                Ok(_) => {
                    app.status_message = Some(format!("Copied MAC: {}", text));
                    app.push_activity(ActivityKind::Info, format!("Copied MAC: {}", text));
                }
                Err(_) => {
                    app.status_message = Some("Failed to copy to clipboard".to_string());
                }
            }
        } else {
            app.status_message = Some("No MAC address available".to_string());
        }
    }
}
