use crate::app::{ActiveOrg, ActivePanel, AppState};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{
        Block, Borders, List, ListItem, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState,
        Wrap,
    },
    Frame,
};

// Theme colors optimized for black background
const THEME_HEADER_FG: Color = Color::Green;
const THEME_BORDER: Color = Color::DarkGray;
const THEME_SECTION_HEADER: Color = Color::Yellow;
const THEME_DEFAULT_TEXT: Color = Color::White;
const THEME_SELECTED_BG: Color = Color::DarkGray;
const THEME_SELECTED_FG: Color = Color::White;
const THEME_SENDER: Color = Color::Cyan;
const THEME_TIME: Color = Color::DarkGray;
const THEME_INPUT_BORDER: Color = Color::Green;
const THEME_LOG_INFO: Color = Color::Gray;
const THEME_LOG_WARN: Color = Color::Yellow;
const THEME_LOG_ERROR: Color = Color::Red;
const THEME_BG: Color = Color::Black;

pub fn render(frame: &mut Frame, state: &AppState) {
    // Fill entire screen with black background
    let bg = ratatui::widgets::Paragraph::new("").style(Style::default().bg(THEME_BG));
    frame.render_widget(bg, frame.area());

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(0)
        .constraints([
            Constraint::Length(1),
            Constraint::Min(1),
            Constraint::Length(5),
            Constraint::Length(10),
            Constraint::Length(1),
        ])
        .split(frame.area());

    render_header(frame, chunks[0], state);
    render_main(frame, chunks[1], state);
    render_input(frame, chunks[2], state);
    render_logs(frame, chunks[3], state);
    render_statusbar(frame, chunks[4], state);
}

fn render_header(frame: &mut Frame, area: Rect, state: &AppState) {
    let title = match &state.active_org {
        ActiveOrg::Mesh(name) => format!("ClawParty - {}", name),
        ActiveOrg::Groups => "ClawParty - Group Chats".to_string(),
        ActiveOrg::Agents => "ClawParty - My Agents".to_string(),
    };

    let status = if state.agent_running {
        Span::styled("Running", Style::default().fg(Color::Green))
    } else {
        Span::styled("Stopped", Style::default().fg(Color::Red))
    };

    let header = Paragraph::new(Line::from(vec![
        Span::styled(
            title,
            Style::default()
                .add_modifier(Modifier::BOLD)
                .fg(THEME_HEADER_FG),
        ),
        Span::raw("  "),
        status,
    ]))
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(THEME_BORDER))
            .style(Style::default().bg(THEME_BG)),
    );

    frame.render_widget(header, area);
}

fn render_main(frame: &mut Frame, area: Rect, state: &AppState) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(25), Constraint::Percentage(75)])
        .split(area);

    render_sidebar(frame, chunks[0], state);
    render_messages(frame, chunks[1], state);
}

