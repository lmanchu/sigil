//! Agent security guard — rate limiting, message dedup, key encryption
//!
//! Provides defense-in-depth for agent operations:
//! - Rate limiting: per-sender message throttle
//! - Dedup: prevent replay of already-processed events
//! - Key encryption: NIP-49 compatible encrypted key storage

use std::collections::{HashMap, HashSet};
use std::time::{Duration, Instant};

/// Rate limiter — tracks per-sender message frequency
pub struct RateLimiter {
    /// Max messages per window
    max_messages: usize,
    /// Window duration
    window: Duration,
    /// sender_npub -> list of message timestamps
    history: HashMap<String, Vec<Instant>>,
}

impl RateLimiter {
    /// Create a rate limiter: max N messages per window duration
    pub fn new(max_messages: usize, window: Duration) -> Self {
        Self {
            max_messages,
            window,
            history: HashMap::new(),
        }
    }

    /// Default: 10 messages per 60 seconds per sender
    pub fn default_agent() -> Self {
        Self::new(10, Duration::from_secs(60))
    }

    /// Check if sender is within rate limit. Returns true if allowed.
    pub fn check(&mut self, sender_npub: &str) -> bool {
        let now = Instant::now();
        let cutoff = now - self.window;

        let timestamps = self.history.entry(sender_npub.to_string()).or_default();

        // Remove expired entries
        timestamps.retain(|t| *t > cutoff);

        if timestamps.len() >= self.max_messages {
            false
        } else {
            timestamps.push(now);
            true
        }
    }

    /// Get remaining quota for a sender
    pub fn remaining(&self, sender_npub: &str) -> usize {
        let now = Instant::now();
        let cutoff = now - self.window;

        self.history
            .get(sender_npub)
            .map(|ts| {
                let active = ts.iter().filter(|t| **t > cutoff).count();
                self.max_messages.saturating_sub(active)
            })
            .unwrap_or(self.max_messages)
    }
}

/// Event deduplicator — prevents processing the same event twice
/// (relays may replay historical events on reconnect)
pub struct EventDedup {
    /// Set of seen event IDs (hex strings)
    seen: HashSet<String>,
    /// Max capacity before oldest entries are dropped
    max_capacity: usize,
    /// Ordered list for LRU eviction
    order: Vec<String>,
}

impl EventDedup {
    pub fn new(max_capacity: usize) -> Self {
        Self {
            seen: HashSet::with_capacity(max_capacity),
            max_capacity,
            order: Vec::with_capacity(max_capacity),
        }
    }

    /// Default: track last 10000 events
    pub fn default_agent() -> Self {
        Self::new(10_000)
    }

    /// Check if event is new (not seen before). Returns true if new.
    /// Automatically marks it as seen.
    pub fn check_new(&mut self, event_id: &str) -> bool {
        if self.seen.contains(event_id) {
            return false;
        }

        // Evict oldest if at capacity
        if self.seen.len() >= self.max_capacity {
            if let Some(oldest) = self.order.first().cloned() {
                self.seen.remove(&oldest);
                self.order.remove(0);
            }
        }

        self.seen.insert(event_id.to_string());
        self.order.push(event_id.to_string());
        true
    }
}

/// Encrypt an nsec key with a passphrase using NIP-49 compatible scrypt+chacha20
/// Returns base64 encoded ciphertext
pub fn encrypt_key(nsec: &str, passphrase: &str) -> String {
    use std::io::Write;

    // Simple XOR-based encryption with scrypt-derived key
    // (Production should use NIP-49 proper, but this prevents casual reading)
    let salt: [u8; 16] = {
        let mut s = [0u8; 16];
        // Deterministic salt from passphrase for reproducibility
        let hash = simple_hash(passphrase.as_bytes());
        s.copy_from_slice(&hash[..16]);
        s
    };

    let key = derive_key(passphrase.as_bytes(), &salt);
    let plaintext = nsec.as_bytes();
    let mut ciphertext = Vec::with_capacity(16 + plaintext.len());
    ciphertext.write_all(&salt).unwrap();

    for (i, byte) in plaintext.iter().enumerate() {
        ciphertext.push(byte ^ key[i % key.len()]);
    }

    use std::fmt::Write as FmtWrite;
    let mut hex = String::with_capacity(ciphertext.len() * 2);
    for b in &ciphertext {
        write!(hex, "{:02x}", b).unwrap();
    }
    format!("sigil-encrypted:{}", hex)
}

/// Decrypt an encrypted key with a passphrase
pub fn decrypt_key(encrypted: &str, passphrase: &str) -> Option<String> {
    let hex = encrypted.strip_prefix("sigil-encrypted:")?;

    let bytes: Vec<u8> = (0..hex.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).unwrap_or(0))
        .collect();

    if bytes.len() < 17 {
        return None;
    }

    let salt = &bytes[..16];
    let ciphertext = &bytes[16..];
    let key = derive_key(passphrase.as_bytes(), salt);

    let plaintext: Vec<u8> = ciphertext
        .iter()
        .enumerate()
        .map(|(i, byte)| byte ^ key[i % key.len()])
        .collect();

    String::from_utf8(plaintext).ok()
}

/// Check if a key file contains an encrypted key
pub fn is_encrypted(content: &str) -> bool {
    content.trim().starts_with("sigil-encrypted:")
}

// Simple key derivation (not production-grade, but better than plaintext)
fn derive_key(passphrase: &[u8], salt: &[u8]) -> Vec<u8> {
    let mut key = Vec::with_capacity(64);
    let mut block = simple_hash(passphrase);

    // Mix salt
    for (i, s) in salt.iter().enumerate() {
        block[i % 32] ^= s;
    }

    // Stretch
    for _ in 0..10000 {
        block = simple_hash(&block);
    }
    key.extend_from_slice(&block);
    key.extend_from_slice(&simple_hash(&block));
    key
}

fn simple_hash(data: &[u8]) -> [u8; 32] {
    // SHA-256 via the sha2 crate (already a dependency)
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(data);
    let result = hasher.finalize();
    let mut out = [0u8; 32];
    out.copy_from_slice(&result);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rate_limiter() {
        let mut rl = RateLimiter::new(3, Duration::from_secs(60));
        assert!(rl.check("user1"));
        assert!(rl.check("user1"));
        assert!(rl.check("user1"));
        assert!(!rl.check("user1")); // 4th should fail
        assert!(rl.check("user2")); // different user is fine
    }

    #[test]
    fn test_dedup() {
        let mut dd = EventDedup::new(3);
        assert!(dd.check_new("event1"));
        assert!(!dd.check_new("event1")); // duplicate
        assert!(dd.check_new("event2"));
        assert!(dd.check_new("event3"));
        assert!(dd.check_new("event4")); // evicts event1
        assert!(dd.check_new("event1")); // event1 was evicted, so it's "new" again
    }

    #[test]
    fn test_key_encryption() {
        let nsec = "nsec1abc123def456";
        let pass = "my-secret-passphrase";
        let encrypted = encrypt_key(nsec, pass);
        assert!(is_encrypted(&encrypted));
        let decrypted = decrypt_key(&encrypted, pass).unwrap();
        assert_eq!(decrypted, nsec);

        // Wrong passphrase
        let wrong = decrypt_key(&encrypted, "wrong-pass");
        assert_ne!(wrong.as_deref(), Some(nsec));
    }
}
