use nostr_sdk::prelude::*;
use std::time::Duration;

/// An agent discovered on the relay network
#[derive(Debug, Clone)]
pub struct DiscoveredAgent {
    pub npub: String,
    pub name: String,
    pub about: Option<String>,
    pub capabilities: Vec<String>,
    #[allow(dead_code)]
    pub picture: Option<String>,
}

/// Search relays for profiles with agent=true in their kind:0 metadata.
/// Returns up to `limit` agents found within `timeout`.
pub async fn discover_agents(
    client: &Client,
    timeout: Duration,
    limit: usize,
) -> Vec<DiscoveredAgent> {
    // Query kind:0 (metadata) events — we'll filter for agent=true client-side
    // since Nostr relays can't filter on JSON content fields
    let filter = Filter::new()
        .kind(Kind::Metadata)
        .limit(500);

    let events = match client.fetch_events(filter, timeout).await {
        Ok(events) => events,
        Err(_) => return vec![],
    };

    let mut agents = vec![];
    for event in events.iter() {
        if let Ok(metadata) = serde_json::from_str::<serde_json::Value>(&event.content) {
            // Check for agent=true in metadata
            let is_agent = metadata
                .get("agent")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);

            if !is_agent {
                continue;
            }

            let name = metadata
                .get("name")
                .or_else(|| metadata.get("display_name"))
                .and_then(|v| v.as_str())
                .unwrap_or("Unknown Agent")
                .to_string();

            let about = metadata
                .get("about")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());

            let picture = metadata
                .get("picture")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());

            let capabilities = metadata
                .get("capabilities")
                .and_then(|v| v.get("skills"))
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str().map(|s| s.to_string()))
                        .collect()
                })
                .unwrap_or_default();

            let npub = event
                .pubkey
                .to_bech32()
                .unwrap_or_default();

            agents.push(DiscoveredAgent {
                npub,
                name,
                about,
                capabilities,
                picture,
            });

            if agents.len() >= limit {
                break;
            }
        }
    }

    agents
}
