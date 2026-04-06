use crate::app::{ActiveOrg, ActivePanel, AppState};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect, Size},
    prelude::StatefulWidget,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame,
};
use tui_scrollview::ScrollView;

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

pub fn render(frame: &mut Frame, state: &mut AppState) {
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

fn render_main(frame: &mut Frame, area: Rect, state: &mut AppState) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(25), Constraint::Percentage(75)])
        .split(area);

    render_sidebar(frame, chunks[0], state);
    render_messages(frame, chunks[1], state);
}

fn render_sidebar(frame: &mut Frame, area: Rect, state: &mut AppState) {
    let mut list_items: Vec<ListItem> = Vec::new();
    let mut global_idx = 0;

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

    let total_items = list_items.len() as u16;
    let visible_height = area.height.saturating_sub(2);

    if total_items > visible_height {
        let content_size = Size::new(area.width, total_items);
        let mut scroll_view = ScrollView::new(content_size);

        let list = List::new(list_items).block(
            Block::default()
                .borders(Borders::ALL)
                .title(title)
                .border_style(Style::default().fg(sidebar_border_color))
                .style(Style::default().bg(THEME_BG)),
        );

        scroll_view.render_widget(list, Rect::new(0, 0, area.width, total_items));
        scroll_view.render(area, frame.buffer_mut(), &mut state.sidebar_scroll);
    } else {
        let list = List::new(list_items).block(
            Block::default()
                .borders(Borders::ALL)
                .title(title)
                .border_style(Style::default().fg(sidebar_border_color))
                .style(Style::default().bg(THEME_BG)),
        );
        frame.render_widget(list, area);
    }
}

fn render_messages(frame: &mut Frame, area: Rect, state: &mut AppState) {
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
            let time = msg.get_time();
            let sender = msg.get_sender();
            let text = msg.get_text();

            let text_lines: Vec<&str> = text.split('\n').collect();

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

            for extra in text_lines.iter().skip(1) {
                lines.push(Line::from(Span::raw(format!("  {}", extra))));
            }

            lines.push(Line::raw(""));
        }
    }

    let total_lines = lines.len() as u16;
    let visible_height = area.height.saturating_sub(2);

    let msg_border_color = if state.active_panel == ActivePanel::Messages {
        Color::Cyan
    } else {
        THEME_BORDER
    };

    if total_lines > visible_height {
        let content_size = Size::new(area.width, total_lines);
        let mut scroll_view = ScrollView::new(content_size);

        let paragraph = Paragraph::new(lines).block(
            Block::default()
                .borders(Borders::ALL)
                .title(title.clone())
                .border_style(Style::default().fg(msg_border_color))
                .style(Style::default().bg(THEME_BG)),
        );

        scroll_view.render_widget(paragraph, Rect::new(0, 0, area.width, total_lines));
        scroll_view.render(area, frame.buffer_mut(), &mut state.messages_scroll);
    } else {
        let paragraph = Paragraph::new(lines).block(
            Block::default()
                .borders(Borders::ALL)
                .title(title.clone())
                .border_style(Style::default().fg(msg_border_color))
                .style(Style::default().bg(THEME_BG)),
        );
        frame.render_widget(paragraph, area);
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

fn render_logs(frame: &mut Frame, area: Rect, state: &mut AppState) {
    let is_focused = state.active_panel == ActivePanel::Logs;
    let border_color = if is_focused {
        Color::Cyan
    } else {
        THEME_BORDER
    };

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

    let total_lines = lines.len() as u16;
    let visible_height = area.height.saturating_sub(2);

    if total_lines > visible_height {
        let content_size = Size::new(area.width, total_lines);
        let mut scroll_view = ScrollView::new(content_size);

        let paragraph = Paragraph::new(lines).block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Logs ")
                .border_style(Style::default().fg(border_color))
                .style(Style::default().bg(THEME_BG)),
        );

        scroll_view.render_widget(paragraph, Rect::new(0, 0, area.width, total_lines));
        scroll_view.render(area, frame.buffer_mut(), &mut state.logs_scroll);
    } else {
        let paragraph = Paragraph::new(lines).block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Logs ")
                .border_style(Style::default().fg(border_color))
                .style(Style::default().bg(THEME_BG)),
        );
        frame.render_widget(paragraph, area);
    }
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
        ActivePanel::Logs => "Logs",
    };

    let mesh_info = state.current_mesh.as_deref().unwrap_or("No mesh");

    let hints = match state.active_panel {
        ActivePanel::Sidebar => "↑↓: Navigate  Tab: Switch  Enter: Select  q: Quit",
        ActivePanel::Messages => "↑↓: Scroll  PgUp/PgDn: Fast scroll  Tab: Switch",
        ActivePanel::Input => "Enter: Send  Tab: Switch  #exit: Quit  #join-party: Join",
        ActivePanel::Logs => "↑↓: Scroll  Tab: Switch",
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
