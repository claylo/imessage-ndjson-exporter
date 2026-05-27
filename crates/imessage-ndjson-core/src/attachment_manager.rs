use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use base64::{Engine, engine::general_purpose::STANDARD as BASE64};
use flate2::Compression;
use flate2::write::GzEncoder;
use hex;
use sha2::{Digest, Sha256};

use crate::converters::{AudioConverter, Converter, ImageConverter, VideoConverter};

/// Compression mode for embedded attachments
#[derive(Debug, Clone, Copy)]
pub enum CompressionMode {
    /// Smart auto-detection based on MIME type
    Auto,
    /// Force gzip compression
    Gzip,
    /// Force zstd compression
    Zstd,
    /// No compression
    None,
}

impl CompressionMode {
    pub fn parse(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "auto" => Some(CompressionMode::Auto),
            "gzip" => Some(CompressionMode::Gzip),
            "zstd" => Some(CompressionMode::Zstd),
            "none" => Some(CompressionMode::None),
            _ => None,
        }
    }
}

/// Actual compression method used
#[derive(Debug, Clone, Copy)]
pub enum CompressionMethod {
    Gzip,
    Zstd,
    None,
}

impl CompressionMethod {
    pub fn as_str(&self) -> &'static str {
        match self {
            CompressionMethod::Gzip => "gzip",
            CompressionMethod::Zstd => "zstd",
            CompressionMethod::None => "none",
        }
    }
}

/// Result of embedding an attachment
pub struct EmbeddedData {
    /// Base64-encoded data
    pub data: String,
    /// Encoding method (always "base64")
    pub encoding: String,
    /// Compression method used
    pub compression: String,
    /// SHA256 hash of original file
    pub content_hash: String,
    /// Original file size in bytes
    pub original_size: usize,
    /// Size after compression (before base64)
    pub compressed_size: usize,
    /// Size after base64 encoding
    pub encoded_size: usize,
}

/// Manages attachment file copying with hash-based deduplication
pub struct AttachmentManager {
    /// Base output directory
    output_dir: PathBuf,

    /// Subdirectory for attachments (e.g., "attachments")
    attachments_subdir: String,

    /// Whether to convert attachments to compatible formats
    convert: bool,

    /// Cache mapping: hash -> copied path (for deduplication)
    hash_cache: HashMap<String, PathBuf>,

    /// Database path (reserved for future platform-specific path resolution)
    _db_path: PathBuf,

    /// Image converter (if available)
    image_converter: Option<ImageConverter>,

    /// Video converter (if available)
    video_converter: Option<VideoConverter>,

    /// Audio converter (if available)
    audio_converter: Option<AudioConverter>,
}

impl AttachmentManager {
    /// Create a new attachment manager
    pub fn new(
        output_dir: &Path,
        attachments_subdir: String,
        convert: bool,
        db_path: PathBuf,
    ) -> Self {
        // Detect converters if conversion is enabled
        let (image_converter, video_converter, audio_converter) = if convert {
            (
                ImageConverter::determine(),
                VideoConverter::determine(),
                AudioConverter::determine(),
            )
        } else {
            (None, None, None)
        };

        Self {
            output_dir: output_dir.to_path_buf(),
            attachments_subdir,
            convert,
            hash_cache: HashMap::new(),
            _db_path: db_path,
            image_converter,
            video_converter,
            audio_converter,
        }
    }

