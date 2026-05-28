use serde::Serialize;

/// Serializable representation of chat context
#[derive(Debug, Serialize, Clone)]
pub struct SerializableChatContext {
    /// Chat ID in database
    pub chat_id: Option<i64>,
    /// Chat identifier (phone number, email, or group chat ID)
    pub chat_identifier: String,
    /// Custom display name for the chat
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
    /// Service name (iMessage, SMS, etc.)
    pub service_name: String,
    /// Participants in the chat
    pub participants: Vec<String>,
}

/// Serializable representation of a sender/participant
#[derive(Debug, Serialize, Clone)]
pub struct SerializableSender {
    /// Handle ID in database
    pub handle_id: Option<i64>,
    /// Contact identifier (phone number or email)
    pub identifier: String,
    /// Resolved contact name
    #[serde(skip_serializing_if = "Option::is_none")]
    pub contact_name: Option<String>,
}
