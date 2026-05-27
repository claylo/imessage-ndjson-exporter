use serde::Serialize;

/// Participant information for a chat
///
/// This structure represents a participant in a conversation, including
/// their contact information and avatar path (if available).
#[derive(Debug, Serialize, Clone)]
pub struct SerializableParticipant {
    /// Handle ID from the iMessage database
    pub handle_id: i32,

    /// Identifier (phone number or email address)
    pub identifier: String,

    /// Contact name (from contacts database, if available)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub contact_name: Option<String>,

    /// Relative path to avatar image (from output directory)
    ///
    /// Format: "avatars/<hash>.jpg"
    /// Will be null if no avatar is available for this contact
    #[serde(skip_serializing_if = "Option::is_none")]
    pub avatar_path: Option<String>,
}
