use nostr_sdk::prelude::*;
use sigil_core::message::SigilMessage;
use sigil_core::tui::TuiMessage;
use std::collections::HashMap;
use tokio::sync::mpsc;

/// A single chat message stored in history
#[derive(Debug, Clone)]
pub struct ChatEntry {
    pub from_me: bool,
    pub sender_npub: String,
    pub content: SigilMessage,
    pub timestamp: u64,
}

/// Event from the Nostr network to the UI
#[derive(Debug)]
pub enum ChatEvent {
    /// Incoming message from someone
    MessageReceived {
        sender_npub: String,
        content: String,
    },
    /// Connection status update
    Connected,
    #[allow(dead_code)]
    RelayError(String),
}

/// Per-conversation message history
#[derive(Debug, Default)]
pub struct ChatHistory {
    /// npub -> messages
    conversations: HashMap<String, Vec<ChatEntry>>,
}

impl ChatHistory {
    pub fn add_message(&mut self, peer_npub: &str, entry: ChatEntry) {
        self.conversations
            .entry(peer_npub.to_string())
            .or_default()
            .push(entry);
    }

    pub fn get_messages(&self, peer_npub: &str) -> &[ChatEntry] {
        self.conversations
            .get(peer_npub)
            .map(|v| v.as_slice())
            .unwrap_or(&[])
    }

    /// Get list of npubs that have conversations, sorted by most recent message
    pub fn active_peers(&self) -> Vec<String> {
        let mut peers: Vec<_> = self.conversations.keys().cloned().collect();
        peers.sort_by(|a, b| {
            let ts_a = self
                .conversations
                .get(a)
                .and_then(|v| v.last())
                .map(|e| e.timestamp)
                .unwrap_or(0);
            let ts_b = self
                .conversations
                .get(b)
                .and_then(|v| v.last())
                .map(|e| e.timestamp)
                .unwrap_or(0);
            ts_b.cmp(&ts_a)
        });
        peers
    }
}

/// Start the Nostr client, returns sender for outgoing messages and receiver for incoming events
pub async fn start_nostr(
    keys: Keys,
    relays: Vec<String>,
) -> Result<
    (
        mpsc::Sender<(PublicKey, String)>,
        mpsc::Receiver<ChatEvent>,
        Client,
    ),
    Box<dyn std::error::Error>,
> {
    let client = Client::new(keys.clone());

    for relay in &relays {
        client.add_relay(relay).await?;
    }
    client.connect().await;

    // Publish user metadata
    let metadata = Metadata::new().name("Sigil User");
    client.set_metadata(&metadata).await?;

    // Subscribe to DMs
    let filter = Filter::new()
        .kinds(vec![Kind::EncryptedDirectMessage, Kind::GiftWrap])
        .pubkey(keys.public_key());
    client.subscribe(filter, None).await?;

    let (out_tx, mut out_rx) = mpsc::channel::<(PublicKey, String)>(64);
    let (ev_tx, ev_rx) = mpsc::channel::<ChatEvent>(256);

    // Outgoing message sender task
    let client_send = client.clone();
    let keys_send = keys.clone();
    tokio::spawn(async move {
        while let Some((to, content)) = out_rx.recv().await {
            // Send NIP-04 for compatibility
            if let Ok(encrypted) = nip04::encrypt(keys_send.secret_key(), &to, &content) {
                let tag = Tag::public_key(to);
                if let Ok(ev) = EventBuilder::new(Kind::EncryptedDirectMessage, encrypted)
                    .tag(tag)
                    .sign(&keys_send)
                    .await
                {
                    let _ = client_send.send_event(ev).await;
                }
            }
        }
    });

    // Incoming message listener task
    let mut notifications = client.notifications();
    let ev_tx2 = ev_tx.clone();
    let keys2 = keys.clone();
    tokio::spawn(async move {
        let _ = ev_tx2.send(ChatEvent::Connected).await;
        loop {
            match notifications.recv().await {
                Ok(RelayPoolNotification::Event { event, .. }) => match event.kind {
                    Kind::GiftWrap => {
                        if let Ok(unwrapped) =
                            UnwrappedGift::from_gift_wrap(&keys2, &event).await
                        {
                            let sender_npub = unwrapped
                                .sender
                                .to_bech32()
                                .unwrap_or_default();
                            let _ = ev_tx2
                                .send(ChatEvent::MessageReceived {
                                    sender_npub,
                                    content: unwrapped.rumor.content.clone(),
                                })
                                .await;
                        }
                    }
                    Kind::EncryptedDirectMessage => {
                        let sender = event.pubkey;
                        if let Ok(content) =
                            nip04::decrypt(keys2.secret_key(), &sender, &event.content)
                        {
                            let sender_npub =
                                sender.to_bech32().unwrap_or_default();
                            let _ = ev_tx2
                                .send(ChatEvent::MessageReceived {
                                    sender_npub,
                                    content,
                                })
                                .await;
                        }
                    }
                    _ => {}
                },
                Ok(RelayPoolNotification::Shutdown) => break,
                Ok(_) => {}
                Err(_) => break,
            }
        }
    });

    Ok((out_tx, ev_rx, client))
}

/// Render a TUI message to terminal-friendly styled lines
pub fn render_tui_for_terminal(tui: &TuiMessage) -> Vec<String> {
    match tui {
        TuiMessage::Text { content } => vec![content.clone()],
        TuiMessage::Buttons { text, items } => {
            let mut lines = vec![];
            if let Some(t) = text {
                lines.push(t.clone());
            }
            let btns: Vec<String> = items
                .iter()
                .enumerate()
                .map(|(i, b)| format!(" [{}] {} ", i + 1, b.label))
                .collect();
            lines.push(btns.join("  "));
            lines
        }
        TuiMessage::Card {
            title,
            description,
            actions,
            ..
        } => {
            let mut lines = vec![];
            lines.push(format!("┌─ {} ─┐", title));
            if let Some(desc) = description {
                lines.push(format!("│ {}", desc));
            }
            if let Some(acts) = actions {
                let btns: Vec<String> = acts
                    .iter()
                    .enumerate()
                    .map(|(i, b)| format!("[{}] {}", i + 1, b.label))
                    .collect();
                lines.push(format!("│ {}", btns.join("  ")));
            }
            lines.push("└────────┘".to_string());
            lines
        }
        TuiMessage::Table { title, rows } => {
            let mut lines = vec![];
            if let Some(t) = title {
                lines.push(format!("── {} ──", t));
            }
            for (k, v) in rows {
                lines.push(format!("  {:<16} {}", k, v));
            }
            lines
        }
    }
}
