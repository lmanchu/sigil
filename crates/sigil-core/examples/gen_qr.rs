//! Generate a QR code SVG for agent onboarding
//! Usage: cargo run --example gen_qr
//! Output: ~/.sigil/echo-agent-qr.svg + ~/.sigil/echo-agent-qr.html

use sigil_core::qr::AgentQrData;
use std::fs;

fn main() {
    let key_data =
        fs::read_to_string(dirs::home_dir().unwrap().join(".sigil/echo-agent.key")).unwrap();
    let keys = nostr_sdk::Keys::parse(key_data.trim()).unwrap();
    let npub = keys.public_key().to_bech32().unwrap();

    let qr = AgentQrData {
        npub: npub.clone(),
        relay: "wss://relay.damus.io".to_string(),
        name: "Echo Agent".to_string(),
        capabilities: vec!["echo".to_string(), "chat".to_string()],
    };

    let uri = qr.to_uri();
    let svg = qr.to_qr_svg().expect("QR generation failed");

    let dir = dirs::home_dir().unwrap().join(".sigil");
    fs::create_dir_all(&dir).ok();

    // Save SVG
    let svg_path = dir.join("echo-agent-qr.svg");
    fs::write(&svg_path, &svg).unwrap();

    // Save HTML wrapper for easy viewing
    let html = format!(
        r#"<!DOCTYPE html>
<html><head><title>Sigil Agent QR</title>
<style>
body {{ font-family: -apple-system, sans-serif; display: flex; flex-direction: column;
       align-items: center; justify-content: center; min-height: 100vh; margin: 0;
       background: #0a0a0a; color: #e0e0e0; }}
.card {{ background: #1a1a2e; border-radius: 16px; padding: 40px; text-align: center;
         box-shadow: 0 4px 24px rgba(0,0,0,0.5); }}
h1 {{ font-size: 24px; margin-bottom: 8px; }}
.npub {{ font-family: monospace; font-size: 11px; color: #888; word-break: break-all;
         max-width: 300px; margin: 12px auto; }}
.uri {{ font-family: monospace; font-size: 10px; color: #666; margin-top: 16px;
        word-break: break-all; max-width: 300px; }}
svg {{ margin: 20px 0; }}
</style></head>
<body>
<div class="card">
  <h1>🔮 Echo Agent</h1>
  <p>Scan to connect via Sigil</p>
  {}
  <div class="npub">{}</div>
  <div class="uri">{}</div>
</div>
</body></html>"#,
        svg, npub, uri
    );

    let html_path = dir.join("echo-agent-qr.html");
    fs::write(&html_path, html).unwrap();

    println!("✅ QR code generated:");
    println!("   SVG:  {}", svg_path.display());
    println!("   HTML: {}", html_path.display());
    println!("   URI:  {}", uri);
    println!();
    println!("Open the HTML to scan with your phone camera.");
}

use nostr_sdk::prelude::ToBech32;
