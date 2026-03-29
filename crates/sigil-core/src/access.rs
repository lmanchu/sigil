//! Agent access control — who can talk to this agent?
//!
//! Two agent modes:
//! - Personal: only owner + explicitly authorized npubs
//! - Service: open to anyone (like LINE/WeChat official accounts)
//!
//! Config stored in ~/.sigil/<agent-name>.access.json

use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fs;
use std::path::PathBuf;

/// Agent access mode
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum AgentMode {
    /// Only owner + authorized npubs can interact
    Personal,
    /// Anyone can interact
    Service,
}

/// Access control configuration for an agent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccessControl {
    /// Agent operating mode
    pub mode: AgentMode,
    /// Owner's npub — always has full access
    pub owner: String,
    /// Additional authorized npubs (personal mode only)
    pub authorized: HashSet<String>,
    /// Message shown to unauthorized users
    #[serde(default = "default_reject_message")]
    pub reject_message: String,
}

fn default_reject_message() -> String {
    "This is a personal agent. You are not authorized to interact with it. Contact the owner to request access.".to_string()
}

impl AccessControl {
    /// Create a new personal agent access control
    pub fn personal(owner_npub: &str) -> Self {
        Self {
            mode: AgentMode::Personal,
            owner: owner_npub.to_string(),
            authorized: HashSet::new(),
            reject_message: default_reject_message(),
        }
    }

    /// Create a new service agent access control (open to all)
    pub fn service(owner_npub: &str) -> Self {
        Self {
            mode: AgentMode::Service,
            owner: owner_npub.to_string(),
            authorized: HashSet::new(),
            reject_message: String::new(),
        }
    }

    /// Check if a sender npub is allowed to interact
    pub fn is_authorized(&self, sender_npub: &str) -> bool {
        match self.mode {
            AgentMode::Service => true,
            AgentMode::Personal => {
                sender_npub == self.owner || self.authorized.contains(sender_npub)
            }
        }
    }

    /// Authorize a new npub (personal mode)
    pub fn authorize(&mut self, npub: &str) {
        self.authorized.insert(npub.to_string());
    }

    /// Revoke access for an npub
    pub fn revoke(&mut self, npub: &str) {
        self.authorized.remove(npub);
    }

    /// Load from file, or create default personal config
    pub fn load(agent_name: &str, owner_npub: &str) -> Self {
        let path = access_path(agent_name);
        if path.exists() {
            let data = fs::read_to_string(&path).unwrap_or_default();
            serde_json::from_str(&data).unwrap_or_else(|_| Self::personal(owner_npub))
        } else {
            let ac = Self::personal(owner_npub);
            ac.save(agent_name);
            ac
        }
    }

    /// Save to file
    pub fn save(&self, agent_name: &str) {
        let path = access_path(agent_name);
        fs::write(&path, serde_json::to_string_pretty(self).unwrap()).ok();
    }
}

fn access_path(agent_name: &str) -> PathBuf {
    let dir = dirs::home_dir().unwrap().join(".sigil");
    fs::create_dir_all(&dir).ok();
    dir.join(format!("{}.access.json", agent_name))
}
