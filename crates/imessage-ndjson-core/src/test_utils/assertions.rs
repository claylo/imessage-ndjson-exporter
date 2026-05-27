//! Custom assertions for validating NDJSON output

use serde_json::Value;
use std::fs;
use std::path::Path;

/// Attachment mode for validation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AttachmentMode {
    /// Reference in-place (original_path field populated)
    Reference,
    /// Copy mode (copied_path field populated)
    Copy,
    /// Embed mode (embedded_data field populated)
    Embed,
}

/// Validates that a file is valid NDJSON format
///
/// Checks that:
/// - Each line is valid JSON
/// - Each line is a JSON object (not array, string, etc.)
/// - File is not empty
///
/// # Arguments
/// * `path` - Path to the NDJSON file
///
/// # Panics
/// Panics with a descriptive message if validation fails
///
/// # Example
/// ```no_run
/// use imessage_ndjson_exporter::test_utils::assert_ndjson_valid;
/// use std::path::Path;
///
/// assert_ndjson_valid(Path::new("output/chat_1.ndjson"));
/// ```
pub fn assert_ndjson_valid(path: &Path) {
    let content = fs::read_to_string(path)
        .unwrap_or_else(|e| panic!("Failed to read NDJSON file {:?}: {}", path, e));

    assert!(!content.is_empty(), "NDJSON file {:?} is empty", path);

    for (line_num, line) in content.lines().enumerate() {
        let line_number = line_num + 1;

        // Parse as JSON
        let json: Value = serde_json::from_str(line).unwrap_or_else(|e| {
            panic!(
                "Line {} in {:?} is not valid JSON: {}\nLine content: {}",
                line_number, path, e, line
            )
        });

        // Verify it's an object
        assert!(
            json.is_object(),
            "Line {} in {:?} is not a JSON object (found {:?})",
            line_number,
            path,
            json
        );
    }
}

/// Validates that a message has the required structure
///
/// Checks for required top-level fields:
/// - message_type
/// - metadata (with rowid, guid, date, service)
/// - sender
/// - chat_context
/// - content
/// - relationships
///
/// # Arguments
/// * `msg` - The message JSON value
///
/// # Panics
/// Panics if required fields are missing
///
/// # Example
/// ```no_run
/// use imessage_ndjson_exporter::test_utils::assert_message_structure;
/// use serde_json::json;
///
/// let msg = json!({
///     "message_type": "normal",
///     "metadata": {
///         "rowid": 1,
///         "guid": "test-guid",
///         "date": "2024-01-01T00:00:00Z",
///         "service": "iMessage"
///     },
///     "sender": {},
///     "chat_context": {},
///     "content": {"components": []},
///     "relationships": {}
/// });
///
/// assert_message_structure(&msg);
/// ```
pub fn assert_message_structure(msg: &Value) {
    // Check top-level fields
    assert!(
        msg.get("message_type").is_some(),
        "Message missing 'message_type' field"
    );
    assert!(
        msg.get("metadata").is_some(),
        "Message missing 'metadata' field"
    );
    assert!(
        msg.get("sender").is_some(),
        "Message missing 'sender' field"
    );
    assert!(
        msg.get("chat_context").is_some(),
        "Message missing 'chat_context' field"
    );
    assert!(
        msg.get("content").is_some(),
        "Message missing 'content' field"
    );
    assert!(
        msg.get("relationships").is_some(),
        "Message missing 'relationships' field"
    );

    // Check metadata subfields
    let metadata = msg.get("metadata").expect("metadata field exists");
    assert!(
        metadata.get("rowid").is_some(),
        "Metadata missing 'rowid' field"
    );
    assert!(
        metadata.get("guid").is_some(),
        "Metadata missing 'guid' field"
    );
    assert!(
        metadata.get("date").is_some(),
        "Metadata missing 'date' field"
    );
    assert!(
        metadata.get("service").is_some(),
        "Metadata missing 'service' field"
    );

    // Check content has components array
    let content = msg.get("content").expect("content field exists");
    assert!(
        content.get("components").is_some(),
        "Content missing 'components' field"
    );
    assert!(
        content.get("components").unwrap().is_array(),
        "Content 'components' is not an array"
    );
}

