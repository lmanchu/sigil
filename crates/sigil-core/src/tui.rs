use serde::{Deserialize, Serialize};

/// TUI message types that agents can send to users
/// Rendered as interactive elements in Sigil client
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum TuiMessage {
    /// Plain text message
    #[serde(rename = "text")]
    Text { content: String },

    /// Row of tappable buttons
    #[serde(rename = "buttons")]
    Buttons {
        text: Option<String>,
        items: Vec<TuiButton>,
    },

    /// Card with title, description, optional image
    #[serde(rename = "card")]
    Card {
        title: String,
        description: Option<String>,
        image_url: Option<String>,
        actions: Option<Vec<TuiButton>>,
    },

    /// Simple key-value table
    #[serde(rename = "table")]
    Table {
        title: Option<String>,
        rows: Vec<(String, String)>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TuiButton {
    pub id: String,
    pub label: String,
    pub style: Option<ButtonStyle>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ButtonStyle {
    Primary,
    Secondary,
    Danger,
}

impl TuiMessage {
    /// Serialize to JSON string for embedding in NIP-44 DM
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }

    /// Parse from JSON string received in NIP-44 DM
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }

    /// Check if a message content string is a TUI message (starts with {)
    pub fn is_tui(content: &str) -> bool {
        let trimmed = content.trim();
        trimmed.starts_with('{') && serde_json::from_str::<TuiMessage>(trimmed).is_ok()
    }
}
