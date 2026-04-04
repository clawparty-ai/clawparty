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
use tokio::time::{sleep, Duration};

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
            sleep(Duration::from_secs(2)).await;

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

            let api_lock = api_client.lock().await;
            let chats = api_lock.get_chats(&mesh).await.ok();
            let endpoints = api_lock.get_endpoints(&mesh).await.ok();
            let agents = api_lock.get_openclaw_agents().await.ok();

            let mut messages = None;
            if let Some(chat_idx) = current_chat {
                let chat_info = {
                    let s = poll_state.read().await;
                    s.chats.get(chat_idx).cloned()
                };
                if let Some(chat) = chat_info {
                    if chat.is_group {
                        if let (Some(creator), Some(group)) = (&chat.creator, &chat.group) {
                            messages =
                                api_lock.get_group_messages(&mesh, creator, group).await.ok();
                        }
                    } else if let Some(peer) = &chat.peer {
                        messages = api_lock.get_peer_messages(&mesh, peer).await.ok();
                    }
                }
            } else if let Some(ref agent) = current_agent {
                messages = api_lock.get_openclaw_messages(&agent.id).await.ok();
            } else if let Some(ref peer) = current_peer {
                messages = api_lock.get_peer_messages(&mesh, peer).await.ok();
            }
            drop(api_lock);

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
                    s.messages = m;
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
                    KeyCode::Char('q') => break,
                    KeyCode::Up => {
                        if s.active_panel != ActivePanel::Input {
                            if s.selected_index > 0 {
                                s.selected_index -= 1;
                            }
                        }
                    }
                    KeyCode::Down => {
                        if s.active_panel != ActivePanel::Input {
                            let items = s.get_sidebar_items();
                            if s.selected_index < items.len().saturating_sub(1) {
                                s.selected_index += 1;
                            }
                        }
                    }
                    KeyCode::Enter => {
                        if s.active_panel == ActivePanel::Input && !s.input_text.is_empty() {
                            let text = s.input_text.clone();
                            s.input_text.clear();

                            // Handle #exit command
                            if text.trim() == "#exit" {
                                drop(s);
                                disable_raw_mode()?;
                                io::stdout().execute(LeaveAlternateScreen)?;
                                return Ok(());
                            }

                            let send_result: anyhow::Result<()> = if let Some(chat_idx) =
                                s.current_chat
                            {
                                if let Some(chat) = s.chats.get(chat_idx).cloned() {
                                    if let Some(ref mesh) = s.current_mesh {
                                        let mesh = mesh.clone();
                                        if chat.is_group {
                                            if let (Some(creator), Some(group)) =
                                                (&chat.creator, &chat.group)
                                            {
                                                let c = creator.clone();
                                                let g = group.clone();
                                                let l = s.api.lock().await;
                                                l.send_group_message(&mesh, &c, &g, &text).await
                                            } else {
                                                Err(anyhow::anyhow!("Invalid group chat"))
                                            }
                                        } else if let Some(peer) = &chat.peer {
                                            let p = peer.clone();
                                            let l = s.api.lock().await;
                                            l.send_peer_message(&mesh, &p, &text).await
                                        } else {
                                            Err(anyhow::anyhow!("Invalid chat"))
                                        }
                                    } else {
                                        Err(anyhow::anyhow!("No mesh selected"))
                                    }
                                } else {
                                    Err(anyhow::anyhow!("No chat selected"))
                                }
                            } else if let Some(ref agent) = s.current_openclaw_agent {
                                let aid = agent.id.clone();
                                let l = s.api.lock().await;
                                l.send_openclaw_message(&aid, &text).await
                            } else if let Some(ref peer) = s.current_peer {
                                if let Some(ref mesh) = s.current_mesh {
                                    let mesh = mesh.clone();
                                    let p = peer.clone();
                                    let l = s.api.lock().await;
                                    l.send_peer_message(&mesh, &p, &text).await
                                } else {
                                    Err(anyhow::anyhow!("No mesh selected"))
                                }
                            } else {
                                let chat_debug = format!(
                                    "No chat/agent/peer: chat={:?}, agent={:?}, peer={:?}",
                                    s.current_chat,
                                    s.current_openclaw_agent.as_ref().map(|a| &a.id),
                                    s.current_peer
                                );
                                s.add_log("ERROR", &chat_debug);
                                Err(anyhow::anyhow!("No chat or agent selected"))
                            };

                            drop(s);
                            if let Err(e) = send_result {
                                let mut s2 = state.write().await;
                                s2.add_log("ERROR", &format!("Failed to send message: {}", e));
                            } else {
                                let mut s2 = state.write().await;
                                s2.add_log("INFO", "Message sent");
                            }
                            continue;
                        }

                        let idx = s.selected_index;
                        s.select_item(idx);

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
                            ActivePanel::Sidebar => ActivePanel::Input,
                            ActivePanel::Messages => ActivePanel::Input,
                            ActivePanel::Input => ActivePanel::Sidebar,
                        };
                    }
                    KeyCode::Backspace => {
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
                        s.active_panel = ActivePanel::Sidebar;
                    }
                    KeyCode::Right => {
                        s.active_panel = ActivePanel::Input;
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
