use crate::file::FileMessage;
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
    /// File attachment
    File(FileMessage),
}

impl SigilMessage {
    /// Parse incoming message content — detect TUI vs file vs plain text
    pub fn parse(content: &str) -> Self {
        if FileMessage::is_file(content) {
            if let Some(f) = FileMessage::from_json(content) {
                return SigilMessage::File(f);
            }
        }
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
            SigilMessage::File(f) => f.to_json(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_plain_text() {
        let msg = SigilMessage::parse("hello world");
        assert!(matches!(msg, SigilMessage::Text(ref s) if s == "hello world"));
    }

    #[test]
    fn test_parse_button_callback() {
        let msg = SigilMessage::parse("sigil:callback:btn_yes");
        assert!(matches!(msg, SigilMessage::ButtonCallback { ref button_id } if button_id == "btn_yes"));
    }

    #[test]
    fn test_parse_tui_message() {
        let json = r#"{"type":"text","content":"hello from agent"}"#;
        let msg = SigilMessage::parse(json);
        assert!(matches!(msg, SigilMessage::Tui(_)));
    }

    #[test]
    fn test_parse_file_message() {
        let json = r#"{"type":"file","url":"https://nostr.build/test.png","mime_type":"image/png"}"#;
        let msg = SigilMessage::parse(json);
        assert!(matches!(msg, SigilMessage::File(_)));
    }

    #[test]
    fn test_text_roundtrip() {
        let original = SigilMessage::Text("test message".into());
        let content = original.to_content();
        let parsed = SigilMessage::parse(&content);
        assert!(matches!(parsed, SigilMessage::Text(ref s) if s == "test message"));
    }

    #[test]
    fn test_callback_roundtrip() {
        let original = SigilMessage::ButtonCallback { button_id: "action_1".into() };
        let content = original.to_content();
        let parsed = SigilMessage::parse(&content);
        assert!(matches!(parsed, SigilMessage::ButtonCallback { ref button_id } if button_id == "action_1"));
    }

    #[test]
    fn test_invalid_json_falls_back_to_text() {
        let msg = SigilMessage::parse("{broken json");
        assert!(matches!(msg, SigilMessage::Text(_)));
    }
}
