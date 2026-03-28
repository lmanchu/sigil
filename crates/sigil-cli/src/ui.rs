use crate::chat::{render_tui_for_terminal, ChatEntry, ChatHistory};
use crate::contacts::ContactBook;
use ratatui::prelude::*;
use ratatui::widgets::*;
use sigil_core::message::SigilMessage;

pub enum InputMode {
    /// Normal mode — navigate contacts
    Normal,
    /// Typing a message
    Editing,
    /// Command mode (/ prefix)
    Command,
}

/// A channel the user has joined
#[derive(Debug, Clone)]
pub struct JoinedChannel {
    pub id: String,    // hex event ID
    pub name: String,
}

pub struct App {
    pub contacts: ContactBook,
    pub history: ChatHistory,
    pub channels: Vec<JoinedChannel>,
    pub input: String,
    pub input_mode: InputMode,
    pub selected_contact: usize,
    #[allow(dead_code)]
    pub scroll_offset: u16,
    pub my_npub: String,
    pub status_line: String,
    pub should_quit: bool,
}

impl App {
    pub fn new(contacts: ContactBook, my_npub: String) -> Self {
        Self {
            contacts,
            history: ChatHistory::default(),
            channels: vec![],
            input: String::new(),
            input_mode: InputMode::Normal,
            selected_contact: 0,
            scroll_offset: 0,
            my_npub,
            status_line: "Connecting...".to_string(),
            should_quit: false,
        }
    }

    /// Get the selected peer key — either "npub..." for DMs or "ch:hexid" for channels
    pub fn selected_peer_key(&self) -> Option<String> {
        let peers = self.peer_list();
        peers.get(self.selected_contact).cloned()
    }

    #[allow(dead_code)]
    pub fn selected_is_channel(&self) -> bool {
        self.selected_peer_key()
            .map(|k| k.starts_with("ch:"))
            .unwrap_or(false)
    }

    /// Combined peer list: channels + contacts + unknown senders
    /// Channels prefixed with "ch:" to distinguish from npubs
    pub fn peer_list(&self) -> Vec<String> {
        let mut peers: Vec<String> = vec![];
        // Channels first
        for ch in &self.channels {
            peers.push(format!("ch:{}", ch.id));
        }
        // Then contacts
        for c in &self.contacts.contacts {
            peers.push(c.npub.clone());
        }
        // Then active conversation partners not in contacts
        for npub in self.history.active_peers() {
            if !peers.contains(&npub) {
                peers.push(npub);
            }
        }
        peers
    }

    /// Get display name for a peer key
    pub fn peer_display_name(&self, key: &str) -> String {
        if let Some(ch_id) = key.strip_prefix("ch:") {
            self.channels
                .iter()
                .find(|c| c.id == ch_id)
                .map(|c| c.name.clone())
                .unwrap_or_else(|| format!("#{}...", &ch_id[..8.min(ch_id.len())]))
        } else {
            self.contacts.display_name(key)
        }
    }
}

pub fn draw(frame: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(1),
            Constraint::Length(3),
            Constraint::Length(1),
        ])
        .split(frame.area());

    let main_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(28), Constraint::Min(1)])
        .split(chunks[0]);

    draw_contacts(frame, app, main_chunks[0]);
    draw_chat(frame, app, main_chunks[1]);
    draw_input(frame, app, chunks[1]);
    draw_status(frame, app, chunks[2]);
}

fn draw_contacts(frame: &mut Frame, app: &App, area: Rect) {
    let peers = app.peer_list();
    let items: Vec<ListItem> = peers
        .iter()
        .enumerate()
        .map(|(i, key)| {
            let is_channel = key.starts_with("ch:");
            let name = app.peer_display_name(key);
            let is_agent = if !is_channel {
                app.contacts.find(key).map(|c| c.is_agent).unwrap_or(false)
            } else {
                false
            };
            let prefix = if is_channel {
                "# "
            } else if is_agent {
                "⚙ "
            } else {
                "  "
            };
            let unread = app
                .history
                .get_messages(key)
                .last()
                .map(|e| !e.from_me)
                .unwrap_or(false);
            let style = if i == app.selected_contact {
                Style::default()
                    .bg(Color::DarkGray)
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD)
            } else if is_channel {
                Style::default().fg(Color::Magenta)
            } else if unread {
                Style::default().fg(Color::Yellow)
            } else {
                Style::default().fg(Color::Gray)
            };
            ListItem::new(format!("{}{}", prefix, name)).style(style)
        })
        .collect();

    let list = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::DarkGray))
            .title(" Contacts ")
            .title_style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
    );
    frame.render_widget(list, area);
}

