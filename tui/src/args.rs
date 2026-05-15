use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(name = "clawparty")]
#[command(about = "Terminal UI for ClawParty (ZeroClaw)")]
pub struct Args {
    #[command(subcommand)]
    pub command: Option<Command>,

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

    // HTTPS Proxy settings (only active in --service mode)
    #[arg(long, default_value = "443")]
    pub proxy_https_port: u16,

    #[arg(long, default_value = "80")]
    pub proxy_http_port: u16,

    #[arg(long, default_value = "~/.clawparty/certs")]
    pub proxy_cert_dir: String,
}

#[derive(Subcommand, Debug)]
pub enum Command {
    /// Manage local users in the agent database
    User {
        #[command(subcommand)]
        user_command: UserCommands,
    },
}

#[derive(Subcommand, Debug)]
pub enum UserCommands {
    /// List all users
    List,
    /// Add a new user
    Add {
        username: String,
        #[arg(long)]
        password: Option<String>,
        #[arg(long, default_value = "user")]
        role: String,
        #[arg(long)]
        expire_days: Option<u32>,
    },
    /// Delete a user
    Delete {
        username: String,
    },
    /// Change user password
    Password {
        username: String,
        #[arg(long)]
        password: Option<String>,
        #[arg(long)]
        expire_days: Option<u32>,
    },
    /// Reset user API token
    Token {
        username: String,
        #[arg(long)]
        expire_days: Option<u32>,
    },
}