    /// Copy an attachment from a resolved path, returning relative path and optionally new MIME type
    ///
    /// This is the path-based version that works with imessage-db where we resolve paths ourselves.
    pub fn copy_attachment_from_path(
        &mut self,
        source_path: Option<&str>,
        transfer_name: Option<&str>,
        filename: Option<&str>,
        mime_type: Option<&str>,
        is_sticker: bool,
        chat_id: i64,
    ) -> Result<(String, Option<String>), String> {
        // 1. Resolve source path
        let source_path = match source_path {
            Some(p) => PathBuf::from(p),
            None => return Err("No file path available".to_string()),
        };

        // 2. Check if file exists
        if !source_path.exists() {
            return Err(format!("File not found: {}", source_path.display()));
        }

        // 3. Compute SHA256 hash
        let hash = match self.compute_hash(&source_path) {
            Ok(h) => h,
            Err(e) => {
                return Err(format!("Failed to hash file: {}", e));
            }
        };

        // 4. Check deduplication cache
        if let Some(cached_path) = self.hash_cache.get(&hash) {
            return Ok((self.make_relative_path(cached_path), None));
        }

        // 5. Determine file extension
        let extension = Self::get_extension_from_parts(transfer_name, filename, &source_path);

        // 6. Build destination path
        let hash_prefix = &hash[..16];
        let dest_dir = self
            .output_dir
            .join(&self.attachments_subdir)
            .join(format!("chat_{}", chat_id));
        let mut dest_path = dest_dir.join(format!("{}.{}", hash_prefix, extension));

        // 7. Create directory if needed
        if let Err(e) = fs::create_dir_all(&dest_dir) {
            return Err(format!("Failed to create directory: {}", e));
        }

        // 8. Convert or copy file
        use crate::converters::{audio, image, sticker, video};

        let mime = mime_type.unwrap_or("");
        let mut extension_changed = false;

        if self.convert {
            if is_sticker && mime.starts_with("image/") {
                extension_changed = sticker::convert_if_needed(
                    &source_path,
                    &mut dest_path,
                    &self.image_converter,
                    &self.video_converter,
                    mime,
                );
            } else if mime.starts_with("image/") {
                extension_changed = image::convert_if_needed(
                    &source_path,
                    &mut dest_path,
                    &self.image_converter,
                    mime,
                );
            } else if mime.starts_with("video/") {
                extension_changed = video::convert_if_needed(
                    &source_path,
                    &mut dest_path,
                    &self.video_converter,
                    mime,
                );
            } else if mime.starts_with("audio/") {
                extension_changed = audio::convert_if_needed(
                    &source_path,
                    &mut dest_path,
                    &self.audio_converter,
                    mime,
                );
            }
        }

        // Fallback to raw copy if conversion didn't happen
        if !extension_changed {
            if let Err(e) = fs::copy(&source_path, &dest_path) {
                return Err(format!("Failed to copy attachment: {}", e));
            }
        }

        // 9. Determine new MIME type if extension changed
        let new_mime = if extension_changed {
            let ext = dest_path.extension().and_then(|e| e.to_str()).unwrap_or("");
            match ext {
                "jpeg" => Some("image/jpeg".to_string()),
                "png" => Some("image/png".to_string()),
                "gif" => Some("image/gif".to_string()),
                "mp4" => Some("video/mp4".to_string()),
                "m4a" => Some("audio/mp4".to_string()),
                _ => None,
            }
        } else {
            None
        };

        // 10. Update cache
        let relative_path = self.make_relative_path(&dest_path);
        self.hash_cache.insert(hash, dest_path);

        Ok((relative_path, new_mime))
    }

    /// Embed an attachment from a resolved path as base64-encoded data
    pub fn embed_attachment_from_path(
        &mut self,
        source_path: Option<&str>,
        mime_type: Option<&str>,
        compression_mode: CompressionMode,
        max_size: usize,
    ) -> Result<EmbeddedData, String> {
        // 1. Resolve source path
        let source_path = match source_path {
            Some(p) => PathBuf::from(p),
            None => return Err("No file path available".to_string()),
        };

        // 2. Check if file exists
        if !source_path.exists() {
            return Err(format!("File not found: {}", source_path.display()));
        }

        // 3. Check size limit
        let metadata = fs::metadata(&source_path)
            .map_err(|e| format!("Failed to read file metadata: {}", e))?;
        let file_size = metadata.len() as usize;

        if file_size > max_size {
            return Err(format!(
                "File too large for embedding: {} bytes (max: {} bytes)",
                file_size, max_size
            ));
        }

        // 4. Read file contents
        let mut file =
            File::open(&source_path).map_err(|e| format!("Failed to open file: {}", e))?;
        let mut file_data = Vec::with_capacity(file_size);
        file.read_to_end(&mut file_data)
            .map_err(|e| format!("Failed to read file: {}", e))?;

        // 5. Compute hash
        let mut hasher = Sha256::new();
        hasher.update(&file_data);
        let content_hash = hex::encode(hasher.finalize());

        // 6. Determine compression method
        let compression_method = Self::should_compress(mime_type, compression_mode);

        // 7. Compress if needed
        let compressed_data = Self::compress_data(&file_data, compression_method)?;
        let compressed_size = compressed_data.len();

        // 8. Base64 encode
        let encoded = BASE64.encode(&compressed_data);
        let encoded_size = encoded.len();

        Ok(EmbeddedData {
            data: encoded,
            encoding: "base64".to_string(),
            compression: compression_method.as_str().to_string(),
            content_hash,
            original_size: file_size,
            compressed_size,
            encoded_size,
        })
    }