fn render_sidebar(frame: &mut Frame, area: Rect, state: &AppState) {
    let mut list_items: Vec<ListItem> = Vec::new();
    let mut global_idx = 0;

    // Local Agents section header
    if !state.local_agents.is_empty() {
        list_items.push(ListItem::new(Line::from(Span::styled(
            "Local Agents",
            Style::default()
                .fg(THEME_SECTION_HEADER)
                .add_modifier(Modifier::BOLD),
        ))));
        for agent in &state.local_agents {
            let style = if global_idx == state.selected_index {
                Style::default()
                    .bg(THEME_SELECTED_BG)
                    .fg(THEME_SELECTED_FG)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(THEME_DEFAULT_TEXT)
            };
            list_items.push(
                ListItem::new(format!(
                    " {} {}",
                    agent.display_emoji(),
                    agent.display_name()
                ))
                .style(style),
            );
            global_idx += 1;
        }
    }

    // Group Chats section header
    if !state.group_chats.is_empty() {
        list_items.push(ListItem::new(Line::from(Span::styled(
            "Group Chats",
            Style::default()
                .fg(THEME_SECTION_HEADER)
                .add_modifier(Modifier::BOLD),
        ))));
        for chat in &state.group_chats {
            let style = if global_idx == state.selected_index {
                Style::default()
                    .bg(THEME_SELECTED_BG)
                    .fg(THEME_SELECTED_FG)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(THEME_DEFAULT_TEXT)
            };
            list_items.push(ListItem::new(format!("# {}", chat.display_name())).style(style));
            global_idx += 1;
        }
    }

    // Members section header
    if !state.members.is_empty() {
        list_items.push(ListItem::new(Line::from(Span::styled(
            "Remote Agents",
            Style::default()
                .fg(THEME_SECTION_HEADER)
                .add_modifier(Modifier::BOLD),
        ))));
        for ep in &state.members {
            let style = if global_idx == state.selected_index {
                Style::default()
                    .bg(THEME_SELECTED_BG)
                    .fg(THEME_SELECTED_FG)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(THEME_DEFAULT_TEXT)
            };
            list_items.push(ListItem::new(format!("● {}", ep.name)).style(style));
            global_idx += 1;
        }
    }

    if list_items.is_empty() {
        list_items.push(ListItem::new("No items"));
    }

    let title = match &state.active_org {
        ActiveOrg::Mesh(_) => " Chats ",
        ActiveOrg::Groups => " Groups ",
        ActiveOrg::Agents => " Agents ",
    };

    let sidebar_border_color = if state.active_panel == ActivePanel::Sidebar {
        Color::Cyan
    } else {
        THEME_BORDER
    };

    let list = List::new(list_items).block(
        Block::default()
            .borders(Borders::ALL)
            .title(title)
            .border_style(Style::default().fg(sidebar_border_color))
            .style(Style::default().bg(THEME_BG)),
    );

    frame.render_widget(list, area);
}

fn render_messages(frame: &mut Frame, area: Rect, state: &AppState) {
    let title = if let Some(chat_idx) = state.current_chat {
        if let Some(chat) = state.chats.get(chat_idx) {
            format!(" {} ", chat.display_name())
        } else {
            " Messages ".to_string()
        }
    } else if let Some(agent) = &state.current_openclaw_agent {
        format!(" {} ", agent.display_name())
    } else if let Some(peer) = &state.current_peer {
        format!(" {} ", peer)
    } else {
        " Messages ".to_string()
    };

    let mut lines: Vec<Line> = Vec::new();

    if state.messages.is_empty() {
        lines.push(Line::raw(
            "No messages yet. Select a chat to view messages.",
        ));
    } else {
        for msg in &state.messages {
            let time = msg.time.as_deref().unwrap_or("");
            let sender = msg.sender.as_deref().unwrap_or("Unknown");
            let text = msg.text.as_deref().unwrap_or("");

            // Split text by newlines into multiple display lines
            let text_lines: Vec<&str> = text.split('\n').collect();

            // First line: [time] sender: content
            lines.push(Line::from(vec![
                Span::styled(format!("[{}] ", time), Style::default().fg(THEME_TIME)),
                Span::styled(
                    format!("{}: ", sender),
                    Style::default()
                        .add_modifier(Modifier::BOLD)
                        .fg(THEME_SENDER),
                ),
                Span::raw(text_lines[0].to_string()),
            ]));

            // Remaining lines: indented
            for extra in text_lines.iter().skip(1) {
                lines.push(Line::from(Span::raw(format!("  {}", extra))));
            }

            // Blank line between messages
            lines.push(Line::raw(""));
        }
    }

    let total_lines = lines.len() as u16;
    // Calculate visible height (area minus borders)
    let visible = area.height.saturating_sub(2);
    // message_scroll is offset from bottom (0 = at bottom)
    let max_scroll = total_lines.saturating_sub(visible);
    // scroll is position from top
    let scroll = if state.user_scrolled_up {
        // user scrolled up: show messages from position (total - visible - offset_from_bottom)
        max_scroll.saturating_sub(state.message_scroll.min(max_scroll))
    } else {
        // at bottom
        max_scroll
    };

    let msg_border_color = if state.active_panel == ActivePanel::Messages {
        Color::Cyan
    } else {
        THEME_BORDER
    };

    frame.render_widget(
        Paragraph::new(lines)
            .wrap(Wrap { trim: false })
            .scroll((scroll, 0))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(title.clone())
                    .border_style(Style::default().fg(msg_border_color))
                    .style(Style::default().bg(THEME_BG)),
            ),
        area,
    );

    // Render scrollbar if content exceeds visible area
    if total_lines > visible {
        let mut scrollbar_state =
            ScrollbarState::new(total_lines as usize).position(scroll as usize);
        frame.render_stateful_widget(
            Scrollbar::new(ScrollbarOrientation::VerticalRight)
                .begin_symbol(Some("▲"))
                .end_symbol(Some("▼"))
                .track_symbol(Some("│"))
                .thumb_symbol("█"),
            area.inner(ratatui::layout::Margin {
                vertical: 1,
                horizontal: 0,
            }),
            &mut scrollbar_state,
        );
    }
}

