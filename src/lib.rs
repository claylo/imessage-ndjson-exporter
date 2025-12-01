/// iMessage NDJSON Exporter Library
///
/// This library provides functionality to export iMessage data from the iMessage
/// database to NDJSON (newline-delimited JSON) format, preserving all message
/// metadata, reactions, edits, attachments, and special features.

pub mod attachment_manager;
pub mod avatar_manager;
pub mod cli;
pub mod contacts;
pub mod converters;
pub mod exporter;
pub mod resolvers;
pub mod serialization;

// Test utilities (only available during testing)
#[cfg(any(test, feature = "test-utils"))]
pub mod test_utils;

pub use exporter::NdjsonExporter;
