//! Thin wrapper around `imessage_db::imessage::repository::MessageRepository`
//! providing the query patterns the exporter needs.

use std::collections::{BTreeSet, HashMap};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use imessage_db::imessage::entities::{Chat, Handle, Message};
use imessage_db::imessage::repository::MessageRepository;
use imessage_db::imessage::types::{
    ChatQueryParams, HandleQueryParams, MessageQueryParams, SortOrder,
};

/// Database wrapper providing high-level query methods for the exporter.
pub struct Database {
    repo: MessageRepository,
    path: PathBuf,
}

impl Database {
    /// Open a read-only connection to the iMessage database.
    pub fn open(path: &Path) -> Result<Self> {
        let repo = MessageRepository::open(path.to_path_buf())
            .context("Failed to open iMessage database")?;
        Ok(Self {
            repo,
            path: path.to_path_buf(),
        })
    }

    /// Load all chats into a HashMap keyed by rowid.
    pub fn load_chats(&self) -> Result<HashMap<i64, Chat>> {
        let mut map = HashMap::new();
        let (batch, _total) = self.repo.get_chats(&ChatQueryParams {
            with_participants: true,
            with_archived: true,
            offset: 0,
            limit: None,
            ..ChatQueryParams::default()
        })?;

        for chat in batch {
            map.insert(chat.rowid, chat);
        }
        Ok(map)
    }

    /// Load all handles into a HashMap keyed by rowid.
    pub fn load_handles(&self) -> Result<HashMap<i64, Handle>> {
        let mut map = HashMap::new();
        let mut offset = 0i64;
        loop {
            let (batch, total) = self.repo.get_handles(&HandleQueryParams {
                offset,
                limit: 10000,
                ..HandleQueryParams::default()
            })?;

            if batch.is_empty() {
                break;
            }

            let batch_len = batch.len() as i64;
            for handle in batch {
                map.insert(handle.rowid, handle);
            }

            offset += batch_len;
            if offset >= total {
                break;
            }
        }
        Ok(map)
    }

    /// Stream messages for a specific chat, in ascending date order.
    ///
    /// `after` and `before` are Unix milliseconds (already converted from user input).
    pub fn messages_for_chat(
        &self,
        chat_guid: &str,
        after: Option<i64>,
        before: Option<i64>,
    ) -> Result<Vec<Message>> {
        let mut messages = Vec::new();
        let mut offset = 0i64;
        let page_size = 500i64;

        loop {
            let (batch, _total) = self.repo.get_messages(&MessageQueryParams {
                chat_guid: Some(chat_guid.to_string()),
                offset,
                limit: page_size,
                after,
                before,
                with_attachments: true,
                with_chats: false,
                with_chat_participants: false,
                sort: SortOrder::Asc,
                order_by: "message.date".to_string(),
                ..MessageQueryParams::default()
            })?;

            if batch.is_empty() {
                break;
            }

            let batch_len = batch.len() as i64;
            messages.extend(batch);
            offset += batch_len;

            if batch_len < page_size {
                break;
            }
        }

        Ok(messages)
    }

    /// Load all tapback messages into a TapbackResolver-compatible structure.
    ///
    /// Returns a map of associated_message_guid -> Vec<Message>.
    pub fn load_tapbacks(&self) -> Result<HashMap<String, Vec<Message>>> {
        let mut tapback_map: HashMap<String, Vec<Message>> = HashMap::new();
        let mut offset = 0i64;
        let page_size = 1000i64;

        loop {
            let (batch, _total) = self.repo.get_messages(&MessageQueryParams {
                offset,
                limit: page_size,
                with_attachments: false,
                with_chats: false,
                sort: SortOrder::Asc,
                order_by: "message.date".to_string(),
                ..MessageQueryParams::default()
            })?;

            if batch.is_empty() {
                break;
            }

            let batch_len = batch.len() as i64;
            for msg in batch {
                // Only include actual tapback types (love, like, etc.)
                // Types 2/3 are normal/app messages that happen to have
                // associated_message_type set, not tapbacks.
                if let Some(ref reaction) = msg.associated_message_type
                    && crate::exporter::is_tapback_type(reaction)
                    && let Some(clean_guid) = msg
                        .associated_message_guid
                        .as_deref()
                        .and_then(parse_target_guid)
                {
                    tapback_map
                        .entry(clean_guid.to_string())
                        .or_default()
                        .push(msg);
                }
            }

            offset += batch_len;
            if batch_len < page_size {
                break;
            }
        }

        Ok(tapback_map)
    }

