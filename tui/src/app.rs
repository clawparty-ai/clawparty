use crate::agent::AgentManager;
use crate::api::ApiClient;
use crate::models::*;
use crate::zeroclaw::ZeroClawDaemon;
use std::fs::OpenOptions;
use std::io::Write;
use std::sync::Arc;
use tokio::sync::Mutex;
use tui_scrollview::ScrollViewState;

#[derive(Debug, Clone, PartialEq)]
pub enum ActivePanel {
    Sidebar,
    #[allow(dead_code)]
    Messages,
    Input,
    Logs,
}

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq)]
pub enum ActiveOrg {
    ZeroClaw,  // ZeroClaw sessions (primary)
    Mesh(String),  // ZTM mesh (secondary, for networking)
    Groups,
    Agents,
}

#[derive(Debug, Clone)]
pub struct SidebarItem {
    #[allow(dead_code)]
    pub label: String,
    pub section: String,
    pub index: usize,
}

#[derive(Debug, Clone)]
pub struct ZeroClawSession {
    pub session_id: String,
    pub name: String,
    #[allow(dead_code)]
    pub last_activity: String,
}

use std::process::Child as ChildProcess;

pub struct AgentProcess {
    pub agent_name: String,
    child: Option<ChildProcess>,
}

impl AgentProcess {
    pub fn new(agent_name: String, child: ChildProcess) -> Self {
        Self { agent_name, child: Some(child) }
    }
}

impl Drop for AgentProcess {
    fn drop(&mut self) {
        if let Some(ref mut child) = self.child {
            let pid = child.id() as i32;
            let _ = unsafe { libc::kill(-pid, libc::SIGKILL) };
            eprintln!("AgentProcess: killed {} (pid {})", self.agent_name, pid);
        }
    }
}

pub struct AppState {
    pub api: Arc<Mutex<ApiClient>>,
    pub meshes: Vec<Mesh>,
    pub current_mesh: Option<String>,
    pub chats: Vec<Chat>,
    pub endpoints: Vec<Endpoint>,
    pub openclaw_agents: Vec<OpenclawAgent>,
    pub messages: Vec<Message>,
    pub logs: Vec<String>,
    pub selected_index: usize,
    pub active_panel: ActivePanel,
    pub active_org: ActiveOrg,
    pub current_chat: Option<usize>,
    pub current_openclaw_agent: Option<OpenclawAgent>,
    pub current_peer: Option<String>,
    pub input_text: String,
    pub agent_running: bool,
    pub local_agents: Vec<OpenclawAgent>,
    pub group_chats: Vec<Chat>,
    pub members: Vec<Endpoint>,
    pub log_file: Option<std::fs::File>,
    pub sidebar_scroll: ScrollViewState,
    pub messages_scroll: ScrollViewState,
    pub logs_scroll: ScrollViewState,
    pub agent_mgr: Option<AgentManager>,
    // ZeroClaw fields
    pub zeroclaw_sessions: Vec<ZeroClawSession>,
    pub current_zeroclaw_session: Option<ZeroClawSession>,
    pub zeroclaw_running: bool,
    pub zeroclaw_mgr: Option<ZeroClawDaemon>,
    pub zeroclaw_pending_tasks: std::collections::HashMap<String, tokio::task::AbortHandle>,
    pub agent_processes: Vec<AgentProcess>,
}

impl AppState {
    pub fn new(api: ApiClient, zeroclaw_only: bool) -> Self {
        let now = chrono::Local::now();
        let timestamp = now.format("%Y%m%d-%H%M%S");
        // Log to ~/.clawparty/ directory instead of current directory
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        let log_dir = format!("{}/.clawparty", home);
        let _ = std::fs::create_dir_all(&log_dir);
        let log_filename = format!("{}/console-log-{}.log", log_dir, timestamp);
        let log_file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&log_filename)
            .ok();

