use crate::chat::{ChatEntry, ChatHistory};
use crate::identity::sigil_dir;
use rusqlite::{params, Connection};
use sigil_core::message::SigilMessage;
use std::path::PathBuf;

fn db_path() -> PathBuf {
    sigil_dir().join("messages.db")
}

pub struct Storage {
    conn: Connection,
}

impl Storage {
    pub fn open() -> Self {
        let path = db_path();
        let conn = Connection::open(&path).expect("open messages.db");
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS messages (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                peer_npub TEXT NOT NULL,
                from_me INTEGER NOT NULL,
                sender_npub TEXT NOT NULL,
                content TEXT NOT NULL,
                timestamp INTEGER NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_messages_peer ON messages(peer_npub, timestamp);",
        )
        .expect("create messages table");
        Storage { conn }
    }

    pub fn save_message(&self, peer_npub: &str, entry: &ChatEntry) {
        let content_str = entry.content.to_content();
        self.conn
            .execute(
                "INSERT INTO messages (peer_npub, from_me, sender_npub, content, timestamp)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                params![
                    peer_npub,
                    entry.from_me as i32,
                    entry.sender_npub,
                    content_str,
                    entry.timestamp as i64,
                ],
            )
            .ok();
    }

    pub fn load_history(&self) -> ChatHistory {
        let mut history = ChatHistory::default();
        let mut stmt = self
            .conn
            .prepare(
                "SELECT peer_npub, from_me, sender_npub, content, timestamp
                 FROM messages ORDER BY timestamp ASC",
            )
            .expect("prepare select");

        let rows = stmt
            .query_map([], |row| {
                let peer_npub: String = row.get(0)?;
                let from_me: i32 = row.get(1)?;
                let sender_npub: String = row.get(2)?;
                let content: String = row.get(3)?;
                let timestamp: i64 = row.get(4)?;
                Ok((peer_npub, from_me, sender_npub, content, timestamp))
            })
            .expect("query messages");

        for row in rows.flatten() {
            let (peer_npub, from_me, sender_npub, content, timestamp) = row;
            let msg = SigilMessage::parse(&content);
            history.add_message(
                &peer_npub,
                ChatEntry {
                    from_me: from_me != 0,
                    sender_npub,
                    content: msg,
                    timestamp: timestamp as u64,
                },
            );
        }
        history
    }

    /// Get count of stored messages
    pub fn message_count(&self) -> usize {
        self.conn
            .query_row("SELECT COUNT(*) FROM messages", [], |row| row.get(0))
            .unwrap_or(0)
    }
}
