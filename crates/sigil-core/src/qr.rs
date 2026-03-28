use qrcode::QrCode;
use serde::{Deserialize, Serialize};

/// Agent onboarding info encoded in QR code
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentQrData {
    /// Agent's Nostr public key (npub)
    pub npub: String,
    /// Preferred relay URL
    pub relay: String,
    /// Agent display name
    pub name: String,
    /// Agent capabilities summary
    pub capabilities: Vec<String>,
}

impl AgentQrData {
    /// Encode as sigil:// URI for QR code
    pub fn to_uri(&self) -> String {
        format!(
            "sigil://agent?npub={}&relay={}&name={}",
            self.npub,
            urlencoding_relay(&self.relay),
            urlencoding_name(&self.name),
        )
    }

    /// Parse from sigil:// URI
    pub fn from_uri(uri: &str) -> Option<Self> {
        let uri = uri.strip_prefix("sigil://agent?")?;
        let mut npub = None;
        let mut relay = None;
        let mut name = None;

        for pair in uri.split('&') {
            let mut parts = pair.splitn(2, '=');
            let key = parts.next()?;
            let value = parts.next()?;
            match key {
                "npub" => npub = Some(value.to_string()),
                "relay" => relay = Some(urldecoding(value)),
                "name" => name = Some(urldecoding(value)),
                _ => {}
            }
        }

        Some(Self {
            npub: npub?,
            relay: relay?,
            name: name?,
            capabilities: vec![],
        })
    }

    /// Generate QR code as SVG string
    pub fn to_qr_svg(&self) -> Result<String, qrcode::types::QrError> {
        let code = QrCode::new(self.to_uri())?;
        let svg = code
            .render::<qrcode::render::svg::Color>()
            .min_dimensions(256, 256)
            .build();
        Ok(svg)
    }
}

fn urlencoding_relay(s: &str) -> String {
    s.replace("://", "%3A%2F%2F").replace('/', "%2F")
}

fn urlencoding_name(s: &str) -> String {
    s.replace(' ', "%20")
}

fn urldecoding(s: &str) -> String {
    s.replace("%3A%2F%2F", "://")
        .replace("%2F", "/")
        .replace("%20", " ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_qr_roundtrip() {
        let data = AgentQrData {
            npub: "npub1abc123".to_string(),
            relay: "wss://relay.damus.io".to_string(),
            name: "Test Agent".to_string(),
            capabilities: vec!["chat".to_string()],
        };

        let uri = data.to_uri();
        assert!(uri.starts_with("sigil://agent?"));

        let parsed = AgentQrData::from_uri(&uri).unwrap();
        assert_eq!(parsed.npub, "npub1abc123");
        assert_eq!(parsed.relay, "wss://relay.damus.io");
        assert_eq!(parsed.name, "Test Agent");
    }
}
