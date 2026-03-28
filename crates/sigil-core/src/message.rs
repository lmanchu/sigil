use crate::tui::TuiMessage;

/// A message received by or sent from an agent
#[derive(Debug, Clone)]
pub enum SigilMessage {
    /// Plain text message
    Text(String),
    /// Structured TUI message (buttons, cards, etc.)
    Tui(TuiMessage),
    /// Button callback from user tapping a TUI button
    ButtonCallback { button_id: String },
}

impl SigilMessage {
    /// Parse incoming message content — detect TUI vs plain text
    pub fn parse(content: &str) -> Self {
        if TuiMessage::is_tui(content) {
            match TuiMessage::from_json(content) {
                Ok(tui) => SigilMessage::Tui(tui),
                Err(_) => SigilMessage::Text(content.to_string()),
            }
        } else if content.starts_with("sigil:callback:") {
            let button_id = content.strip_prefix("sigil:callback:").unwrap_or("");
            SigilMessage::ButtonCallback {
                button_id: button_id.to_string(),
            }
        } else {
            SigilMessage::Text(content.to_string())
        }
    }

    /// Serialize for sending
    pub fn to_content(&self) -> String {
        match self {
            SigilMessage::Text(s) => s.clone(),
            SigilMessage::Tui(tui) => tui.to_json().unwrap_or_default(),
            SigilMessage::ButtonCallback { button_id } => {
                format!("sigil:callback:{}", button_id)
            }
        }
    }
}
