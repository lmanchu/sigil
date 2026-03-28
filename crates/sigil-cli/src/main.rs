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
use sigil_core::channel::{self, ChannelInfo};
use sigil_core::message::SigilMessage;
use sigil_core::qr::AgentQrData;
use sigil_core::registry::{self, AgentRegistryEntry};
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
    /// Create a new group channel
    Channel {
        /// Channel name
        name: String,
        /// Channel description
        #[arg(short, long)]
        about: Option<String>,
        /// Relay URL
        #[arg(short, long, default_value = "wss://relay.damus.io")]
        relay: String,
    },
    /// Join a channel and show messages
    Join {
        /// Channel event ID (hex or note1...)
        channel_id: String,
        /// Relay URL
        #[arg(short, long, default_value = "wss://relay.damus.io")]
        relay: String,
    },
    /// Register this agent in the Sigil agent registry
    Register {
        /// Agent skills (comma-separated)
        #[arg(short, long)]
        skills: Option<String>,
        /// Relay URL
        #[arg(short, long, default_value = "wss://relay.damus.io")]
        relay: String,
    },
    /// Search the agent registry
    Registry {
        /// Filter by skill
        #[arg(short, long)]
        skill: Option<String>,
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
        Some(Commands::Discover { relay, limit }) => cmd_discover(&relay, limit).await,
        Some(Commands::Channel { name, about, relay }) => cmd_channel(&name, about.as_deref(), &relay).await,
        Some(Commands::Join { channel_id, relay }) => cmd_join(&channel_id, &relay).await,
        Some(Commands::Register { skills, relay }) => cmd_register(skills.as_deref(), &relay).await,
        Some(Commands::Registry { skill, relay }) => cmd_registry(skill.as_deref(), &relay).await,
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

async fn cmd_channel(name: &str, about: Option<&str>, relay: &str) -> Result<(), Box<dyn std::error::Error>> {
    let (keys, _, _) = identity::load_or_create_identity();
    let client = Client::new(keys.clone());
    client.add_relay(relay).await?;
    client.connect().await;
    tokio::time::sleep(Duration::from_secs(2)).await;

    let info = ChannelInfo {
        name: name.to_string(),
        about: about.map(|s| s.to_string()),
        picture: None,
    };

    let channel_id = channel::create_channel(&client, &keys, &info).await?;
    println!("Channel created!");
    println!("  Name: {}", name);
    println!("  ID:   {}", channel_id.to_hex());
    println!();
    println!("Join:  sigil join {}", channel_id.to_hex());
    println!("Share: sigil join {} --relay {}", channel_id.to_hex(), relay);

    client.disconnect().await;
    Ok(())
}

async fn cmd_join(channel_id_str: &str, relay: &str) -> Result<(), Box<dyn std::error::Error>> {
    let (keys, profile, _) = identity::load_or_create_identity();
    let channel_id = EventId::from_hex(channel_id_str)?;

    let client = Client::new(keys.clone());
    client.add_relay(relay).await?;
    client.connect().await;
    tokio::time::sleep(Duration::from_secs(2)).await;

    // Fetch channel info
    if let Some(info) = channel::fetch_channel_info(&client, channel_id).await? {
        println!("Channel: {}", info.name);
        if let Some(about) = &info.about {
            println!("  {}", about);
        }
    } else {
        println!("Channel: {}", &channel_id_str[..16]);
    }
    println!();

    // Fetch history
    let messages = channel::fetch_channel_messages(&client, channel_id, 50).await?;
    let contacts = ContactBook::load();
    for msg in &messages {
        let sender_npub = msg.sender.to_bech32().unwrap_or_default();
        let name = contacts.display_name(&sender_npub);
        println!("  {}: {}", name, msg.content);
    }
    if messages.is_empty() {
        println!("  (no messages yet)");
    }
    println!();

    // Subscribe and interactive loop
    let filter = channel::channel_filter(channel_id);
    client.subscribe(filter, None).await?;

    println!("Type a message and press Enter. Ctrl+C to exit.");
    println!();

    let mut notifications = client.notifications();
    let display_name = profile.display_name.clone();

    // Spawn listener
    let keys_listen = keys.clone();
    let _contacts = contacts;
    tokio::spawn(async move {
        loop {
            match notifications.recv().await {
                Ok(RelayPoolNotification::Event { event, .. }) => {
                    if event.kind == Kind::ChannelMessage && event.pubkey != keys_listen.public_key() {
                        let sender = event.pubkey.to_bech32().unwrap_or_default();
                        let short = if sender.len() > 16 { &sender[..16] } else { &sender };
                        println!("  {}: {}", short, event.content);
                    }
                }
                Ok(RelayPoolNotification::Shutdown) => break,
                _ => {}
            }
        }
    });

    // Read stdin for sending
    let stdin = io::BufRead::lines(io::BufReader::new(io::stdin()));
    for line in stdin {
        let line = line?;
        if line.is_empty() {
            continue;
        }
        channel::send_channel_message(&client, &keys, channel_id, &line, Some(relay)).await?;
        println!("  {}: {}", display_name, line);
    }

    client.disconnect().await;
    Ok(())
}

async fn cmd_register(skills: Option<&str>, relay: &str) -> Result<(), Box<dyn std::error::Error>> {
    let (keys, profile, _) = identity::load_or_create_identity();
    let client = Client::new(keys.clone());
    client.add_relay(relay).await?;
    client.connect().await;
    tokio::time::sleep(Duration::from_secs(2)).await;

    let skill_list: Vec<String> = skills
        .unwrap_or("")
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();

    let entry = AgentRegistryEntry {
        name: profile.display_name.clone(),
        about: None,
        picture: None,
        framework: Some("sigil".to_string()),
        skills: skill_list.clone(),
        tui: true,
        relay: Some(relay.to_string()),
        version: Some("0.3.0".to_string()),
    };

    let event_id = registry::publish_agent(&client, &keys, &entry).await?;
    println!("Registered in agent registry!");
    println!("  Name:   {}", profile.display_name);
    println!("  Skills: {}", if skill_list.is_empty() { "(none)".to_string() } else { skill_list.join(", ") });
    println!("  Event:  {}", event_id.to_hex());

    client.disconnect().await;
    Ok(())
}

async fn cmd_registry(skill: Option<&str>, relay: &str) -> Result<(), Box<dyn std::error::Error>> {
    println!("Searching agent registry on {}...", relay);

    let keys = Keys::generate();
    let client = Client::new(keys);
    client.add_relay(relay).await?;
    client.connect().await;
    tokio::time::sleep(Duration::from_secs(2)).await;

    let agents = registry::search_agents(&client, skill, 50).await?;

    if agents.is_empty() {
        println!("No agents found in registry.");
        if skill.is_some() {
            println!("Try without --skill filter, or agents need to `sigil register`.");
        }
    } else {
        println!("Found {} agent(s):\n", agents.len());
        let contacts = ContactBook::load();
        for (i, (pk, entry)) in agents.iter().enumerate() {
            let npub = pk.to_bech32().unwrap_or_default();
            let saved = contacts.find(&npub).is_some();
            let marker = if saved { " (saved)" } else { "" };
            println!("  {}. {} {}{}", i + 1, entry.name,
                entry.framework.as_deref().unwrap_or(""), marker);
            if let Some(about) = &entry.about {
                println!("     {}", about);
            }
            if !entry.skills.is_empty() {
                println!("     skills: {}", entry.skills.join(", "));
            }
            let short = if npub.len() > 30 { format!("{}...", &npub[..30]) } else { npub };
            println!("     npub: {}", short);
            println!();
        }
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

    // Load saved channels
    let channels = load_channels();
    let channel_ids: Vec<String> = channels.iter().map(|c| c.id.clone()).collect();

    // Start Nostr client
    let relays = vec![relay.clone()];
    let (out_tx, mut ev_rx, _client) =
        chat::start_nostr(keys.clone(), relays, channel_ids).await?;

    // Setup terminal
    enable_raw_mode()?;
    io::stdout().execute(EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(io::stdout());
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new(contacts, my_npub.clone());
    app.history = history;
    app.channels = channels;
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
                                if let Some(peer_key) = app.selected_peer_key() {
                                    let content = app.input.clone();
                                    let msg = SigilMessage::parse(&content);
                                    let entry = ChatEntry {
                                        from_me: true,
                                        sender_npub: my_npub.clone(),
                                        content: msg,
                                        timestamp: now_ts(),
                                    };
                                    db.save_message(&peer_key, &entry);
                                    app.history.add_message(&peer_key, entry);

                                    if let Some(ch_id) = peer_key.strip_prefix("ch:") {
                                        // Channel message
                                        let _ = out_tx
                                            .send(chat::OutgoingMessage::Channel(
                                                ch_id.to_string(),
                                                content,
                                            ))
                                            .await;
                                    } else if let Ok(pk) = PublicKey::from_bech32(&peer_key) {
                                        // DM
                                        let _ = out_tx
                                            .send(chat::OutgoingMessage::Dm(pk, content))
                                            .await;
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
                ChatEvent::ChannelMessage {
                    channel_id,
                    sender_npub,
                    content,
                } => {
                    let key = format!("ch:{}", channel_id);
                    let msg = SigilMessage::parse(&content);
                    let is_me = sender_npub == my_npub;
                    let entry = ChatEntry {
                        from_me: is_me,
                        sender_npub: sender_npub.clone(),
                        content: msg,
                        timestamp: now_ts(),
                    };
                    db.save_message(&key, &entry);
                    app.history.add_message(&key, entry);
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
        Some("/join") => {
            if parts.len() >= 2 {
                let ch_id = parts[1];
                let name = parts.get(2).copied().unwrap_or("Channel");
                let ch = ui::JoinedChannel {
                    id: ch_id.to_string(),
                    name: name.to_string(),
                };
                if !app.channels.iter().any(|c| c.id == ch_id) {
                    app.channels.push(ch);
                    save_channels(&app.channels);
                    app.status_line = format!("Joined #{}", name);
                } else {
                    app.status_line = "Already joined.".to_string();
                }
            } else {
                app.status_line = "Usage: /join <channel_id> [name]".to_string();
            }
        }
        Some("/discover") => {
            app.status_line = "Use CLI: sigil discover (discovery requires relay query)".to_string();
        }
        Some("/quit") | Some("/q") => {
            app.should_quit = true;
        }
        Some("/help") => {
            app.status_line =
                "/add | /join <id> [name] | /whoami | /quit | j/k | i | q"
                    .to_string();
        }
        _ => {
            app.status_line = format!("Unknown command: {}", cmd);
        }
    }
}

fn load_channels() -> Vec<ui::JoinedChannel> {
    let path = identity::sigil_dir().join("channels.json");
    if path.exists() {
        let data = std::fs::read_to_string(&path).unwrap_or_default();
        #[derive(serde::Deserialize)]
        struct Ch {
            id: String,
            name: String,
        }
        let channels: Vec<Ch> = serde_json::from_str(&data).unwrap_or_default();
        channels
            .into_iter()
            .map(|c| ui::JoinedChannel {
                id: c.id,
                name: c.name,
            })
            .collect()
    } else {
        vec![]
    }
}

fn save_channels(channels: &[ui::JoinedChannel]) {
    let path = identity::sigil_dir().join("channels.json");
    #[derive(serde::Serialize)]
    struct Ch<'a> {
        id: &'a str,
        name: &'a str,
    }
    let data: Vec<Ch> = channels.iter().map(|c| Ch { id: &c.id, name: &c.name }).collect();
    std::fs::write(&path, serde_json::to_string_pretty(&data).unwrap()).ok();
}

fn now_ts() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs()
}