fn draw_chat(frame: &mut Frame, app: &App, area: Rect) {
    let peer_key = app.selected_peer_key();
    let title = match &peer_key {
        Some(key) => {
            let name = app.peer_display_name(key);
            if key.starts_with("ch:") {
                format!(" # {} ", name)
            } else {
                let is_agent = app.contacts.find(key).map(|c| c.is_agent).unwrap_or(false);
                if is_agent {
                    format!(" ⚙ {} ", name)
                } else {
                    format!(" {} ", name)
                }
            }
        }
        None => " No conversation selected ".to_string(),
    };

    let messages = peer_key
        .as_ref()
        .map(|key| app.history.get_messages(key))
        .unwrap_or(&[]);

    let mut lines: Vec<Line> = vec![];
    for entry in messages {
        let rendered = render_entry(entry, &app.contacts);
        for line in rendered {
            lines.push(line);
        }
    }

    // Auto-scroll: show last N lines that fit
    let visible_height = area.height.saturating_sub(2) as usize;
    let start = if lines.len() > visible_height {
        lines.len() - visible_height
    } else {
        0
    };

    let visible_lines: Vec<Line> = lines.into_iter().skip(start).collect();

    let chat = Paragraph::new(visible_lines).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::DarkGray))
            .title(title)
            .title_style(Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
    );
    frame.render_widget(chat, area);
}

fn render_entry<'a>(entry: &ChatEntry, contacts: &ContactBook) -> Vec<Line<'a>> {
    let sender = if entry.from_me {
        "You".to_string()
    } else {
        contacts.display_name(&entry.sender_npub)
    };

    let (prefix_style, msg_style) = if entry.from_me {
        (
            Style::default().fg(Color::Cyan),
            Style::default().fg(Color::White),
        )
    } else {
        (
            Style::default().fg(Color::Green),
            Style::default().fg(Color::White),
        )
    };

    match &entry.content {
        SigilMessage::Text(text) => {
            vec![Line::from(vec![
                Span::styled(format!("{}: ", sender), prefix_style),
                Span::styled(text.clone(), msg_style),
            ])]
        }
        SigilMessage::Tui(tui) => {
            let mut lines = vec![Line::from(Span::styled(
                format!("{}:", sender),
                prefix_style,
            ))];
            let tui_lines = render_tui_for_terminal(tui);
            for tl in tui_lines {
                lines.push(Line::from(Span::styled(
                    format!("  {}", tl),
                    Style::default().fg(Color::Yellow),
                )));
            }
            lines
        }
        SigilMessage::ButtonCallback { button_id } => {
            vec![Line::from(vec![
                Span::styled(format!("{}: ", sender), prefix_style),
                Span::styled(
                    format!("[callback: {}]", button_id),
                    Style::default().fg(Color::DarkGray),
                ),
            ])]
        }
    }
}

fn draw_input(frame: &mut Frame, app: &App, area: Rect) {
    let (title, style) = match app.input_mode {
        InputMode::Normal => (" Press 'i' to type, '/' for commands ", Style::default().fg(Color::DarkGray)),
        InputMode::Editing => (" Message (Enter to send, Esc to cancel) ", Style::default().fg(Color::Yellow)),
        InputMode::Command => (" Command ", Style::default().fg(Color::Magenta)),
    };

    let input = Paragraph::new(app.input.as_str()).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(style)
            .title(title)
            .title_style(style),
    );
    frame.render_widget(input, area);

    // Show cursor in editing mode
    if matches!(app.input_mode, InputMode::Editing | InputMode::Command) {
        frame.set_cursor_position(Position::new(
            area.x + app.input.len() as u16 + 1,
            area.y + 1,
        ));
    }
}

fn draw_status(frame: &mut Frame, app: &App, area: Rect) {
    let npub_short = if app.my_npub.len() > 20 {
        format!("{}...", &app.my_npub[..20])
    } else {
        app.my_npub.clone()
    };
    let status = Line::from(vec![
        Span::styled(" sigil ", Style::default().fg(Color::Black).bg(Color::Cyan).add_modifier(Modifier::BOLD)),
        Span::raw(" "),
        Span::styled(&app.status_line, Style::default().fg(Color::Green)),
        Span::styled(
            format!("  {} ", npub_short),
            Style::default().fg(Color::DarkGray),
        ),
    ]);
    frame.render_widget(Paragraph::new(status), area);
}
