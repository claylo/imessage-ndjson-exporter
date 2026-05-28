/*!
 Defines routines for converting video files.
*/

use std::path::{Path, PathBuf};

use crate::converters::{
    common::{ensure_paths, run_command},
    models::{Converter, VideoConverter},
};

/// Convert video if needed, updating the destination path
///
/// Returns true if the file was converted and extension changed, false otherwise
///
/// - Attachment `MOV` files convert to `MP4` (H.264 video, AAC audio)
/// - First attempts remuxing (fast, no re-encoding)
/// - Falls back to software re-encoding if remuxing fails
/// - Other formats pass through unchanged
pub fn convert_if_needed(
    from: &Path,
    to: &mut PathBuf,
    converter: &Option<VideoConverter>,
    mime_type: &str,
) -> bool {
    // Check if this is a MOV file that needs conversion
    let is_mov = matches!(
        mime_type.to_lowercase().as_str(),
        "video/mov" | "video/quicktime"
    );

    if !is_mov {
        return false;
    }

    // If no converter available, can't convert
    let Some(converter) = converter else {
        return false;
    };

    // Create new path with .mp4 extension
    let mut converted_path = to.clone();
    converted_path.set_extension("mp4");

    // Show progress message (per user requirement)
    eprintln!(
        "Converting video: {} → {}",
        from.display(),
        converted_path.display()
    );

    // Attempt conversion
    if convert_mov(from, &converted_path, converter).is_some() {
        // Conversion succeeded, update the output path
        *to = converted_path;
        return true;
    }

    // Conversion failed, will fall back to raw copy
    eprintln!("Warning: Unable to convert video file: {}", from.display());
    false
}

/// Convert a MOV video file to MP4
///
/// Two-stage strategy (software-only encoding):
/// 1. Try remuxing without re-encoding (fast, preserves quality)
/// 2. Fall back to software re-encoding with H.264 if remuxing fails
///
/// Uses FFmpeg for all operations
fn convert_mov(from: &Path, to: &Path, converter: &VideoConverter) -> Option<()> {
    let (from_path, to_path) = ensure_paths(from, to)?;

    // Stage 1: Try remuxing (container change only, no re-encoding)
    let remux_args = vec!["-i", from_path, "-c", "copy", "-f", "mp4", to_path];

    if run_command(converter.name(), remux_args).is_some() {
        return Some(());
    }

    // Stage 2: Remux failed, fall back to software re-encoding
    // -c:v libx264: H.264 video codec (software encoding)
    // -preset fast: Balance speed vs compression
    // -c:a copy: Copy audio stream unchanged
    // -movflags +faststart: Optimize for web playback (moov atom at beginning)
    let encode_args = vec![
        "-i",
        from_path,
        "-c:v",
        "libx264",
        "-preset",
        "fast",
        "-c:a",
        "copy",
        "-movflags",
        "+faststart",
        to_path,
    ];

    run_command(converter.name(), encode_args)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mov_detection() {
        // MOV variants should be detected
        assert!(matches!(
            "video/mov".to_lowercase().as_str(),
            "video/mov" | "video/quicktime"
        ));
        assert!(matches!(
            "video/MOV".to_lowercase().as_str(),
            "video/mov" | "video/quicktime"
        ));
        assert!(matches!(
            "video/quicktime".to_lowercase().as_str(),
            "video/mov" | "video/quicktime"
        ));

        // Other types should not match
        assert!(!matches!(
            "video/mp4".to_lowercase().as_str(),
            "video/mov" | "video/quicktime"
        ));
        assert!(!matches!(
            "video/avi".to_lowercase().as_str(),
            "video/mov" | "video/quicktime"
        ));
    }

    #[test]
    fn test_convert_if_needed_no_converter() {
        let from = PathBuf::from("/tmp/test.mov");
        let mut to = PathBuf::from("/tmp/test.mov");

        // No converter available
        let result = convert_if_needed(&from, &mut to, &None, "video/mov");

        // Should return false (no conversion)
        assert!(!result);
        // Path should be unchanged
        assert_eq!(to, PathBuf::from("/tmp/test.mov"));
    }

    #[test]
    fn test_convert_if_needed_non_mov() {
        let from = PathBuf::from("/tmp/test.mp4");
        let mut to = PathBuf::from("/tmp/test.mp4");

        // Even with converter, non-MOV files shouldn't convert
        let converter = Some(VideoConverter::Ffmpeg);
        let result = convert_if_needed(&from, &mut to, &converter, "video/mp4");

        // Should return false (no conversion needed)
        assert!(!result);
        // Path should be unchanged
        assert_eq!(to, PathBuf::from("/tmp/test.mp4"));
    }
}
