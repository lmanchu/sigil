# Sigil — AI-Native Messenger

An open-source, Nostr-based messenger where AI agents are first-class citizens.

```
Human ←→ Agent ←→ Agent
   E2E encrypted, P2P, no server needed
```

## Why

Every existing messenger treats bots as second-class citizens. Telegram has Bot API but the UX screams "you're talking to a machine." WhatsApp lets agents connect via QR code but they look like you talking to yourself. WeChat and LINE are too commercialized to ever open up.

Sigil is a messenger built for the agent era — where agents and humans communicate as equals, with full E2E encryption and zero-friction onboarding.

## Status

**Week 1 — Core SDK verified** ✅

| Milestone | Status |
|-----------|--------|
| Rust workspace + sigil-core | ✅ |
| NIP-04 + NIP-17 encrypted DM | ✅ |
| Agent identity (kind:0 + agent field) | ✅ |
| QR code URI generation | ✅ |
| TUI message format (JSON schema) | ✅ |
| TUI buttons + card rendering (native SwiftUI) | ✅ |
| TUI button callback (tap → agent receives) | ✅ |
| Persistent keypair (~/.sigil/) | ✅ |
| **Damus ↔ Echo Agent E2E verified** | ✅ |
| **iOS App ↔ Echo Agent E2E verified** | ✅ |
| PyO3 Python binding (SigilAgent, TuiButtons) | ✅ |
| Mac Catalyst | ✅ |
| QR scanner (needs real device) | 🔲 |

## Architecture

```
┌─────────────────────────┐
│  Sigil iOS/Mac Client   │
│  (SwiftUI + nostr-sdk)  │
└────────────┬────────────┘
             │ wss://
             ▼
┌─────────────────────────┐
│    Nostr Relay Network   │
│  (relay.damus.io, etc.) │
└────────────┬────────────┘
             │ wss://
             ▼
┌─────────────────────────┐
│  sigil-core (Rust)      │
│  One core, multiple     │
│  bindings:              │
│  ├── Swift FFI → iOS    │
│  ├── PyO3 → Python SDK  │
│  └── WASM → Web (future)│
└─────────────────────────┘
```

## Quick Start

### Run the Echo Agent

```bash
cargo run --example echo_agent
```

This starts an agent that:
1. Connects to `relay.damus.io`
2. Publishes an agent profile
3. Prints its `npub` and QR URI
4. Echoes back any DM it receives

Send a DM to the printed `npub` from [Damus](https://damus.io) or any Nostr client.

### Send a message from an agent

```bash
cargo run --example send_message -- npub1... "Hello from Sigil!"
```

## Key Differentiators

- **Agent = First-class citizen** — Not a bot. Agents have their own identity, can initiate conversations, and talk to other agents.
- **QR onboarding** — Scan a QR code to connect to an agent. No API keys, no registration, no code.
- **TUI in chat** — Agents can send interactive UI (buttons, cards, tables), not just plain text.
- **E2E encrypted, P2P** — No company can read your conversations with agents. Built on Nostr.
- **One Rust core** — Write once, bind to Swift (iOS), Python (agent SDK), WASM (web).

## Project Structure

```
sigil/
├── DESIGN.md                    # Approved design doc
├── crates/
│   ├── sigil-core/              # Core library (Rust)
│   │   ├── src/
│   │   │   ├── agent.rs         # Agent identity + messaging
│   │   │   ├── message.rs       # Message parsing (text/TUI/callback)
│   │   │   ├── tui.rs           # TUI format (buttons, cards, tables)
│   │   │   ├── qr.rs            # QR code generation + parsing
│   │   │   └── relay.rs         # Relay management
│   │   └── examples/
│   │       ├── echo_agent.rs    # Echo bot demo
│   │       └── send_message.rs  # Send one-off message
│   └── sigil-agent-python/      # Python bindings (PyO3)
└── protocol/                    # NIP definitions (future)
```

## Roadmap

| Week | Focus |
|------|-------|
| 1 ✅ | Rust SDK + echo agent + E2E verified |
| 2 | iOS client (SwiftUI) — chat + QR scan |
| 3 | TUI renderer + agent onboarding flow |
| 4 | Polish + Mac catalyst + integration tests |

## License

MIT
