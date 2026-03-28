mod chat;
mod contacts;
mod identity;
mod ui;

use chat::{ChatEntry, ChatEvent};
use clap::{Parser, Subcommand};
use contacts::ContactBook;
use crossterm::event::{self, Event, KeyCode, KeyModifiers};
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use crossterm::ExecutableCommand;
use nostr_sdk::prelude::*;
use ratatui::prelude::*;
use sigil_core::message::SigilMessage;
use sigil_core::qr::AgentQrData;
use std::io;
use std::time::Duration;
use ui::{App, InputMode};

#[derive(Parser)]
#[command(name = "sigil", about = "Sigil — AI-Native Terminal Messenger")]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Start the TUI messenger (default)
    Chat {
        /// Relay URL
        #[arg(short, long, default_value = "wss://relay.damus.io")]
        relay: String,
    },
    /// Add a contact by npub or sigil:// URI
    Add {
        /// npub1... or sigil://agent?...
        address: String,
        /// Display name (required for npub, optional for URI)
        #[arg(short, long)]
        name: Option<String>,
    },
    /// Show your identity info
    Whoami,
    /// List contacts
    Contacts,
    /// Generate QR code URI for your identity
    Qr {
        /// Relay URL
        #[arg(short, long, default_value = "wss://relay.damus.io")]
        relay: String,
    },
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    match cli.command {
        Some(Commands::Add { address, name }) => {
            cmd_add(&address, name.as_deref());
            Ok(())
        }
        Some(Commands::Whoami) => {
            cmd_whoami();
            Ok(())
        }
        Some(Commands::Contacts) => {
            cmd_contacts();
            Ok(())
        }
        Some(Commands::Qr { relay }) => {
            cmd_qr(&relay);
            Ok(())
        }
        Some(Commands::Chat { relay }) => run_tui(relay).await,
        None => {
            let relay = std::env::var("SIGIL_RELAY")
                .unwrap_or_else(|_| "wss://relay.damus.io".to_string());
            run_tui(relay).await
        }
    }
}

fn cmd_add(address: &str, name: Option<&str>) {
    let mut book = ContactBook::load();

    if address.starts_with("sigil://") {
        match book.add_from_uri(address) {
            Some(contact) => {
                println!("Added agent: {} ({})", contact.name, &contact.npub[..20]);
                if !contact.capabilities.is_empty() {
                    println!("  Capabilities: {}", contact.capabilities.join(", "));
                }
            }
            None => println!("Contact already exists or invalid URI."),
        }
    } else if address.starts_with("npub") {
        let display = name.unwrap_or("Unknown");
        if book.add_npub(address, display) {
            println!("Added: {} ({}...)", display, &address[..20]);
        } else {
            println!("Contact already exists.");
        }
    } else {
        eprintln!("Invalid address. Use npub1... or sigil://agent?...");
        std::process::exit(1);
    }
}

fn cmd_whoami() {
    let (keys, profile, is_new) = identity::load_or_create_identity();
    if is_new {
        println!("New identity created!");
        println!();
    }
    let npub = keys.public_key().to_bech32().unwrap_or_default();
    println!("  Name:  {}", if profile.display_name.is_empty() { "(not set)" } else { &profile.display_name });
    println!("  npub:  {}", npub);
    println!("  Key:   ~/.sigil/user.key");
}

fn cmd_contacts() {
    let book = ContactBook::load();
    if book.contacts.is_empty() {
        println!("No contacts. Add one with: sigil add <npub> --name <name>");
        return;
    }
    for c in &book.contacts {
        let badge = if c.is_agent { "⚙" } else { " " };
        let short_npub = if c.npub.len() > 20 {
            format!("{}...", &c.npub[..20])
        } else {
            c.npub.clone()
        };
        println!("  {} {} ({})", badge, c.name, short_npub);
    }
}

fn cmd_qr(relay: &str) {
    let (keys, profile, _) = identity::load_or_create_identity();
    let data = AgentQrData {
        npub: keys.public_key().to_bech32().unwrap_or_default(),
        relay: relay.to_string(),
        name: if profile.display_name.is_empty() {
            "Sigil User".to_string()
        } else {
            profile.display_name
        },
        capabilities: vec![],
    };
    println!("{}", data.to_uri());
}

