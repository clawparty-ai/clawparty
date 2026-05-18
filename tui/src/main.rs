#[macro_use]
mod log_macros;
mod agent;
mod args;
mod models;
mod api;
mod app;
mod ui;
mod zeroclaw;
mod proxy;
mod static_files;
mod wiki;
mod db;
mod tasks;
mod webshare;
mod radar;
mod agents;
mod groupchats;
mod kanban;
mod global_config;

use agent::AgentManager;
use args::Args;
use api::ApiClient;
use zeroclaw::ZeroClawDaemon;
use app::{AppState, ActivePanel, ActiveOrg, AgentProcess};
use models::AgentConfig;
use clap::Parser;
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io::{self, IsTerminal};
use std::process::Command;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use tokio::time::{sleep, Duration, timeout};

fn set_all_agents_running(data_dir: &str) {
    let expanded = data_dir.replace("~", &std::env::var("HOME").unwrap_or_else(|_| ".".to_string()));
    let db_path = format!("{}/ztm.db", expanded);
    let conn = match rusqlite::Connection::open(&db_path) {
        Ok(c) => c,
        Err(e) => {
            ts_eprint!("DB open for update error: {}", e);
            return;
        }
    };
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as f64;
    match conn.execute(
        "UPDATE agents SET status = 'running', updated_at = ?1 WHERE deleted = 0 AND status != 'running'",
        rusqlite::params![now],
    ) {
        Ok(n) => {
            if n > 0 {
                ts_print!("Updated {} agent(s) status to running", n);
            }
        }
        Err(e) => ts_eprint!("DB update error: {}", e),
    }
}

