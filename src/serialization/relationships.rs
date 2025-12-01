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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json;

    #[test]
    fn test_tapback_serialization() {
        let tapback = SerializableTapback {
            tapback_type: "loved".to_string(),
            emoji: None,
            added_by: SerializableSender {
                handle_id: Some(1),
                identifier: "user@example.com".to_string(),
                contact_name: Some("Test User".to_string()),
            },
            timestamp: "2024-01-01T12:00:00Z".to_string(),
            message_part_index: 0,
            is_from_me: false,
        };

        let json = serde_json::to_value(&tapback).unwrap();
        assert_eq!(json["tapback_type"], "loved");
        assert_eq!(json["timestamp"], "2024-01-01T12:00:00Z");
        assert_eq!(json["message_part_index"], 0);
        assert_eq!(json["is_from_me"], false);
        assert_eq!(json["added_by"]["identifier"], "user@example.com");
    }

    #[test]
    fn test_edit_history_structure() {
        let edit_history = EditHistory {
            status: "edited".to_string(),
            versions: Some(vec![
                EditVersion {
                    text: "Original text".to_string(),
                    timestamp: "2024-01-01T10:00:00Z".to_string(),
                    components: vec![],
                },
                EditVersion {
                    text: "Edited text".to_string(),
                    timestamp: "2024-01-01T10:05:00Z".to_string(),
                    components: vec!["component1".to_string()],
                },
            ]),
        };

        let json = serde_json::to_value(&edit_history).unwrap();
        assert_eq!(json["status"], "edited");
        assert!(json["versions"].is_array());
        let versions = json["versions"].as_array().unwrap();
        assert_eq!(versions.len(), 2);
        assert_eq!(versions[0]["text"], "Original text");
        assert_eq!(versions[1]["text"], "Edited text");
    }

    #[test]
    fn test_thread_reply_metadata() {
        let relationships = SerializableRelationships {
            thread_originator_guid: Some("original-message-guid".to_string()),
            thread_originator_part: Some("0".to_string()),
            num_replies: 5,
            tapbacks: vec![],
            edit_history: None,
        };

        let json = serde_json::to_value(&relationships).unwrap();
        assert_eq!(json["thread_originator_guid"], "original-message-guid");
        assert_eq!(json["thread_originator_part"], "0");
        assert_eq!(json["num_replies"], 5);
        assert!(json["tapbacks"].is_array());
    }

    #[test]
    fn test_tapback_with_emoji() {
        let tapback = SerializableTapback {
            tapback_type: "emoji".to_string(),
            emoji: Some("🎉".to_string()),
            added_by: SerializableSender {
                handle_id: Some(2),
                identifier: "+15551234567".to_string(),
                contact_name: None,
            },
            timestamp: "2024-01-15T14:30:00Z".to_string(),
            message_part_index: 1,
            is_from_me: true,
        };

        let json = serde_json::to_value(&tapback).unwrap();
        assert_eq!(json["tapback_type"], "emoji");
        assert_eq!(json["emoji"], "🎉");
        assert_eq!(json["is_from_me"], true);
    }

    #[test]
    fn test_relationships_no_replies() {
        let relationships = SerializableRelationships {
            thread_originator_guid: None,
            thread_originator_part: None,
            num_replies: 0,
            tapbacks: vec![],
            edit_history: None,
        };

        let json = serde_json::to_value(&relationships).unwrap();
        assert_eq!(json["num_replies"], 0);
        assert!(json.get("thread_originator_guid").is_none() || json["thread_originator_guid"].is_null());
        assert!(json.get("edit_history").is_none() || json["edit_history"].is_null());
    }
}