        Self {
            api: Arc::new(Mutex::new(api)),
            meshes: Vec::new(),
            current_mesh: None,
            chats: Vec::new(),
            endpoints: Vec::new(),
            openclaw_agents: Vec::new(),
            messages: Vec::new(),
            logs: Vec::new(),
            selected_index: 0,
            active_panel: ActivePanel::Sidebar,
            // Default to ZeroClaw if available, otherwise Mesh
            active_org: if zeroclaw_only { ActiveOrg::ZeroClaw } else { ActiveOrg::Mesh(String::new()) },
            current_chat: None,
            current_openclaw_agent: None,
            current_peer: None,
            input_text: String::new(),
            agent_running: false,
            local_agents: Vec::new(),
            group_chats: Vec::new(),
            members: Vec::new(),
            log_file,
            sidebar_scroll: ScrollViewState::new(),
            messages_scroll: ScrollViewState::new(),
            logs_scroll: ScrollViewState::new(),
            agent_mgr: None,
            // ZeroClaw fields
            zeroclaw_sessions: Vec::new(),
            current_zeroclaw_session: None,
            zeroclaw_running: false,
            zeroclaw_mgr: None,
            zeroclaw_pending_tasks: std::collections::HashMap::new(),
            agent_processes: Vec::new(),
        }
    }

    pub fn add_log(&mut self, level: &str, message: &str) {
        let now = chrono::Local::now();
        let timestamp = now.format("%H:%M:%S");
        let log_entry = format!("[{}] [{}] {}", timestamp, level, message);

        // Write to log file
        if let Some(ref mut file) = self.log_file {
            let _ = writeln!(*file, "{}", log_entry);
        }

        self.logs.push(log_entry);
        if self.logs.len() > 500 {
            self.logs.remove(0);
        }
    }

    // Trim messages to max history length to prevent memory growth
    pub fn trim_messages(&mut self) {
        if self.messages.len() > 100 {
            self.messages = self.messages.split_off(self.messages.len() - 100);
        }
    }

    // Get the display name for the current conversation
    #[allow(dead_code)]
    pub fn current_conversation_name(&self) -> String {
        if let Some(chat_idx) = self.current_chat {
            if let Some(chat) = self.chats.get(chat_idx) {
                return chat.display_name();
            }
        }
        if let Some(ref agent) = self.current_openclaw_agent {
            return agent.display_name();
        }
        if let Some(ref peer) = self.current_peer {
            return peer.clone();
        }
        "No conversation selected".to_string()
    }

    // Check if we have an active conversation target
    #[allow(dead_code)]
    pub fn has_active_conversation(&self) -> bool {
        self.current_chat.is_some()
            || self.current_openclaw_agent.is_some()
            || self.current_peer.is_some()
    }

    pub fn refresh_sections(&mut self) {
        // ZeroClaw sessions are always shown (primary feature)
        // Local agents and group chats are always shown (independent of mesh)
        self.local_agents = self.openclaw_agents.clone();
        // Deduplicate group chats by group+creator
        let mut seen = std::collections::HashSet::new();
        self.group_chats = self
            .chats
            .iter()
            .filter(|c| c.is_group)
            .filter(|c| {
                let key = format!(
                    "{}:{}",
                    c.creator.as_deref().unwrap_or(""),
                    c.group.as_deref().unwrap_or("")
                );
                seen.insert(key)
            })
            .cloned()
            .collect();

        // Remote agents (members) only shown when mesh is available
        if self.current_mesh.is_some() {
            self.members = self.endpoints.clone();
        } else {
            self.members.clear();
        }
    }

    pub fn get_sidebar_items(&self) -> Vec<SidebarItem> {
        let mut items = Vec::new();

        // ZeroClaw Sessions (PRIMARY - always shown first)
        if !self.zeroclaw_sessions.is_empty() {
            items.push(SidebarItem {
                label: "🦀 ZeroClaw Sessions".to_string(),
                section: "zeroclaw_header".to_string(),
                index: 0,
            });
            for (i, session) in self.zeroclaw_sessions.iter().enumerate() {
                let prefix = if let Some(ref current) = self.current_zeroclaw_session {
                    if current.session_id == session.session_id { "▶ " } else { "  " }
                } else { "  " };
                items.push(SidebarItem {
                    label: format!("{}🦀 {}", prefix, session.name),
                    section: "zeroclaw_sessions".to_string(),
                    index: i,
                });
            }
        } else {
            // Show "New Session" option even if no sessions exist
            items.push(SidebarItem {
                label: "🦀 ZeroClaw Sessions".to_string(),
                section: "zeroclaw_header".to_string(),
                index: 0,
            });
            items.push(SidebarItem {
                label: "  + New Session".to_string(),
                section: "zeroclaw_new".to_string(),
                index: 0,
            });
        }

        // OpenClaw Agents (secondary)
        if !self.local_agents.is_empty() {
            items.push(SidebarItem {
                label: "🤖 OpenClaw Agents".to_string(),
                section: "openclaw_header".to_string(),
                index: 0,
            });
            for (i, agent) in self.local_agents.iter().enumerate() {
                items.push(SidebarItem {
                    label: format!("  {} {}", agent.display_emoji(), agent.display_name()),
                    section: "local_agents".to_string(),
                    index: i,
                });
            }
        }

        // Group Chats
        if !self.group_chats.is_empty() {
            items.push(SidebarItem {
                label: "Group Chats".to_string(),
                section: "groups_header".to_string(),
                index: 0,
            });
            for (i, chat) in self.group_chats.iter().enumerate() {
                items.push(SidebarItem {
                    label: format!("# {}", chat.display_name()),
                    section: "groups".to_string(),
                    index: i,
                });
            }
        }

        // Remote Agents
        if !self.members.is_empty() {
            items.push(SidebarItem {
                label: "Remote Agents".to_string(),
                section: "remote_agents_header".to_string(),
                index: 0,
            });
            for (i, ep) in self.members.iter().enumerate() {
                items.push(SidebarItem {
                    label: format!("● {}", ep.name),
                    section: "remote_agents".to_string(),
                    index: i,
                });
            }
        }

        items
    }

    pub fn select_item(&mut self, index: usize) {
        let items = self.get_sidebar_items();
        if index >= items.len() {
            return;
        }
        self.selected_index = index;
        let item = &items[index];

        // Update active_org based on selection
        match item.section.as_str() {
            "zeroclaw_sessions" => {
                self.active_org = ActiveOrg::ZeroClaw;
                if item.index < self.zeroclaw_sessions.len() {
                    self.current_zeroclaw_session =
                        Some(self.zeroclaw_sessions[item.index].clone());
                    self.current_openclaw_agent = None;
                    self.current_chat = None;
                    self.current_peer = None;
                }
            }
            "zeroclaw_new" => {
                // Handled in event loop
            }
            "local_agents" => {
                self.active_org = ActiveOrg::Agents;
                if item.index < self.local_agents.len() {
                    self.current_openclaw_agent = Some(self.local_agents[item.index].clone());
                    self.current_chat = None;
                    self.current_peer = None;
                    self.current_zeroclaw_session = None;
                }
            }
            "zeroclaw_new" => {
                // Handled in event loop
            }
            "groups" => {
                if item.index < self.group_chats.len() {
                    let group = &self.group_chats[item.index];
                    if let Some(orig_idx) = self.chats.iter().position(|c| {
                        c.is_group && c.group == group.group && c.creator == group.creator
                    }) {
                        self.current_chat = Some(orig_idx);
                        self.current_openclaw_agent = None;
                        self.current_peer = None;
                        self.current_zeroclaw_session = None;
                    }
                }
            }
            "remote_agents" => {
                if item.index < self.members.len() {
                    let ep = &self.members[item.index];
                    self.current_peer = Some(ep.name.clone());
                    self.current_openclaw_agent = None;
                    self.current_zeroclaw_session = None;

                    if let Some(chat_idx) = self
                        .chats
                        .iter()
                        .position(|c| c.peer.as_deref() == Some(&ep.name))
                    {
                        self.current_chat = Some(chat_idx);
                    } else {
                        self.current_chat = None;
                    }
                }
            }
            _ => {}
        }
    }
}