fn read_agents_from_db(data_dir: &str) -> Vec<AgentConfig> {
    let _ = db::init_clawparty_db(data_dir);
    match db::list_agents(data_dir) {
        Ok(agents) => agents
            .into_iter()
            .map(|a| AgentConfig {
                agent_name: a.agent_name,
                directory: a.directory,
                port: a.port,
                status: a.status,
            })
            .collect(),
        Err(e) => {
            ts_eprint!("read_agents_from_db error: {}", e);
            vec![]
        }
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    // Handle user management subcommands
    if let Some(args::Command::User { user_command }) = args.command {
        return handle_user_command(user_command, &args.data);
    }

    if args.service {
        let (_agent_mgr, _zeroclaw_mgr) = run_service_mode(args).await?;
        return Ok(());
    }

    // Check if we have a TTY
    if !io::stdin().is_terminal() {
        ts_eprint!("Error: TUI requires a terminal (TTY).");
        ts_eprint!("This program should be run interactively, not piped.");
        return Err(anyhow::anyhow!("Not a TTY"));
    }

    // Create log channel
    let (log_tx, mut log_rx) = mpsc::channel::<String>(100);

    // Setup terminal
    enable_raw_mode()?;
    io::stdout().execute(EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(io::stdout());
    let mut terminal = Terminal::new(backend)?;

    // Create API client (for ZTM agent)
    let api = ApiClient::new(args.api_host.clone(), args.token.clone());

    // Create app state - ZeroClaw first!
    let mut state = AppState::new(api, args.zeroclaw_only);
    state.add_log("INFO", &format!("ZeroClaw mode: {}", if args.zeroclaw_only { "Standalone" } else { "With ZTM Agent" }));

    // Find zeroclaw binary path
    let zeroclaw_bin = args.zeroclaw_bin.clone().unwrap_or_else(|| {
        if let Ok(exe) = std::env::current_exe() {
            if let Some(dir) = exe.parent() {
                let zeroclaw = dir.join("zeroclaw");
                if zeroclaw.exists() {
                    return zeroclaw.to_string_lossy().to_string();
                }
            }
        }
        "zeroclaw".to_string()
    });

    // Start ZeroClaw daemon FIRST (always start this)
    state.add_log("INFO", "🦀 Starting ZeroClaw daemon...");
    let zeroclaw_mgr = zeroclaw::ZeroClawDaemon::new(
        zeroclaw_bin,
        args.data.clone(),
        42617, // ZeroClaw Gateway port
        log_tx.clone(),
    );

    // Wait for ZeroClaw to be ready (20 second timeout)
    let mut zeroclaw_ready = false;
    for i in 0..40 {
        sleep(Duration::from_millis(500)).await;
        if zeroclaw::ZeroClawDaemon::check_health("http://localhost:42617").await {
            zeroclaw_ready = true;
            break;
        }
        if i == 0 {
            state.add_log("INFO", "Waiting for ZeroClaw Gateway...");
        }
    }

    if !zeroclaw_ready {
        state.add_log("ERROR", "❌ ZeroClaw daemon failed to start within timeout");
        drop(zeroclaw_mgr);
        disable_raw_mode()?;
        io::stdout().execute(LeaveAlternateScreen)?;
        return Err(anyhow::anyhow!("ZeroClaw startup failed"));
    }

    state.zeroclaw_running = true;
    state.zeroclaw_mgr = Some(zeroclaw_mgr);
    state.add_log("INFO", "✅ ZeroClaw daemon started successfully");

    // Fetch ZeroClaw sessions (always do this early)
    let zeroclaw_sessions_result = zeroclaw::ZeroClawDaemon::get_sessions("http://localhost:42617").await;
    match zeroclaw_sessions_result {
        Ok(sessions) => {
            state.zeroclaw_sessions = sessions.clone();
            state.add_log("INFO", &format!("Loaded {} ZeroClaw sessions", sessions.len()));
            // Auto-select first session if available
            if let Some(first_session) = sessions.first() {
                state.current_zeroclaw_session = Some(first_session.clone());
                state.active_org = ActiveOrg::ZeroClaw;
            }
        }
        Err(e) => state.add_log("WARN", &format!("Failed to fetch ZeroClaw sessions: {}", e)),
    }

    // Sync agents from filesystem to clawparty.db
    crate::agents::sync_agents_from_fs(&args.data);

    // Read agents from DB and start zeroclaw daemon for each non-running agent
    let zeroclaw_bin_for_agents = args.zeroclaw_bin.clone().unwrap_or_else(|| {
        if let Ok(exe) = std::env::current_exe() {
            if let Some(dir) = exe.parent() {
                let zc = dir.join("zeroclaw");
                if zc.exists() {
                    return zc.to_string_lossy().to_string();
                }
            }
        }
        "zeroclaw".to_string()
    });
    let agent_configs = read_agents_from_db(&args.data);
    if !agent_configs.is_empty() {
        state.add_log("INFO", &format!("Found {} agents in DB, starting zeroclaw daemons...", agent_configs.len()));
    }
    for agent_cfg in &agent_configs {
        state.add_log("INFO", &format!("Starting agent {} on port {}", agent_cfg.agent_name, agent_cfg.port));
        let child = match std::process::Command::new(&zeroclaw_bin_for_agents)
            .args([
                "daemon",
                "--config-dir",
                &agent_cfg.directory,
                "-p",
                &agent_cfg.port.to_string(),
            ])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .stdin(std::process::Stdio::null())
            .spawn()
        {
            Ok(c) => c,
            Err(e) => {
                state.add_log("ERROR", &format!("Failed to spawn agent {}: {}", agent_cfg.agent_name, e));
                continue;
            }
        };
        let pid = child.id();
        state.add_log("INFO", &format!("Agent {} started (pid {})", agent_cfg.agent_name, pid));
        state.agent_processes.push(AgentProcess::new(agent_cfg.agent_name.clone(), child));
    }

    // Keep resolved pipy_bin for watchdog restart
    // ZTM Agent is optional (for mesh networking)
    if !args.zeroclaw_only {
        let pipy_bin = args.pipy_bin.clone().unwrap_or_else(|| {
            if let Ok(exe) = std::env::current_exe() {
                if let Some(dir) = exe.parent() {
                    let ztm = dir.join("ztm");
                    if ztm.exists() {
                        return ztm.to_string_lossy().to_string();
                    }
                }
            }
            "ztm".to_string()
        });

        if args.no_health_check {
            // Skip health checks, start ZTM agent unconditionally
            state.add_log("INFO", &format!("Starting ZTM agent ({}) [health check disabled]...", pipy_bin));
            let agent_mgr = AgentManager::new(
                pipy_bin.clone(),
                args.data.clone(),
                args.listen.clone(),
                args.token.clone(),
                log_tx.clone(),
            );
            state.agent_running = true;
            state.agent_mgr = Some(agent_mgr);
            set_all_agents_running(&args.data);
            state.add_log("INFO", "All agent statuses set to running in DB");
        } else {
            // Check if ZTM agent is already running
            let agent_already_running = {
                let api_lock = state.api.lock().await;
                api_lock.check_health().await
            };

            if agent_already_running {
                state.add_log("INFO", "ZTM Agent is already running");
                state.agent_running = true;
            } else {
                // Start the ZTM agent
                state.add_log("INFO", &format!("Starting ZTM agent ({})...", pipy_bin));
                let agent_mgr = AgentManager::new(
                    pipy_bin.clone(),
                    args.data.clone(),
                    args.listen.clone(),
                    args.token.clone(),
                    log_tx.clone(),
                );
                // Wait for agent to be ready
                let mut ready = false;
                for i in 0..20 {
                    sleep(Duration::from_millis(500)).await;
                    let api_lock = state.api.lock().await;
                    if api_lock.check_health().await {
                        ready = true;
                        drop(api_lock);
                        break;
                    }
                    drop(api_lock);
                    if i == 0 {
                        state.add_log("INFO", "Waiting for ZTM agent to start...");
                    }
                }
                if ready {
                    state.agent_running = true;
                    state.add_log("INFO", "ZTM Agent started successfully");
                    set_all_agents_running(&args.data);
                    state.add_log("INFO", "All agent statuses set to running in DB");
                } else {
                    state.add_log("WARN", "ZTM Agent failed to start (mesh features unavailable)");
                }
                state.agent_mgr = Some(agent_mgr);
            }
        }

        // Fetch mesh data if agent is running
        if state.agent_running {
            let meshes_result = {
                let api_lock = state.api.lock().await;
                api_lock.get_meshes().await
            };
            match meshes_result {
                Ok(meshes) => {
                    state.meshes = meshes;
                    if let Some(mesh) = state.meshes.first() {
                        state.current_mesh = Some(mesh.name.clone());
                    }
                }
                Err(e) => {
                    state.add_log("ERROR", &format!("Failed to fetch meshes: {}", e));
                }
            }

            // Fetch openclaw agents (local agents)
            let agents_result = {
                let api_lock = state.api.lock().await;
                api_lock.get_openclaw_agents().await
            };
            match agents_result {
                Ok(agents) => state.openclaw_agents = agents,
                Err(e) => state.add_log("ERROR", &format!("Failed to fetch agents: {}", e)),
            }
        }
    } else {
        state.add_log("INFO", "ZTM Agent disabled (--zeroclaw-only mode)");
    }

    // Fetch chats and endpoints if mesh is available
    if let Some(ref mesh) = state.current_mesh {
        if state.agent_running {
            let mesh = mesh.clone();
            let chats_result = {
                let api_lock = state.api.lock().await;
                api_lock.get_chats(&mesh).await
            };
            match chats_result {
                Ok(chats) => state.chats = chats,
                Err(e) => state.add_log("ERROR", &format!("Failed to fetch chats: {}", e)),
            }

            let endpoints_result = {
                let api_lock = state.api.lock().await;
                api_lock.get_endpoints(&mesh).await
            };
            match endpoints_result {
                Ok(endpoints) => state.endpoints = endpoints,
                Err(e) => state.add_log("ERROR", &format!("Failed to fetch endpoints: {}", e)),
            }
        }
    }

    state.refresh_sections();

    // Auto-select the first item
    state.select_item(0);

    // Fetch messages for the selected item
    if state.current_zeroclaw_session.is_some() {
        let sid = state.current_zeroclaw_session.as_ref().unwrap().session_id.clone();
        let msgs = state.api.lock().await.get_zeroclaw_messages(&sid).await.ok();
        if let Some(m) = msgs {
            state.messages = m;
            state.trim_messages();
        }
    } else if state.current_chat.is_some() {
        // ... (similar to original code)
    }

    let state = Arc::new(RwLock::new(state));

    // Log receiver task
    let log_state = state.clone();
    tokio::spawn(async move {
        while let Some(line) = log_rx.recv().await {
            let mut s = log_state.write().await;
            s.add_log("AGENT", &line);
            s.logs_scroll.scroll_to_bottom();
        }
    });

    // OpenClaw message loader task: auto-load messages when switching agents
    let loader_state = state.clone();
    tokio::spawn(async move {
        let mut last_agent_id: Option<String> = None;
        loop {
            sleep(Duration::from_millis(200)).await;

            let agent_id = {
                let s = loader_state.read().await;
                if let Some(ref agent) = s.current_openclaw_agent {
                    if last_agent_id.as_ref() != Some(&agent.id) {
                        Some(agent.id.clone())
                    } else {
                        None
                    }
                } else {
                    None
                }
            };

            if let Some(agent_id) = agent_id {
                let api = {
                    let s = loader_state.read().await;
                    s.api.clone()
                };

                let msgs = {
                    let client = api.lock().await;
                    client.get_openclaw_messages(&agent_id).await
                };

                let mut s = loader_state.write().await;
                if let Some(ref agent) = s.current_openclaw_agent {
                    if agent.id == agent_id {
                        match msgs {
                            Ok(messages) => {
                                s.messages = messages;
                                s.trim_messages();
                                s.messages_scroll.scroll_to_bottom();
                            }
                            Err(e) => {
                                s.add_log("ERROR", &format!("Failed to load OpenClaw messages for {}: {}", agent_id, e));
                            }
                        }
                    }
                }
                last_agent_id = Some(agent_id);
            }
        }
    });

    // Watchdog task: health-check ZTM agent and auto-restart if hung
    let poll_state = state.clone();
    let watchdog_interval = args.watchdog_interval;
    let no_health_check = args.no_health_check;
    tokio::spawn(async move {
        loop {
            if watchdog_interval == 0 || no_health_check {
                sleep(Duration::from_secs(60)).await;
                continue;
            }

            sleep(Duration::from_secs(watchdog_interval)).await;

            let should_check = {
                let s = poll_state.read().await;
                s.agent_running
            };

            if !should_check {
                continue;
            }

            let healthy = {
                let s = poll_state.read().await;
                let api = s.api.clone();
                drop(s);
                let api_guard = api.lock().await;
                api_guard.check_health().await
            };

            if !healthy {
                let mut s = poll_state.write().await;
                s.add_log("WARN", "ZTM Agent health check failed, restarting...");

                if let Some(ref mut mgr) = s.agent_mgr {
                    mgr.restart();
                }
                drop(s);

                // Wait for restart to complete
                let api = {
                    let s = poll_state.read().await;
                    s.api.clone()
                };
                let mut ready = false;
                for _i in 0..20 {
                    sleep(Duration::from_millis(500)).await;
                    let api_guard = api.lock().await;
                    if api_guard.check_health().await {
                        ready = true;
                        break;
                    }
                }

                let mut s = poll_state.write().await;
                if ready {
                    s.agent_running = true;
                    s.add_log("INFO", "ZTM Agent restarted successfully");
                } else {
                    s.agent_running = false;
                    s.add_log("ERROR", "ZTM Agent restart failed");
                }
            }
        }
    });

    // Main event loop
    loop {
        let state_clone = state.clone();
        terminal.draw(move |frame| {
            if let Ok(mut s) = state_clone.try_write() {
                ui::render(frame, &mut s);
            }
        })?;

        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                if key.kind != KeyEventKind::Press {
                    continue;
                }

                let mut s = state.write().await;

                match key.code {
                    KeyCode::Char('q') => {
                        if s.active_panel != ActivePanel::Input {
                            break;
                        }
                        s.input_text.push('q');
                    }
                    KeyCode::Up => {
                        match s.active_panel {
                            ActivePanel::Sidebar => {
                                let items_len = s.get_sidebar_items().len();
                                if s.selected_index > 0 {
                                    s.selected_index -= 1;
                                    let idx = s.selected_index;
                                    s.select_item(idx);
                                }
                            }
                            ActivePanel::Messages => {
                                s.messages_scroll.scroll_up();
                            }
                            ActivePanel::Input => {}
                            ActivePanel::Logs => {
                                s.logs_scroll.scroll_up();
                            }
                        }
                    }
                    KeyCode::Down => {
                        match s.active_panel {
                            ActivePanel::Sidebar => {
                                let items_len = s.get_sidebar_items().len();
                                if s.selected_index + 1 < items_len {
                                    s.selected_index += 1;
                                    let idx = s.selected_index;
                                    s.select_item(idx);
                                }
                            }
                            ActivePanel::Messages => {
                                s.messages_scroll.scroll_down();
                            }
                            ActivePanel::Input => {}
                            ActivePanel::Logs => {
                                s.logs_scroll.scroll_down();
                            }
                        }
                    }
                    KeyCode::Enter => {
                        // Handle Enter in different panels
                        if s.active_panel == ActivePanel::Input && !s.input_text.is_empty() {
                            let text = s.input_text.clone();
                            s.input_text.clear();

                            // Debug: log what was entered
                            s.add_log("DEBUG", &format!("Input received: '{}'", text));

                            // Handle #exit command
                            if text.trim() == "#exit" {
                                s.add_log("INFO", "Exiting...");
                                // Stop the agent before exiting
                                if let Some(mut mgr) = s.agent_mgr.take() {
                                    s.add_log("INFO", "Stopping agent process...");
                                    mgr.stop();
                                    drop(s);
                                    drop(mgr);
                                } else {
                                    s.add_log("INFO", "No agent process to stop (agent was already running)");
                                    drop(s);
                                }
                                disable_raw_mode()?;
                                io::stdout().execute(LeaveAlternateScreen)?;
                                return Ok(());
                            }

                            // Collect what we need before dropping the lock
                            let cur_zeroclaw = s.current_zeroclaw_session.as_ref().map(|z| z.session_id.clone());
                            let cur_openclaw = s.current_openclaw_agent.as_ref().map(|a| a.id.clone());
                            let api_client = s.api.clone();
                            let state_clone = state.clone();
                            let text_clone = text.clone();
                            drop(s);

                            // ZeroClaw: independent background task
                            if let Some(ref zc_session_id) = cur_zeroclaw {
                                let sid = zc_session_id.clone();
                                let api_cl = api_client.clone();
                                let st_cl = state_clone.clone();
                                let txt_cl = text_clone.clone();

                                tokio::spawn(async move {
                                    let result = timeout(Duration::from_secs(60), async {
                                        let l = api_cl.lock().await;
                                        l.send_zeroclaw_message(&sid, &txt_cl).await
                                    }).await;
                                    
                                    match result {
                                        Ok(Ok(response)) => {
                                            let msgs = {
                                                let l = api_cl.lock().await;
                                                l.get_zeroclaw_messages(&sid).await
                                            };
                                            let mut s = st_cl.write().await;
                                            if let Ok(msgs) = msgs {
                                                s.messages = msgs;
                                                s.trim_messages();
                                            }
                                            s.add_log("ZERO", &response);
                                            s.add_log("INFO", "ZeroClaw response received");
                                        }
                                        Ok(Err(e)) => {
                                            let mut s = st_cl.write().await;
                                            s.add_log("ERROR", &format!("Failed: {}", e));
                                        }
                                        Err(_) => {
                                            let mut s = st_cl.write().await;
                                            s.add_log("ERROR", "ZeroClaw response timeout (60 seconds)");
                                        }
                                    }
                                });
                            } else if let Some(ref agent_id) = cur_openclaw {
                                let agent_id = agent_id.clone();
                                let api_cl = api_client.clone();
                                let st_cl = state_clone.clone();
                                let txt_cl = text_clone.clone();

                                tokio::spawn(async move {
                                    let result = {
                                        let client = api_cl.lock().await;
                                        client.send_openclaw_message(&agent_id, &txt_cl).await
                                    };

                                    match result {
                                        Ok(()) => {
                                            let msgs = {
                                                let client = api_cl.lock().await;
                                                client.get_openclaw_messages(&agent_id).await
                                            };

                                            let mut s = st_cl.write().await;
                                            match msgs {
                                                Ok(messages) => {
                                                    s.messages = messages;
                                                    s.trim_messages();
                                                    s.messages_scroll.scroll_to_bottom();
                                                }
                                                Err(e) => {
                                                    s.add_log("ERROR", &format!("Failed to reload messages: {}", e));
                                                }
                                            }
                                            s.add_log("INFO", "OpenClaw message sent");
                                        }
                                        Err(e) => {
                                            let mut s = st_cl.write().await;
                                            s.add_log("ERROR", &format!("Failed to send OpenClaw message: {}", e));
                                        }
                                    }
                                });
                            }
                        } else {
                            // Enter in sidebar selects item or creates new ZeroClaw session
                            let idx = s.selected_index;
                            let items = s.get_sidebar_items();
                            if idx < items.len() {
                                let item = &items[idx];
                                if item.section == "zeroclaw_new" {
                                    // Create new ZeroClaw session
                                    let api_client = s.api.clone();
                                    let state_clone = state.clone();
                                    tokio::spawn(async move {
                                        let result = {
                                            let client = api_client.lock().await;
                                            client.create_zeroclaw_session(None).await
                                        };

                                        match result {
                                            Ok(session) => {
                                                let mut s = state_clone.write().await;
                                                s.zeroclaw_sessions.push(session.clone());
                                                s.current_zeroclaw_session = Some(session);
                                                s.active_org = ActiveOrg::ZeroClaw;
                                                s.add_log("INFO", "New ZeroClaw session created");

                                                // Load messages for the new session
                                                let sid = s.current_zeroclaw_session.as_ref().unwrap().session_id.clone();
                                                drop(s);
                                                let msgs = {
                                                    let client = api_client.lock().await;
                                                    client.get_zeroclaw_messages(&sid).await
                                                };
                                                if let Ok(msgs) = msgs {
                                                    let mut s = state_clone.write().await;
                                                    s.messages = msgs;
                                                    s.trim_messages();
                                                }
                                            }
                                            Err(e) => {
                                                state_clone.write().await.add_log("ERROR", &format!("Failed to create session: {}", e));
                                            }
                                        }
                                    });
                                } else {
                                    s.select_item(idx);
                                    s.messages_scroll.scroll_to_bottom();
                                }
                            }
                        }
                    }
                    KeyCode::Tab => {
                        s.active_panel = match s.active_panel {
                            ActivePanel::Sidebar => ActivePanel::Messages,
                            ActivePanel::Messages => ActivePanel::Input,
                            ActivePanel::Input => ActivePanel::Logs,
                            ActivePanel::Logs => ActivePanel::Sidebar,
                        };
                    }
                    KeyCode::Backspace | KeyCode::Delete => {
                        if s.active_panel == ActivePanel::Input {
                            s.input_text.pop();
                        }
                    }
                    KeyCode::Char(c) => {
                        if s.active_panel == ActivePanel::Input {
                            s.input_text.push(c);
                        }
                    }
                    KeyCode::Left => {
                        if s.active_panel != ActivePanel::Input {
                            s.active_panel = ActivePanel::Sidebar;
                        }
                    }
                    KeyCode::Right => {
                        if s.active_panel != ActivePanel::Input {
                            s.active_panel = ActivePanel::Input;
                        }
                    }
                    KeyCode::PageUp => {
                        s.messages_scroll.scroll_page_up();
                    }
                    KeyCode::PageDown => {
                        s.messages_scroll.scroll_page_down();
                    }
                    _ => {}
                }
            }
        }
    }

    disable_raw_mode()?;
    io::stdout().execute(LeaveAlternateScreen)?;

    // Stop the agent before exiting
    if let Ok(mut s) = state.try_write() {
        if let Some(mut mgr) = s.agent_mgr.take() {
            ts_eprint!("TUI: stopping agent process");
            mgr.stop();
            drop(s);
            drop(mgr);
        }
    }

    Ok(())
}

