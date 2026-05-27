use std::collections::HashMap;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};

use hex;
use sha2::{Digest, Sha256};

/// Manages avatar file copying with hash-based deduplication
pub struct AvatarManager {
    /// Base output directory
    output_dir: PathBuf,

    /// Subdirectory for avatars (always "avatars")
    avatars_subdir: String,

    /// Cache mapping: hash -> copied path (for deduplication)
    hash_cache: HashMap<String, PathBuf>,
}

impl AvatarManager {
    /// Create a new avatar manager
    pub fn new(output_dir: &Path) -> Self {
        Self {
            output_dir: output_dir.to_path_buf(),
            avatars_subdir: "avatars".to_string(),
            hash_cache: HashMap::new(),
        }
    }

    /// Copy an avatar file, returning relative path or None if file doesn't exist
    ///
    /// This method:
    /// 1. Checks if the source file exists
    /// 2. Computes SHA256 hash of the file contents
    /// 3. Checks the deduplication cache
    /// 4. Copies the file if not already cached
    /// 5. Returns relative path from output_dir
    pub fn copy_avatar(&mut self, source_path: &Path) -> Option<String> {
        // 1. Check if file exists
        if !source_path.exists() {
            return None;
        }

        // 2. Compute SHA256 hash
        let hash = match self.compute_hash(source_path) {
            Ok(h) => h,
            Err(_) => return None,
        };

        // 3. Check deduplication cache
        if let Some(cached_path) = self.hash_cache.get(&hash) {
            // File already copied, return relative path
            return Some(self.make_relative_path(cached_path));
        }

        // 4. Determine file extension
        let extension = source_path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("jpg");

        // 5. Build destination path
        // Format: {output_dir}/avatars/{hash_prefix}.{ext}
        let hash_prefix = &hash[..16]; // First 16 chars (64 bits entropy)
        let dest_dir = self.output_dir.join(&self.avatars_subdir);
        let dest_path = dest_dir.join(format!("{}.{}", hash_prefix, extension));

        // 6. Create directory if needed
        if fs::create_dir_all(&dest_dir).is_err() {
            return None;
        }

        // 7. Copy file
        if fs::copy(source_path, &dest_path).is_err() {
            return None;
        }

        // 8. Update cache
        let relative_path = self.make_relative_path(&dest_path);
        self.hash_cache.insert(hash, dest_path);

        Some(relative_path)
    }

    /// Compute SHA256 hash of a file
    fn compute_hash(&self, path: &Path) -> Result<String, std::io::Error> {
        let mut file = fs::File::open(path)?;
        let mut hasher = Sha256::new();
        let mut buffer = vec![0; 8192];

        loop {
            let bytes_read = file.read(&mut buffer)?;
            if bytes_read == 0 {
                break;
            }
            hasher.update(&buffer[..bytes_read]);
        }

        Ok(hex::encode(hasher.finalize()))
    }

    /// Convert absolute path to relative path from output_dir
    fn make_relative_path(&self, path: &Path) -> String {
        path.strip_prefix(&self.output_dir)
            .ok()
            .and_then(|p| p.to_str())
            .unwrap_or("")
            .to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_relative_path_conversion() {
        let manager = AvatarManager::new(Path::new("/tmp/export"));
        let abs_path = PathBuf::from("/tmp/export/avatars/a3f2c8d9.jpg");
        let rel_path = manager.make_relative_path(&abs_path);
        assert_eq!(rel_path, "avatars/a3f2c8d9.jpg");
    }
}