async fn run_tui(relay: String) -> Result<(), Box<dyn std::error::Error>> {
    // Load or create identity
    let (keys, profile, is_new) = identity::load_or_create_identity();

    if is_new || profile.display_name.is_empty() {
        // First-run setup (outside TUI)
        println!("╔══════════════════════════════════════╗");
        println!("║     Welcome to Sigil Messenger       ║");
        println!("╚══════════════════════════════════════╝");
        println!();
        println!("  Your npub: {}", keys.public_key().to_bech32().unwrap_or_default());
        println!();
        print!("  Enter your display name: ");
        io::Write::flush(&mut io::stdout())?;
        let mut name = String::new();
        io::stdin().read_line(&mut name)?;
        let name = name.trim().to_string();
        let name = if name.is_empty() {
            "Sigil User".to_string()
        } else {
            name
        };
        let profile = identity::UserProfile {
            display_name: name.clone(),
            is_agent: false,
        };
        identity::save_profile(&profile);
        println!("  Saved! Starting messenger as '{}'...", name);
        println!();
    }

    let (_, profile, _) = identity::load_or_create_identity();
    let contacts = ContactBook::load();
    let my_npub = keys.public_key().to_bech32().unwrap_or_default();

    // Start Nostr client
    let relays = vec![relay.clone()];
    let (out_tx, mut ev_rx, _client) = chat::start_nostr(keys.clone(), relays).await?;

    // Setup terminal
    enable_raw_mode()?;
    io::stdout().execute(EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(io::stdout());
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new(contacts, my_npub.clone());
    app.status_line = format!("Connected as {} | relay: {}", profile.display_name, relay);

    // Main event loop
    loop {
        terminal.draw(|f| ui::draw(f, &app))?;

        // Poll for terminal events with timeout to also check Nostr events
        if crossterm::event::poll(Duration::from_millis(50))? {
            if let Event::Key(key) = event::read()? {
                match app.input_mode {
                    InputMode::Normal => match key.code {
                        KeyCode::Char('q') => {
                            app.should_quit = true;
                        }
                        KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                            app.should_quit = true;
                        }
                        KeyCode::Char('i') | KeyCode::Enter => {
                            app.input_mode = InputMode::Editing;
                        }
                        KeyCode::Char('/') => {
                            app.input_mode = InputMode::Command;
                            app.input = "/".to_string();
                        }
                        KeyCode::Up | KeyCode::Char('k') => {
                            if app.selected_contact > 0 {
                                app.selected_contact -= 1;
                            }
                        }
                        KeyCode::Down | KeyCode::Char('j') => {
                            let max = app.peer_list().len().saturating_sub(1);
                            if app.selected_contact < max {
                                app.selected_contact += 1;
                            }
                        }
                        _ => {}
                    },
                    InputMode::Editing => match key.code {
                        KeyCode::Enter => {
                            if !app.input.is_empty() {
                                if let Some(peer_npub) = app.selected_peer_npub() {
                                    let content = app.input.clone();
                                    // Parse recipient public key
                                    if let Ok(pk) = PublicKey::from_bech32(&peer_npub) {
                                        let msg = SigilMessage::parse(&content);
                                        app.history.add_message(
                                            &peer_npub,
                                            ChatEntry {
                                                from_me: true,
                                                sender_npub: my_npub.clone(),
                                                content: msg,
                                                timestamp: std::time::SystemTime::now()
                                                    .duration_since(std::time::UNIX_EPOCH)
                                                    .unwrap()
                                                    .as_secs(),
                                            },
                                        );
                                        let _ = out_tx.send((pk, content)).await;
                                    }
                                    app.input.clear();
                                }
                            }
                        }
                        KeyCode::Esc => {
                            app.input.clear();
                            app.input_mode = InputMode::Normal;
                        }
                        KeyCode::Backspace => {
                            app.input.pop();
                        }
                        KeyCode::Char(c) => {
                            app.input.push(c);
                        }
                        _ => {}
                    },
                    InputMode::Command => match key.code {
                        KeyCode::Enter => {
                            handle_command(&app.input.clone(), &mut app);
                            app.input.clear();
                            app.input_mode = InputMode::Normal;
                        }
                        KeyCode::Esc => {
                            app.input.clear();
                            app.input_mode = InputMode::Normal;
                        }
                        KeyCode::Backspace => {
                            app.input.pop();
                            if app.input.is_empty() {
                                app.input_mode = InputMode::Normal;
                            }
                        }
                        KeyCode::Char(c) => {
                            app.input.push(c);
                        }
                        _ => {}
                    },
                }
            }
        }

        // Check for Nostr events
        while let Ok(ev) = ev_rx.try_recv() {
            match ev {
                ChatEvent::MessageReceived {
                    sender_npub,
                    content,
                } => {
                    let msg = SigilMessage::parse(&content);
                    app.history.add_message(
                        &sender_npub,
                        ChatEntry {
                            from_me: false,
                            sender_npub: sender_npub.clone(),
                            content: msg,
                            timestamp: std::time::SystemTime::now()
                                .duration_since(std::time::UNIX_EPOCH)
                                .unwrap()
                                .as_secs(),
                        },
                    );
                    // Auto-select sender if no contact selected or empty list
                    if app.peer_list().len() == 1 {
                        app.selected_contact = 0;
                    }
                }
                ChatEvent::Connected => {
                    app.status_line =
                        format!("Connected as {} | relay: {}", profile.display_name, relay);
                }
                ChatEvent::RelayError(e) => {
                    app.status_line = format!("Relay error: {}", e);
                }
            }
        }

        if app.should_quit {
            break;
        }
    }

    // Cleanup
    disable_raw_mode()?;
    io::stdout().execute(LeaveAlternateScreen)?;
    println!("Goodbye.");
    Ok(())
}

fn handle_command(cmd: &str, app: &mut App) {
    let parts: Vec<&str> = cmd.trim().splitn(3, ' ').collect();
    match parts.first().map(|s| *s) {
        Some("/add") => {
            if parts.len() >= 2 {
                let address = parts[1];
                let name = parts.get(2).copied();
                if address.starts_with("sigil://") {
                    match app.contacts.add_from_uri(address) {
                        Some(c) => {
                            app.status_line = format!("Added agent: {}", c.name);
                        }
                        None => {
                            app.status_line = "Already exists or invalid URI".to_string();
                        }
                    }
                } else if address.starts_with("npub") {
                    let display = name.unwrap_or("Unknown");
                    if app.contacts.add_npub(address, display) {
                        app.status_line = format!("Added: {}", display);
                    } else {
                        app.status_line = "Already exists.".to_string();
                    }
                } else {
                    app.status_line = "Usage: /add <npub|sigil://...> [name]".to_string();
                }
            } else {
                app.status_line = "Usage: /add <npub|sigil://...> [name]".to_string();
            }
        }
        Some("/whoami") => {
            app.status_line = format!("npub: {}", app.my_npub);
        }
        Some("/quit") | Some("/q") => {
            app.should_quit = true;
        }
        Some("/help") => {
            app.status_line =
                "/add <addr> [name] | /whoami | /quit | j/k=nav | i=type | q=quit".to_string();
        }
        _ => {
            app.status_line = format!("Unknown command: {}", cmd);
        }
    }
}
