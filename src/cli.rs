use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "yscan", about = "TUI network scanner", version)]
pub struct Cli {
    /// Network interface to scan (e.g., en0)
    #[arg(short = 'i', long)]
    pub interface: Option<String>,

    /// Color theme (dark, light, dracula, nord)
    #[arg(long, default_value = "dark")]
    pub theme: String,

    #[command(subcommand)]
    pub command: Option<Command>,
}

#[derive(Subcommand)]
pub enum Command {
    /// Launch with synthetic demo data
    Demo,
    /// One-shot scan, print results to stdout
    Scan {
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
}
