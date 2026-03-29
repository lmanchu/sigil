use nostr_sdk::prelude::*;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;

/// Agent capabilities advertised in profile metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentCapabilities {
    pub skills: Vec<String>,
    pub tui: bool,
    pub framework: Option<String>,
}

/// Agent profile metadata — extends Nostr kind:0 profile
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentProfile {
    pub name: String,
    pub about: Option<String>,
    pub picture: Option<String>,
    pub agent: bool,
    pub capabilities: Option<AgentCapabilities>,
}

/// Message handler callback type
pub type MessageHandler = Arc<dyn Fn(String, PublicKey) -> Option<String> + Send + Sync>;

/// A Sigil agent that connects to Nostr relays and handles messages
pub struct SigilAgent {
    pub keys: Keys,
    pub profile: AgentProfile,
    pub relays: Vec<String>,
    client: Arc<RwLock<Option<Client>>>,
    handler: Option<MessageHandler>,
}

impl SigilAgent {
    /// Create a new agent with generated keys
    pub fn new(name: &str, relays: Vec<String>) -> Self {
        Self {
            keys: Keys::generate(),
            profile: AgentProfile {
                name: name.to_string(),
                about: None,
                picture: None,
                agent: true,
                capabilities: Some(AgentCapabilities {
                    skills: vec![],
                    tui: false,
                    framework: None,
                }),
            },
            relays,
            client: Arc::new(RwLock::new(None)),
            handler: None,
        }
    }

    /// Create agent from existing secret key
    pub fn from_key(name: &str, secret_key: &str, relays: Vec<String>) -> Result<Self, Error> {
        let keys = Keys::parse(secret_key)?;
        Ok(Self {
            keys,
            profile: AgentProfile {
                name: name.to_string(),
                about: None,
                picture: None,
                agent: true,
                capabilities: Some(AgentCapabilities {
                    skills: vec![],
                    tui: false,
                    framework: None,
                }),
            },
            relays,
            client: Arc::new(RwLock::new(None)),
            handler: None,
        })
    }

    /// Set the message handler
    pub fn on_message<F>(&mut self, handler: F)
    where
        F: Fn(String, PublicKey) -> Option<String> + Send + Sync + 'static,
    {
        self.handler = Some(Arc::new(handler));
    }

    /// Get the agent's npub
    pub fn npub(&self) -> String {
        self.keys.public_key().to_bech32().unwrap_or_default()
    }

