use serde::Serialize;

/// Serializable representation of an attachment
#[derive(Debug, Serialize, Clone)]
pub struct SerializableAttachment {
    /// Attachment GUID in database
    pub guid: Option<String>,
    /// Original filename
    pub filename: Option<String>,
    /// Filename when sent/received
    pub transfer_name: Option<String>,
    /// MIME type (e.g., "image/jpeg", "video/mp4")
    pub mime_type: Option<String>,
    /// Uniform Type Identifier
    pub uti: Option<String>,
    /// File size in bytes
    pub size_bytes: i64,
    /// Audio message transcription (if available)
    pub transcription: Option<String>,
    /// Attachment dimensions (for images/videos)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dimensions: Option<AttachmentDimensions>,
    /// Whether this is a sticker
    pub is_sticker: bool,
    /// Sticker metadata (if this is a sticker)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sticker_metadata: Option<StickerMetadata>,
    /// Original absolute path to attachment file (for reference-in-place mode)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub original_path: Option<String>,
    /// Relative path to copied attachment (e.g., "attachments/chat_123/abc123.jpg")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub copied_path: Option<String>,
    /// Error message if copy failed (e.g., "File not found: /path/to/file")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub copy_error: Option<String>,
    /// Base64-encoded attachment data (when embedded)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub embedded_data: Option<String>,
    /// Encoding method ("base64")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub embedded_encoding: Option<String>,
    /// Compression method ("gzip", "zstd", or "none")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub embedded_compression: Option<String>,
    /// SHA256 hash of original file content (for deduplication checking)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content_hash: Option<String>,
}

/// Attachment dimensions (width and height in points)
#[derive(Debug, Serialize, Clone)]
pub struct AttachmentDimensions {
    pub width: f64,
    pub height: f64,
}

/// Sticker-specific metadata
#[derive(Debug, Serialize, Clone)]
pub struct StickerMetadata {
    /// Source of the sticker (Genmoji, Memoji, App, UserGenerated)
    pub source: String,
    /// Genmoji generation prompt (only for Genmoji stickers)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub genmoji_prompt: Option<String>,
    /// Sticker effect (if available)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub effect: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json;

    #[test]
    fn test_attachment_dimensions() {
        let dimensions = AttachmentDimensions {
            width: 1920.0,
            height: 1080.0,
        };

        let json = serde_json::to_value(&dimensions).unwrap();
        assert_eq!(json["width"], 1920.0);
        assert_eq!(json["height"], 1080.0);
    }

    #[test]
    fn test_mime_type_mapping() {
        let attachment = SerializableAttachment {
            guid: Some("attachment-123".to_string()),
            filename: Some("photo.jpg".to_string()),
            transfer_name: Some("IMG_001.jpg".to_string()),
            mime_type: Some("image/jpeg".to_string()),
            uti: Some("public.jpeg".to_string()),
            size_bytes: 1024000,
            transcription: None,
            dimensions: Some(AttachmentDimensions {
                width: 1920.0,
                height: 1080.0,
            }),
            is_sticker: false,
            sticker_metadata: None,
            original_path: Some("/path/to/photo.jpg".to_string()),
            copied_path: None,
            copy_error: None,
            embedded_data: None,
            embedded_encoding: None,
            embedded_compression: None,
            content_hash: None,
        };

        let json = serde_json::to_value(&attachment).unwrap();
        assert_eq!(json["mime_type"], "image/jpeg");
        assert_eq!(json["uti"], "public.jpeg");
        assert_eq!(json["filename"], "photo.jpg");
        assert_eq!(json["size_bytes"], 1024000);
        assert_eq!(json["is_sticker"], false);
    }

    #[test]
    fn test_sticker_metadata() {
        let metadata = StickerMetadata {
            source: "Genmoji".to_string(),
            genmoji_prompt: Some("happy cat".to_string()),
            effect: Some("shiny".to_string()),
        };

        let json = serde_json::to_value(&metadata).unwrap();
        assert_eq!(json["source"], "Genmoji");
        assert_eq!(json["genmoji_prompt"], "happy cat");
        assert_eq!(json["effect"], "shiny");
    }

    #[test]
    fn test_attachment_reference_mode() {
        let attachment = SerializableAttachment {
            guid: Some("ref-123".to_string()),
            filename: Some("file.txt".to_string()),
            transfer_name: None,
            mime_type: Some("text/plain".to_string()),
            uti: None,
            size_bytes: 100,
            transcription: None,
            dimensions: None,
            is_sticker: false,
            sticker_metadata: None,
            original_path: Some("/original/path/file.txt".to_string()),
            copied_path: None,
            copy_error: None,
            embedded_data: None,
            embedded_encoding: None,
            embedded_compression: None,
            content_hash: None,
        };

        let json = serde_json::to_value(&attachment).unwrap();
        assert_eq!(json["original_path"], "/original/path/file.txt");
        assert!(json.get("copied_path").is_none() || json["copied_path"].is_null());
        assert!(json.get("embedded_data").is_none() || json["embedded_data"].is_null());
    }

    #[test]
    fn test_attachment_copy_mode() {
        let attachment = SerializableAttachment {
            guid: Some("copy-123".to_string()),
            filename: Some("image.png".to_string()),
            transfer_name: None,
            mime_type: Some("image/png".to_string()),
            uti: None,
            size_bytes: 50000,
            transcription: None,
            dimensions: None,
            is_sticker: false,
            sticker_metadata: None,
            original_path: Some("/original/image.png".to_string()),
            copied_path: Some("attachments/chat_1/abc123.png".to_string()),
            copy_error: None,
            embedded_data: None,
            embedded_encoding: None,
            embedded_compression: None,
            content_hash: Some("abc123def456".to_string()),
        };

        let json = serde_json::to_value(&attachment).unwrap();
        assert_eq!(json["copied_path"], "attachments/chat_1/abc123.png");
        assert_eq!(json["content_hash"], "abc123def456");
    }

    #[test]
    fn test_attachment_embed_mode() {
        let attachment = SerializableAttachment {
            guid: Some("embed-123".to_string()),
            filename: Some("data.bin".to_string()),
            transfer_name: None,
            mime_type: Some("application/octet-stream".to_string()),
            uti: None,
            size_bytes: 200,
            transcription: None,
            dimensions: None,
            is_sticker: false,
            sticker_metadata: None,
            original_path: None,
            copied_path: None,
            copy_error: None,
            embedded_data: Some("SGVsbG8gV29ybGQh".to_string()), // "Hello World!" in base64
            embedded_encoding: Some("base64".to_string()),
            embedded_compression: Some("none".to_string()),
            content_hash: None,
        };

        let json = serde_json::to_value(&attachment).unwrap();
        assert_eq!(json["embedded_data"], "SGVsbG8gV29ybGQh");
        assert_eq!(json["embedded_encoding"], "base64");
        assert_eq!(json["embedded_compression"], "none");
    }
}
