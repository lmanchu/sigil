//! Sigil Core — Nostr-based AI agent messaging
//!
//! Architecture:
//! ```text
//! sigil-core (this crate)
//!   ├── agent.rs    — Agent identity, keypair, profile management
//!   ├── relay.rs    — Nostr relay connection + reconnect logic
//!   ├── message.rs  — NIP-44 encrypted DM send/receive
//!   ├── tui.rs      — TUI message format (buttons, cards, text)
//!   └── qr.rs       — QR code generation for agent onboarding
//! ```

pub mod agent;
pub mod message;
pub mod qr;
pub mod relay;
pub mod tui;

pub use agent::SigilAgent;
