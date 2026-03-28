//! Agent Registry — custom addressable event for agent discovery
//!
//! Uses kind:31990 (addressable range 30000-39999, replaceable by d-tag)
//! This allows agents to publish structured profiles that can be queried
//! without parsing kind:0 metadata JSON.
//!
//! NIP-XX (Sigil Agent Registry):
//!   kind: 31990
//!   d-tag: agent npub (for replaceability)
//!   content: JSON AgentRegistryEntry
//!   tags: ["d", npub], ["t", skill1], ["t", skill2], ...

use nostr_sdk::prelude::*;
use serde::{Deserialize, Serialize};

/// The kind number for Sigil Agent Registry entries
pub const AGENT_REGISTRY_KIND: u16 = 31990;

/// Structured agent profile for the registry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentRegistryEntry {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub about: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub picture: Option<String>,
    /// Framework used (e.g. "sigil", "langchain", "crewai")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub framework: Option<String>,
    /// Skills/capabilities
    pub skills: Vec<String>,
    /// Whether the agent supports TUI messages
    #[serde(default)]
    pub tui: bool,
    /// Preferred relay for contacting this agent
    #[serde(skip_serializing_if = "Option::is_none")]
    pub relay: Option<String>,
    /// Version string
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
}

/// Publish an agent registry entry. Replaces any existing entry for this agent.
pub async fn publish_agent(
    client: &Client,
    keys: &Keys,
    entry: &AgentRegistryEntry,
) -> Result<EventId, Error> {
    let npub = keys.public_key().to_bech32().map_err(|e| Error::Generic(e.to_string()))?;
    let content =
        serde_json::to_string(entry).map_err(|e| Error::Generic(e.to_string()))?;

    let mut tags = vec![
        Tag::identifier(&npub),
    ];

    // Add skill tags for filtering
    for skill in &entry.skills {
        Tag::hashtag(skill);
        tags.push(Tag::hashtag(skill));
    }

    let event = EventBuilder::new(Kind::Custom(AGENT_REGISTRY_KIND), content)
        .tags(tags)
        .sign(keys)
        .await
        .map_err(Error::Builder)?;

    let id = event.id;
    client.send_event(event).await.map_err(Error::Client)?;
    Ok(id)
}

/// Search the registry for agents. Optionally filter by skill tag.
pub async fn search_agents(
    client: &Client,
    skill_filter: Option<&str>,
    limit: usize,
) -> Result<Vec<(PublicKey, AgentRegistryEntry)>, Error> {
    let mut filter = Filter::new()
        .kind(Kind::Custom(AGENT_REGISTRY_KIND))
        .limit(limit);

    if let Some(skill) = skill_filter {
        filter = filter.hashtag(skill);
    }

    let events = client
        .fetch_events(filter, std::time::Duration::from_secs(10))
        .await
        .map_err(Error::Client)?;

    let mut agents = vec![];
    for event in events.iter() {
        if let Ok(entry) = serde_json::from_str::<AgentRegistryEntry>(&event.content) {
            agents.push((event.pubkey, entry));
        }
    }

    Ok(agents)
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Nostr client error: {0}")]
    Client(#[from] nostr_sdk::client::Error),
    #[error("Builder error: {0}")]
    Builder(#[from] nostr_sdk::event::builder::Error),
    #[error("{0}")]
    Generic(String),
}
