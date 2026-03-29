//! Agent Registry — NIP-AE compatible agent definition events
//!
//! Uses kind:4199 (Agent Definition) as defined in NIP-AE.
//! Agents publish structured profiles using tags for relay-level filtering.
//!
//! NIP-AE Agent Definition (kind:4199):
//!   d-tag: agent slug (for replaceability)
//!   tags: ["title", name], ["role", role], ["description", desc],
//!         ["t", "agent"], ["t", skill1], ["framework", fw], ["ver", ver]
//!   content: markdown description
//!
//! Also supports legacy kind:31990 for reading (backwards compat).

use nostr_sdk::prelude::*;
use serde::{Deserialize, Serialize};

/// NIP-AE Agent Definition kind
pub const AGENT_DEFINITION_KIND: u16 = 4199;

/// Legacy kind (read-only backwards compat)
pub const LEGACY_REGISTRY_KIND: u16 = 31990;

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

/// Publish an agent definition (NIP-AE kind:4199).
/// Replaces any existing definition for this agent slug.
pub async fn publish_agent(
    client: &Client,
    keys: &Keys,
    entry: &AgentRegistryEntry,
) -> Result<EventId, Error> {
    // Build markdown content from about field
    let content = entry.about.clone().unwrap_or_default();

    // NIP-AE tag structure
    let mut tags = vec![
        Tag::identifier(entry.name.to_lowercase().replace(' ', "-")),
        Tag::custom(TagKind::custom("title"), vec![entry.name.clone()]),
        Tag::hashtag("agent"),
    ];

    if let Some(about) = &entry.about {
        Tag::custom(TagKind::custom("description"), vec![about.clone()]);
        tags.push(Tag::custom(
            TagKind::custom("description"),
            vec![about.clone()],
        ));
    }

    if let Some(fw) = &entry.framework {
        tags.push(Tag::custom(TagKind::custom("framework"), vec![fw.clone()]));
    }

    if let Some(ver) = &entry.version {
        tags.push(Tag::custom(TagKind::custom("ver"), vec![ver.clone()]));
    }

    if entry.tui {
        tags.push(Tag::custom(
            TagKind::custom("tool"),
            vec!["tui".to_string()],
        ));
    }

    // Add skill tags for relay-level filtering
    for skill in &entry.skills {
        tags.push(Tag::hashtag(skill));
    }

    let event = EventBuilder::new(Kind::Custom(AGENT_DEFINITION_KIND), content)
        .tags(tags)
        .sign(keys)
        .await
        .map_err(Error::Builder)?;

    let id = event.id;
    client.send_event(event).await.map_err(Error::Client)?;
    Ok(id)
}

/// Search for agents. Queries both NIP-AE kind:4199 and legacy kind:31990.
pub async fn search_agents(
    client: &Client,
    skill_filter: Option<&str>,
    limit: usize,
) -> Result<Vec<(PublicKey, AgentRegistryEntry)>, Error> {
    // Query both new and legacy kinds
    let mut filter = Filter::new()
        .kinds(vec![
            Kind::Custom(AGENT_DEFINITION_KIND),
            Kind::Custom(LEGACY_REGISTRY_KIND),
        ])
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
        if event.kind == Kind::Custom(AGENT_DEFINITION_KIND) {
            // Parse NIP-AE format (tags-based)
            if let Some(entry) = parse_nip_ae_event(event) {
                agents.push((event.pubkey, entry));
            }
        } else {
            // Legacy JSON content format
            if let Ok(entry) = serde_json::from_str::<AgentRegistryEntry>(&event.content) {
                agents.push((event.pubkey, entry));
            }
        }
    }

    Ok(agents)
}

/// Parse a NIP-AE kind:4199 event into an AgentRegistryEntry
fn parse_nip_ae_event(event: &Event) -> Option<AgentRegistryEntry> {
    let tags = event.tags.iter().collect::<Vec<_>>();

    let name = tags
        .iter()
        .find(|t| t.kind() == TagKind::custom("title"))
        .and_then(|t| t.content())
        .map(|s| s.to_string())
        .unwrap_or_else(|| "Unknown Agent".to_string());

    let about = if event.content.is_empty() {
        tags.iter()
            .find(|t| t.kind() == TagKind::custom("description"))
            .and_then(|t| t.content())
            .map(|s| s.to_string())
    } else {
        Some(event.content.clone())
    };

    let framework = tags
        .iter()
        .find(|t| t.kind() == TagKind::custom("framework"))
        .and_then(|t| t.content())
        .map(|s| s.to_string());

    let version = tags
        .iter()
        .find(|t| t.kind() == TagKind::custom("ver"))
        .and_then(|t| t.content())
        .map(|s| s.to_string());

    let skills: Vec<String> = tags
        .iter()
        .filter(|t| t.kind() == TagKind::custom("t"))
        .filter_map(|t| t.content().map(|s| s.to_string()))
        .filter(|s| s != "agent")
        .collect();

    let tui = tags
        .iter()
        .any(|t| t.kind() == TagKind::custom("tool") && t.content() == Some("tui"));

    Some(AgentRegistryEntry {
        name,
        about,
        picture: None,
        framework,
        skills,
        tui,
        relay: None,
        version,
    })
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
