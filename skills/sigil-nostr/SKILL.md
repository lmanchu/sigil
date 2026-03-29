---
name: sigil-nostr
description: Nostr P2P messaging gateway for AI agents. Send and receive E2E encrypted messages via the Nostr protocol. Enables your agent to be reachable from Sigil Messenger, Damus, and any Nostr client.
metadata: {"clawdbot": {"emoji": "⚡", "user-invocable": true}, "category": "messaging", "author": "lmanchu", "version": "0.1.0", "repository": "https://github.com/lmanchu/sigil"}
---

# Sigil Nostr — P2P Encrypted Messaging for AI Agents

Give your AI agent a Nostr identity. Users can message your agent from Sigil Messenger, Damus, Primal, or any Nostr client — all E2E encrypted.

## What This Does

- Creates a persistent Nostr keypair for your agent (~/.sigil/<agent-name>.key)
- Connects to Nostr relays and listens for NIP-04 encrypted DMs
- Routes incoming messages to your agent, sends replies back via Nostr
- Supports TUI components (buttons, cards, tables) for rich responses
- Personal agent mode: only whitelisted npubs can interact

## Quick Start

### 1. Install Sigil CLI

```bash
cargo install --git https://github.com/lmanchu/sigil sigil-cli
```

### 2. Start as Agent Bridge

The simplest way — pipe messages between Nostr and your agent:

```bash
# Generate agent identity
sigil whoami

# Your agent's npub is shown — users can DM this to reach your agent
# Add your npub to the access whitelist:
# Edit ~/.sigil/<agent>.access.json → add npub to "authorized" array
```

### 3. Use from OpenClaw / Hermes

This skill works with both OpenClaw and Hermes agent frameworks.

**OpenClaw**: Add to your openclaw.json channels config:
```json
{
  "channels": {
    "nostr": {
      "enabled": true,
      "relay": "wss://relay.damus.io",
      "dmPolicy": "personal",
      "allowFrom": ["npub1yourownpubkey..."]
    }
  }
}
```

**Hermes**: Install as a skill:
```bash
# Copy this skill directory to ~/.hermes/skills/sigil-nostr/
cp -r skills/sigil-nostr ~/.hermes/skills/
```

## Architecture

```
User (Sigil/Damus/Primal)
    ↓ NIP-04 Encrypted DM
Nostr Relay (relay.damus.io)
    ↓ WebSocket
Sigil Bridge (this skill)
    ↓ stdin/stdout or HTTP
Your Agent (OpenClaw / Hermes / Custom)
```

## Agent Bridge Script

For standalone use, run the bridge directly:

```bash
# Start the Nostr bridge for your agent
cd /path/to/sigil
cargo run --example hermes_bridge
```

Or use the Node.js bridge for OpenClaw integration:

```bash
node skills/sigil-nostr/bridge.js
```

## Security

- **Personal mode** (default): only owner + authorized npubs can interact
- **Service mode**: open to anyone (like a LINE official account)
- **Rate limiting**: 10 messages/minute per sender
- **Event dedup**: prevents relay replay attacks
- **Key encryption**: `sigil encrypt-key` for passphrase protection

## Configuration

Config file: `~/.sigil/<agent-name>.access.json`

```json
{
  "mode": "personal",
  "owner": "npub1...",
  "authorized": ["npub1friend1...", "npub1friend2..."],
  "reject_message": "This is a personal agent. Contact the owner for access."
}
```

## TUI Messages

Your agent can send rich interactive messages:

```json
{"type": "buttons", "text": "What do you need?", "items": [
  {"id": "search", "label": "Search", "style": "primary"},
  {"id": "help", "label": "Help", "style": "secondary"}
]}
```

Supported types: `text`, `buttons`, `card`, `table`

When a user taps a button, your agent receives: `sigil:callback:<button_id>`

## Links

- [Sigil GitHub](https://github.com/lmanchu/sigil)
- [Landing Page](https://lmanchu.github.io/sigil/)
- [Sigil Messenger on App Store](https://apps.apple.com/app/sigil-messenger/id6761321150) (pending review)
- [Agent Registry (kind:31990)](https://github.com/lmanchu/sigil#agent-registry-kind31990)
