mod agent;
mod args;
mod models;
mod api;
mod app;
mod ui;

use agent::AgentManager;
use args::Args;
use api::ApiClient;
use app::{AppState, ActivePanel, ActiveOrg};
use clap::Parser;
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io::{self, IsTerminal};
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use tokio::time::{sleep, timeout, Duration};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    // Check if we have a TTY
    if !io::stdin().is_terminal() {
        eprintln!("Error: TUI requires a terminal (TTY).");
        eprintln!("This program should be run interactively, not piped.");
        return Err(anyhow::anyhow!("Not a TTY"));
    }

    // Create log channel
    let (log_tx, mut log_rx) = mpsc::channel::<String>(100);

    // Setup terminal
    enable_raw_mode()?;
    io::stdout().execute(EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(io::stdout());
    let mut terminal = Terminal::new(backend)?;

    // Determine pipy binary path
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

    // Create API client
    let api = ApiClient::new(args.api_host.clone(), args.token.clone());

    // Create app state
    let mut state = AppState::new(api);
    state.add_log("INFO", &format!("Connecting to {}", args.api_host));

    // Check if agent is already running
    let agent_already_running = {
        let api_lock = state.api.lock().await;
        api_lock.check_health().await
    };

    if agent_already_running {
        state.add_log("INFO", "Agent is already running");
        state.agent_running = true;
    } else {
        // Start the agent (output captured to log channel)
        state.add_log("INFO", &format!("Starting agent ({})...", pipy_bin));
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
                state.add_log("INFO", "Waiting for agent to start...");
            }
        }
        if ready {
            state.agent_running = true;
            state.add_log("INFO", "Agent started successfully");
        } else {
            state.add_log("ERROR", "Agent failed to start within timeout");
        }
        // Store agent manager in state for cleanup on exit
        state.agent_mgr = Some(agent_mgr);
    }

    // Fetch initial data if agent is running
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
                    state.active_org = ActiveOrg::Mesh(mesh.name.clone());
                }
            }
            Err(e) => {
                state.add_log("ERROR", &format!("Failed to fetch meshes: {}", e));
            }
        }

        // Always fetch openclaw agents (local agents)
        let agents_result = {
            let api_lock = state.api.lock().await;
            api_lock.get_openclaw_agents().await
        };
        match agents_result {
            Ok(agents) => state.openclaw_agents = agents,
            Err(e) => state.add_log("ERROR", &format!("Failed to fetch agents: {}", e)),
        }
    }

    // Fetch chats and endpoints (only if mesh is available)
    if let Some(ref mesh) = state.current_mesh {
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

    state.refresh_sections();

    // Auto-select the first item
    state.select_item(0);

    // Fetch messages for the selected item
    if state.current_chat.is_some() {
        let chat_idx = state.current_chat.unwrap();
        if let Some(chat) = state.chats.get(chat_idx).cloned() {
            if let Some(ref mesh) = state.current_mesh {
                let mesh = mesh.clone();
                if chat.is_group {
                    if let (Some(creator), Some(group)) = (&chat.creator, &chat.group) {
                        let c = creator.clone();
                        let g = group.clone();
                        if let Ok(msgs) = state.api.lock().await.get_group_messages(&mesh, &c, &g).await {
                            state.messages = msgs;
                        }
                    }
                } else if let Some(peer) = &chat.peer {
                    let p = peer.clone();
                    if let Ok(msgs) = state.api.lock().await.get_peer_messages(&mesh, &p).await {
                        state.messages = msgs;
                    }
                }
            }
        }
    } else if let Some(ref peer) = state.current_peer {
        if let Some(ref mesh) = state.current_mesh {
            let mesh = mesh.clone();
            let p = peer.clone();
            if let Ok(msgs) = state.api.lock().await.get_peer_messages(&mesh, &p).await {
                state.messages = msgs;
            }
        }
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

    // Polling task
    let poll_state = state.clone();
    tokio::spawn(async move {
        loop {
            // Fast poll when in openclaw agent chat (streaming responses)
            let poll_interval = {
                let s = poll_state.read().await;
                if s.current_openclaw_agent.is_some() { 500 } else { 2000 }
            };
            sleep(Duration::from_millis(poll_interval)).await;

            let (agent_running, current_mesh, current_chat, current_agent, current_peer) = {
                let s = poll_state.read().await;
                (
                    s.agent_running,
                    s.current_mesh.clone(),
                    s.current_chat,
                    s.current_openclaw_agent.clone(),
                    s.current_peer.clone(),
                )
            };

            if !agent_running {
                continue;
            }

            let mesh = match current_mesh {
                Some(m) => m,
                None => continue,
            };

            let api_client = {
                let s = poll_state.read().await;
                s.api.clone()
            };

            // Each API call acquires and releases the lock independently
            let chats = {
                let l = api_client.lock().await;
                l.get_chats(&mesh).await.ok()
            };

            let endpoints = {
                let l = api_client.lock().await;
                l.get_endpoints(&mesh).await.ok()
            };

            let agents = {
                let l = api_client.lock().await;
                l.get_openclaw_agents().await.ok()
            };

            let mut messages = None;
            if let Some(chat_idx) = current_chat {
                let chat_info = {
                    let s = poll_state.read().await;
                    s.chats.get(chat_idx).cloned()
                };
                if let Some(chat) = chat_info {
                    if chat.is_group {
                        if let (Some(creator), Some(group)) = (&chat.creator, &chat.group) {
                            messages = {
                                let l = api_client.lock().await;
                                l.get_group_messages(&mesh, creator, group).await.ok()
                            };
                        }
                    } else if let Some(peer) = &chat.peer {
                        messages = {
                            let l = api_client.lock().await;
                            l.get_peer_messages(&mesh, peer).await.ok()
                        };
                    }
                }
            } else if let Some(ref agent) = current_agent {
                messages = {
                    let l = api_client.lock().await;
                    l.get_openclaw_messages(&agent.id).await.ok()
                };
            } else if let Some(ref peer) = current_peer {
                messages = {
                    let l = api_client.lock().await;
                    l.get_peer_messages(&mesh, peer).await.ok()
                };
            }

            {
                let mut s = poll_state.write().await;
                if let Some(c) = chats {
                    s.chats = c;
                }
                if let Some(e) = endpoints {
                    s.endpoints = e;
                }
                if let Some(a) = agents {
                    s.openclaw_agents = a;
                }
                if let Some(m) = messages {
                    let old_len = s.messages.len();
                    s.messages = m;
                    let new_len = s.messages.len();
                    if new_len > old_len {
                        s.messages_scroll.scroll_to_bottom();
                    }
                }
                s.refresh_sections();
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

                            // Handle #join-party command
                            let trimmed = text.trim();
                            if trimmed == "#join-party" || trimmed == "#join" || trimmed.starts_with("#join-party ") || trimmed.starts_with("#join ") {
                                // Parse optional name parameter: #join name=张三
                                let mut custom_name: Option<String> = None;
                                let parts: Vec<&str> = trimmed.split_whitespace().collect();
                                for part in &parts[1..] {
                                    if let Some(stripped) = part.strip_prefix("name=") {
                                        custom_name = Some(stripped.to_string());
                                    }
                                }

                                // Get api_client before dropping s
                                let api_client_for_join = s.api.clone();
                                drop(s);
                                
                                let state_clone = state.clone();
                                tokio::spawn(async move {
                                    state_clone.write().await.add_log("INFO", "Joining clawparty...");
                                    
                                    // Build CLI command with environment variables
                                    let mut cmd = std::process::Command::new("ztm");
                                    
                                    // Set environment variables for CLI to connect to agent
                                    let client = api_client_for_join.lock().await;
                                    let api_url = client.base_url().to_string();
                                    let api_token = client.token().to_string();
                                    drop(client); // Release the lock
                                    
                                    // Extract host:port from base_url (e.g., "http://127.0.0.1:7778" -> "127.0.0.1:7778")
                                    let api_host = api_url
                                        .trim_start_matches("http://")
                                        .trim_start_matches("https://")
                                        .to_string();
                                    
                                    cmd.env("ZTM_CONFIG", &api_host);
                                    cmd.env("ZTM_API_TOKEN", &api_token);
                                    
                                    cmd.arg("join").arg("party");
                                    
                                    // Add --name parameter if provided
                                    if let Some(ref name) = custom_name {
                                        cmd.arg("--name").arg(name);
                                    }
                                    
                                    // Execute CLI command
                                    match cmd.output() {
                                        Ok(output) => {
                                            let stdout = String::from_utf8_lossy(&output.stdout);
                                            let stderr = String::from_utf8_lossy(&output.stderr);
                                            
                                            // Log all output for debugging
                                            if !stdout.is_empty() {
                                                for line in stdout.lines() {
                                                    state_clone.write().await.add_log("INFO", &format!("[CLI] {}", line));
                                                }
                                            }
                                            if !stderr.is_empty() {
                                                for line in stderr.lines() {
                                                    state_clone.write().await.add_log("INFO", &format!("[CLI-ERR] {}", line));
                                                }
                                            }
                                            
                                            if output.status.success() {
                                                let msg = if !stdout.is_empty() {
                                                    stdout.lines()
                                                        .find(|line| line.contains("Successfully joined"))
                                                        .unwrap_or("Successfully joined clawparty!")
                                                } else {
                                                    "Successfully joined clawparty!"
                                                };
                                                state_clone.write().await.add_log("INFO", &format!("Join result: {}", msg));
                                            } else {
                                                state_clone.write().await.add_log("ERROR", &format!("Join party failed: {}", output.status));
                                            }
                                        }
                                        Err(e) => {
                                            state_clone.write().await.add_log("ERROR", &format!("Failed to execute ztm command: {}", e));
                                        }
                                    }
                                });
                                continue;
                            }

                            // Handle #default command
                            if trimmed.starts_with("#default ") {
                                let agent_name = trimmed["#default ".len()..].trim();
                                if agent_name.is_empty() {
                                    s.add_log("ERROR", "Usage: #default <agent-name>");
                                } else {
                                    // Check if agent exists in local agents
                                    let agent_exists = s.local_agents.iter().any(|a| a.id == agent_name);
                                    if !agent_exists {
                                        s.add_log("ERROR", &format!("Agent '{}' not found in local agents", agent_name));
                                    } else {
                                        let api_client = s.api.clone();
                                        let agent_name_owned = agent_name.to_string();
                                        drop(s);

                                        state.write().await.add_log("INFO", &format!("Setting default auto-reply agent to '{}'...", agent_name_owned));

                                        let state_clone = state.clone();
                                        tokio::spawn(async move {
                                            let result = {
                                                let client = api_client.lock().await;
                                                client.set_default_auto_reply(&agent_name_owned).await
                                            };

                                            let mut s = state_clone.write().await;
                                            match result {
                                                Ok(()) => s.add_log("INFO", &format!("Default auto-reply agent set to '{}'", agent_name_owned)),
                                                Err(e) => s.add_log("ERROR", &format!("Failed to set default auto-reply: {}", e)),
                                            }
                                        });
                                    }
                                }
                                continue;
                            }

                            // Handle #leave command
                            if trimmed.starts_with("#leave ") {
                                let mesh_name = trimmed["#leave ".len()..].trim();
                                if mesh_name.is_empty() {
                                    s.add_log("ERROR", "Usage: #leave <mesh-name>");
                                } else {
                                    let api_client = s.api.clone();
                                    let mesh_name_owned = mesh_name.to_string();
                                    drop(s);

                                    state.write().await.add_log("INFO", &format!("Leaving mesh '{}'...", mesh_name_owned));

                                    let state_clone = state.clone();
                                    tokio::spawn(async move {
                                        let result = {
                                            let client = api_client.lock().await;
                                            client.leave_mesh(&mesh_name_owned).await
                                        };

                                        let mut s = state_clone.write().await;
                                        match result {
                                            Ok(()) => s.add_log("INFO", &format!("Left mesh '{}'", mesh_name_owned)),
                                            Err(e) => s.add_log("ERROR", &format!("Failed to leave mesh: {}", e)),
                                        }
                                    });
                                }
                                continue;
                            }

                            // Collect what we need before dropping the lock
                            let cur_chat = s.current_chat;
                            let cur_agent = s.current_openclaw_agent.as_ref().map(|a| a.id.clone());
                            let cur_peer = s.current_peer.clone();
                            let cur_mesh = s.current_mesh.clone();
                            let api_client = s.api.clone();
                            let state_clone = state.clone();
                            let text_clone = text.clone();
                            drop(s);

                            // Spawn send in a separate task to avoid blocking the main event loop
                            tokio::spawn(async move {
                                let send_fut = async {
                                    if let Some(chat_idx) = cur_chat {
                                        let s_read = state_clone.read().await;
                                        let chat = s_read.chats.get(chat_idx).cloned();
                                        drop(s_read);
                                        if let Some(chat) = chat {
                                            if let Some(ref mesh) = cur_mesh {
                                                if chat.is_group {
                                                    if let (Some(creator), Some(group)) = (&chat.creator, &chat.group) {
                                                        let c = creator.clone();
                                                        let g = group.clone();
                                                        let l = api_client.lock().await;
                                                        l.send_group_message(mesh, &c, &g, &text_clone).await
                                                    } else {
                                                        Err(anyhow::anyhow!("Invalid group chat"))
                                                    }
                                                } else if let Some(peer) = &chat.peer {
                                                    let p = peer.clone();
                                                    let l = api_client.lock().await;
                                                    l.send_peer_message(mesh, &p, &text_clone).await
                                                } else {
                                                    Err(anyhow::anyhow!("Invalid chat"))
                                                }
                                            } else {
                                                Err(anyhow::anyhow!("No mesh selected"))
                                            }
                                        } else {
                                            Err(anyhow::anyhow!("Chat not found"))
                                        }
                                    } else if let Some(ref aid) = cur_agent {
                                        let l = api_client.lock().await;
                                        l.send_openclaw_message(aid, &text_clone).await
                                    } else if let Some(ref peer) = cur_peer {
                                        if let Some(ref mesh) = cur_mesh {
                                            let l = api_client.lock().await;
                                            l.send_peer_message(mesh, peer, &text_clone).await
                                        } else {
                                            Err(anyhow::anyhow!("No mesh selected"))
                                        }
                                    } else {
                                        Err(anyhow::anyhow!("No chat or agent selected"))
                                    }
                                };

                                let send_result = timeout(Duration::from_secs(10), send_fut).await;

                                let mut s = state_clone.write().await;
                                match send_result {
                                    Ok(Ok(())) => s.add_log("INFO", "Message sent"),
                                    Ok(Err(e)) => s.add_log("ERROR", &format!("Failed: {}", e)),
                                    Err(_) => s.add_log("INFO", "Message sent (response pending)"),
                                }
                            });
                            continue;
                        }

                        // Enter in sidebar selects item
                        let idx = s.selected_index;
                        s.select_item(idx);
                        s.messages_scroll.scroll_to_bottom();

                        if let Some(chat_idx) = s.current_chat {
                            if let Some(chat) = s.chats.get(chat_idx).cloned() {
                                if let Some(ref mesh) = s.current_mesh {
                                    let mesh = mesh.clone();
                                    if chat.is_group {
                                        if let (Some(creator), Some(group)) =
                                            (&chat.creator, &chat.group)
                                        {
                                            let c = creator.clone();
                                            let g = group.clone();
                                            if let Ok(msgs) = {
                                                let l = s.api.lock().await;
                                                l.get_group_messages(&mesh, &c, &g).await
                                            } {
                                                s.messages = msgs;
                                            }
                                        }
                                    } else if let Some(peer) = &chat.peer {
                                        let p = peer.clone();
                                        if let Ok(msgs) = {
                                            let l = s.api.lock().await;
                                            l.get_peer_messages(&mesh, &p).await
                                        } {
                                            s.messages = msgs;
                                        }
                                    }
                                }
                            }
                        } else if let Some(ref peer) = s.current_peer {
                            if let Some(ref mesh) = s.current_mesh {
                                let mesh = mesh.clone();
                                let p = peer.clone();
                                if let Ok(msgs) = {
                                    let l = s.api.lock().await;
                                    l.get_peer_messages(&mesh, &p).await
                                } {
                                    s.messages = msgs;
                                }
                            }
                        }

                        if let Some(ref agent) = s.current_openclaw_agent {
                            let aid = agent.id.clone();
                            if let Ok(msgs) = {
                                let l = s.api.lock().await;
                                l.get_openclaw_messages(&aid).await
                            } {
                                s.messages = msgs;
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
                    KeyCode::Char('\x08') | KeyCode::Char('\x7f') => {
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
            eprintln!("TUI: stopping agent process");
            mgr.stop();
            drop(s);
            drop(mgr);
        }
    }

    Ok(())
}
