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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json;

    #[test]
    fn test_text_component_basic() {
        let component = ContentComponent::Text {
            text: "Hello, world!".to_string(),
            attributes: vec![],
        };

        let json = serde_json::to_value(&component).unwrap();
        assert_eq!(json["type"], "text");
        assert_eq!(json["text"], "Hello, world!");
        assert!(json["attributes"].is_array());
        assert_eq!(json["attributes"].as_array().unwrap().len(), 0);
    }

    #[test]
    fn test_text_effects_mention() {
        let effect = TextEffect::Mention {
            identifier: "user@example.com".to_string(),
        };

        let json = serde_json::to_value(&effect).unwrap();
        assert_eq!(json["type"], "mention");
        assert_eq!(json["identifier"], "user@example.com");
    }

    #[test]
    fn test_text_effects_link() {
        let effect = TextEffect::Link {
            url: "https://example.com".to_string(),
        };

        let json = serde_json::to_value(&effect).unwrap();
        assert_eq!(json["type"], "link");
        assert_eq!(json["url"], "https://example.com");
    }

    #[test]
    fn test_text_effects_otp() {
        let effect = TextEffect::OTP;

        let json = serde_json::to_value(&effect).unwrap();
        assert_eq!(json["type"], "otp");
        // OTP has no additional fields
        assert_eq!(json.as_object().unwrap().len(), 1); // Just "type"
    }

    #[test]
    fn test_text_effects_style() {
        let effect = TextEffect::Style {
            style: "bold".to_string(),
        };

        let json = serde_json::to_value(&effect).unwrap();
        assert_eq!(json["type"], "style");
        assert_eq!(json["style"], "bold");
    }

    #[test]
    fn test_text_attribute_with_effects() {
        let attr = TextAttribute {
            start: 0,
            end: 5,
            effects: vec![
                TextEffect::Style {
                    style: "bold".to_string(),
                },
                TextEffect::Link {
                    url: "https://example.com".to_string(),
                },
            ],
        };

        let json = serde_json::to_value(&attr).unwrap();
        assert_eq!(json["start"], 0);
        assert_eq!(json["end"], 5);
        assert_eq!(json["effects"].as_array().unwrap().len(), 2);
        assert_eq!(json["effects"][0]["type"], "style");
        assert_eq!(json["effects"][1]["type"], "link");
    }

    #[test]
    fn test_app_component_structure() {
        let component = ContentComponent::App {
            balloon_bundle_id: "com.apple.messages.URLBalloonProvider".to_string(),
            app_name: Some("Link Preview".to_string()),
            app_type: "url".to_string(),
            metadata: Some(serde_json::json!({"url": "https://example.com"})),
        };

        let json = serde_json::to_value(&component).unwrap();
        assert_eq!(json["type"], "app");
        assert_eq!(json["balloon_bundle_id"], "com.apple.messages.URLBalloonProvider");
        assert_eq!(json["app_name"], "Link Preview");
        assert_eq!(json["app_type"], "url");
        assert!(json["metadata"].is_object());
    }

    #[test]
    fn test_retracted_component() {
        let component = ContentComponent::Retracted;

        let json = serde_json::to_value(&component).unwrap();
        assert_eq!(json["type"], "retracted");
        // Retracted has no additional fields
        assert_eq!(json.as_object().unwrap().len(), 1); // Just "type"
    }

    #[test]
    fn test_content_with_text_and_subject() {
        let content = SerializableContent {
            text: Some("Message body".to_string()),
            subject: Some("Important Message".to_string()),
            components: vec![],
        };

        let json = serde_json::to_value(&content).unwrap();
        assert_eq!(json["text"], "Message body");
        assert_eq!(json["subject"], "Important Message");
        assert!(json["components"].is_array());
    }

    #[test]
    fn test_expressive_effect_screen() {
        let effect = ExpressiveEffect::Screen {
            effect: "fireworks".to_string(),
        };

        let json = serde_json::to_value(&effect).unwrap();
        assert_eq!(json["type"], "screen");
        assert_eq!(json["effect"], "fireworks");
    }

    #[test]
    fn test_expressive_effect_bubble() {
        let effect = ExpressiveEffect::Bubble {
            effect: "slam".to_string(),
        };

        let json = serde_json::to_value(&effect).unwrap();
        assert_eq!(json["type"], "bubble");
        assert_eq!(json["effect"], "slam");
    }
}
