//! Test execution context for integration tests

use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;
use serde_json::Value;
use imessage_ndjson_exporter::exporter::NdjsonExporter;
use imessage_ndjson_exporter::attachment_manager::CompressionMode;

/// Test execution context providing temporary directories and helper methods
///
/// Automatically cleans up temp directories when dropped.
pub struct TestContext {
    /// Temporary directory (auto-deleted on drop)
    pub temp_dir: TempDir,
    /// Path to test database (copied to temp dir)
    pub db_path: PathBuf,
    /// Output directory for export
    pub output_dir: PathBuf,
}

impl TestContext {
    /// Creates a new test context
    ///
    /// - Creates a temporary directory
    /// - Copies test.db to the temp directory
    /// - Sets up an output directory
    ///
    /// # Panics
    /// Panics if temp directory creation or database copying fails
    pub fn new() -> Self {
        let temp_dir = TempDir::new().expect("Failed to create temp directory");

        // Copy test database to temp location
        let test_data_root = get_test_data_root();
        let source_db = test_data_root.join("db/test.db");
        let db_path = temp_dir.path().join("test.db");

        fs::copy(&source_db, &db_path)
            .expect(&format!("Failed to copy test database from {:?}", source_db));

        // Create output directory
        let output_dir = temp_dir.path().join("output");
        fs::create_dir(&output_dir)
            .expect("Failed to create output directory");

        Self {
            temp_dir,
            db_path,
            output_dir,
        }
    }

    /// Runs the exporter with default test settings
    ///
    /// # Returns
    /// Result indicating success or failure
    ///
    /// # Example
    /// ```no_run
    /// use common::TestContext;
    ///
    /// let ctx = TestContext::new();
    /// ctx.run_export().unwrap();
    /// ```
    pub fn run_export(&self) -> anyhow::Result<()> {
        self.run_export_with_options(
            false, // copy_attachments
            false, // convert_attachments
            false, // embed_attachments
            false, // include_avatars
        )
    }

    /// Runs the exporter with custom options
    ///
    /// # Arguments
    /// * `copy_attachments` - Whether to copy attachments
    /// * `convert_attachments` - Whether to convert attachments
    /// * `embed_attachments` - Whether to embed attachments
    /// * `include_avatars` - Whether to include avatars
    ///
    /// # Returns
    /// Result indicating success or failure
    pub fn run_export_with_options(
        &self,
        copy_attachments: bool,
        convert_attachments: bool,
        embed_attachments: bool,
        include_avatars: bool,
    ) -> anyhow::Result<()> {
        let exporter = NdjsonExporter::new(
            &self.db_path,
            &self.output_dir,
            None,  // custom_name
            false, // show_progress (disable for tests)
            None,  // conversation_filter
            None,  // contacts_path
            copy_attachments,
            convert_attachments,
            Some("attachments".to_string()),  // attachments_dir
            embed_attachments,
            10 * 1024 * 1024,  // max_embed_size (10MB)
            CompressionMode::Auto,
            include_avatars,
            None,  // start_date
            None,  // end_date
        )?;

        exporter.export()?;
        Ok(())
    }

    /// Reads and parses a chat's NDJSON file
    ///
    /// # Arguments
    /// * `chat_id` - The chat ID (e.g., 1 for chat_1.ndjson)
    ///
    /// # Returns
    /// Vec of parsed JSON messages
    ///
    /// # Panics
    /// Panics if file doesn't exist or contains invalid JSON
    ///
    /// # Example
    /// ```no_run
    /// use common::TestContext;
    ///
    /// let ctx = TestContext::new();
    /// ctx.run_export().unwrap();
    /// let messages = ctx.read_chat_ndjson(1);
    /// assert_eq!(messages.len(), 3);
    /// ```
    pub fn read_chat_ndjson(&self, chat_id: i32) -> Vec<Value> {
        let file_path = self.output_dir.join(format!("chat_{}.ndjson", chat_id));
        let content = fs::read_to_string(&file_path)
            .expect(&format!("Failed to read {:?}", file_path));

        content
            .lines()
            .map(|line| {
                serde_json::from_str(line)
                    .expect(&format!("Invalid JSON in chat_{}.ndjson: {}", chat_id, line))
            })
            .collect()
    }

