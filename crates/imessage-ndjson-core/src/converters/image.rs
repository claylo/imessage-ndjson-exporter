/*!
 Defines routines for converting image files.
*/

use std::path::{Path, PathBuf};

use crate::converters::{
    common::{ensure_paths, run_command},
    models::{Converter, ImageConverter},
};

/// Convert image if needed, updating the destination path
///
/// Returns true if the file was converted and extension changed, false otherwise
///
/// - Attachment `HEIC` files convert to `JPEG`
/// - Other formats pass through unchanged
pub fn convert_if_needed(
    from: &Path,
    to: &mut PathBuf,
    converter: &Option<ImageConverter>,
    mime_type: &str,
) -> bool {
    // Check if this is a HEIC file that needs conversion
    let is_heic = matches!(
        mime_type.to_lowercase().as_str(),
        "image/heic" | "image/heif"
    );

    if !is_heic {
        return false;
    }

    // If no converter available, can't convert
    let Some(converter) = converter else {
        return false;
    };

    // Create new path with .jpeg extension
    let mut converted_path = to.clone();
    converted_path.set_extension("jpeg");

    // Attempt conversion
    if convert_heic(from, &converted_path, converter).is_some() {
        // Conversion succeeded, update the output path
        *to = converted_path;
        return true;
    }

    // Conversion failed, will fall back to raw copy
    eprintln!("Warning: Unable to convert HEIC image: {}", from.display());
    false
}

/// Convert a HEIC image file to JPEG
///
/// This uses either:
/// - macOS builtin `sips` program
/// - ImageMagick's `magick` command
///
/// Docs: <https://www.unix.com/man-page/osx/1/sips/> (or `man sips`)
fn convert_heic(from: &Path, to: &Path, converter: &ImageConverter) -> Option<()> {
    let (from_path, to_path) = ensure_paths(from, to)?;

    let args = match converter {
        ImageConverter::Sips => {
            vec!["-s", "format", "jpeg", from_path, "-o", to_path]
        }
        ImageConverter::Imagemagick => {
            vec![from_path, to_path]
        }
    };

    run_command(converter.name(), args)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_heic_detection() {
        // HEIC should be detected (case-insensitive)
        assert!(matches!(
            "image/heic".to_lowercase().as_str(),
            "image/heic" | "image/heif"
        ));
        assert!(matches!(
            "image/HEIC".to_lowercase().as_str(),
            "image/heic" | "image/heif"
        ));
        assert!(matches!(
            "image/heif".to_lowercase().as_str(),
            "image/heic" | "image/heif"
        ));

        // Other types should not match
        assert!(!matches!(
            "image/jpeg".to_lowercase().as_str(),
            "image/heic" | "image/heif"
        ));
        assert!(!matches!(
            "image/png".to_lowercase().as_str(),
            "image/heic" | "image/heif"
        ));
    }

    #[test]
    fn test_convert_if_needed_no_converter() {
        let from = PathBuf::from("/tmp/test.heic");
        let mut to = PathBuf::from("/tmp/test.heic");

        // No converter available
        let result = convert_if_needed(&from, &mut to, &None, "image/heic");

        // Should return false (no conversion)
        assert!(!result);
        // Path should be unchanged
        assert_eq!(to, PathBuf::from("/tmp/test.heic"));
    }

    #[test]
    fn test_convert_if_needed_non_heic() {
        let from = PathBuf::from("/tmp/test.jpg");
        let mut to = PathBuf::from("/tmp/test.jpg");

        // Even with converter, non-HEIC files shouldn't convert
        let converter = Some(ImageConverter::Sips);
        let result = convert_if_needed(&from, &mut to, &converter, "image/jpeg");

        // Should return false (no conversion needed)
        assert!(!result);
        // Path should be unchanged
        assert_eq!(to, PathBuf::from("/tmp/test.jpg"));
    }
}
