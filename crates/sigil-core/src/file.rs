//! File sharing via NIP-94 file metadata events
//!
//! NIP-94 defines kind:1063 for file metadata.
//! Files are uploaded to a hosting service (e.g. nostr.build, void.cat)
//! and the URL + metadata is broadcast as a Nostr event.
//!
//! For DM file sharing, we embed a file reference in the encrypted message.

use serde::{Deserialize, Serialize};

/// File metadata that can be embedded in a message or published as kind:1063
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileMessage {
    /// URL where the file is hosted
    pub url: String,
    /// MIME type
    pub mime_type: String,
    /// Original filename
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filename: Option<String>,
    /// File size in bytes
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size: Option<u64>,
    /// SHA-256 hash of the file
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hash: Option<String>,
    /// Thumbnail URL for images
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thumb: Option<String>,
    /// Image dimensions
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dim: Option<String>,
}

/// A Sigil file reference embedded in DM content
/// Format: {"type":"file", ...FileMessage fields}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum FileContent {
    #[serde(rename = "file")]
    File(FileMessage),
}

impl FileMessage {
    /// Create a file message from a URL and MIME type
    pub fn new(url: &str, mime_type: &str) -> Self {
        Self {
            url: url.to_string(),
            mime_type: mime_type.to_string(),
            filename: None,
            size: None,
            hash: None,
            thumb: None,
            dim: None,
        }
    }

    /// Serialize as JSON to embed in a DM
    pub fn to_json(&self) -> String {
        let content = FileContent::File(self.clone());
        serde_json::to_string(&content).unwrap_or_default()
    }

    /// Try to parse a file message from DM content
    pub fn from_json(json: &str) -> Option<Self> {
        let content: FileContent = serde_json::from_str(json).ok()?;
        match content {
            FileContent::File(f) => Some(f),
        }
    }

    /// Check if content string is a file message
    pub fn is_file(content: &str) -> bool {
        let trimmed = content.trim();
        trimmed.contains("\"type\":\"file\"") && trimmed.starts_with('{')
    }

    /// Get a display string for the terminal
    pub fn display(&self) -> String {
        let name = self.filename.as_deref().unwrap_or("file");
        let size = self
            .size
            .map(|s| {
                if s > 1_048_576 {
                    format!(" ({:.1} MB)", s as f64 / 1_048_576.0)
                } else if s > 1024 {
                    format!(" ({:.1} KB)", s as f64 / 1024.0)
                } else {
                    format!(" ({} B)", s)
                }
            })
            .unwrap_or_default();
        format!("[{}{}] {}", name, size, self.url)
    }
}

/// Upload a file to nostr.build (free, no auth required)
/// Returns the URL of the uploaded file.
pub async fn upload_to_nostr_build(
    file_path: &std::path::Path,
) -> Result<String, Box<dyn std::error::Error>> {
    let data = tokio::fs::read(file_path).await?;
    let filename = file_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("file");

    let mime = mime_from_ext(file_path);

    let part = reqwest::multipart::Part::bytes(data)
        .file_name(filename.to_string())
        .mime_str(&mime)?;

    let form = reqwest::multipart::Form::new().part("file", part);

    let client = reqwest::Client::new();
    let resp = client
        .post("https://nostr.build/api/v2/upload/files")
        .multipart(form)
        .send()
        .await?;

    let json: serde_json::Value = resp.json().await?;

    // nostr.build returns {"status":"success","data":[{"url":"..."}]}
    let url = json
        .get("data")
        .and_then(|d| d.as_array())
        .and_then(|arr| arr.first())
        .and_then(|item| item.get("url"))
        .and_then(|u| u.as_str())
        .ok_or("Failed to get URL from nostr.build response")?;

    Ok(url.to_string())
}

fn mime_from_ext(path: &std::path::Path) -> String {
    match path.extension().and_then(|e| e.to_str()) {
        Some("png") => "image/png",
        Some("jpg") | Some("jpeg") => "image/jpeg",
        Some("gif") => "image/gif",
        Some("webp") => "image/webp",
        Some("pdf") => "application/pdf",
        Some("txt") => "text/plain",
        Some("mp4") => "video/mp4",
        Some("mp3") => "audio/mpeg",
        Some("zip") => "application/zip",
        _ => "application/octet-stream",
    }
    .to_string()
}
