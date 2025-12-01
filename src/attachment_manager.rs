use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use flate2::write::GzEncoder;
use flate2::Compression;
use hex;
use imessage_database::tables::attachment::Attachment;
use imessage_database::util::platform::Platform;
use sha2::{Digest, Sha256};

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
    pub fn from_str(s: &str) -> Option<Self> {
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
    #[allow(dead_code)]
    convert: bool,

    /// Cache mapping: hash -> copied path (for deduplication)
    hash_cache: HashMap<String, PathBuf>,

    /// Platform (macOS or iOS)
    platform: Platform,

    /// Database path (for resolving attachment paths)
    db_path: PathBuf,
}

impl AttachmentManager {
    /// Create a new attachment manager
    pub fn new(
        output_dir: &Path,
        attachments_subdir: String,
        convert: bool,
        platform: Platform,
        db_path: PathBuf,
    ) -> Self {
        Self {
            output_dir: output_dir.to_path_buf(),
            attachments_subdir,
            convert,
            hash_cache: HashMap::new(),
            platform,
            db_path,
        }
    }

    /// Copy an attachment, returning relative path or error message
    ///
    /// This method:
    /// 1. Resolves the attachment path using platform-specific logic
    /// 2. Checks if the file exists
    /// 3. Computes SHA256 hash of the file contents
    /// 4. Checks the deduplication cache
    /// 5. Copies the file if not already cached
    /// 6. Returns relative path from output_dir
    pub fn copy_attachment(
        &mut self,
        attachment: &Attachment,
        chat_id: i32,
    ) -> Result<String, String> {
        // 1. Resolve attachment path using platform-specific logic
        let source_path = match attachment.resolved_attachment_path(
            &self.platform,
            &self.db_path,
            None,
        ) {
            Some(path) => PathBuf::from(path),
            None => {
                return Err("No file path in database".to_string());
            }
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
            // File already copied, return relative path
            return Ok(self.make_relative_path(cached_path));
        }

        // 5. Determine file extension
        let extension = self.get_extension(attachment, &source_path);

        // 6. Build destination path
        // Format: {output_dir}/{attachments_subdir}/chat_{chat_id}/{hash_prefix}.{ext}
        let hash_prefix = &hash[..16]; // First 16 chars (64 bits entropy)
        let dest_dir = self
            .output_dir
            .join(&self.attachments_subdir)
            .join(format!("chat_{}", chat_id));
        let dest_path = dest_dir.join(format!("{}.{}", hash_prefix, extension));

        // 7. Create directory if needed
        if let Err(e) = fs::create_dir_all(&dest_dir) {
            return Err(format!("Failed to create directory: {}", e));
        }

        // 8. Copy file
        // Note: conversion is stubbed for now, always copies raw
        if let Err(e) = fs::copy(&source_path, &dest_path) {
            return Err(format!("Failed to copy file: {}", e));
        }

        // 9. Update cache
        let relative_path = self.make_relative_path(&dest_path);
        self.hash_cache.insert(hash, dest_path);

        Ok(relative_path)
    }

    /// Embed an attachment as base64-encoded data
    ///
    /// This method:
    /// 1. Resolves the attachment path
    /// 2. Checks if the file exists
    /// 3. Checks size limit
    /// 4. Computes SHA256 hash
    /// 5. Reads file contents
    /// 6. Compresses if appropriate
    /// 7. Base64 encodes
    /// 8. Returns embedded data with metadata
    pub fn embed_attachment(
        &mut self,
        attachment: &Attachment,
        compression_mode: CompressionMode,
        max_size: usize,
    ) -> Result<EmbeddedData, String> {
        // 1. Resolve attachment path
        let source_path = match attachment.resolved_attachment_path(
            &self.platform,
            &self.db_path,
            None,
        ) {
            Some(path) => PathBuf::from(path),
            None => {
                return Err("No file path in database".to_string());
            }
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
        let mut file = File::open(&source_path)
            .map_err(|e| format!("Failed to open file: {}", e))?;
        let mut file_data = Vec::with_capacity(file_size);
        file.read_to_end(&mut file_data)
            .map_err(|e| format!("Failed to read file: {}", e))?;

        // 5. Compute hash
        let mut hasher = Sha256::new();
        hasher.update(&file_data);
        let content_hash = hex::encode(hasher.finalize());

        // 6. Determine compression method
        let compression_method =
            Self::should_compress(attachment.mime_type.as_deref(), compression_mode);

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
            CompressionMethod::Zstd => zstd::encode_all(data, 3)
                .map_err(|e| format!("Zstd compression failed: {}", e)),
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

    /// Get file extension from attachment or source path
    fn get_extension(&self, attachment: &Attachment, source_path: &Path) -> String {
        // Try transfer_name first
        if let Some(ref transfer_name) = attachment.transfer_name {
            if let Some(ext) = Path::new(transfer_name).extension() {
                if let Some(ext_str) = ext.to_str() {
                    return ext_str.to_string();
                }
            }
        }

        // Try filename next
        if let Some(ref filename) = attachment.filename {
            if let Some(ext) = Path::new(filename).extension() {
                if let Some(ext_str) = ext.to_str() {
                    return ext_str.to_string();
                }
            }
        }

        // Fall back to source path extension
        if let Some(ext) = source_path.extension() {
            if let Some(ext_str) = ext.to_str() {
                return ext_str.to_string();
            }
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
            Platform::macOS,
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
            Platform::macOS,
            PathBuf::new(),
        );

        let abs_path = temp_dir.path().join("attachments").join("chat_1").join("test.jpg");
        let relative = manager.make_relative_path(&abs_path);

        assert_eq!(relative, "attachments/chat_1/test.jpg");
    }

    // Note: The following tests are commented out because Attachment doesn't implement Default
    // These extension extraction behaviors will be tested through integration tests

    // #[test]
    // fn test_extension_extraction() {
    //     // Tests that transfer_name takes precedence
    // }

    // #[test]
    // fn test_extension_fallback() {
    //     // Tests fallback to source path extension
    // }

    // #[test]
    // fn test_extension_default() {
    //     // Tests default "bin" extension when none found
    // }
}
