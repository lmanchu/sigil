//! Hermes Bridge Agent — connects Hermes Agent's 155+ skills to Sigil
//!
//! Receives Nostr DMs, routes to `hermes chat -q "..." -Q --yolo`,
//! sends the response back via Sigil.
//!
//! Run: cargo run --example hermes_bridge

use nostr_sdk::prelude::*;
use sigil_core::qr::AgentQrData;
use std::fs;
use std::path::PathBuf;
use tokio::process::Command as TokioCommand;

fn key_path() -> PathBuf {
    let dir = dirs::home_dir().unwrap().join(".sigil");
    fs::create_dir_all(&dir).ok();
    dir.join("hermes-bridge.key")
}

async fn call_hermes(query: &str) -> String {
    let result = TokioCommand::new("hermes")
        .args(["chat", "-q", query, "-Q", "--yolo"])
        .output()
        .await;

    match result {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout).to_string();
            let response = stdout.trim().to_string();
            if response.is_empty() {
                let stderr = String::from_utf8_lossy(&output.stderr).to_string();
                if !stderr.is_empty() {
                    format!("(hermes error: {})", stderr.lines().last().unwrap_or("unknown"))
                } else {
                    "(no response)".to_string()
                }
            } else if response.len() > 2000 {
                format!("{}...\n\n(truncated)", &response[..2000])
            } else {
                response
            }
        }
        Err(e) => format!("Failed to call hermes: {}", e),
    }
}

fn build_skill_menu() -> String {
    r#"{"type":"buttons","text":"I'm connected to 155+ Hermes & OpenClaw skills. What do you need?","items":[{"id":"skill:research","label":"🔍 Research","style":"primary"},{"id":"skill:github","label":"🐙 GitHub","style":"secondary"},{"id":"skill:linear","label":"📋 Linear","style":"secondary"},{"id":"skill:wells","label":"💰 Finance","style":"secondary"},{"id":"skill:polymarket","label":"📊 Polymarket","style":"secondary"},{"id":"skill:todo","label":"✅ Tasks","style":"secondary"}]}"#.to_string()
}

fn build_info_table() -> String {
    r#"{"type":"table","title":"Hermes Bridge Agent","rows":[["Engine","Hermes Agent v0.4.0"],["Skills","155+ (OpenClaw + Hermes builtin)"],["Model","Claude Sonnet 4.5 (via CLIProxy)"],["Protocol","Nostr NIP-04 E2E Encrypted"],["Mode","Quiet + YOLO (auto-approve tools)"]]}"#.to_string()
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    let relay = std::env::var("SIGIL_RELAY")
        .unwrap_or_else(|_| "wss://relay.damus.io".to_string());

    // Load or create persistent keypair
    let key_file = key_path();
    let keys = if key_file.exists() {
        let secret = fs::read_to_string(&key_file)?;
        Keys::parse(secret.trim())?
    } else {
        let k = Keys::generate();
        fs::write(&key_file, k.secret_key().to_bech32()?)?;
        println!("New keypair saved to {}", key_file.display());
        k
    };

    let client = Client::new(keys.clone());
    client.add_relay(&relay).await?;
    client.connect().await;

    // Publish agent profile
    let metadata = Metadata::new()
        .name("Hermes Bridge")
        .about("155+ skills via encrypted Nostr DM")
        .custom_field("agent", serde_json::json!(true))
        .custom_field("capabilities", serde_json::json!({
            "skills": ["research", "github", "linear", "finance", "polymarket", "tasks"],
            "tui": true,
            "framework": "sigil+hermes"
        }));
    client.set_metadata(&metadata).await?;

    // Subscribe to DMs
    let filter = Filter::new()
        .kinds(vec![Kind::EncryptedDirectMessage, Kind::GiftWrap])
        .pubkey(keys.public_key());
    client.subscribe(filter, None).await?;

    let qr = AgentQrData {
        npub: keys.public_key().to_bech32()?,
        relay: relay.clone(),
        name: "Hermes Bridge".to_string(),
        capabilities: vec!["research".into(), "github".into(), "finance".into()],
    };

    println!("╔══════════════════════════════════════════╗");
    println!("║  Sigil × Hermes Bridge Agent             ║");
    println!("║  155+ skills via encrypted Nostr DM       ║");
    println!("╚══════════════════════════════════════════╝");
    println!();
    println!("  npub:   {}", keys.public_key().to_bech32()?);
    println!("  relay:  {}", relay);
    println!("  QR:     {}", qr.to_uri());
    println!();
    println!("Listening...");
    println!();

    let mut notifications = client.notifications();

    loop {
        match notifications.recv().await {
            Ok(RelayPoolNotification::Event { event, .. }) => {
                let (sender, content) = match event.kind {
                    Kind::EncryptedDirectMessage => {
                        let sender = event.pubkey;
                        match nip04::decrypt(keys.secret_key(), &sender, &event.content) {
                            Ok(c) => (sender, c),
                            Err(_) => continue,
                        }
                    }
                    Kind::GiftWrap => {
                        match UnwrappedGift::from_gift_wrap(&keys, &event).await {
                            Ok(uw) => (uw.sender, uw.rumor.content.clone()),
                            Err(_) => continue,
                        }
                    }
                    _ => continue,
                };

                let sender_short = sender.to_bech32().unwrap_or_default();
                let sender_short = if sender_short.len() > 20 {
                    format!("{}...", &sender_short[..20])
                } else {
                    sender_short.clone()
                };

                println!("[{}] → {}", sender_short, &content[..content.len().min(60)]);

                let lower = content.to_lowercase();

                // Handle local commands (no hermes call needed)
                let reply = if content.starts_with("sigil:callback:") {
                    let id = content.strip_prefix("sigil:callback:").unwrap_or("");
                    format!("Callback: {}", id)
                } else if lower == "menu" || lower == "help" || lower == "/help" {
                    build_skill_menu()
                } else if lower == "info" || lower == "status" || lower == "/info" {
                    build_info_table()
                } else if lower == "skills" || lower == "/skills" {
                    "Categories: research, github, linear, polymarket, wells, pm, todo, social-content, apple-notes, imessage, code-execution, browser, and 140+ more. Just ask!".to_string()
                } else {
                    // Route to Hermes (async!)
                    println!("[{}] Calling hermes...", sender_short);
                    let response = call_hermes(&content).await;
                    println!("[{}] ← {}...", sender_short, &response[..response.len().min(60)]);
                    response
                };

                // Send reply via NIP-04
                if let Ok(encrypted) = nip04::encrypt(keys.secret_key(), &sender, &reply) {
                    let tag = Tag::public_key(sender);
                    if let Ok(ev) = EventBuilder::new(Kind::EncryptedDirectMessage, encrypted)
                        .tag(tag)
                        .sign(&keys)
                        .await
                    {
                        let _ = client.send_event(ev).await;
                    }
                }
            }
            Ok(RelayPoolNotification::Shutdown) => break,
            Ok(_) => {}
            Err(e) => {
                eprintln!("Notification error: {}", e);
            }
        }
    }

    Ok(())
}
