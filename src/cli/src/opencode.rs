use std::io::{BufRead, BufReader};
use std::os::unix::process::CommandExt;
use std::process::{Child, Command, Stdio};

use reqwest::Client;
use tokio::sync::mpsc;

pub struct OpenCodeDaemon {
    process: Option<Child>,
    pid: u32,
}

impl OpenCodeDaemon {
    pub fn pid(&self) -> u32 {
        self.pid
    }

    pub async fn check_health(url: &str) -> bool {
        Client::new()
            .get(format!("{}/global/health", url))
            .send()
            .await
            .map(|r| r.status().is_success())
            .unwrap_or(false)
    }

    pub async fn create_session(base_url: &str) -> anyhow::Result<String> {
        let resp = Client::new()
            .post(format!("{}/session", base_url))
            .header("Content-Type", "application/json")
            .json(&serde_json::json!({"title": "ClawParty Agent"}))
            .send()
            .await?;

        if resp.status().is_success() {
            let result: serde_json::Value = resp.json().await?;
            Ok(result["id"].as_str().unwrap_or("").to_string())
        } else {
            anyhow::bail!("Failed to create session: {}", resp.status())
        }
    }

    pub async fn get_or_create_session(base_url: &str) -> anyhow::Result<String> {
        // Try to reuse the most recent "ClawParty Agent" session
        let resp = Client::new()
            .get(format!("{}/session", base_url))
            .send()
            .await?;

        if resp.status().is_success() {
            let sessions: Vec<serde_json::Value> = resp.json().await?;
            let existing = sessions.iter().find(|s| {
                s.get("title").and_then(|t| t.as_str()) == Some("ClawParty Agent")
            });
            if let Some(session) = existing {
                if let Some(id) = session["id"].as_str() {
                    return Ok(id.to_string());
                }
            }
        }
        // No existing session found, create a new one
        Self::create_session(base_url).await
    }

    pub async fn send_message(
        base_url: &str,
        session_id: &str,
        text: &str,
    ) -> anyhow::Result<String> {
        let resp = Client::new()
            .post(format!("{}/session/{}/message", base_url, session_id))
            .header("Content-Type", "application/json")
            .json(&serde_json::json!({
                "parts": [
                    {"type": "text", "text": text}
                ],
                "agent": "build",
            }))
            .send()
            .await?;

        let body = resp.text().await?;
        Ok(body)
    }
}

impl OpenCodeDaemon {
    pub fn new(
        opencode_bin: String,
        _data_dir: &str,
        agent_name: &str,
        agent_dir: &str,
        port: u16,
        log_tx: mpsc::Sender<String>,
    ) -> Self {
        let db_path = format!("{}/opencode.db", agent_dir);

        let mut child = Command::new(&opencode_bin)
            .args([
                "serve",
                "--port",
                &port.to_string(),
            ])
            .current_dir(agent_dir)
            .env("OPENCODE_DB", &db_path)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .stdin(Stdio::null())
            .process_group(0)
            .spawn()
            .expect("Failed to start OpenCode serve");

        let pid = child.id();

        // Capture stdout
        if let Some(stdout) = child.stdout.take() {
            let tx = log_tx.clone();
            let agent = agent_name.to_string();
            std::thread::spawn(move || {
                let reader = BufReader::new(stdout);
                for line in reader.lines() {
                    if let Ok(line) = line {
                        let _ = tx.try_send(format!("[OpenCode {}] {}", agent, line));
                    }
                }
            });
        }

        // Capture stderr
        if let Some(stderr) = child.stderr.take() {
            let tx = log_tx.clone();
            let agent = agent_name.to_string();
            std::thread::spawn(move || {
                let reader = BufReader::new(stderr);
                for line in reader.lines() {
                    if let Ok(line) = line {
                        let _ = tx.try_send(format!("[OpenCode {} ERR] {}", agent, line));
                    }
                }
            });
        }

        Self {
            process: Some(child),
            pid,
        }
    }

    pub fn stop(&mut self) {
        if let Some(child) = self.process.take() {
            let pid = child.id() as i32;
            let _ = unsafe { libc::kill(-pid, libc::SIGKILL) };
            ts_eprint!("OpenCodeDaemon: killed process group {}", pid);
        }
    }
}

impl Drop for OpenCodeDaemon {
    fn drop(&mut self) {
        self.stop();
    }
}