fn render_input(frame: &mut Frame, area: Rect, state: &AppState) {
    let is_focused = state.active_panel == ActivePanel::Input;
    let border_color = if is_focused {
        THEME_INPUT_BORDER
    } else {
        THEME_BORDER
    };

    let input = Paragraph::new(format!("{}_", state.input_text)).block(
        Block::default()
            .borders(Borders::ALL)
            .title(" Message (Enter to send) ")
            .border_style(Style::default().fg(border_color))
            .style(Style::default().bg(THEME_BG)),
    );

    frame.render_widget(input, area);
}

fn render_logs(frame: &mut Frame, area: Rect, state: &AppState) {
    let lines: Vec<Line> = state
        .logs
        .iter()
        .map(|log| {
            if log.contains("[ERROR]") {
                Line::from(Span::styled(
                    log.clone(),
                    Style::default()
                        .fg(THEME_LOG_ERROR)
                        .add_modifier(Modifier::BOLD),
                ))
            } else if log.contains("[WARN]") {
                Line::from(Span::styled(
                    log.clone(),
                    Style::default().fg(THEME_LOG_WARN),
                ))
            } else {
                Line::from(Span::styled(
                    log.clone(),
                    Style::default().fg(THEME_LOG_INFO),
                ))
            }
        })
        .collect();

    let logs = Paragraph::new(lines).block(
        Block::default()
            .borders(Borders::ALL)
            .title(" Logs ")
            .border_style(Style::default().fg(THEME_BORDER))
            .style(Style::default().bg(THEME_BG)),
    );

    frame.render_widget(logs, area);
}

fn render_statusbar(frame: &mut Frame, area: Rect, state: &AppState) {
    let current_chat_name = state.current_conversation_name();
    let chat_name = if current_chat_name == "No conversation selected" {
        "No chat selected".to_string()
    } else {
        current_chat_name
    };

    let panel_name = match state.active_panel {
        ActivePanel::Sidebar => "Sidebar",
        ActivePanel::Messages => "Messages",
        ActivePanel::Input => "Input",
    };

    let mesh_info = state.current_mesh.as_deref().unwrap_or("No mesh");

    let hints = match state.active_panel {
        ActivePanel::Sidebar => "↑↓: Navigate  Tab: Switch  Enter: Select  q: Quit",
        ActivePanel::Messages => "↑↓: Scroll  PgUp/PgDn: Fast scroll  Tab: Switch",
        ActivePanel::Input => "Enter: Send  Tab: Switch  #exit: Quit  #join-party: Join",
    };

    let status_line = Line::from(vec![
        Span::styled(" ", Style::default().fg(Color::White)),
        Span::styled(
            chat_name,
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(" | ", Style::default().fg(THEME_BORDER)),
        Span::styled(mesh_info, Style::default().fg(Color::Yellow)),
        Span::styled(" | ", Style::default().fg(THEME_BORDER)),
        Span::styled(
            format!("[{}]", panel_name),
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("  ", Style::default()),
        Span::styled(hints, Style::default().fg(THEME_TIME)),
    ]);

    frame.render_widget(
        Paragraph::new(status_line).style(Style::default().bg(Color::DarkGray)),
        area,
    );
}
