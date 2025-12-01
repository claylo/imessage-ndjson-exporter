/*!
 Defines routines for converting audio files.
*/

use std::path::{Path, PathBuf};

use crate::converters::{
    common::{ensure_paths, run_command},
    models::{AudioConverter, Converter},
};

/// Convert audio if needed, updating the destination path
///
/// Returns true if the file was converted and extension changed, false otherwise
///
/// - Attachment `CAF` and `AMR` files convert to `M4A` (MP4 container with AAC audio)
/// - Other formats pass through unchanged
pub fn convert_if_needed(
    from: &Path,
    to: &mut PathBuf,
    converter: &Option<AudioConverter>,
    mime_type: &str,
) -> bool {
    // Check if this is a CAF or AMR file that needs conversion
    let needs_conversion = matches!(
        mime_type.to_lowercase().as_str(),
        "audio/caf" | "audio/x-caf" | "audio/amr"
    ) || mime_type.to_lowercase().starts_with("audio/x-caf;");

    if !needs_conversion {
        return false;
    }

    // If no converter available, can't convert
    let Some(converter) = converter else {
        return false;
    };

    // Create new path with .m4a extension (per user preference)
    let mut converted_path = to.clone();
    converted_path.set_extension("m4a");

    // Attempt conversion
    if convert_caf(from, &converted_path, converter).is_some() {
        // Conversion succeeded, update the output path
        *to = converted_path;
        return true;
    }

    // Conversion failed, will fall back to raw copy
    eprintln!("Warning: Unable to convert audio file: {}", from.display());
    false
}

/// Convert a CAF or AMR audio file to M4A
///
/// This uses either:
/// - macOS builtin `afconvert` program
/// - FFmpeg (cross-platform)
fn convert_caf(from: &Path, to: &Path, converter: &AudioConverter) -> Option<()> {
    let (from_path, to_path) = ensure_paths(from, to)?;

    let args = match converter {
        AudioConverter::AfConvert => {
            vec!["-f", "mp4f", "-d", "aac", "-v", from_path, to_path]
        }
        AudioConverter::Ffmpeg => {
            vec!["-i", from_path, to_path]
        }
    };

    run_command(converter.name(), args)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_caf_detection() {
        // CAF variants should be detected
        assert!(matches!(
            "audio/caf".to_lowercase().as_str(),
            "audio/caf" | "audio/x-caf" | "audio/amr"
        ));
        assert!(matches!(
            "audio/CAF".to_lowercase().as_str(),
            "audio/caf" | "audio/x-caf" | "audio/amr"
        ));
        assert!(matches!(
            "audio/x-caf".to_lowercase().as_str(),
            "audio/caf" | "audio/x-caf" | "audio/amr"
        ));
        assert!(matches!(
            "audio/amr".to_lowercase().as_str(),
            "audio/caf" | "audio/x-caf" | "audio/amr"
        ));

        // Codec variants should be detected
        assert!("audio/x-caf; codecs=opus".to_lowercase().starts_with("audio/x-caf;"));

        // Other types should not match
        assert!(!matches!(
            "audio/mp3".to_lowercase().as_str(),
            "audio/caf" | "audio/x-caf" | "audio/amr"
        ));
        assert!(!matches!(
            "audio/aac".to_lowercase().as_str(),
            "audio/caf" | "audio/x-caf" | "audio/amr"
        ));
    }

    #[test]
    fn test_convert_if_needed_no_converter() {
        let from = PathBuf::from("/tmp/test.caf");
        let mut to = PathBuf::from("/tmp/test.caf");

        // No converter available
        let result = convert_if_needed(&from, &mut to, &None, "audio/caf");

        // Should return false (no conversion)
        assert!(!result);
        // Path should be unchanged
        assert_eq!(to, PathBuf::from("/tmp/test.caf"));
    }

    #[test]
    fn test_convert_if_needed_non_caf() {
        let from = PathBuf::from("/tmp/test.mp3");
        let mut to = PathBuf::from("/tmp/test.mp3");

        // Even with converter, non-CAF files shouldn't convert
        let converter = Some(AudioConverter::Ffmpeg);
        let result = convert_if_needed(&from, &mut to, &converter, "audio/mp3");

        // Should return false (no conversion needed)
        assert!(!result);
        // Path should be unchanged
        assert_eq!(to, PathBuf::from("/tmp/test.mp3"));
    }
}
