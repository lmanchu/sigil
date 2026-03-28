# Changelog

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
