use serde::Serialize;
use super::attachments::SerializableAttachment;

/// Serializable representation of message content
#[derive(Debug, Serialize, Clone)]
pub struct SerializableContent {
    /// Plain text content (if any)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    /// Message subject (email-style)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subject: Option<String>,
    /// Components that make up the message body
    pub components: Vec<ContentComponent>,
}

/// Individual component of message content
#[derive(Debug, Serialize, Clone)]
#[serde(tag = "type")]
pub enum ContentComponent {
    #[serde(rename = "text")]
    Text {
        text: String,
        attributes: Vec<TextAttribute>,
    },
    #[serde(rename = "attachment")]
    Attachment(SerializableAttachment),
    #[serde(rename = "app")]
    App {
        balloon_bundle_id: String,
        app_name: Option<String>,
        app_type: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        metadata: Option<serde_json::Value>,
    },
    #[serde(rename = "retracted")]
    Retracted,
}

/// Text formatting attributes
#[derive(Debug, Serialize, Clone)]
pub struct TextAttribute {
    /// Start index in the text
    pub start: usize,
    /// End index in the text
    pub end: usize,
    /// Effects applied to this range
    pub effects: Vec<TextEffect>,
}

/// Text effects (mentions, links, styling, etc.)
#[derive(Debug, Serialize, Clone)]
#[serde(tag = "type")]
pub enum TextEffect {
    #[serde(rename = "mention")]
    Mention { identifier: String },
    #[serde(rename = "link")]
    Link { url: String },
    #[serde(rename = "otp")]
    OTP,
    #[serde(rename = "conversion")]
    Conversion,
    #[serde(rename = "style")]
    Style { style: String },
    #[serde(rename = "animated")]
    Animated { animation: String },
    #[serde(rename = "default")]
    Default,
}

/// Expressive send style (screen/bubble effects)
#[derive(Debug, Serialize, Clone)]
#[serde(tag = "type")]
pub enum ExpressiveEffect {
    #[serde(rename = "screen")]
    Screen { effect: String },
    #[serde(rename = "bubble")]
    Bubble { effect: String },
}
