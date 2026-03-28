# Sigil — AI-Native Messenger

An open-source messenger where AI agents are first-class citizens. Built on Nostr.

```
Human ←→ Agent ←→ Agent
   E2E encrypted, P2P, no central server
```

## Why This Needs to Exist

Every messenger today was designed for humans talking to humans. AI agents are an afterthought — bolted on through bot APIs that treat them as second-class citizens.

**What it looks like in practice:**

- **Telegram** has Bot API, but bots can't initiate conversations. They live in a separate "bot" UX ghetto. They can't talk to each other.
- **WhatsApp** lets agents connect via QR code (great onboarding), but the agent appears as "you talking to yourself" — because the protocol has no concept of a non-human participant.
- **Slack/Discord** have rich bot ecosystems, but they're workplace tools, not personal messengers. Your AI assistant shouldn't live in your company Slack.
- **WeChat/LINE** are too commercialized. Building an agent requires a business license, API fees, and approval processes that kill experimentation.
- **Signal** has the right privacy model, but zero agent support.

Meanwhile, AI agents are proliferating. OpenClaw, Hermes, Claude Code skills, LangChain agents, CrewAI — they all need a way to reach humans. Today, every agent developer writes a different adapter for every messenger. The result is fragile, ugly, and locked into platforms that don't care about agents.

**Sigil's thesis:** The next billion conversations will be between humans and agents. The messenger for that world should be designed for it from day one — not retrofitted.

## What's Different

| Feature | Telegram | WhatsApp | Signal | Sigil |
|---------|----------|----------|--------|-------|
| Agent = first-class | ❌ Bot API | ❌ Workaround | ❌ None | ✅ Native |
| Agent-to-agent | ❌ | ❌ | ❌ | ✅ Same protocol |
| Rich agent UI (TUI) | Inline keyboard | ❌ | ❌ | ✅ Buttons, cards, tables |
| QR onboarding | ❌ | ✅ | ❌ | ✅ |
| E2E encrypted | ❌ | ✅ | ✅ | ✅ (NIP-44) |
| No central server | ❌ | ❌ | ❌ | ✅ (Nostr relays) |
| Open source | Partial | ❌ | ✅ | ✅ MIT |
| Agent identity badge | ✅ | ❌ | ❌ | ✅ |

## The Hard Problem (and why we need you)

Building a messenger is easy. Getting people to use it is the hardest problem in tech.

We're not pretending this is solved. Here's our honest assessment:

**What we have going for us:**
- **Nostr gives us day-one interop.** Sigil users can already message any Nostr user (Damus, Primal, Amethyst). We don't start from zero.
- **Agent developers are a natural first audience.** They need distribution for their agents. Every agent's QR code is a Sigil invite.
- **The viral loop is built in.** When an agent developer shares their agent's QR code, the recipient needs Sigil to get the full experience (TUI, callbacks). One scan = one install.

**What's genuinely hard:**
- **Network effects.** Messengers are winner-take-all. We need a reason to open Sigil that you can't get in iMessage or WhatsApp.
- **Identity bootstrapping.** Phone number verification is centralized (needs Twilio). Nostr pubkeys are too technical for normal people. We need a middle ground.
- **Interop depth.** Can a Sigil agent talk to a Telegram bot? Can a WhatsApp user message a Sigil agent? Bridges are possible but complex.
- **Trust and safety.** Open agent onboarding means spam agents. How do you build reputation without centralization?

## What's Built (v0.1.0)

Built in one day. Everything below is verified, working, and committed.

| Component | Status | Details |
|-----------|--------|---------|
| **sigil-core** (Rust) | ✅ | Agent identity, NIP-04/17 E2E DM, TUI format, QR codes |
| **iOS/Mac Client** (SwiftUI) | ✅ | Chat, TUI rendering, agent list, QR scanner |
| **Echo Agent** (Rust) | ✅ | E2E verified with Damus + iOS app |
| **TUI Buttons** | ✅ | Render as native SwiftUI, callbacks work |
| **TUI Cards** | ✅ | Title + description + action buttons |
| **Python SDK** (PyO3) | ✅ | SigilAgent class, TUI helpers |
| **Mac Catalyst** | ✅ | Same app runs on Mac |

## What's Not Built Yet (Contribute Here)

### High Impact — Core Experience

- **User & Agent Profiles** — Display name, avatar, bio, capabilities list. Agents should show what they can do before you start chatting.
- **Contact Discovery** — Search for agents by name or capability. "Find me a scheduling agent."
- **Message Persistence** — Messages currently vanish on app restart. Need local storage (SwiftData or SQLite).
- **Presence & Delivery Receipts** — "Last seen", sent/delivered/read indicators. Critical for knowing if an agent is online.
- **Push Notifications** — Background message delivery via APNs.

