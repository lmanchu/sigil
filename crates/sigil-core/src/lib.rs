//! Sigil Core — Nostr-based AI agent messaging
//!
//! Architecture:
//! ```text
//! sigil-core (this crate)
//!   ├── agent.rs     — Agent identity, keypair, profile management
//!   ├── relay.rs     — Nostr relay connection + reconnect logic
//!   ├── message.rs   — NIP-44 encrypted DM send/receive
//!   ├── tui.rs       — TUI message format (buttons, cards, text)
//!   ├── qr.rs        — QR code generation for agent onboarding
//!   ├── channel.rs   — NIP-28 public channel (group chat)
//!   ├── registry.rs  — Agent registry (kind:31990, discovery)
//!   ├── file.rs      — File sharing (NIP-94, nostr.build upload)
//!   ├── access.rs    — Access control (personal/service agent modes)
//!   └── guard.rs     — Rate limiting, dedup, key encryption
//! ```

pub mod access;
pub mod agent;
pub mod channel;
pub mod file;
pub mod guard;
pub mod message;
pub mod qr;
pub mod registry;
pub mod relay;
pub mod tui;

pub use agent::SigilAgent;