/// Validates that attachments in a message match the expected mode
///
/// # Arguments
/// * `msg` - The message JSON value
/// * `mode` - The expected attachment mode
///
/// # Panics
/// Panics if attachments don't match the expected mode
///
/// # Example
/// ```no_run
/// use imessage_ndjson_exporter::test_utils::{assert_attachment_mode, AttachmentMode};
/// use serde_json::json;
///
/// let msg = json!({
///     "content": {
///         "components": [
///             {
///                 "type": "attachment",
///                 "original_path": "/path/to/file.jpg"
///             }
///         ]
///     }
/// });
///
/// assert_attachment_mode(&msg, AttachmentMode::Reference);
/// ```
pub fn assert_attachment_mode(msg: &Value, mode: AttachmentMode) {
    let components = msg
        .get("content")
        .and_then(|c| c.get("components"))
        .and_then(|c| c.as_array())
        .expect("Message should have content.components array");

    for component in components {
        // Only check attachment components
        if component.get("type").and_then(|t| t.as_str()) != Some("attachment") {
            continue;
        }

        match mode {
            AttachmentMode::Reference => {
                assert!(
                    component.get("original_path").is_some(),
                    "Attachment in reference mode should have 'original_path' field"
                );
                assert!(
                    component.get("copied_path").is_none(),
                    "Attachment in reference mode should not have 'copied_path' field"
                );
                assert!(
                    component.get("embedded_data").is_none(),
                    "Attachment in reference mode should not have 'embedded_data' field"
                );
            }
            AttachmentMode::Copy => {
                assert!(
                    component.get("copied_path").is_some(),
                    "Attachment in copy mode should have 'copied_path' field"
                );
                assert!(
                    component.get("embedded_data").is_none(),
                    "Attachment in copy mode should not have 'embedded_data' field"
                );
            }
            AttachmentMode::Embed => {
                assert!(
                    component.get("embedded_data").is_some(),
                    "Attachment in embed mode should have 'embedded_data' field"
                );
                assert!(
                    component.get("embedded_encoding").is_some(),
                    "Attachment in embed mode should have 'embedded_encoding' field"
                );

                // Verify base64 encoding is valid
                let encoded = component
                    .get("embedded_data")
                    .and_then(|d| d.as_str())
                    .expect("embedded_data should be a string");

                use base64::Engine;
                base64::engine::general_purpose::STANDARD
                    .decode(encoded)
                    .expect("embedded_data should be valid base64");
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_assert_ndjson_valid_success() {
        let mut file = NamedTempFile::new().unwrap();
        writeln!(file, r#"{{"key": "value"}}"#).unwrap();
        writeln!(file, r#"{{"another": "object"}}"#).unwrap();
        file.flush().unwrap();

        // Should not panic
        assert_ndjson_valid(file.path());
    }

    #[test]
    #[should_panic(expected = "is not valid JSON")]
    fn test_assert_ndjson_valid_invalid_json() {
        let mut file = NamedTempFile::new().unwrap();
        writeln!(file, r#"{{"invalid": "json""#).unwrap();
        file.flush().unwrap();

        assert_ndjson_valid(file.path());
    }

    #[test]
    #[should_panic(expected = "is not a JSON object")]
    fn test_assert_ndjson_valid_not_object() {
        let mut file = NamedTempFile::new().unwrap();
        writeln!(file, r#"["array", "not", "object"]"#).unwrap();
        file.flush().unwrap();

        assert_ndjson_valid(file.path());
    }

    #[test]
    fn test_assert_message_structure_success() {
        let msg = json!({
            "message_type": "normal",
            "metadata": {
                "rowid": 1,
                "guid": "test-guid",
                "date": "2024-01-01T00:00:00Z",
                "service": "iMessage"
            },
            "sender": {},
            "chat_context": {},
            "content": {"components": []},
            "relationships": {}
        });

        // Should not panic
        assert_message_structure(&msg);
    }

    #[test]
    #[should_panic(expected = "missing 'message_type'")]
    fn test_assert_message_structure_missing_type() {
        let msg = json!({
            "metadata": {"rowid": 1, "guid": "x", "date": "x", "service": "x"},
            "sender": {},
            "chat_context": {},
            "content": {"components": []},
            "relationships": {}
        });

        assert_message_structure(&msg);
    }

    #[test]
    fn test_assert_attachment_mode_reference() {
        let msg = json!({
            "content": {
                "components": [
                    {
                        "type": "attachment",
                        "original_path": "/path/to/file.jpg"
                    }
                ]
            }
        });

        // Should not panic
        assert_attachment_mode(&msg, AttachmentMode::Reference);
    }

    #[test]
    fn test_assert_attachment_mode_embed() {
        let msg = json!({
            "content": {
                "components": [
                    {
                        "type": "attachment",
                        "embedded_data": "SGVsbG8gV29ybGQh",  // "Hello World!" in base64
                        "embedded_encoding": "base64"
                    }
                ]
            }
        });

        // Should not panic
        assert_attachment_mode(&msg, AttachmentMode::Embed);
    }
}
