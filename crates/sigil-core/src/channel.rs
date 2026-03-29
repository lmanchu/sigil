//! NIP-28 Public Channel support for group chat
//!
//! Kind 40: Channel Creation
//! Kind 41: Channel Metadata update
//! Kind 42: Channel Message

use nostr_sdk::prelude::*;
use serde::{Deserialize, Serialize};

/// Channel metadata stored in kind:40 content
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelInfo {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub about: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub picture: Option<String>,
}

/// A message in a channel
#[derive(Debug, Clone)]
pub struct ChannelMsg {
    pub channel_id: EventId,
    pub sender: PublicKey,
    pub content: String,
    pub timestamp: Timestamp,
}

/// Create a new NIP-28 channel. Returns the creation event ID (= channel ID).
pub async fn create_channel(
    client: &Client,
    keys: &Keys,
    info: &ChannelInfo,
) -> Result<EventId, Error> {
    let content = serde_json::to_string(info).map_err(|e| Error::Generic(e.to_string()))?;
    let event = EventBuilder::new(Kind::ChannelCreation, content)
        .sign(keys)
        .await
        .map_err(Error::Builder)?;
    let id = event.id;
    client.send_event(event).await.map_err(Error::Client)?;
    Ok(id)
}

/// Send a message to a NIP-28 channel.
pub async fn send_channel_message(
    client: &Client,
    keys: &Keys,
    channel_id: EventId,
    content: &str,
    relay_hint: Option<&str>,
) -> Result<(), Error> {
    let mut tags = vec![
        Tag::event(channel_id),
        Tag::custom(TagKind::custom("marker"), vec!["root".to_string()]),
    ];
    if let Some(relay) = relay_hint {
        tags.push(Tag::custom(
            TagKind::custom("relay"),
            vec![relay.to_string()],
        ));
    }

    let event = EventBuilder::new(Kind::ChannelMessage, content)
        .tags(tags)
        .sign(keys)
        .await
        .map_err(Error::Builder)?;
    client.send_event(event).await.map_err(Error::Client)?;
    Ok(())
}

/// Subscribe to messages in a channel. Returns a filter to use with client.subscribe().
pub fn channel_filter(channel_id: EventId) -> Filter {
    Filter::new().kind(Kind::ChannelMessage).event(channel_id)
}

/// Fetch existing channel messages (history).
pub async fn fetch_channel_messages(
    client: &Client,
    channel_id: EventId,
    limit: usize,
) -> Result<Vec<ChannelMsg>, Error> {
    let filter = Filter::new()
        .kind(Kind::ChannelMessage)
        .event(channel_id)
        .limit(limit);

    let events = client
        .fetch_events(filter, std::time::Duration::from_secs(5))
        .await
        .map_err(Error::Client)?;

    let mut messages: Vec<ChannelMsg> = events
        .iter()
        .map(|e| ChannelMsg {
            channel_id,
            sender: e.pubkey,
            content: e.content.clone(),
            timestamp: e.created_at,
        })
        .collect();

    messages.sort_by_key(|m| m.timestamp);
    Ok(messages)
}

/// Fetch channel info from a kind:40 event.
pub async fn fetch_channel_info(
    client: &Client,
    channel_id: EventId,
) -> Result<Option<ChannelInfo>, Error> {
    let filter = Filter::new().kind(Kind::ChannelCreation).id(channel_id);

    let events = client
        .fetch_events(filter, std::time::Duration::from_secs(5))
        .await
        .map_err(Error::Client)?;

    let result = if let Some(event) = events.iter().next() {
        let info: ChannelInfo =
            serde_json::from_str(&event.content).map_err(|e| Error::Generic(e.to_string()))?;
        Some(info)
    } else {
        None
    };
    Ok(result)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_channel_info_serialization() {
        let info = ChannelInfo {
            name: "sigil-dev".into(),
            about: Some("Development discussion".into()),
            picture: None,
        };
        let json = serde_json::to_string(&info).unwrap();
        let parsed: ChannelInfo = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.name, "sigil-dev");
        assert_eq!(parsed.about.as_deref(), Some("Development discussion"));
        assert!(parsed.picture.is_none());
    }

    #[test]
    fn test_channel_info_minimal() {
        let json = r#"{"name":"test"}"#;
        let info: ChannelInfo = serde_json::from_str(json).unwrap();
        assert_eq!(info.name, "test");
        assert!(info.about.is_none());
    }

    #[test]
    fn test_channel_info_skips_none_fields() {
        let info = ChannelInfo {
            name: "ch".into(),
            about: None,
            picture: None,
        };
        let json = serde_json::to_string(&info).unwrap();
        assert!(!json.contains("about"));
        assert!(!json.contains("picture"));
    }

    #[test]
    fn test_channel_filter_kind() {
        let id = EventId::all_zeros();
        let filter = channel_filter(id);
        // Filter should target kind:42 (ChannelMessage)
        let json = filter.as_json();
        assert!(json.contains("42"));
    }
}
