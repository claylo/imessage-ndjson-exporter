//! Test utilities for imessage-ndjson-exporter
//!
//! This module provides shared utilities for both unit and integration tests:
//! - `fixtures` - Load test data files from test_data/
//! - `assertions` - Custom assertions for validating NDJSON output
//! - `database` - Test database helpers

pub mod assertions;
pub mod database;
pub mod fixtures;

// Re-export commonly used items for convenience
pub use assertions::{
    AttachmentMode, assert_attachment_mode, assert_message_structure, assert_ndjson_valid,
};
pub use fixtures::{get_test_db_path, get_test_sticker, load_plist_file, load_typedstream_file};