    /// Determine which compression method to use
    fn should_compress(
        mime_type: Option<&str>,
        compression_mode: CompressionMode,
    ) -> CompressionMethod {
        match compression_mode {
            CompressionMode::Gzip => CompressionMethod::Gzip,
            CompressionMode::Zstd => CompressionMethod::Zstd,
            CompressionMode::None => CompressionMethod::None,
            CompressionMode::Auto => {
                // Highly compressed formats - skip compression
                let skip_compression = [
                    "image/jpeg",
                    "image/heic",
                    "image/heif",
                    "video/mp4",
                    "video/quicktime",
                    "audio/mp4",
                    "audio/aac",
                    "audio/mpeg",
                    "image/png",
                    "image/webp",
                ];

                if mime_type
                    .map(|m| skip_compression.contains(&m))
                    .unwrap_or(false)
                {
                    CompressionMethod::None
                } else {
                    // Everything else: use zstd (fast, good compression)
                    CompressionMethod::Zstd
                }
            }
        }
    }

    /// Compress data using specified method
    fn compress_data(data: &[u8], method: CompressionMethod) -> Result<Vec<u8>, String> {
        match method {
            CompressionMethod::None => Ok(data.to_vec()),
            CompressionMethod::Gzip => {
                let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
                encoder
                    .write_all(data)
                    .map_err(|e| format!("Gzip compression failed: {}", e))?;
                encoder
                    .finish()
                    .map_err(|e| format!("Gzip compression failed: {}", e))
            }
            CompressionMethod::Zstd => {
                zstd::encode_all(data, 3).map_err(|e| format!("Zstd compression failed: {}", e))
            }
        }
    }

    /// Compute SHA256 hash of file contents
    fn compute_hash(&self, path: &Path) -> Result<String, std::io::Error> {
        let mut file = File::open(path)?;
        let mut hasher = Sha256::new();
        let mut buffer = vec![0; 8192]; // 8KB buffer

        loop {
            let bytes_read = file.read(&mut buffer)?;
            if bytes_read == 0 {
                break;
            }
            hasher.update(&buffer[..bytes_read]);
        }

        let result = hasher.finalize();
        Ok(hex::encode(result))
    }

    /// Convert absolute path to relative path from output_dir
    fn make_relative_path(&self, path: &Path) -> String {
        path.strip_prefix(&self.output_dir)
            .unwrap_or(path)
            .to_string_lossy()
            .to_string()
    }

    /// Get file extension from transfer_name, filename, or source path
    fn get_extension_from_parts(
        transfer_name: Option<&str>,
        filename: Option<&str>,
        source_path: &Path,
    ) -> String {
        // Try transfer_name first
        if let Some(name) = transfer_name {
            if let Some(ext) = Path::new(name).extension().and_then(|e| e.to_str()) {
                return ext.to_string();
            }
        }

        // Try filename next
        if let Some(name) = filename {
            if let Some(ext) = Path::new(name).extension().and_then(|e| e.to_str()) {
                return ext.to_string();
            }
        }

        // Fall back to source path extension
        if let Some(ext) = source_path.extension().and_then(|e| e.to_str()) {
            return ext.to_string();
        }

        // Default to "bin" if no extension found
        "bin".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::TempDir;

    #[test]
    fn test_hash_computation() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        let mut file = File::create(&file_path).unwrap();
        file.write_all(b"Hello, World!").unwrap();

        let manager = AttachmentManager::new(
            temp_dir.path(),
            "attachments".to_string(),
            false,
            PathBuf::new(),
        );

        let hash = manager.compute_hash(&file_path).unwrap();

        // SHA256 of "Hello, World!" is known
        assert_eq!(
            hash,
            "dffd6021bb2bd5b0af676290809ec3a53191dd81c7f70a4b28688a362182986f"
        );
    }

    #[test]
    fn test_relative_path_conversion() {
        let temp_dir = TempDir::new().unwrap();
        let manager = AttachmentManager::new(
            temp_dir.path(),
            "attachments".to_string(),
            false,
            PathBuf::new(),
        );

        let abs_path = temp_dir
            .path()
            .join("attachments")
            .join("chat_1")
            .join("test.jpg");
        let relative = manager.make_relative_path(&abs_path);

        assert_eq!(relative, "attachments/chat_1/test.jpg");
    }
}