    /// Count replies per message GUID via a single GROUP BY query.
    pub fn load_reply_counts(&self) -> Result<HashMap<String, i64>> {
        use rusqlite::Connection;

        let conn =
            Connection::open_with_flags(&self.path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
                .context("Failed to open DB for reply counts")?;

        let mut stmt = conn.prepare(
            "SELECT thread_originator_guid, COUNT(*) \
             FROM message \
             WHERE thread_originator_guid IS NOT NULL \
             GROUP BY thread_originator_guid",
        )?;

        let mut counts = HashMap::new();
        let mut rows = stmt.query([])?;
        while let Some(row) = rows.next()? {
            let guid: String = row.get(0)?;
            let count: i64 = row.get(1)?;
            counts.insert(guid, count);
        }

        Ok(counts)
    }

    /// Get participants for a specific chat by querying the chat with participants loaded.
    pub fn chat_participants(&self, chat_rowid: i64, chats: &HashMap<i64, Chat>) -> Vec<Handle> {
        if let Some(chat) = chats.get(&chat_rowid) {
            chat.participants.clone()
        } else {
            Vec::new()
        }
    }

    /// Get a raw rusqlite connection for direct SQL queries (contacts filter, etc.)
    ///
    /// This exposes the underlying connection for cases where we need
    /// direct SQL access (e.g., querying chat_handle_join for conversation filtering).
    pub fn connection(&self) -> &MessageRepository {
        &self.repo
    }

    /// Build chatroom participants map: chat_id -> set of handle rowids.
    ///
    /// Uses the participants already loaded on each Chat.
    pub fn build_chatroom_participants(
        &self,
        chats: &HashMap<i64, Chat>,
    ) -> HashMap<i64, BTreeSet<i64>> {
        let mut map: HashMap<i64, BTreeSet<i64>> = HashMap::new();
        for (chat_id, chat) in chats {
            let handle_ids: BTreeSet<i64> = chat.participants.iter().map(|h| h.rowid).collect();
            if !handle_ids.is_empty() {
                map.insert(*chat_id, handle_ids);
            }
        }
        map
    }
}

/// Extract the clean 36-char target message GUID from an associated_message_guid value.
///
/// The raw value can be in several formats:
/// - `"p:N/GUID"` — tapback on part N of the message
/// - `"bp:GUID"` — tapback on the whole message (bubble)
/// - `"GUID"` — bare GUID
pub(crate) fn parse_target_guid(raw: &str) -> Option<&str> {
    if let Some(rest) = raw.strip_prefix("p:") {
        // "p:0/F445FB06-..." -> skip past the slash
        let guid = rest.split_once('/')?.1;
        guid.get(..36)
    } else if let Some(rest) = raw.strip_prefix("bp:") {
        rest.get(..36)
    } else {
        raw.get(..36)
    }
}

/// Extract the part index from an associated_message_guid value.
///
/// Returns the component index the tapback targets (0 for single-component messages).
pub(crate) fn parse_tapback_part_index(raw: &str) -> usize {
    if let Some(rest) = raw.strip_prefix("p:") {
        rest.split_once('/')
            .and_then(|(idx, _)| idx.parse().ok())
            .unwrap_or(0)
    } else {
        0
    }
}
