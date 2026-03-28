use serde::{Deserialize, Serialize};
use sigil_core::qr::AgentQrData;
use std::fs;
use std::path::PathBuf;

use crate::identity::sigil_dir;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Contact {
    /// Nostr npub (bech32)
    pub npub: String,
    /// Display name
    pub name: String,
    /// Preferred relay
    pub relay: Option<String>,
    /// Whether this contact is a known agent
    pub is_agent: bool,
    /// Agent capabilities (if known)
    pub capabilities: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ContactBook {
    pub contacts: Vec<Contact>,
}

fn contacts_path() -> PathBuf {
    sigil_dir().join("contacts.json")
}

impl ContactBook {
    pub fn load() -> Self {
        let path = contacts_path();
        if path.exists() {
            let data = fs::read_to_string(&path).unwrap_or_default();
            serde_json::from_str(&data).unwrap_or_default()
        } else {
            ContactBook::default()
        }
    }

    pub fn save(&self) {
        let path = contacts_path();
        fs::write(&path, serde_json::to_string_pretty(self).unwrap()).ok();
    }

    /// Add contact from npub string. Returns true if new.
    pub fn add_npub(&mut self, npub: &str, name: &str) -> bool {
        if self.contacts.iter().any(|c| c.npub == npub) {
            return false;
        }
        self.contacts.push(Contact {
            npub: npub.to_string(),
            name: name.to_string(),
            relay: None,
            is_agent: false,
            capabilities: vec![],
        });
        self.save();
        true
    }

    /// Add contact from sigil:// URI. Returns true if new.
    pub fn add_from_uri(&mut self, uri: &str) -> Option<Contact> {
        let data = AgentQrData::from_uri(uri)?;
        if self.contacts.iter().any(|c| c.npub == data.npub) {
            return None;
        }
        let contact = Contact {
            npub: data.npub.clone(),
            name: data.name.clone(),
            relay: Some(data.relay.clone()),
            is_agent: true,
            capabilities: data.capabilities.clone(),
        };
        self.contacts.push(contact.clone());
        self.save();
        Some(contact)
    }

    /// Find contact by npub
    pub fn find(&self, npub: &str) -> Option<&Contact> {
        self.contacts.iter().find(|c| c.npub == npub)
    }

    /// Get display name for an npub, falling back to truncated npub
    pub fn display_name(&self, npub: &str) -> String {
        self.find(npub)
            .map(|c| c.name.clone())
            .unwrap_or_else(|| {
                if npub.len() > 16 {
                    format!("{}...", &npub[..16])
                } else {
                    npub.to_string()
                }
            })
    }
}
