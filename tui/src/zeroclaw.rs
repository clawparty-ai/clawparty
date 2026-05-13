use std::io::{BufRead, BufReader};
use std::os::unix::process::CommandExt;
use std::process::{Child, Command, Stdio};
use tokio::sync::mpsc;
use reqwest::Client;

pub struct ZeroClawDaemon {
    process: Option<Child>,
}

impl ZeroClawDaemon {
    // Static method to check if ZeroClaw Gateway is healthy
    pub async fn check_health(url: &str) -> bool {
        Client::new()
            .get(format!("{}/health", url))
            .send()
            .await
            .map(|r| r.status().is_success())
            .unwrap_or(false)
    }

    // Static method to get sessions
    pub async fn get_sessions(base_url: &str) -> anyhow::Result<Vec<crate::app::ZeroClawSession>> {
        let resp = Client::new()
            .get(format!("{}/api/sessions", base_url))
            .send()
            .await?;
        
        if resp.status().is_success() {
            let result: serde_json::Value = resp.json().await?;
            let sessions: Vec<crate::app::ZeroClawSession> = result["sessions"]
                .as_array()
                .unwrap_or(&vec![])
                .iter()
                .filter_map(|s| {
                    Some(crate::app::ZeroClawSession {
                        session_id: s["session_id"].as_str()?.to_string(),
                        name: s["name"].as_str()
                            .unwrap_or(s["session_id"].as_str().unwrap_or(""))
                            .to_string(),
                        last_activity: s["last_activity"].as_str()?.to_string(),
                    })
                })
                .collect();
            Ok(sessions)
        } else {
            anyhow::bail!("Failed to get sessions: {}", resp.status())
        }
    }

    // Static method to create a new session
    pub async fn create_session(base_url: &str, name: Option<&str>) -> anyhow::Result<crate::app::ZeroClawSession> {
        let body = serde_json::json!({
            "name": name.unwrap_or("default")
        });
        let resp = Client::new()
            .post(format!("{}/api/sessions", base_url))
            .json(&body)
            .send()
            .await?;
        
        if resp.status().is_success() {
            let result: serde_json::Value = resp.json().await?;
            Ok(crate::app::ZeroClawSession {
                session_id: result["session_id"].as_str().unwrap_or("").to_string(),
                name: result["name"].as_str()
                    .unwrap_or(result["session_id"].as_str().unwrap_or(""))
                    .to_string(),
                last_activity: "".to_string(),
            })
        } else {
            anyhow::bail!("Failed to create session: {}", resp.status())
        }
    }
}

impl ZeroClawDaemon {
    pub fn new(
        zeroclaw_bin: String,
        data_dir: String,
        port: u16,
        log_tx: mpsc::Sender<String>,
    ) -> Self {
        // Expand ~ to home directory
        let expanded_data = data_dir.replace(
            "~",
            &std::env::var("HOME").unwrap_or_else(|_| ".".to_string()),
        );
        let config_dir = format!("{}/.zeroclaw", expanded_data);

        let mut child = Command::new(&zeroclaw_bin)
            .args([
                "daemon",
                "--port",
                &port.to_string(),
                "--config-dir",
                &config_dir,
            ])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .stdin(Stdio::null())
            .process_group(0)
            .spawn()
            .expect("Failed to start ZeroClaw daemon");

        // Capture stdout
        if let Some(stdout) = child.stdout.take() {
            let tx = log_tx.clone();
            std::thread::spawn(move || {
                let reader = BufReader::new(stdout);
                for line in reader.lines() {
                    if let Ok(line) = line {
                        let _ = tx.try_send(format!("[ZeroClaw] {}", line));
                    }
                }
            });
        }

        // Capture stderr
        if let Some(stderr) = child.stderr.take() {
            let tx = log_tx.clone();
            std::thread::spawn(move || {
                let reader = BufReader::new(stderr);
                for line in reader.lines() {
                    if let Ok(line) = line {
                        let _ = tx.try_send(format!("[ZeroClaw ERR] {}", line));
                    }
                }
            });
        }

        Self {
            process: Some(child),
        }
    }

    pub fn stop(&mut self) {
        if let Some(child) = self.process.take() {
            let pid = child.id() as i32;
            // Kill the entire process group (parent + all children)
            let _ = unsafe { libc::kill(-pid, libc::SIGKILL) };
            eprintln!("ZeroClawDaemon: killed daemon process group {}", pid);
        } else {
            eprintln!("ZeroClawDaemon: no process to kill");
        }
    }
}

impl Drop for ZeroClawDaemon {
    fn drop(&mut self) {
        self.stop();
    }
}
