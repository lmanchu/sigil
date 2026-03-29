# Changelog

## v0.5.0 — 2026-03-29 (Day 2)

### Hermes Bridge Agent
- `cargo run --example hermes_bridge` — bridges 155+ Hermes/OpenClaw skills to Sigil
- Any Nostr user can DM the bridge and get responses from:
  - OpenClaw skills: wells (finance), pm (product), research, todo, social-content
  - Hermes builtins: github, linear, polymarket, imessage, apple-notes
  - Code execution, browser automation, and more
- TUI menu with skill categories, info table
- Auto-detects skill context (polymarket, github, linear) from message content
- Uses `hermes chat -q "..." -Q --yolo` for non-interactive execution

## v0.4.0 — 2026-03-28 (Day 1, session 2 continued)

### NIP-28 Group Chat
- `sigil channel <name>` creates a public NIP-28 channel
- `sigil join <channel_id>` joins channel with history + live messages
- Agents and humans share the same channels
- Interactive stdin loop for sending messages

### Agent Registry (kind:31990)
- Custom addressable Nostr event for structured agent profiles
- `sigil register --skills chat,calendar` publishes agent to registry
- `sigil registry` searches registry, optionally filtered by `--skill`
- Skills stored as hashtag tags for relay-level filtering
- Replaces hacking agent metadata into kind:0

### CLI Publish
- `cargo install --git https://github.com/lmanchu/sigil sigil-cli` works
- MIT license added
- Proper crates.io metadata (keywords, categories, repository)
- Both sigil-core and sigil-cli versioned at 0.4.0

### SQLite Message Persistence
- Chat history stored in ~/.sigil/messages.db
- Auto-save on send and receive
- Auto-load on startup — messages survive restarts
- `sigil whoami` shows stored message count

### Agent Discovery (kind:0)
- `sigil discover` searches relay for agents with `agent=true` metadata
- Shows name, about, capabilities, npub for each found agent
- Marks already-saved contacts with `(saved)` badge

## v0.2.0 — 2026-03-28 (Day 1, session 2)

### sigil-cli (Ratatui TUI Messenger)
- Full terminal messenger client with Ratatui
- Two-panel layout: contacts list (left) + chat (right) + input (bottom)
- Vim-style navigation: j/k to select contacts, i to type, / for commands
- First-run identity wizard: auto-generate keys, prompt for display name
- Persistent identity: ~/.sigil/user.key + ~/.sigil/user.json
- Contact book: ~/.sigil/contacts.json with agent metadata
- Add contacts via `sigil add <npub>` or `sigil add sigil://agent?...`
- In-TUI commands: /add, /whoami, /quit, /help
- Subcommands: sigil chat, sigil add, sigil whoami, sigil contacts, sigil qr
- NIP-04 encrypted messaging (send + receive)
- TUI message rendering in terminal: buttons, cards, tables
- Auto-scroll chat, unread indicators, agent badges (⚙)
- Status bar with connection info and npub

### Verified
- [x] `sigil whoami` generates and persists identity
- [x] `sigil add sigil://...` imports agent contact
- [x] `sigil contacts` lists contacts with agent badges
- [x] `sigil qr` generates shareable URI
- [x] Clean build (zero warnings)

## v0.1.0 — 2026-03-28 (Day 1)

First working prototype. Concept to functioning app in one day.

### sigil-core (Rust)
- Agent identity with persistent keypair (~/.sigil/)
- NIP-04 + NIP-17 dual protocol encrypted DM
- TUI message format: buttons, cards, tables (custom JSON schema)
- QR code generation (SVG + sigil:// URI)
- Message parsing: text / TUI / button callback
- Verified E2E with Damus (real Nostr client)

### iOS Client (SwiftUI)
- Chat UI with message bubbles (sent/received)
- TUI renderer: buttons, cards, tables as native SwiftUI
- Button tap → sigil:callback:{id} → agent receives
- Agent list with 🤖 AGENT badge
- Add agent manually or via debug button
- QR scanner (camera, needs real device)
- Mac Catalyst enabled

### sigil-agent-python (PyO3)
- SigilAgent class: create, npub, nsec, qr_uri, send, send_buttons
- TuiButtons.create() / TuiCard.create() helpers
- Install via maturin develop

### Examples
- echo_agent: connect, listen, echo + TUI responses (menu/status/info)
- send_message: one-off message to any npub
- send_tui: buttons + card + table demo
- gen_qr: QR code SVG + HTML viewer
- python_agent.py: Python SDK usage

### Verified
- [x] Damus → Echo Agent → reply (NIP-04)
- [x] iOS App → Echo Agent → reply (NIP-04)
- [x] TUI buttons render as native SwiftUI
- [x] TUI card render as native SwiftUI
- [x] Button callback: tap → agent receives button ID
- [x] QR code scannable (sigil:// URI)
- [x] Agent proactively sends message to user
