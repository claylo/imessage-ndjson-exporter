//! Test fixture utilities for loading test data files

use std::fs;
use std::path::{Path, PathBuf};

/// Returns the path to the test database (test_data/db/test.db)
///
/// # Example
/// ```no_run
/// use imessage_ndjson_core::test_utils::get_test_db_path;
///
/// let db_path = get_test_db_path();
/// assert!(db_path.exists());
/// ```
pub fn get_test_db_path() -> PathBuf {
    get_test_data_root().join("db/test.db")
}

/// Loads a typedstream file from test_data/typedstream/
///
/// # Arguments
/// * `name` - The filename without path (e.g., "Mention", "SingleLink")
///
/// # Returns
/// The binary contents of the typedstream file
///
/// # Example
/// ```no_run
/// use imessage_ndjson_core::test_utils::load_typedstream_file;
///
/// let data = load_typedstream_file("Mention").unwrap();
/// assert!(!data.is_empty());
/// ```
pub fn load_typedstream_file(name: &str) -> anyhow::Result<Vec<u8>> {
    let path = get_test_data_root().join(format!("typedstream/{}", name));
    Ok(fs::read(&path)?)
}

/// Loads and parses a plist file
///
/// # Arguments
/// * `path` - The path to the plist file (relative to project root or absolute)
///
/// # Returns
/// The parsed plist value
///
/// # Example
/// ```no_run
/// use imessage_ndjson_core::test_utils::load_plist_file;
/// use std::path::Path;
///
/// let plist = load_plist_file(Path::new("test_data/app_message/PollCreate.plist")).unwrap();
/// ```
pub fn load_plist_file(path: &Path) -> anyhow::Result<plist::Value> {
    let data = fs::read(path)?;
    let value = plist::from_bytes(&data)?;
    Ok(value)
}

/// Returns the path to a test sticker file from test_data/stickers/
///
/// # Arguments
/// * `name` - The sticker filename (e.g., "comic.heic", "shiny.heic")
///
/// # Example
/// ```no_run
/// use imessage_ndjson_core::test_utils::get_test_sticker;
///
/// let sticker_path = get_test_sticker("comic.heic");
/// assert!(sticker_path.exists());
/// ```
pub fn get_test_sticker(name: &str) -> PathBuf {
    get_test_data_root().join(format!("stickers/{}", name))
}

/// Returns the root path to the test_data directory
///
/// This function attempts to find the test_data directory by looking in:
/// 1. Current directory (test_data/)
/// 2. Parent directory (../test_data/)
/// 3. Cargo manifest directory
fn get_test_data_root() -> PathBuf {
    // Try current directory
    let current_dir = std::env::current_dir().unwrap();
    let test_data_here = current_dir.join("test_data");
    if test_data_here.exists() {
        return test_data_here;
    }

    // Try parent directory (when running from subdirectory)
    let test_data_parent = current_dir.parent().unwrap().join("test_data");
    if test_data_parent.exists() {
        return test_data_parent;
    }

    // Try from CARGO_MANIFEST_DIR (most reliable for single-crate projects)
    if let Ok(manifest_dir) = std::env::var("CARGO_MANIFEST_DIR") {
        let manifest_path = PathBuf::from(&manifest_dir);
        let test_data_manifest = manifest_path.join("test_data");
        if test_data_manifest.exists() {
            return test_data_manifest;
        }

        // In a workspace, test_data may be at the workspace root.
        // Walk up from CARGO_MANIFEST_DIR looking for it.
        let mut ancestor = manifest_path.parent();
        while let Some(dir) = ancestor {
            let test_data_ancestor = dir.join("test_data");
            if test_data_ancestor.exists() {
                return test_data_ancestor;
            }
            ancestor = dir.parent();
        }
    }

    // Fallback: assume we're in project root
    PathBuf::from("test_data")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_test_db_path() {
        let db_path = get_test_db_path();
        assert!(db_path.to_str().unwrap().ends_with("test_data/db/test.db"));
    }

    #[test]
    fn test_get_test_sticker() {
        let sticker_path = get_test_sticker("comic.heic");
        assert!(
            sticker_path
                .to_str()
                .unwrap()
                .ends_with("test_data/stickers/comic.heic")
        );
    }

    #[test]
    fn test_get_test_data_root() {
        let root = get_test_data_root();
        assert!(root.to_str().unwrap().ends_with("test_data"));
    }
}
