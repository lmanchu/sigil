mod chat;
mod contacts;
mod discovery;
mod identity;
mod storage;
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
use storage::Storage;
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
    /// Discover agents on the relay network
    Discover {
        /// Relay URL
        #[arg(short, long, default_value = "wss://relay.damus.io")]
        relay: String,
        /// Max agents to find
        #[arg(short, long, default_value = "20")]
        limit: usize,
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
        Some(Commands::Discover { relay, limit }) => cmd_discover(&relay, limit).await,
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
    let db = Storage::open();
    println!("  Msgs:  {} stored", db.message_count());
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

async fn cmd_discover(relay: &str, limit: usize) -> Result<(), Box<dyn std::error::Error>> {
    println!("Searching for agents on {}...", relay);

    let keys = Keys::generate(); // ephemeral keys for discovery
    let client = Client::new(keys);
    client.add_relay(relay).await?;
    client.connect().await;
    tokio::time::sleep(Duration::from_secs(2)).await;

    let agents = discovery::discover_agents(&client, Duration::from_secs(10), limit).await;

    if agents.is_empty() {
        println!("No agents found. (Agents need agent=true in their kind:0 metadata)");
    } else {
        println!("Found {} agent(s):\n", agents.len());
        let book = ContactBook::load();
        for (i, agent) in agents.iter().enumerate() {
            let already = book.contacts.iter().any(|c| c.npub == agent.npub);
            let marker = if already { " (saved)" } else { "" };
            println!("  {}. ⚙ {}{}", i + 1, agent.name, marker);
            if let Some(about) = &agent.about {
                let short = if about.len() > 60 { format!("{}...", &about[..60]) } else { about.clone() };
                println!("     {}", short);
            }
            let short_npub = if agent.npub.len() > 30 {
                format!("{}...", &agent.npub[..30])
            } else {
                agent.npub.clone()
            };
            println!("     npub: {}", short_npub);
            if !agent.capabilities.is_empty() {
                println!("     skills: {}", agent.capabilities.join(", "));
            }
            println!();
        }
        println!("Add an agent: sigil add <npub> --name <name>");
    }

    client.disconnect().await;
    Ok(())
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

    // Open storage and load history
    let db = Storage::open();
    let history = db.load_history();
    let msg_count = db.message_count();

    // Start Nostr client
    let relays = vec![relay.clone()];
    let (out_tx, mut ev_rx, _client) = chat::start_nostr(keys.clone(), relays).await?;

    // Setup terminal
    enable_raw_mode()?;
    io::stdout().execute(EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(io::stdout());
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new(contacts, my_npub.clone());
    app.history = history;
    app.status_line = format!(
        "Connected as {} | {} msgs | relay: {}",
        profile.display_name, msg_count, relay
    );

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
                                    if let Ok(pk) = PublicKey::from_bech32(&peer_npub) {
                                        let msg = SigilMessage::parse(&content);
                                        let entry = ChatEntry {
                                            from_me: true,
                                            sender_npub: my_npub.clone(),
                                            content: msg,
                                            timestamp: now_ts(),
                                        };
                                        db.save_message(&peer_npub, &entry);
                                        app.history.add_message(&peer_npub, entry);
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
                    let entry = ChatEntry {
                        from_me: false,
                        sender_npub: sender_npub.clone(),
                        content: msg,
                        timestamp: now_ts(),
                    };
                    db.save_message(&sender_npub, &entry);
                    app.history.add_message(&sender_npub, entry);
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
    println!("Goodbye. ({} messages saved)", db.message_count());
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
        Some("/discover") => {
            app.status_line = "Use CLI: sigil discover (discovery requires relay query)".to_string();
        }
        Some("/quit") | Some("/q") => {
            app.should_quit = true;
        }
        Some("/help") => {
            app.status_line =
                "/add <addr> [name] | /discover | /whoami | /quit | j/k=nav | i=type | q=quit"
                    .to_string();
        }
        _ => {
            app.status_line = format!("Unknown command: {}", cmd);
        }
    }
}

fn now_ts() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs()
}
