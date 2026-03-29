//! Hermes Bridge Agent — connects Hermes Agent's 155+ skills to Sigil
//!
//! Receives Nostr DMs, routes to `hermes chat -q "..." -Q --yolo`,
//! sends the response back via Sigil.
//!
//! This gives any Sigil/Damus/Nostr user access to:
//! - OpenClaw skills (wells, pm, research, social-content, etc.)
//! - Hermes builtins (github, linear, polymarket, imessage, etc.)
//! - Apple integrations (notes, reminders, Find My)
//! - Code execution, browser automation, and more
//!
//! Run: cargo run --example hermes_bridge

use sigil_core::SigilAgent;
use sigil_core::qr::AgentQrData;
use nostr_sdk::prelude::ToBech32;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

fn key_path() -> PathBuf {
    let dir = dirs::home_dir().unwrap().join(".sigil");
    fs::create_dir_all(&dir).ok();
    dir.join("hermes-bridge.key")
}

fn call_hermes(query: &str, skills: Option<&str>) -> String {
    let mut cmd = Command::new("hermes");
    cmd.arg("chat")
        .arg("-q")
        .arg(query)
        .arg("-Q")      // quiet mode — only final response
        .arg("--yolo"); // auto-approve tool calls

    if let Some(s) = skills {
        cmd.arg("-s").arg(s);
    }

    match cmd.output() {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout).to_string();
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();

            // Extract the actual response (last meaningful lines)
            let response = stdout.trim().to_string();
            if response.is_empty() {
                if !stderr.is_empty() {
                    format!("(hermes error: {})", stderr.lines().last().unwrap_or("unknown"))
                } else {
                    "(no response from hermes)".to_string()
                }
            } else {
                // Truncate very long responses for DM
                if response.len() > 2000 {
                    format!("{}...\n\n(truncated, {} chars total)", &response[..2000], response.len())
                } else {
                    response
                }
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
async fn main() {
    tracing_subscriber::fmt::init();

    let relay = std::env::var("SIGIL_RELAY")
        .unwrap_or_else(|_| "wss://relay.damus.io".to_string());

    // Load or create persistent keypair
    let key_file = key_path();
    let mut agent = if key_file.exists() {
        let secret = fs::read_to_string(&key_file).expect("read key file");
        SigilAgent::from_key("Hermes Bridge", secret.trim(), vec![relay.clone()])
            .expect("parse key")
    } else {
        let a = SigilAgent::new("Hermes Bridge", vec![relay.clone()]);
        fs::write(&key_file, a.keys.secret_key().to_bech32().unwrap()).ok();
        println!("New keypair saved to {}", key_file.display());
        a
    };

    agent.on_message(|msg, sender| {
        let sender_short = sender.to_bech32().unwrap_or_default();
        let sender_short = if sender_short.len() > 20 {
            format!("{}...", &sender_short[..20])
        } else {
            sender_short
        };

        let lower = msg.to_lowercase();

        // Handle callbacks from TUI buttons
        if msg.starts_with("sigil:callback:") {
            let button_id = msg.strip_prefix("sigil:callback:").unwrap_or("");
            if let Some(skill) = button_id.strip_prefix("skill:") {
                return Some(format!("Skill '{}' loaded. Send me a message and I'll use it.", skill));
            }
            return Some(format!("Callback: {}", button_id));
        }

        // Menu / help
        if lower == "menu" || lower == "help" || lower == "/help" {
            return Some(build_skill_menu());
        }

        // Info / status
        if lower == "info" || lower == "status" || lower == "/info" {
            return Some(build_info_table());
        }

        // Skills list
        if lower == "skills" || lower == "/skills" {
            return Some("Available skill categories: research, github, linear, polymarket, wells (finance), pm (product), todo, social-content, apple-notes, imessage, code-execution, browser, and 140+ more. Just ask naturally!".to_string());
        }

        // Route to Hermes
        println!("[{}] Query: {}", sender_short, msg);

        // Detect if user is asking for a specific skill
        let skills = if lower.contains("polymarket") || lower.contains("prediction") {
            Some("polymarket")
        } else if lower.contains("github") || lower.contains("repo") || lower.contains("pr ") {
            Some("github-issues,github-pr-workflow")
        } else if lower.contains("linear") || lower.contains("issue") || lower.contains("ticket") {
            Some("linear")
        } else {
            None
        };

        let response = call_hermes(&msg, skills);
        println!("[{}] Response: {}...", sender_short, &response[..response.len().min(80)]);

        Some(response)
    });

    // Print onboarding info
    let qr = AgentQrData {
        npub: agent.npub(),
        relay: relay.clone(),
        name: "Hermes Bridge".to_string(),
        capabilities: vec![
            "research".to_string(),
            "github".to_string(),
            "finance".to_string(),
            "tasks".to_string(),
        ],
    };

    println!("╔══════════════════════════════════════════╗");
    println!("║  Sigil × Hermes Bridge Agent             ║");
    println!("║  155+ skills via encrypted Nostr DM       ║");
    println!("╚══════════════════════════════════════════╝");
    println!();
    println!("  npub:   {}", agent.npub());
    println!("  relay:  {}", relay);
    println!("  QR:     {}", qr.to_uri());
    println!();
    println!("Commands: menu, info, skills");
    println!("Or just ask anything naturally.");
    println!("Listening...");
    println!();

    if let Err(e) = agent.start().await {
        eprintln!("Agent error: {}", e);
    }
}
