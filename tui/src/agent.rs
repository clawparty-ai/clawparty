use std::io::{BufRead, BufReader};
use std::process::{Child, Command, Stdio};
use tokio::sync::mpsc;

pub struct AgentManager {
    process: Option<Child>,
}

impl AgentManager {
    pub fn new(
        pipy_bin: String,
        data_dir: String,
        listen_addr: String,
        token: String,
        log_tx: mpsc::Sender<String>,
    ) -> Self {
        let expanded_data = data_dir.replace(
            "~",
            &std::env::var("HOME").unwrap_or_else(|_| ".".to_string()),
        );

        let mut child = Command::new(&pipy_bin)
            .args([
                "run",
                "agent",
                "--listen",
                &listen_addr,
                "--data",
                &expanded_data,
                "--api-token",
                &token,
            ])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .stdin(Stdio::null())
            .spawn()
            .expect("Failed to start agent");

        // Capture stdout
        if let Some(stdout) = child.stdout.take() {
            let tx = log_tx.clone();
            std::thread::spawn(move || {
                let reader = BufReader::new(stdout);
                for line in reader.lines() {
                    if let Ok(line) = line {
                        let _ = tx.try_send(format!("[OUT] {}", line));
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
                        let _ = tx.try_send(format!("[ERR] {}", line));
                    }
                }
            });
        }

        Self {
            process: Some(child),
        }
    }

    pub fn stop(&mut self) {
        if let Some(mut child) = self.process.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

impl Drop for AgentManager {
    fn drop(&mut self) {
        self.stop();
    }
}
