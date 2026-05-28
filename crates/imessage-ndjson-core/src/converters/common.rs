use std::fs;
use std::path::Path;
use std::process::{Command, Stdio};

/// Execute external command silently
///
/// Returns Some(()) on success, None on failure
pub fn run_command(command: &str, args: Vec<&str>) -> Option<()> {
    Command::new(command)
        .args(&args)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .ok()
        .filter(|status| status.success())
        .map(|_| ())
}

/// Ensure paths are valid strings and create parent directory if needed
///
/// Returns None if path conversion fails or directory creation fails
pub fn ensure_paths<'a>(from: &'a Path, to: &'a Path) -> Option<(&'a str, &'a str)> {
    let from_str = from.to_str()?;
    let to_str = to.to_str()?;

    // Create parent directory if it doesn't exist
    if let Some(parent) = to.parent() {
        fs::create_dir_all(parent).ok()?;
    }

    Some((from_str, to_str))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_run_command_success() {
        // 'echo' should succeed on all platforms
        #[cfg(not(target_family = "windows"))]
        {
            let result = run_command("echo", vec!["test"]);
            assert!(result.is_some());
        }
    }

    #[test]
    fn test_run_command_failure() {
        // Non-existent command should fail
        let result = run_command("fake_nonexistent_command_xyz", vec![]);
        assert!(result.is_none());
    }

    #[test]
    fn test_ensure_paths() {
        let from = PathBuf::from("/tmp/source.txt");
        let to = PathBuf::from("/tmp/dest.txt");

        let result = ensure_paths(&from, &to);
        assert!(result.is_some());

        let (from_str, to_str) = result.unwrap();
        assert_eq!(from_str, "/tmp/source.txt");
        assert_eq!(to_str, "/tmp/dest.txt");
    }
}