    /// Connect to relays and start listening
    pub async fn start(&self) -> Result<(), Error> {
        let client = Client::new(self.keys.clone());

        for relay in &self.relays {
            client.add_relay(relay).await?;
        }
        client.connect().await;

        // Publish agent profile (kind:0 with agent=true)
        let metadata = Metadata::new()
            .name(&self.profile.name)
            .custom_field("agent", serde_json::json!(true));

        let metadata = match &self.profile.about {
            Some(about) => metadata.about(about),
            None => metadata,
        };

        let metadata = match &self.profile.capabilities {
            Some(caps) => metadata.custom_field("capabilities", serde_json::json!(caps)),
            None => metadata,
        };

        client.set_metadata(&metadata).await?;
        tracing::info!("Agent '{}' connected. npub: {}", self.profile.name, self.npub());

        // Store client
        {
            let mut c = self.client.write().await;
            *c = Some(client.clone());
        }

        // Subscribe to both NIP-04 (legacy DM) and NIP-17 (GiftWrap)
        // Damus and most clients still use NIP-04 by default
        let filter = Filter::new()
            .kinds(vec![Kind::EncryptedDirectMessage, Kind::GiftWrap])
            .pubkey(self.keys.public_key());

        client.subscribe(filter, None).await?;

        // Listen for messages via broadcast receiver
        let mut notifications = client.notifications();
        let handler = self.handler.clone();
        let keys = self.keys.clone();
        let client_for_reply = client.clone();

        loop {
            match notifications.recv().await {
                Ok(RelayPoolNotification::Event { event, .. }) => {
                    match event.kind {
                        Kind::GiftWrap => {
                            // NIP-17 modern encrypted DM
                            match UnwrappedGift::from_gift_wrap(&keys, &event).await {
                                Ok(unwrapped) => {
                                    let content = unwrapped.rumor.content.clone();
                                    let sender = unwrapped.sender;
                                    tracing::info!("[NIP-17] From {}: {}",
                                        sender.to_bech32().unwrap_or_default(),
                                        &content[..content.len().min(50)]
                                    );
                                    if let Some(ref h) = handler {
                                        if let Some(reply) = h(content, sender) {
                                            let empty_tags: Vec<Tag> = vec![];
                                            let _ = client_for_reply
                                                .send_private_msg(sender, reply, empty_tags)
                                                .await;
                                        }
                                    }
                                }
                                Err(e) => tracing::debug!("Gift unwrap failed: {}", e),
                            }
                        }
                        Kind::EncryptedDirectMessage => {
                            // NIP-04 legacy DM (Damus default)
                            let sender = event.pubkey;
                            match nip04::decrypt(keys.secret_key(), &sender, &event.content) {
                                Ok(content) => {
                                    tracing::info!("[NIP-04] From {}: {}",
                                        sender.to_bech32().unwrap_or_default(),
                                        &content[..content.len().min(50)]
                                    );
                                    if let Some(ref h) = handler {
                                        if let Some(reply) = h(content, sender) {
                                            if let Ok(encrypted) = nip04::encrypt(
                                                keys.secret_key(), &sender, &reply
                                            ) {
                                                let tag = Tag::public_key(sender);
                                                if let Ok(ev) = EventBuilder::new(
                                                    Kind::EncryptedDirectMessage,
                                                    encrypted,
                                                ).tag(tag).sign(&keys).await {
                                                    let _ = client_for_reply.send_event(ev).await;
                                                }
                                            }
                                        }
                                    }
                                }
                                Err(e) => tracing::debug!("NIP-04 decrypt failed: {}", e),
                            }
                        }
                        _ => {}
                    }
                }
                Ok(RelayPoolNotification::Shutdown) => break,
                Ok(_) => {}
                Err(e) => {
                    tracing::warn!("Notification error: {}", e);
                }
            }
        }

        Ok(())
    }

    /// Send a message to a specific public key
    pub async fn send(&self, to: PublicKey, content: &str) -> Result<(), Error> {
        let client = self.client.read().await;
        if let Some(c) = client.as_ref() {
            let empty_tags: Vec<Tag> = vec![];
            c.send_private_msg(to, content, empty_tags).await?;
        }
        Ok(())
    }
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Nostr SDK error: {0}")]
    NostrSdk(#[from] nostr_sdk::client::Error),
    #[error("Key error: {0}")]
    Key(#[from] nostr_sdk::key::Error),
    #[error("Signer error: {0}")]
    Signer(#[from] nostr_sdk::signer::SignerError),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_agent_has_generated_keys() {
        let agent = SigilAgent::new("test-bot", vec!["wss://relay.damus.io".into()]);
        assert_eq!(agent.profile.name, "test-bot");
        assert!(agent.profile.agent);
        assert!(!agent.npub().is_empty());
        assert!(agent.npub().starts_with("npub1"));
    }

    #[test]
    fn test_from_key_roundtrip() {
        let keys = Keys::generate();
        let nsec = keys.secret_key().to_bech32().unwrap();
        let agent = SigilAgent::from_key("keyed-bot", &nsec, vec![]).unwrap();
        assert_eq!(agent.keys.public_key(), keys.public_key());
    }

    #[test]
    fn test_from_key_invalid() {
        let result = SigilAgent::from_key("bad", "not-a-key", vec![]);
        assert!(result.is_err());
    }

    #[test]
    fn test_agent_capabilities_default() {
        let agent = SigilAgent::new("cap-bot", vec![]);
        let caps = agent.profile.capabilities.as_ref().unwrap();
        assert!(caps.skills.is_empty());
        assert!(!caps.tui);
        assert!(caps.framework.is_none());
    }

    #[test]
    fn test_agent_profile_serialization() {
        let profile = AgentProfile {
            name: "test".into(),
            about: Some("A test agent".into()),
            picture: None,
            agent: true,
            capabilities: Some(AgentCapabilities {
                skills: vec!["weather".into(), "translate".into()],
                tui: true,
                framework: Some("sigil".into()),
            }),
        };
        let json = serde_json::to_string(&profile).unwrap();
        let parsed: AgentProfile = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.name, "test");
        assert!(parsed.agent);
        assert_eq!(parsed.capabilities.unwrap().skills.len(), 2);
    }
}
