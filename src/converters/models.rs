use std::process::Command;

/// Trait for format converters
pub trait Converter {
    /// Determine the converter type for the current environment
    /// Returns None if no suitable converter is found
    fn determine() -> Option<Self>
    where
        Self: Sized;

    /// The name of the program this variant represents
    fn name(&self) -> &'static str;
}

/// Image converter options
#[derive(Debug, Clone)]
pub enum ImageConverter {
    /// macOS builtin image converter
    Sips,
    /// ImageMagick (cross-platform)
    Imagemagick,
}

/// Video converter options
#[derive(Debug, Clone)]
pub enum VideoConverter {
    /// FFmpeg (required)
    Ffmpeg,
}

/// Audio converter options
#[derive(Debug, Clone)]
pub enum AudioConverter {
    /// macOS builtin audio converter
    AfConvert,
    /// FFmpeg (cross-platform)
    Ffmpeg,
}

/// Check if a command exists on the system
#[cfg(not(target_family = "windows"))]
fn exists(name: &str) -> bool {
    Command::new("which")
        .arg(name)
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

/// Windows version of exists check
#[cfg(target_family = "windows")]
fn exists(name: &str) -> bool {
    Command::new("where")
        .arg(name)
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

impl Converter for ImageConverter {
    fn determine() -> Option<Self> {
        if exists("sips") {
            Some(ImageConverter::Sips)
        } else if exists("magick") {
            Some(ImageConverter::Imagemagick)
        } else {
            None
        }
    }

    fn name(&self) -> &'static str {
        match self {
            ImageConverter::Sips => "sips",
            ImageConverter::Imagemagick => "magick",
        }
    }
}

impl Converter for VideoConverter {
    fn determine() -> Option<Self> {
        if exists("ffmpeg") {
            Some(VideoConverter::Ffmpeg)
        } else {
            None
        }
    }

    fn name(&self) -> &'static str {
        match self {
            VideoConverter::Ffmpeg => "ffmpeg",
        }
    }
}

impl Converter for AudioConverter {
    fn determine() -> Option<Self> {
        if exists("afconvert") {
            Some(AudioConverter::AfConvert)
        } else if exists("ffmpeg") {
            Some(AudioConverter::Ffmpeg)
        } else {
            None
        }
    }

    fn name(&self) -> &'static str {
        match self {
            AudioConverter::AfConvert => "afconvert",
            AudioConverter::Ffmpeg => "ffmpeg",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_exists_command() {
        // 'ls' should exist on Unix systems
        #[cfg(not(target_family = "windows"))]
        assert!(exists("ls"));

        // Fake command should not exist
        assert!(!exists("fake_nonexistent_command_xyz_123"));
    }

    #[test]
    fn test_converter_determination() {
        // These tests check that determination doesn't panic
        // Actual availability depends on system
        let _ = ImageConverter::determine();
        let _ = VideoConverter::determine();
        let _ = AudioConverter::determine();
    }

    #[test]
    fn test_converter_names() {
        assert_eq!(ImageConverter::Sips.name(), "sips");
        assert_eq!(ImageConverter::Imagemagick.name(), "magick");
        assert_eq!(VideoConverter::Ffmpeg.name(), "ffmpeg");
        assert_eq!(AudioConverter::AfConvert.name(), "afconvert");
        assert_eq!(AudioConverter::Ffmpeg.name(), "ffmpeg");
    }
}
