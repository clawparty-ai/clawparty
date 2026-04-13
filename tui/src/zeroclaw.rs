use std::io::{BufRead, BufReader};
use std::os::unix::process::CommandExt;
use std::process::{Child, Command, Stdio};
use tokio::sync::mpsc;

pub struct ZeroClawDaemon {
    process: Option<Child>,
}

impl ZeroClawDaemon {
    pub fn new(
        zeroclaw_bin: String,
        data_dir: String,
        port: u16,
        log_tx: mpsc::Sender<String>,
    ) -> Self {
        let expanded_data = data_dir.replace(
            "~",
            &std::env::var("HOME").unwrap_or_else(|_| ".".to_string()),
        );

        // ZeroClaw config is in ~/.clawparty/.zeroclaw/
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