    /// Asserts that a chat file exists
    ///
    /// # Arguments
    /// * `chat_id` - The chat ID
    ///
    /// # Panics
    /// Panics if file doesn't exist
    pub fn assert_chat_file_exists(&self, chat_id: i32) {
        let file_path = self.output_dir.join(format!("chat_{}.ndjson", chat_id));
        assert!(
            file_path.exists(),
            "Expected chat_{}.ndjson to exist at {:?}",
            chat_id,
            file_path
        );
    }

    /// Asserts that a participants file exists
    ///
    /// # Arguments
    /// * `chat_id` - The chat ID
    ///
    /// # Panics
    /// Panics if file doesn't exist
    pub fn assert_participants_file_exists(&self, chat_id: i32) {
        let file_path = self.output_dir.join(format!("chat_{}_participants.ndjson", chat_id));
        assert!(
            file_path.exists(),
            "Expected chat_{}_participants.ndjson to exist at {:?}",
            chat_id,
            file_path
        );
    }

    /// Counts the total number of messages across all chat files
    ///
    /// # Returns
    /// Total message count
    pub fn count_total_messages(&self) -> usize {
        let mut count = 0;

        for entry in fs::read_dir(&self.output_dir).expect("Failed to read output directory") {
            let entry = entry.expect("Failed to read directory entry");
            let path = entry.path();

            if let Some(filename) = path.file_name().and_then(|n| n.to_str()) {
                if filename.starts_with("chat_") && filename.ends_with(".ndjson") && !filename.contains("participants") {
                    let content = fs::read_to_string(&path).expect("Failed to read chat file");
                    count += content.lines().count();
                }
            }
        }

        count
    }

    /// Lists all chat files in the output directory
    ///
    /// # Returns
    /// Vec of chat IDs
    pub fn list_chat_files(&self) -> Vec<i32> {
        let mut chat_ids = Vec::new();

        for entry in fs::read_dir(&self.output_dir).expect("Failed to read output directory") {
            let entry = entry.expect("Failed to read directory entry");
            let path = entry.path();

            if let Some(filename) = path.file_name().and_then(|n| n.to_str()) {
                if filename.starts_with("chat_") && filename.ends_with(".ndjson") && !filename.contains("participants") {
                    // Extract chat ID from filename: chat_123.ndjson -> 123
                    if let Some(id_str) = filename.strip_prefix("chat_").and_then(|s| s.strip_suffix(".ndjson")) {
                        if let Ok(id) = id_str.parse::<i32>() {
                            chat_ids.push(id);
                        }
                    }
                }
            }
        }

        chat_ids.sort();
        chat_ids
    }
}

impl Default for TestContext {
    fn default() -> Self {
        Self::new()
    }
}

/// Returns the path to the test_data directory
fn get_test_data_root() -> PathBuf {
    // Try current directory
    let current_dir = std::env::current_dir().unwrap();
    let test_data_here = current_dir.join("test_data");
    if test_data_here.exists() {
        return test_data_here;
    }

    // Try parent directory (when running from subdirectory)
    if let Some(parent) = current_dir.parent() {
        let test_data_parent = parent.join("test_data");
        if test_data_parent.exists() {
            return test_data_parent;
        }
    }

    // Try from CARGO_MANIFEST_DIR (most reliable)
    if let Ok(manifest_dir) = std::env::var("CARGO_MANIFEST_DIR") {
        let test_data_manifest = PathBuf::from(manifest_dir).join("test_data");
        if test_data_manifest.exists() {
            return test_data_manifest;
        }
    }

    // Fallback: assume we're in project root
    PathBuf::from("test_data")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_context_creation() {
        let ctx = TestContext::new();
        assert!(ctx.db_path.exists(), "Test database should exist");
        assert!(ctx.output_dir.exists(), "Output directory should exist");
    }

    #[test]
    fn test_context_cleanup() {
        let temp_path = {
            let ctx = TestContext::new();
            ctx.temp_dir.path().to_path_buf()
        };
        // After ctx is dropped, temp directory should be cleaned up
        assert!(!temp_path.exists(), "Temp directory should be cleaned up");
    }
}
