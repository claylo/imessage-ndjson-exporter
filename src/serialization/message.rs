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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json;

    /// Helper to create a minimal SerializableMessage for testing
    fn create_test_message(message_type: &str, service: &str) -> SerializableMessage {
        SerializableMessage {
            message_type: message_type.to_string(),
            metadata: MessageMetadata {
                rowid: 1,
                guid: "test-guid".to_string(),
                date: "2024-01-01T00:00:00Z".to_string(),
                date_read: None,
                date_delivered: None,
                date_edited: None,
                service: service.to_string(),
                is_from_me: false,
                is_read: false,
                chat_id: Some(1),
                is_deleted: false,
            },
            sender: SerializableSender {
                handle_id: Some(1),
                identifier: "test@example.com".to_string(),
                contact_name: None,
            },
            chat_context: SerializableChatContext {
                chat_id: Some(1),
                chat_identifier: "test_chat".to_string(),
                display_name: None,
                service_name: service.to_string(),
                participants: vec![],
            },
            content: SerializableContent {
                text: None,
                subject: None,
                components: vec![],
            },
            relationships: SerializableRelationships {
                tapbacks: vec![],
                edit_history: None,
                thread_originator_guid: None,
                thread_originator_part: None,
                num_replies: 0,
            },
            expressive_effect: None,
            announcement: None,
        }
    }

    #[test]
    fn test_message_metadata_serialization() {
        let metadata = MessageMetadata {
            rowid: 123,
            guid: "ABCD-1234-EFGH-5678".to_string(),
            date: "2024-01-15T10:30:45Z".to_string(),
            date_read: Some("2024-01-15T10:31:00Z".to_string()),
            date_delivered: Some("2024-01-15T10:30:50Z".to_string()),
            date_edited: None,
            service: "iMessage".to_string(),
            is_from_me: true,
            is_read: true,
            chat_id: Some(42),
            is_deleted: false,
        };

        let json = serde_json::to_value(&metadata).unwrap();

        assert_eq!(json["rowid"], 123);
        assert_eq!(json["guid"], "ABCD-1234-EFGH-5678");
        assert_eq!(json["service"], "iMessage");
        assert_eq!(json["is_from_me"], true);
        assert_eq!(json["is_read"], true);
        assert_eq!(json["is_deleted"], false);
    }

    #[test]
    fn test_service_type_imessage() {
        let msg = create_test_message("normal", "iMessage");
        assert_eq!(msg.metadata.service, "iMessage");

        let json = serde_json::to_value(&msg).unwrap();
        assert_eq!(json["metadata"]["service"], "iMessage");
    }

    #[test]
    fn test_service_type_sms() {
        let msg = create_test_message("normal", "SMS");
        assert_eq!(msg.metadata.service, "SMS");

        let json = serde_json::to_value(&msg).unwrap();
        assert_eq!(json["metadata"]["service"], "SMS");
    }

    #[test]
    fn test_service_type_rcs() {
        let msg = create_test_message("normal", "RCS");
        assert_eq!(msg.metadata.service, "RCS");
    }

    #[test]
    fn test_service_type_satellite() {
        let msg = create_test_message("normal", "Satellite");
        assert_eq!(msg.metadata.service, "Satellite");
    }

    #[test]
    fn test_announcement_group_action_serialization() {
        let action = SerializableGroupAction::ParticipantAdded {
            participant_handle_id: 42,
        };

        let json = serde_json::to_value(&action).unwrap();
        assert_eq!(json["type"], "participant_added");
        assert_eq!(json["participant_handle_id"], 42);
    }

    #[test]
    fn test_announcement_name_change() {
        let action = SerializableGroupAction::NameChange {
            new_name: "New Group Name".to_string(),
        };

        let json = serde_json::to_value(&action).unwrap();
        assert_eq!(json["type"], "name_change");
        assert_eq!(json["new_name"], "New Group Name");
    }

    #[test]
    fn test_edited_message_type() {
        let msg = create_test_message("edited", "iMessage");
        assert_eq!(msg.message_type, "edited");

        let json = serde_json::to_value(&msg).unwrap();
        assert_eq!(json["message_type"], "edited");
    }

    #[test]
    fn test_deleted_message_flag() {
        let mut msg = create_test_message("normal", "iMessage");
        msg.metadata.is_deleted = true;

        assert!(msg.metadata.is_deleted);

        let json = serde_json::to_value(&msg).unwrap();
        assert_eq!(json["metadata"]["is_deleted"], true);
    }

    #[test]
    fn test_announcement_fully_unsent() {
        let announcement = SerializableAnnouncement::FullyUnsent;

        let json = serde_json::to_value(&announcement).unwrap();
        assert_eq!(json["type"], "fully_unsent");
    }

    #[test]
    fn test_announcement_audio_kept() {
        let announcement = SerializableAnnouncement::AudioMessageKept;

        let json = serde_json::to_value(&announcement).unwrap();
        assert_eq!(json["type"], "audio_message_kept");
    }
}
