use std::io::{BufRead, BufReader};
use std::os::unix::process::CommandExt;
use std::process::{Child, Command, Stdio};
use tokio::sync::mpsc;

pub struct AgentManager {
    process: Option<Child>,
    pipy_bin: String,
    data_dir: String,
    listen_addr: String,
    token: String,
    log_tx: mpsc::Sender<String>,
}

impl AgentManager {
    pub fn new(
        pipy_bin: String,
        data_dir: String,
        listen_addr: String,
        token: String,
        log_tx: mpsc::Sender<String>,
    ) -> Self {
        let mut mgr = Self {
            process: None,
            pipy_bin,
            data_dir,
            listen_addr,
            token,
            log_tx,
        };
        mgr.spawn();
        mgr
    }

    fn spawn(&mut self) {
        let expanded_data = self.data_dir.replace(
            "~",
            &std::env::var("HOME").unwrap_or_else(|_| ".".to_string()),
        );

        let mut child = Command::new(&self.pipy_bin)
            .args([
                "run",
                "agent",
                "--listen",
                &self.listen_addr,
                "--data",
                &expanded_data,
                "--api-token",
                &self.token,
                "--no-auth",
            ])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .stdin(Stdio::null())
            .process_group(0)
            .spawn()
            .expect("Failed to start agent");

        // Capture stdout
        if let Some(stdout) = child.stdout.take() {
            let tx = self.log_tx.clone();
            std::thread::spawn(move || {
                let reader = BufReader::new(stdout);
                for line in reader.lines() {
                    if let Ok(line) = line {
                        let _ = tx.try_send(line);
                    }
                }
            });
        }

        // Capture stderr
        if let Some(stderr) = child.stderr.take() {
            let tx = self.log_tx.clone();
            std::thread::spawn(move || {
                let reader = BufReader::new(stderr);
                for line in reader.lines() {
                    if let Ok(line) = line {
                        let _ = tx.try_send(line);
                    }
                }
            });
        }

        self.process = Some(child);
    }

    pub fn restart(&mut self) {
        self.stop();
        self.spawn();
    }

    pub fn stop(&mut self) {
        if let Some(child) = self.process.take() {
            let pid = child.id() as i32;
            // Kill the entire process group (parent + all children)
            let _ = unsafe { libc::kill(-pid, libc::SIGKILL) };
            eprintln!("AgentManager: killed agent process group {}", pid);
        } else {
            eprintln!("AgentManager: no process to kill");
        }
    }
}

impl Drop for AgentManager {
    fn drop(&mut self) {
        self.stop();
    }
}