async fn run_service_mode(args: Args) -> anyhow::Result<(Option<AgentManager>, ZeroClawDaemon)> {
    let _ = env_logger::Builder::from_env("RUST_LOG")
        .format(|buf, record| {
            use std::io::Write;
            writeln!(buf, "{} [{}] {}", chrono::Local::now().format("%Y-%m-%d %H:%M:%S%.3f"), record.level(), record.args())
        })
        .try_init();
    ts_print!("🀄 ClawParty Service Mode");
    ts_print!("========================");
    ts_print!("ZeroClaw mode: {}", if args.zeroclaw_only { "Standalone" } else { "With ZTM Agent" });

    let zeroclaw_bin = args.zeroclaw_bin.clone().unwrap_or_else(|| {
        if let Ok(exe) = std::env::current_exe() {
            if let Some(dir) = exe.parent() {
                let zeroclaw = dir.join("zeroclaw");
                if zeroclaw.exists() {
                    return zeroclaw.to_string_lossy().to_string();
                }
            }
        }
        "zeroclaw".to_string()
    });
    ts_print!("🦀 ZeroClaw binary: {}", zeroclaw_bin);

    let (log_tx, mut log_rx) = mpsc::channel::<String>(100);

    ts_print!("\n🔄 Starting ZeroClaw daemon...");
    let zeroclaw_bin_for_service = zeroclaw_bin.clone();
    let zeroclaw_mgr = zeroclaw::ZeroClawDaemon::new(
        zeroclaw_bin_for_service,
        args.data.clone(),
        42617,
        log_tx.clone(),
    );

    let client = reqwest::Client::new();
    let mut zeroclaw_ready = false;
    for i in 0..40 {
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
        if let Ok(resp) = client.get("http://localhost:42617/health").send().await {
            if resp.status().is_success() {
                zeroclaw_ready = true;
                break;
            }
        }
        if i == 0 {
            ts_eprint!("Waiting for ZeroClaw Gateway...");
        }
    }

    if !zeroclaw_ready {
        ts_eprint!("❌ ZeroClaw daemon failed to start within timeout");
        return Err(anyhow::anyhow!("ZeroClaw startup failed"));
    }
    ts_print!("✅ ZeroClaw daemon started successfully on port 42617");

    // Sync agents from filesystem to clawparty.db
    crate::agents::sync_agents_from_fs(&args.data);

    // Start all ZeroClaw agents from DB before ZTM
    let agent_configs = read_agents_from_db(&args.data);
    if !agent_configs.is_empty() {
        ts_print!("📋 Found {} agent(s) in DB, starting zeroclaw daemons...", agent_configs.len());
    }
    for agent_cfg in &agent_configs {
        ts_print!("🔄 Starting agent {} on port {}...", agent_cfg.agent_name, agent_cfg.port);
        match std::process::Command::new(&zeroclaw_bin)
            .args([
                "daemon",
                "--config-dir",
                &agent_cfg.directory,
                "-p",
                &agent_cfg.port.to_string(),
            ])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .stdin(std::process::Stdio::null())
            .spawn()
        {
            Ok(child) => {
                ts_print!("✅ Agent {} started (pid {})", agent_cfg.agent_name, child.id());
            }
            Err(e) => {
                ts_eprint!("❌ Failed to start agent {}: {}", agent_cfg.agent_name, e);
            }
        }
    }

    let api = ApiClient::new(args.api_host.clone(), args.token.clone());
    let agent_mgr_arc = Arc::new(tokio::sync::Mutex::new(None::<AgentManager>));

    if !args.zeroclaw_only {
        let pipy_bin = args.pipy_bin.clone().unwrap_or_else(|| {
            if let Ok(exe) = std::env::current_exe() {
                if let Some(dir) = exe.parent() {
                    let ztm = dir.join("ztm");
                    if ztm.exists() {
                        return ztm.to_string_lossy().to_string();
                    }
                }
            }
            "ztm".to_string()
        });
        ts_print!("📦 ZTM binary: {}", pipy_bin);

        if args.no_health_check {
            ts_print!("\n🔄 Starting ZTM agent ({}) [health check disabled]...", pipy_bin);
            let mgr = AgentManager::new(
                pipy_bin.clone(),
                args.data.clone(),
                args.listen.clone(),
                args.token.clone(),
                log_tx.clone(),
            );
            let mut guard = agent_mgr_arc.lock().await;
            *guard = Some(mgr);
            drop(guard);
            set_all_agents_running(&args.data);
        } else {
            if api.check_health().await {
                ts_print!("✅ ZTM Agent is already running at {}", args.api_host);
            } else {
                ts_print!("\n🔄 Starting ZTM agent ({})...", pipy_bin);
                let mgr = AgentManager::new(
                    pipy_bin.clone(),
                    args.data.clone(),
                    args.listen.clone(),
                    args.token.clone(),
                    log_tx.clone(),
                );

                let mut ready = false;
                for i in 0..20 {
                    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
                    if api.check_health().await {
                        ready = true;
                        break;
                    }
                    if i == 0 {
                        ts_eprint!("Waiting for ZTM agent to start...");
                    }
                }

                if ready {
                    ts_print!("✅ ZTM Agent started successfully");
                    set_all_agents_running(&args.data);
                } else {
                    ts_print!("⚠️ ZTM Agent failed to start (mesh features unavailable)");
                }

                let mut guard = agent_mgr_arc.lock().await;
                *guard = Some(mgr);
                drop(guard);
            }
        }
    } else {
        ts_print!("ZTM Agent disabled (--zeroclaw-only mode)");
    }

    ts_print!("\n📋 Service Mode Ready");
    ts_print!("========================");
    ts_print!("ZeroClaw Gateway: http://localhost:42617");
    if !args.zeroclaw_only {
        ts_print!("ZTM Agent API: {}", args.api_host);
    }
    ts_print!("\nPress Ctrl+C to stop...");

    // Service-mode watchdog
    if !args.zeroclaw_only && args.watchdog_interval > 0 && !args.no_health_check {
        let api_watch = api.clone();
        let agent_mgr_watch = agent_mgr_arc.clone();
        let watchdog_interval = args.watchdog_interval;
        let pipy_bin_watch = args.pipy_bin.clone().unwrap_or_else(|| {
            if let Ok(exe) = std::env::current_exe() {
                if let Some(dir) = exe.parent() {
                    let ztm = dir.join("ztm");
                    if ztm.exists() {
                        return ztm.to_string_lossy().to_string();
                    }
                }
            }
            "ztm".to_string()
        });
        let data_watch = args.data.clone();
        let listen_watch = args.listen.clone();
        let token_watch = args.token.clone();
        let log_tx_watch = log_tx.clone();

        tokio::spawn(async move {
            loop {
                tokio::time::sleep(tokio::time::Duration::from_secs(watchdog_interval)).await;

                if api_watch.check_health().await {
                    continue;
                }

                ts_eprint!("Watchdog: ZTM Agent health check failed, restarting...");

                let mut guard = agent_mgr_watch.lock().await;
                if let Some(ref mut mgr) = *guard {
                    mgr.restart();
                } else {
                    *guard = Some(AgentManager::new(
                        pipy_bin_watch.clone(),
                        data_watch.clone(),
                        listen_watch.clone(),
                        token_watch.clone(),
                        log_tx_watch.clone(),
                    ));
                }
                drop(guard);

                let mut ready = false;
                for _i in 0..20 {
                    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
                    if api_watch.check_health().await {
                        ready = true;
                        ts_eprint!("Watchdog: ZTM Agent restarted successfully");
                        break;
                    }
                }

                if !ready {
                    ts_eprint!("Watchdog: ZTM Agent restart failed");
                }
            }
        });
    }

    // Open browser if requested
    if args.open {
        ts_print!("🌐 Opening browser to http://{}", if args.zeroclaw_only { "localhost:42617" } else { &args.api_host });
        #[cfg(target_os = "macos")]
        {
            let _ = Command::new("open").arg(if args.zeroclaw_only { "http://localhost:42617" } else { &args.api_host }).spawn();
        }
        #[cfg(target_os = "linux")]
        {
            let _ = Command::new("xdg-open").arg(if args.zeroclaw_only { "http://localhost:42617" } else { &args.api_host }).spawn();
        }
        #[cfg(target_os = "windows")]
        {
            let _ = Command::new("start").arg(if args.zeroclaw_only { "http://localhost:42617" } else { &args.api_host }).spawn();
        }
    }

    // Start HTTPS proxy in service mode
    let proxy_https_port = args.proxy_https_port;
    let proxy_http_port = args.proxy_http_port;
    let proxy_cert_dir = args.proxy_cert_dir.clone();
    tokio::spawn(async move {
        proxy::start(proxy_https_port, proxy_http_port, &proxy_cert_dir, &args.data).await;
    });

    use tokio::signal::unix::{signal, SignalKind};
    let mut sigint = signal(SignalKind::interrupt())?;
    let mut sigterm = signal(SignalKind::terminate())?;

    loop {
        tokio::select! {
            Some(log_msg) = log_rx.recv() => {
                ts_print!("{}", log_msg);
            }
            _ = sigint.recv() => {
                ts_print!("\nReceived SIGINT, shutting down...");
                let _ = Command::new("pkill").args(["-9", "-f", "zeroclaw"]).spawn();
                break;
            }
            _ = sigterm.recv() => {
                ts_print!("\nReceived SIGTERM, shutting down...");
                let _ = Command::new("pkill").args(["-9", "-f", "zeroclaw"]).spawn();
                break;
            }
        }
    }

    let agent_mgr = agent_mgr_arc.lock().await.take();
    Ok((agent_mgr, zeroclaw_mgr))
}

