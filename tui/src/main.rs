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
        // Keep agent_mgr alive
        std::mem::forget(agent_mgr);
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
                    // Only auto-scroll if user hasn't scrolled up, or new messages arrived
                    if !s.user_scrolled_up || new_len > old_len {
                        s.user_scrolled_up = false;
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
            if let Ok(s) = state_clone.try_read() {
                ui::render(frame, &s);
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
                                }
                            }
                            ActivePanel::Messages => {
                                // Scrolling UP means increasing offset from bottom (going back in history)
                                s.message_scroll = s.message_scroll.saturating_add(3);
                                s.user_scrolled_up = true;
                            }
                            ActivePanel::Input => {}
                        }
                    }
                    KeyCode::Down => {
                        match s.active_panel {
                            ActivePanel::Sidebar => {
                                let items = s.get_sidebar_items();
                                if s.selected_index < items.len().saturating_sub(1) {
                                    s.selected_index += 1;
                                }
                            }
                            ActivePanel::Messages => {
                                // Scrolling DOWN means decreasing offset from bottom (going forward)
                                s.message_scroll = s.message_scroll.saturating_sub(3);
                                // If back at bottom, reset flag
                                if s.message_scroll == 0 {
                                    s.user_scrolled_up = false;
                                }
                            }
                            ActivePanel::Input => {}
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
                                drop(s);
                                disable_raw_mode()?;
                                io::stdout().execute(LeaveAlternateScreen)?;
                                return Ok(());
                            }

                            // Handle #join-party command
                            let trimmed = text.trim();
                            if trimmed == "#join-party" || trimmed == "#join" {
                                let api_client = s.api.clone();
                                drop(s);
                                state.write().await.add_log("INFO", "Joining clawparty...");
                                
                                let join_fut = async {
                                    // Check if already joined
                                    let meshes = {
                                        let client = api_client.lock().await;
                                        client.get_meshes().await?
                                    };
                                    if !meshes.is_empty() {
                                        return Ok::<String, anyhow::Error>("Already joined clawparty!".to_string());
                                    }
                                    
                                    // Generate username
                                    let names = ["red-hawk", "thunder-cloud", "morning-star", "running-deer", "little-wolf",
                                        "william-wallace", "princess-isabella", "sacagawea", "pocahontas", "crazy-moon",
                                        "red-cloud", "chief-joseph", "white-buffalo", "morning-star", "sitting-bull"];
                                    let mut rng = rand::rng();
                                    let name_idx = rand::Rng::random_range(&mut rng, 0..names.len());
                                    let username = names[name_idx];
                                    let ep_name = format!("{}-lobster", username);
                                    
                                    // Get agent identity
                                    let identity = {
                                        let client = api_client.lock().await;
                                        client.get_identity().await?
                                    };
                                    
                                    // Post to registration server
                                    let reg_url = "https://join.clawparty.ai/invite";
                                    let mut pass_key = String::new();
                                    let chars = b"abcdefghijklmnopqrstuvwxyz";
                                    for _ in 0..16 {
                                        let idx = rand::Rng::random_range(&mut rng, 0..chars.len());
                                        pass_key.push(chars[idx] as char);
                                    }
                                    
                                    let invite_body = serde_json::json!({
                                        "PublicKey": identity,
                                        "UserName": username,
                                        "EpName": ep_name,
                                        "PassKey": pass_key
                                    });
                                    
                                    let reg_client = reqwest::Client::builder()
                                        .danger_accept_invalid_certs(true)
                                        .build()?;
                                    let resp = reg_client
                                        .post(reg_url)
                                        .json(&invite_body)
                                        .send()
                                        .await?;
                                    
                                    if !resp.status().is_success() {
                                        let err_text = resp.text().await.unwrap_or_default();
                                        anyhow::bail!("Registration failed: {}", err_text);
                                    }
                                    
                                    let result: serde_json::Value = resp.json().await?;
                                    let final_username = result["UserName"].as_str().unwrap_or(username);
                                    let final_ep_name = result["EpName"].as_str().unwrap_or(&ep_name);
                                    let permit = result["Permit"].as_str().unwrap_or("");
                                    
                                    // Save permit to file
                                    let permit_path = format!("{}/.clawparty/permit.json", 
                                        std::env::var("HOME").unwrap_or_else(|_| ".".to_string()));
                                    std::fs::create_dir_all(format!("{}/.clawparty", 
                                        std::env::var("HOME").unwrap_or_else(|_| ".".to_string())))?;
                                    std::fs::write(&permit_path, permit)?;
                                    
                                    // Join mesh
                                    {
                                        let client = api_client.lock().await;
                                        client.join_mesh("clawparty", final_ep_name, permit).await?;
                                    }
                                    
                                    Ok(format!("Successfully joined clawparty as '{}' (endpoint: {})", 
                                        final_username, final_ep_name))
                                };
                                
                                match join_fut.await {
                                    Ok(msg) => {
                                        let mut s = state.write().await;
                                        s.add_log("INFO", &msg);
                                    }
                                    Err(e) => {
                                        let mut s = state.write().await;
                                        s.add_log("ERROR", &format!("Join party failed: {}", e));
                                    }
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
                        s.message_scroll = u16::MAX; // auto-scroll to bottom on new chat

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
                            ActivePanel::Input => ActivePanel::Sidebar,
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
                        s.message_scroll = s.message_scroll.saturating_add(10);
                        s.user_scrolled_up = true;
                    }
                    KeyCode::PageDown => {
                        s.message_scroll = s.message_scroll.saturating_sub(10);
                        if s.message_scroll == 0 {
                            s.user_scrolled_up = false;
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    disable_raw_mode()?;
    io::stdout().execute(LeaveAlternateScreen)?;

    Ok(())
}
