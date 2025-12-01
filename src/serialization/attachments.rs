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