fn expand_data_dir(data_dir: &str) -> String {
    data_dir.replace("~", &std::env::var("HOME").unwrap_or_else(|_| ".".to_string()))
}

fn generate_random_string(len: usize) -> String {
    use rand::Rng;
    let chars: Vec<char> = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789".chars().collect();
    let mut rng = rand::rng();
    (0..len).map(|_| chars[rng.random_range(0..chars.len())]).collect()
}

fn hash_password(_conn: &rusqlite::Connection, password: &str) -> (String, String, String) {
    use sha2::{Sha256, Digest};
    let salt = generate_random_string(16);
    let mut hasher = Sha256::new();
    hasher.update(format!("{}{}", salt, password).as_bytes());
    let hash = hasher.finalize().iter().map(|b| format!("{:02x}", b)).collect();
    let token = generate_random_string(32);
    (hash, salt, token)
}

fn default_expire_days(days: Option<u32>) -> f64 {
    match days {
        Some(d) => {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs() as f64;
            now + (d as f64 * 86400.0)
        }
        None => 0.0, // 0 = never expire
    }
}

fn format_expire(expire: f64) -> String {
    if expire <= 0.0 {
        "never".to_string()
    } else {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as f64;
        let days_left = ((expire - now) / 86400.0).round();
        if days_left < 0.0 {
            format!("expired ({} days ago)", days_left.abs())
        } else {
            format!("{} days left", days_left)
        }
    }
}

