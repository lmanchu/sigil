// Standalone E2E test — sends messages to any Sigil agent and verifies replies
// Run: cargo run -p sigil-cli --bin sigil-test
// Target: set SIGIL_TARGET=<npub> or defaults to first contact
// Timeout: set SIGIL_TIMEOUT=30 for longer waits (e.g. hermes bridge)

use nostr_sdk::prelude::*;
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    let sigil_dir = dirs::home_dir().unwrap().join(".sigil");
    let user_key = std::fs::read_to_string(sigil_dir.join("user.key"))?;
    let keys = Keys::parse(user_key.trim())?;

    // Target: env var or first contact
    let target_npub = if let Ok(npub) = std::env::var("SIGIL_TARGET") {
        npub
    } else {
        let contacts_data = std::fs::read_to_string(sigil_dir.join("contacts.json"))?;
        let contacts: serde_json::Value = serde_json::from_str(&contacts_data)?;
        contacts["contacts"][0]["npub"]
            .as_str()
            .expect("no contacts found")
            .to_string()
    };

    let timeout_secs: u64 = std::env::var("SIGIL_TIMEOUT")
        .unwrap_or_else(|_| "15".to_string())
        .parse()
        .unwrap_or(15);

    let target_pk = PublicKey::from_bech32(&target_npub)?;

    println!("=== Sigil E2E Test ===");
    println!("  User:    {}", keys.public_key().to_bech32()?);
    println!(
        "  Target:  {}...",
        &target_npub[..30.min(target_npub.len())]
    );
    println!("  Timeout: {}s", timeout_secs);
    println!();

    let client = Client::new(keys.clone());
    client.add_relay("wss://relay.damus.io").await?;
    client.connect().await;
    tokio::time::sleep(Duration::from_secs(3)).await;
    println!("  Connected to relay.\n");

    let filter = Filter::new()
        .kinds(vec![Kind::EncryptedDirectMessage])
        .pubkey(keys.public_key());
    client.subscribe(filter, None).await?;

    let mut notifications = client.notifications();

    // Send test messages
    let tests = vec![
        ("menu", "TUI/menu response"),
        ("info", "info/status response"),
        ("what is 1+1?", "AI response"),
    ];

    for (msg, desc) in &tests {
        println!("[TEST] Sending '{}' (expect: {})...", msg, desc);
        send_nip04(&client, &keys, &target_pk, msg).await?;
        tokio::time::sleep(Duration::from_millis(500)).await;
    }

    println!();
    println!("Waiting for replies ({}s)...", timeout_secs);

    let mut replies = 0;
    let expected = tests.len();
    let deadline = tokio::time::Instant::now() + Duration::from_secs(timeout_secs);

    loop {
        let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
        if remaining.is_zero() {
            break;
        }

        tokio::select! {
            result = notifications.recv() => {
                if let Ok(RelayPoolNotification::Event { event, .. }) = result {
                    if event.kind == Kind::EncryptedDirectMessage && event.pubkey == target_pk {
                        if let Ok(content) = nip04::decrypt(keys.secret_key(), &target_pk, &event.content) {
                            replies += 1;
                            let label = if content.contains("\"type\"") {
                                "TUI"
                            } else if content.contains("Echo:") {
                                "ECHO"
                            } else {
                                "RESP"
                            };
                            let short = if content.chars().count() > 80 {
                                let s: String = content.chars().take(80).collect();
                                format!("{}...", s)
                            } else {
                                content.clone()
                            };
                            println!("  ✅ Reply #{} [{}]: {}", replies, label, short);
                        }
                    }
                }
            }
            _ = tokio::time::sleep(remaining) => {
                break;
            }
        }
    }

    println!();
    if replies >= expected {
        println!("🎉 ALL {} REPLIES RECEIVED", replies);
    } else {
        println!(
            "⚠️  Got {}/{} replies (some may be delayed)",
            replies, expected
        );
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
            println!(
                "  → sent (success: {}, failed: {})",
                output.success.len(),
                output.failed.len()
            );
        }
        Err(e) => {
            eprintln!("  → send warning: {} (may still arrive)", e);
        }
    }
    Ok(())
}
