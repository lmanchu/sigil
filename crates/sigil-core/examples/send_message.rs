//! Send a one-off message to a specific npub
//! Usage: cargo run --example send_message -- <npub> <message>

use nostr_sdk::prelude::*;
use std::fs;

#[tokio::main]
async fn main() {
    let args: Vec<String> = std::env::args().collect();
    let recipient_npub = args.get(1).expect("Usage: send_message <npub> <message>");
    let message = args[2..].join(" ");

    let key_data = fs::read_to_string(
        dirs::home_dir().unwrap().join(".sigil/echo-agent.key")
    ).unwrap();
    let keys = Keys::parse(key_data.trim()).unwrap();
    
    let client = Client::new(keys.clone());
    client.add_relay("wss://relay.damus.io").await.unwrap();
    client.connect().await;
    // Wait for relay connection to establish
    tokio::time::sleep(std::time::Duration::from_secs(3)).await;

    let recipient = PublicKey::parse(recipient_npub).unwrap();

    let encrypted = nip04::encrypt(keys.secret_key(), &recipient, &message).unwrap();
    let tag = Tag::public_key(recipient);
    let event = EventBuilder::new(Kind::EncryptedDirectMessage, encrypted)
        .tag(tag)
        .sign(&keys)
        .await
        .unwrap();
    
    client.send_event(event).await.unwrap();
    println!("✅ Sent to {}: {}", recipient_npub, message);
    
    tokio::time::sleep(std::time::Duration::from_secs(2)).await;
}