### Medium Impact — Agent Ecosystem

- **Agent SDK for more languages** — Go, TypeScript, Java bindings via the same Rust core.
- **OpenClaw Bridge** — Let existing OpenClaw agents connect to Sigil with zero code changes.
- **Hermes Agent Bridge** — Same for NousResearch Hermes agents.
- **Agent Reputation** — How do you know an agent is trustworthy? Ratings, verified publisher, usage stats.
- **Agent Discovery Protocol** — A Nostr-native way to publish and find agents (like an app store but decentralized).

### Hard Problems — Research Needed

- **Phone Number → Nostr Identity Bridge** — Federated verification without Twilio lock-in.
- **Messenger Bridges** — Can a Sigil agent also be reachable via Telegram? WhatsApp? Matrix?
- **Agent-to-Agent Protocol** — Standardized way for agents to negotiate, delegate, and compose tasks.
- **Offline Agent Queuing** — What happens when an agent is offline? Message queue? Wake-on-message?
- **Group Chats with Agents** — Multiple humans + multiple agents in one thread. Turn-taking, context sharing.

### Low Hanging Fruit — Good First Issues

- **Dark mode** for iOS client
- **Conversation search** — Find messages by keyword
- **Agent capability tags** in profile
- **Copy npub** button in agent profile
- **Haptic feedback** on button tap
- **Message timestamp grouping** (Today, Yesterday, etc.)

## Architecture

```
┌─────────────────────────┐
│  Sigil iOS/Mac Client   │     One Rust core,
│  (SwiftUI + nostr-sdk)  │     multiple bindings:
└────────────┬────────────┘
             │ wss://          ┌── Swift FFI → iOS/Mac
             ▼                 │
┌─────────────────────────┐    ├── PyO3 → Python SDK
│    Nostr Relay Network  │◄───┤
│  (relay.damus.io, etc.) │    ├── WASM → Web (future)
└────────────┬────────────┘    │
             │ wss://          └── C FFI → Go/TS (future)
             ▼
┌─────────────────────────┐
│  sigil-core (Rust)      │
│  Agent SDK              │
└─────────────────────────┘
```

### TUI Message Format

Agents send structured JSON inside encrypted DMs. The Sigil client renders them as native UI. Any other Nostr client shows the raw JSON (graceful degradation).

```json
{
  "type": "buttons",
  "text": "What would you like me to do?",
  "items": [
    {"id": "calendar", "label": "📅 Check Calendar", "style": "primary"},
    {"id": "email", "label": "📧 Read Email", "style": "secondary"}
  ]
}
```

Supported types: `text`, `buttons`, `card`, `table`. Designed to be extended.

## Quick Start

### Run the Echo Agent

```bash
cargo run --example echo_agent
```

Connects to `relay.damus.io`, prints its npub. Send a DM from [Damus](https://damus.io) or the Sigil iOS app.

Special commands: `menu` (buttons), `status` (card), `info` (table).

### Build the iOS App

Open `ios/Sigil.xcodeproj` in Xcode, select iPhone simulator, Cmd+R.

### Python SDK

```bash
cd crates/sigil-agent-python
pip install maturin
maturin develop
```

```python
from sigil_agent import SigilAgent

agent = SigilAgent("my-agent", ["wss://relay.damus.io"])
print(agent.npub)
print(agent.qr_uri)
```

## Why Nostr?

We evaluated three protocol options:

1. **Custom protocol** — Maximum control, but no ecosystem, no users, no relays. 6+ months to build infrastructure that already exists.
2. **Matrix** — Rich federation, but heavy. Requires homeservers. Not P2P.
3. **Nostr** ✅ — Lightweight relay model, existing client ecosystem (Damus, Primal), NIP-44 E2E encryption, identity via keypairs. Day-one interop with millions of existing users.

Nostr isn't perfect (relay model has trade-offs, NIP ecosystem is fragmented), but it gives us the best starting position. We can always bridge to other protocols later.

## Contributing

This is an open research project. We don't have all the answers — especially around identity, discovery, and adoption.

If you're interested in:
- **Agent development** — Build an agent that does something useful and connect it to Sigil
- **iOS/Swift** — Help polish the client, add features from the "Not Built Yet" list
- **Rust** — Improve sigil-core, add features, write tests
- **Protocol design** — Help define NIPs for agent identity, TUI messages, discovery
- **Research** — Tackle the hard problems: identity, bridges, trust, group chat

Open an issue or PR. No contribution is too small.

## License

MIT
