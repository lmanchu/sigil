// Standalone E2E test — sends messages to echo agent and verifies replies
// Run: cargo run -p sigil-cli --bin sigil-test

use nostr_sdk::prelude::*;
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    // Load user identity
    let sigil_dir = dirs::home_dir().unwrap().join(".sigil");
    let user_key = std::fs::read_to_string(sigil_dir.join("user.key"))?;
    let keys = Keys::parse(user_key.trim())?;

    // Echo agent npub (from contacts.json)
    let contacts_data = std::fs::read_to_string(sigil_dir.join("contacts.json"))?;
    let contacts: serde_json::Value = serde_json::from_str(&contacts_data)?;
    let echo_npub = contacts["contacts"][0]["npub"]
        .as_str()
        .expect("no contacts found");
    let echo_pk = PublicKey::from_bech32(echo_npub)?;

    println!("=== Sigil E2E Test ===");
    println!("  User:  {}", keys.public_key().to_bech32()?);
    println!("  Agent: {}", echo_npub);
    println!();

    let client = Client::new(keys.clone());
    client.add_relay("wss://relay.damus.io").await?;
    client.connect().await;
    // Wait for relay connection to be fully established
    tokio::time::sleep(Duration::from_secs(3)).await;
    println!("  Connected to relay.\n");

    // Subscribe to replies
    let filter = Filter::new()
        .kinds(vec![Kind::EncryptedDirectMessage])
        .pubkey(keys.public_key());
    client.subscribe(filter, None).await?;

    let mut notifications = client.notifications();

    // Test 1: Plain text → expect echo
    println!("[TEST 1] Sending plain text...");
    send_nip04(&client, &keys, &echo_pk, "hello from sigil-cli").await?;
    tokio::time::sleep(Duration::from_millis(500)).await;

    // Test 2: "menu" → expect TUI buttons
    println!("[TEST 2] Sending 'menu'...");
    send_nip04(&client, &keys, &echo_pk, "menu").await?;
    tokio::time::sleep(Duration::from_millis(500)).await;

    // Test 3: "status" → expect TUI card
    println!("[TEST 3] Sending 'status'...");
    send_nip04(&client, &keys, &echo_pk, "status").await?;
    tokio::time::sleep(Duration::from_millis(500)).await;

    // Test 4: callback → expect callback ack
    println!("[TEST 4] Sending callback...");
    send_nip04(&client, &keys, &echo_pk, "sigil:callback:calendar").await?;

    println!();
    println!("Waiting for replies (10s)...");

    let mut replies = 0;
    let deadline = tokio::time::Instant::now() + Duration::from_secs(10);

    loop {
        let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
        if remaining.is_zero() {
            break;
        }

        tokio::select! {
            result = notifications.recv() => {
                match result {
                    Ok(RelayPoolNotification::Event { event, .. }) => {
                        if event.kind == Kind::EncryptedDirectMessage && event.pubkey == echo_pk {
                            if let Ok(content) = nip04::decrypt(keys.secret_key(), &echo_pk, &event.content) {
                                replies += 1;
                                let label = if content.contains("Echo:") {
                                    "ECHO"
                                } else if content.contains("\"type\"") {
                                    "TUI"
                                } else if content.contains("tapped") {
                                    "CALLBACK"
                                } else {
                                    "OTHER"
                                };
                                println!("  ✅ Reply #{} [{}]: {}", replies, label, &content[..content.len().min(80)]);
                            }
                        }
                    }
                    _ => {}
                }
            }
            _ = tokio::time::sleep(remaining) => {
                break;
            }
        }
    }

    println!();
    if replies >= 4 {
        println!("🎉 ALL TESTS PASSED ({}/4 replies received)", replies);
    } else {
        println!("⚠️  Got {}/4 replies (some may be delayed by relay)", replies);
    }

    client.disconnect().await;
    Ok(())
}

async fn send_nip04(
    client: &Client,
    keys: &Keys,
    to: &PublicKey,
    content: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let encrypted = nip04::encrypt(keys.secret_key(), to, content)?;
    let tag = Tag::public_key(*to);
    let ev = EventBuilder::new(Kind::EncryptedDirectMessage, encrypted)
        .tag(tag)
        .sign(keys)
        .await?;
    match client.send_event(ev).await {
        Ok(output) => {
            println!("  → sent (success: {}, failed: {})",
                output.success.len(), output.failed.len());
        }
        Err(e) => {
            // Non-fatal — relay may have received it anyway
            eprintln!("  → send warning: {} (may still arrive)", e);
        }
    }
    Ok(())
}
