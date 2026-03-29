//! Send a TUI message (buttons + card) to demonstrate rich agent UI
//! Usage: cargo run --example send_tui -- <npub>

use nostr_sdk::prelude::*;
use sigil_core::tui::{ButtonStyle, TuiButton, TuiMessage};
use std::fs;

#[tokio::main]
async fn main() {
    let args: Vec<String> = std::env::args().collect();
    let recipient_npub = args.get(1).expect("Usage: send_tui <npub>");

    let key_data =
        fs::read_to_string(dirs::home_dir().unwrap().join(".sigil/echo-agent.key")).unwrap();
    let keys = Keys::parse(key_data.trim()).unwrap();

    let client = Client::new(keys.clone());
    client.add_relay("wss://relay.damus.io").await.unwrap();
    client.connect().await;
    tokio::time::sleep(std::time::Duration::from_secs(3)).await;

    let recipient = PublicKey::parse(recipient_npub).unwrap();

    // 1. Send a buttons message
    let buttons_msg = TuiMessage::Buttons {
        text: Some("What would you like me to do?".to_string()),
        items: vec![
            TuiButton {
                id: "calendar".into(),
                label: "📅 Check Calendar".into(),
                style: Some(ButtonStyle::Primary),
            },
            TuiButton {
                id: "email".into(),
                label: "📧 Read Email".into(),
                style: Some(ButtonStyle::Secondary),
            },
            TuiButton {
                id: "tasks".into(),
                label: "✅ Show Tasks".into(),
                style: Some(ButtonStyle::Secondary),
            },
        ],
    };

    let content = buttons_msg.to_json().unwrap();
    let encrypted = nip04::encrypt(keys.secret_key(), &recipient, &content).unwrap();
    let tag = Tag::public_key(recipient);
    let event = EventBuilder::new(Kind::EncryptedDirectMessage, encrypted)
        .tag(tag.clone())
        .sign(&keys)
        .await
        .unwrap();
    client.send_event(event).await.unwrap();
    println!("✅ Sent buttons TUI message");

    tokio::time::sleep(std::time::Duration::from_secs(1)).await;

    // 2. Send a card message
    let card_msg = TuiMessage::Card {
        title: "Sigil Agent Status".to_string(),
        description: Some("Your AI-Native Messenger agent is online and ready.".to_string()),
        image_url: None,
        actions: Some(vec![TuiButton {
            id: "details".into(),
            label: "View Details".into(),
            style: Some(ButtonStyle::Primary),
        }]),
    };

    let content2 = card_msg.to_json().unwrap();
    let encrypted2 = nip04::encrypt(keys.secret_key(), &recipient, &content2).unwrap();
    let event2 = EventBuilder::new(Kind::EncryptedDirectMessage, encrypted2)
        .tag(tag.clone())
        .sign(&keys)
        .await
        .unwrap();
    client.send_event(event2).await.unwrap();
    println!("✅ Sent card TUI message");

    tokio::time::sleep(std::time::Duration::from_secs(1)).await;

    // 3. Send a table message
    let table_msg = TuiMessage::Table {
        title: Some("System Status".to_string()),
        rows: vec![
            ("Protocol".into(), "Nostr (NIP-04 + NIP-17)".into()),
            ("Relay".into(), "relay.damus.io".into()),
            ("Encryption".into(), "E2E (NIP-44)".into()),
            ("Uptime".into(), "Just born 🎉".into()),
        ],
    };

    let content3 = table_msg.to_json().unwrap();
    let encrypted3 = nip04::encrypt(keys.secret_key(), &recipient, &content3).unwrap();
    let event3 = EventBuilder::new(Kind::EncryptedDirectMessage, encrypted3)
        .tag(tag)
        .sign(&keys)
        .await
        .unwrap();
    client.send_event(event3).await.unwrap();
    println!("✅ Sent table TUI message");

    tokio::time::sleep(std::time::Duration::from_secs(2)).await;
    println!("Done — check Damus DMs!");
}