fn handle_user_command(cmd: args::UserCommands, data_dir: &str) -> anyhow::Result<()> {
    use args::UserCommands;
    use std::io::{self, Write};

    let expanded = expand_data_dir(data_dir);
    let db_path = format!("{}/ztm.db", expanded);
    let conn = rusqlite::Connection::open(&db_path)
        .map_err(|e| anyhow::anyhow!("Failed to open database at {}: {}", db_path, e))?;

    match cmd {
        UserCommands::List => {
            let mut stmt = conn.prepare("SELECT username, role, created_at, expire FROM users ORDER BY username")
                .map_err(|e| anyhow::anyhow!("Failed to prepare query: {}", e))?;
            let rows = stmt.query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, f64>(2)?,
                    row.get::<_, f64>(3)?,
                ))
            }).map_err(|e| anyhow::anyhow!("Failed to query users: {}", e))?;

            ts_print!("{:<20} {:<10} {:<20} {:<20}", "USERNAME", "ROLE", "CREATED", "EXPIRE");
            ts_print!("{}", "-".repeat(70));
            for row in rows {
                let (username, role, created, expire) = row?;
                let created_str = chrono::DateTime::from_timestamp(created as i64, 0)
                    .map(|dt| dt.format("%Y-%m-%d %H:%M").to_string())
                    .unwrap_or_else(|| created.to_string());
                ts_print!("{:<20} {:<10} {:<20} {:<20}", username, role, created_str, format_expire(expire));
            }
        }
        UserCommands::Add { username, password, role, expire_days } => {
            let password = match password {
                Some(p) => p,
                None => {
                    print!("Password: ");
                    io::stdout().flush()?;
                    let mut input = String::new();
                    io::stdin().read_line(&mut input)?;
                    input.trim().to_string()
                }
            };
            if password.is_empty() {
                anyhow::bail!("Password cannot be empty");
            }
            let expire = default_expire_days(expire_days);
            let (hash, salt, token) = hash_password(&conn, &password);
            conn.execute(
                "INSERT INTO users (username, password_hash, salt, api_token, role, expire) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                [&username, &hash, &salt, &token, &role, &expire.to_string()],
            ).map_err(|e| anyhow::anyhow!("Failed to add user '{}': {}", username, e))?;
            ts_print!("User '{}' added. Role: {}, Expire: {}", username, role, format_expire(expire));
            ts_print!("API token: {}", token);
        }
        UserCommands::Delete { username } => {
            let changes = conn.execute("DELETE FROM users WHERE username = ?1", [&username])
                .map_err(|e| anyhow::anyhow!("Failed to delete user '{}': {}", username, e))?;
            if changes == 0 {
                anyhow::bail!("User '{}' not found", username);
            }
            ts_print!("User '{}' deleted.", username);
        }
        UserCommands::Password { username, password, expire_days } => {
            let password = match password {
                Some(p) => p,
                None => {
                    print!("New password: ");
                    io::stdout().flush()?;
                    let mut input = String::new();
                    io::stdin().read_line(&mut input)?;
                    input.trim().to_string()
                }
            };
            if password.is_empty() {
                anyhow::bail!("Password cannot be empty");
            }
            let expire = default_expire_days(expire_days);
            let (hash, salt, token) = hash_password(&conn, &password);
            let changes = conn.execute(
                "UPDATE users SET password_hash = ?1, salt = ?2, api_token = ?3, expire = ?4 WHERE username = ?5",
                [&hash, &salt, &token, &expire.to_string(), &username],
            ).map_err(|e| anyhow::anyhow!("Failed to update password for '{}': {}", username, e))?;
            if changes == 0 {
                anyhow::bail!("User '{}' not found", username);
            }
            ts_print!("Password changed for '{}'.", username);
            ts_print!("New API token: {}", token);
            ts_print!("Expire: {}", format_expire(expire));
        }
        UserCommands::Token { username, expire_days } => {
            let new_token = generate_random_string(32);
            let expire = default_expire_days(expire_days);
            let changes = conn.execute(
                "UPDATE users SET api_token = ?1, expire = ?2 WHERE username = ?3",
                [&new_token, &expire.to_string(), &username],
            ).map_err(|e| anyhow::anyhow!("Failed to reset token for '{}': {}", username, e))?;
            if changes == 0 {
                anyhow::bail!("User '{}' not found", username);
            }
            ts_print!("Token reset for '{}'.", username);
            ts_print!("New API token: {}", new_token);
            ts_print!("Expire: {}", format_expire(expire));
        }
    }

    Ok(())
}
