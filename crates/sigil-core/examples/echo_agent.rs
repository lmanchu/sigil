//! Echo Agent — simplest possible Sigil agent
//!
//! Connects to a Nostr relay, listens for DMs, echoes back.
//! Saves keypair to ~/.sigil/echo-agent.key for persistent identity.
//!
//! Run: cargo run --example echo_agent

use sigil_core::SigilAgent;
use sigil_core::qr::AgentQrData;
use nostr_sdk::prelude::ToBech32;
use std::fs;
use std::path::PathBuf;

fn key_path() -> PathBuf {
    let dir = dirs::home_dir().unwrap().join(".sigil");
    fs::create_dir_all(&dir).ok();
    dir.join("echo-agent.key")
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let relay = std::env::var("SIGIL_RELAY")
        .unwrap_or_else(|_| "wss://relay.damus.io".to_string());

    // Load or create persistent keypair
    let key_file = key_path();
    let mut agent = if key_file.exists() {
        let secret = fs::read_to_string(&key_file).expect("read key file");
        SigilAgent::from_key("Echo Agent", secret.trim(), vec![relay.clone()])
            .expect("parse key")
    } else {
        let a = SigilAgent::new("Echo Agent", vec![relay.clone()]);
        fs::write(&key_file, a.keys.secret_key().to_bech32().unwrap()).ok();
        println!("🔑 New keypair saved to {}", key_file.display());
        a
    };

    agent.on_message(|msg, sender| {
        println!("📨 From {}: {}", sender.to_bech32().unwrap_or_default(), msg);
        Some(format!("🔁 Echo: {}", msg))
    });

    // Print onboarding info
    let qr = AgentQrData {
        npub: agent.npub(),
        relay: relay.clone(),
        name: "Echo Agent".to_string(),
        capabilities: vec!["echo".to_string()],
    };

    println!("╔══════════════════════════════════════╗");
    println!("║  Sigil Echo Agent                    ║");
    println!("╚══════════════════════════════════════╝");
    println!();
    println!("  npub:  {}", agent.npub());
    println!("  relay: {}", relay);
    println!("  QR:    {}", qr.to_uri());
    println!();
    println!("Send a DM to the npub above using Damus or any Nostr client.");
    println!("Listening for messages...");
    println!();

    if let Err(e) = agent.start().await {
        eprintln!("Agent error: {}", e);
    }
}
