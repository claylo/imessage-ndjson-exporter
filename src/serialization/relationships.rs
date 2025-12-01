use serde::Serialize;
use super::chat::SerializableSender;

/// Serializable representation of message relationships (tapbacks, replies, edits)
#[derive(Debug, Serialize, Clone)]
pub struct SerializableRelationships {
    /// Thread originator GUID (if this is a reply)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thread_originator_guid: Option<String>,
    /// Which part of the original message this reply points to
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thread_originator_part: Option<String>,
    /// Number of replies to this message
    pub num_replies: i32,
    /// Tapbacks/reactions on this message
    pub tapbacks: Vec<SerializableTapback>,
    /// Edit history (if message was edited or unsent)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub edit_history: Option<EditHistory>,
}

/// Serializable representation of a tapback/reaction
#[derive(Debug, Serialize, Clone)]
pub struct SerializableTapback {
    /// Type of tapback (loved, liked, disliked, laughed, emphasized, questioned, emoji, sticker)
    pub tapback_type: String,
    /// Custom emoji (for emoji tapbacks)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub emoji: Option<String>,
    /// Who added the tapback
    pub added_by: SerializableSender,
    /// When the tapback was added
    pub timestamp: String,
    /// Which part of the message the tapback is on
    pub message_part_index: usize,
    /// Whether the user (database owner) added this tapback
    pub is_from_me: bool,
}

/// Edit history for a message
#[derive(Debug, Serialize, Clone)]
pub struct EditHistory {
    /// Status (edited or unsent)
    pub status: String,
    /// List of previous versions (for edited messages)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub versions: Option<Vec<EditVersion>>,
}

/// A single version in the edit history
#[derive(Debug, Serialize, Clone)]
pub struct EditVersion {
    /// The text of this version
    pub text: String,
    /// When this version was created
    pub timestamp: String,
    /// Components of this version
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub components: Vec<String>,
}
