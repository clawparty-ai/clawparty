use clap::Parser;

#[derive(Parser, Debug)]
#[command(name = "clawparty")]
#[command(about = "Terminal UI for ClawParty")]
pub struct Args {
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
}
