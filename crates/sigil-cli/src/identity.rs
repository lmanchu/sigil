use nostr_sdk::prelude::*;
use serde::{Deserialize, Serialize};
use sigil_core::guard;
use std::fs;
use std::io;
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
        let raw = fs::read_to_string(&kp).expect("read user.key");
        let secret = if guard::is_encrypted(raw.trim()) {
            // Prompt for passphrase
            let pass = prompt_passphrase("Enter key passphrase: ");
            guard::decrypt_key(raw.trim(), &pass).expect("wrong passphrase or corrupted key")
        } else {
            raw.trim().to_string()
        };
        let keys = Keys::parse(&secret).expect("parse user key");
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
        // Save key — plaintext for now, user can encrypt with `sigil encrypt-key`
        fs::write(&kp, keys.secret_key().to_bech32().unwrap()).ok();
        (keys, profile, true)
    }
}

/// Encrypt the user's key file with a passphrase
pub fn encrypt_key_file() {
    let kp = key_path();
    let raw = fs::read_to_string(&kp).expect("read user.key");

    if guard::is_encrypted(raw.trim()) {
        println!("Key is already encrypted.");
        return;
    }

    let pass1 = prompt_passphrase("Set passphrase: ");
    let pass2 = prompt_passphrase("Confirm passphrase: ");

    if pass1 != pass2 {
        eprintln!("Passphrases don't match.");
        return;
    }

    if pass1.is_empty() {
        eprintln!("Passphrase cannot be empty.");
        return;
    }

    let encrypted = guard::encrypt_key(raw.trim(), &pass1);
    fs::write(&kp, &encrypted).expect("write encrypted key");
    println!("Key encrypted. You'll need the passphrase to start sigil.");
}

/// Decrypt the user's key file (remove encryption)
pub fn decrypt_key_file() {
    let kp = key_path();
    let raw = fs::read_to_string(&kp).expect("read user.key");

    if !guard::is_encrypted(raw.trim()) {
        println!("Key is not encrypted.");
        return;
    }

    let pass = prompt_passphrase("Enter passphrase: ");
    match guard::decrypt_key(raw.trim(), &pass) {
        Some(nsec) => {
            fs::write(&kp, &nsec).expect("write decrypted key");
            println!("Key decrypted.");
        }
        None => {
            eprintln!("Wrong passphrase or corrupted key.");
        }
    }
}

fn prompt_passphrase(prompt: &str) -> String {
    eprint!("{}", prompt);
    io::Write::flush(&mut io::stderr()).ok();
    let mut input = String::new();
    io::stdin().read_line(&mut input).unwrap_or(0);
    input.trim().to_string()
}

pub fn save_profile(profile: &UserProfile) {
    let pp = profile_path();
    fs::write(&pp, serde_json::to_string_pretty(profile).unwrap()).ok();
}
