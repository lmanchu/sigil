use nostr_sdk::prelude::*;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserProfile {
    pub display_name: String,
    pub is_agent: bool,
}

pub fn sigil_dir() -> PathBuf {
    let dir = dirs::home_dir().unwrap().join(".sigil");
    fs::create_dir_all(&dir).ok();
    dir
}

fn key_path() -> PathBuf {
    sigil_dir().join("user.key")
}

fn profile_path() -> PathBuf {
    sigil_dir().join("user.json")
}

/// Load or create user identity. Returns (Keys, UserProfile, is_new).
pub fn load_or_create_identity() -> (Keys, UserProfile, bool) {
    let kp = key_path();
    let pp = profile_path();

    if kp.exists() && pp.exists() {
        let secret = fs::read_to_string(&kp).expect("read user.key");
        let keys = Keys::parse(secret.trim()).expect("parse user key");
        let profile: UserProfile =
            serde_json::from_str(&fs::read_to_string(&pp).expect("read user.json"))
                .expect("parse user.json");
        (keys, profile, false)
    } else {
        let keys = Keys::generate();
        let profile = UserProfile {
            display_name: String::new(),
            is_agent: false,
        };
        // Save key immediately
        fs::write(&kp, keys.secret_key().to_bech32().unwrap()).ok();
        (keys, profile, true)
    }
}

pub fn save_profile(profile: &UserProfile) {
    let pp = profile_path();
    fs::write(&pp, serde_json::to_string_pretty(profile).unwrap()).ok();
}
