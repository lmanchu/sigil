/// Relay configuration and management
/// Currently delegates to nostr-sdk Client's built-in relay handling.
/// This module exists for future custom relay selection logic
/// (e.g., agent-specific relays, relay scoring, fallback chains).

// Relay-related types re-exported from nostr-sdk for convenience
pub use nostr_sdk::RelayPoolNotification;
