/*!
 Defines routines for converting sticker image files.
*/

use std::fs::{create_dir_all, read_dir, remove_dir_all};
use std::path::{Path, PathBuf};

use crate::converters::{
    common::{ensure_paths, run_command},
    models::{Converter, ImageConverter, VideoConverter},
};

/// Convert sticker if needed, updating the destination path
///
/// Returns true if the file was converted and extension changed, false otherwise
///
/// - Sticker `HEIC` files convert to `PNG` (supports transparency)
/// - Sticker `HEICS` (HEIC sequence) files convert to `GIF` (animated)
/// - Other formats pass through unchanged
pub fn convert_if_needed(
    from: &Path,
    to: &mut PathBuf,
    image_converter: &Option<ImageConverter>,
    video_converter: &Option<VideoConverter>,
    mime_type: &str,
) -> bool {
    // Determine if this needs conversion and what type
    let output_ext = match mime_type.to_lowercase().as_str() {
        "image/heic" | "image/heif" => "png",
        "image/heics" | "image/heic-sequence" => "gif",
        _ => return false,
    };

    // For animated stickers (HEICS → GIF), we need ffmpeg
    if output_ext == "gif" {
        if let Some(converter) = video_converter {
            let mut converted_path = to.clone();
            converted_path.set_extension("gif");

            if convert_heics(from, &converted_path, converter).is_some() {
                *to = converted_path;
                return true;
            }
            eprintln!(
                "Warning: Unable to convert HEICS sticker: {}",
                from.display()
            );
        }
        return false;
    }

    // For static stickers (HEIC → PNG), we need sips/imagemagick
    if output_ext == "png" {
        if let Some(converter) = image_converter {
            let mut converted_path = to.clone();
            converted_path.set_extension("png");

            if convert_heic(from, &converted_path, converter).is_some() {
                *to = converted_path;
                return true;
            }
            eprintln!(
                "Warning: Unable to convert HEIC sticker: {}",
                from.display()
            );
        }
        return false;
    }

    false
}

/// Convert a HEIC sticker file to PNG
///
/// Sticker HEIC files contain 5 images: 320x320, 160x160, 96x96, 64x64, and 40x40
/// We extract only the highest resolution (first image).
fn convert_heic(from: &Path, to: &Path, converter: &ImageConverter) -> Option<()> {
    let (from_path, to_path) = ensure_paths(from, to)?;

    match converter {
        ImageConverter::Sips => {
            let args = vec!["-s", "format", "png", from_path, "-o", to_path];
            run_command(converter.name(), args)
        }
        ImageConverter::Imagemagick => {
            // Extract only the first (highest resolution) image
            let formatted_from = format!("{from_path}[0]");
            let args = vec![&formatted_from, to_path];
            run_command(converter.name(), args)
        }
    }
}

/// Convert a HEICS (HEIC sequence) animated sticker to GIF
///
/// HEICS files contain 4 video streams:
/// - Stream 0: First still frame
/// - Stream 1: Alpha mask for first still
/// - Stream 2: Video data (animation frames)
/// - Stream 3: Alpha masks for animation frames
///
/// This function extracts all frames, applies transparency masks, and creates an animated GIF.
fn convert_heics(from: &Path, to: &Path, video_converter: &VideoConverter) -> Option<()> {
    let (from_path, to_path) = ensure_paths(from, to)?;

    // Frames per second in the original sticker (Apple standard)
    let fps = 10;

    // Directory to store intermediate renders
    let tmp_path = PathBuf::from("/tmp/imessage-ndjson");
    if !tmp_path.exists()
        && let Err(why) = create_dir_all(&tmp_path)
    {
        eprintln!("Unable to create {}: {why}", tmp_path.display());
        return None;
    }
    let tmp = tmp_path.to_str()?;

    match video_converter {
        VideoConverter::Ffmpeg => {
            // Extract video frames (stream 2)
            run_command(
                video_converter.name(),
                vec![
                    "-i",
                    from_path,
                    "-map",
                    "0:2",
                    "-y",
                    &format!("{tmp}/frame_%04d.png"),
                ],
            )?;

            // Extract alpha masks (stream 3)
            run_command(
                video_converter.name(),
                vec![
                    "-i",
                    from_path,
                    "-map",
                    "0:3",
                    "-y",
                    &format!("{tmp}/alpha_%04d.png"),
                ],
            )?;

            // Apply transparency masks to frames
            let files = read_dir(tmp).ok()?;
            let num_frames = files.into_iter().count() / 2;
            (0..num_frames).try_for_each(|item| {
                run_command(
                    video_converter.name(),
                    vec![
                        "-i",
                        &format!("{tmp}/frame_{item:04}.png"),
                        "-i",
                        &format!("{tmp}/alpha_{item:04}.png"),
                        "-filter_complex",
                        "[1:v]format=gray,geq=lum='p(X,Y)':a='p(X,Y)'[mask];[0:v][mask]alphamerge",
                        &format!("{tmp}/merged_{item:04}.png"),
                    ],
                )
            })?;

            // Generate transparency palette from first frame
            run_command(
                video_converter.name(),
                vec![
                    "-i",
                    &format!("{tmp}/merged_0001.png"),
                    "-vf",
                    "palettegen=reserve_transparent=1",
                    &format!("{tmp}/palette.png"),
                ],
            )?;

            // Create the final GIF
            run_command(
                video_converter.name(),
                vec![
                    "-i",
                    &format!("{tmp}/merged_%04d.png"),
                    "-i",
                    &format!("{tmp}/palette.png"),
                    "-lavfi",
                    &format!("fps={fps},paletteuse=alpha_threshold=128"),
                    "-gifflags",
                    "-offsetting",
                    to_path,
                ],
            )?;

            // Clean up temporary files
            remove_dir_all(tmp).ok()?;

            Some(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_heic_sticker_detection() {
        assert!(matches!(
            "image/heic".to_lowercase().as_str(),
            "image/heic" | "image/heif"
        ));
    }

    #[test]
    fn test_heics_detection() {
        assert!(matches!(
            "image/heics".to_lowercase().as_str(),
            "image/heics" | "image/heic-sequence"
        ));
        assert!(matches!(
            "image/heic-sequence".to_lowercase().as_str(),
            "image/heics" | "image/heic-sequence"
        ));
    }

    #[test]
    fn test_convert_if_needed_no_converter() {
        let from = PathBuf::from("/tmp/test.heic");
        let mut to = PathBuf::from("/tmp/test.heic");

        let result = convert_if_needed(&from, &mut to, &None, &None, "image/heic");
        assert!(!result);
    }
}
