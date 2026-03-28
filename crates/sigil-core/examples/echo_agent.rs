//! Echo Agent — simplest possible Sigil agent
//!
//! Connects to a Nostr relay, listens for DMs, echoes back.
//! Prints QR code URI for onboarding.
//!
//! Run: cargo run --example echo_agent

use sigil_core::SigilAgent;
use sigil_core::qr::AgentQrData;
use nostr_sdk::prelude::ToBech32;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let relay = std::env::var("SIGIL_RELAY")
        .unwrap_or_else(|_| "wss://relay.damus.io".to_string());

    let mut agent = SigilAgent::new("Echo Agent", vec![relay.clone()]);

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
    println!("Listening for messages...");
    println!();

    if let Err(e) = agent.start().await {
        eprintln!("Agent error: {}", e);
    }
}
