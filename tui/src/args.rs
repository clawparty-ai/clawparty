use clap::Parser;

#[derive(Parser, Debug)]
#[command(name = "clawparty")]
#[command(about = "Terminal UI for ClawParty (ZeroClaw)")]
pub struct Args {
    // ZeroClaw Gateway settings
    #[arg(long, default_value = "http://localhost:42617")]
    pub zeroclaw_host: String,

    #[arg(long)]
    pub zeroclaw_bin: Option<String>,

    // ZTM Agent settings (optional, for mesh networking)
    #[arg(long, default_value = "http://localhost:6789")]
    pub api_host: String,

    #[arg(long, default_value = "enjoy-party")]
    pub token: String,

    #[arg(long)]
    pub pipy_bin: Option<String>,

    #[arg(long, default_value = "~/.clawparty")]
    pub data: String,

    #[arg(long, default_value = "127.0.0.1:6789")]
    pub listen: String,

    // Mode settings
    #[arg(short, long, default_value = "false")]
    pub service: bool,

    #[arg(short, long, default_value = "false")]
    pub open: bool,

    // ZeroClaw-only mode (no ZTM agent)
    #[arg(long, default_value = "false")]
    pub zeroclaw_only: bool,

    // Watchdog: interval in seconds to health-check ZTM agent and auto-restart if hung (0 = disabled)
    #[arg(long, default_value = "30")]
    pub watchdog_interval: u64,
}
