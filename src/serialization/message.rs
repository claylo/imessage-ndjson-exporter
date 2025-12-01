use serde::Serialize;
use super::{
    chat::{SerializableChatContext, SerializableSender},
    content::{ExpressiveEffect, SerializableContent},
    relationships::SerializableRelationships,
};

/// Main serializable message structure
///
/// This represents a complete message with all metadata, content, and relationships.
/// Each message is self-contained and includes full chat context.
#[derive(Debug, Serialize, Clone)]
pub struct SerializableMessage {
    /// Type of message (normal, edited, tapback, app, announcement, etc.)
    pub message_type: String,

    /// Core message metadata
    pub metadata: MessageMetadata,

    /// Sender information
    pub sender: SerializableSender,

    /// Chat context (includes participants, display name, etc.)
    pub chat_context: SerializableChatContext,

    /// Message content (text, attachments, apps)
    pub content: SerializableContent,

    /// Relationships (tapbacks, replies, edit history)
    pub relationships: SerializableRelationships,

    /// Expressive send style (screen/bubble effects)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expressive_effect: Option<ExpressiveEffect>,

    /// Announcement metadata (for announcement-type messages)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub announcement: Option<SerializableAnnouncement>,
}

/// Core message metadata
#[derive(Debug, Serialize, Clone)]
pub struct MessageMetadata {
    /// Row ID in database
    pub rowid: i32,
    /// Globally unique identifier
    pub guid: String,
    /// When the message was sent
    pub date: String,
    /// When the message was read (if read)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub date_read: Option<String>,
    /// When the message was delivered (if delivered)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub date_delivered: Option<String>,
    /// When the message was last edited (if edited)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub date_edited: Option<String>,
    /// Service type (iMessage, SMS, RCS, Satellite)
    pub service: String,
    /// Whether the database owner sent this message
    pub is_from_me: bool,
    /// Whether the message was read
    pub is_read: bool,
    /// Chat ID this message belongs to
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chat_id: Option<i32>,
    /// Whether this message was deleted from a chat
    pub is_deleted: bool,
}

/// Group action types (subset of announcement types)
#[derive(Debug, Serialize, Clone)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SerializableGroupAction {
    /// New participant was added to the group
    ParticipantAdded {
        participant_handle_id: i32,
    },
    /// Participant was removed from the group
    ParticipantRemoved {
        participant_handle_id: i32,
    },
    /// Group name was changed
    NameChange {
        new_name: String,
    },
    /// Participant left the group
    ParticipantLeft,
    /// Group icon was changed
    GroupIconChanged,
    /// Group icon was removed
    GroupIconRemoved,
    /// Chat background was changed
    ChatBackgroundChanged,
    /// Chat background was removed
    ChatBackgroundRemoved,
}

/// Announcement message types
#[derive(Debug, Serialize, Clone)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SerializableAnnouncement {
    /// All parts of the message were unsent
    FullyUnsent,
    /// Group-related action occurred
    GroupAction(SerializableGroupAction),
    /// User kept an audio message
    AudioMessageKept,
}
