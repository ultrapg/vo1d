use anyhow::{Context, Result};
use std::path::Path;

/// File system operations tool.
pub struct FileOps;

impl FileOps {
    /// Read a file's contents with optional line range.
    pub fn read(path: &Path, start_line: Option<usize>, end_line: Option<usize>) -> Result<String> {
        let content = std::fs::read_to_string(path)
            .with_context(|| format!("Failed to read file: {}", path.display()))?;

        match (start_line, end_line) {
            (Some(start), Some(end)) => {
                let lines: Vec<&str> = content.lines().collect();
                let start = start.max(1).saturating_sub(1);
                let end = end.min(lines.len());
                Ok(lines[start..end].join("\n"))
            }
            (Some(start), None) => {
                let lines: Vec<&str> = content.lines().collect();
                let start = start.max(1).saturating_sub(1);
                Ok(lines[start..].join("\n"))
            }
            _ => {
                // Show file size info for large files
                if content.len() > 1_000_000 {
                    Ok(format!(
                        "[File too large: {} bytes. Showing first 1000 lines]\n{}",
                        content.len(),
                        content.lines().take(1000).collect::<Vec<_>>().join("\n")
                    ))
                } else {
                    Ok(content)
                }
            }
        }
    }

    /// Write content to a file (atomic write with backup).
    pub fn write(path: &Path, content: &str, append: bool) -> Result<String> {
        // Create parent directories
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create parent directories: {}", parent.display()))?;
        }

        if append {
            let mut file = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(path)
                .with_context(|| format!("Failed to open file for append: {}", path.display()))?;
            use std::io::Write;
            file.write_all(content.as_bytes())
                .with_context(|| format!("Failed to append to file: {}", path.display()))?;
            Ok(format!("Appended {} bytes to {}", content.len(), path.display()))
        } else {
            // Atomic write: write to temp, then rename
            let tmp_path = path.with_extension("tmp");
            std::fs::write(&tmp_path, content)
                .with_context(|| format!("Failed to write temporary file: {}", tmp_path.display()))?;
            std::fs::rename(&tmp_path, path)
                .with_context(|| format!("Failed to rename temporary file to: {}", path.display()))?;
            Ok(format!("Wrote {} bytes to {}", content.len(), path.display()))
        }
    }

    /// List directory contents with metadata.
    pub fn list_dir(path: &Path, pattern: Option<&str>) -> Result<String> {
        if !path.is_dir() {
            anyhow::bail!("'{}' is not a directory", path.display());
        }

        let mut entries: Vec<_> = std::fs::read_dir(path)
            .with_context(|| format!("Failed to read directory: {}", path.display()))?
            .filter_map(|e| e.ok())
            .collect();

        entries.sort_by_key(|e| e.file_name());

        let mut output = String::new();
        output.push_str(&format!("Directory listing for: {}\n\n", path.display()));
        output.push_str(&format!("{:<50} {:>10} {:<20} {}\n", "Name", "Size", "Modified", "Type"));
        output.push_str(&"-".repeat(100));
        output.push('\n');

        for entry in &entries {
            let name = entry.file_name().to_string_lossy().to_string();

            // Apply glob filter if specified
            if let Some(pat) = pattern {
                let glob = glob_match(&name, pat);
                if !glob {
                    continue;
                }
            }

            let metadata = match entry.metadata() {
                Ok(m) => m,
                Err(_) => continue,
            };

            let size = if metadata.is_dir() {
                "<DIR>".to_string()
            } else {
                let bytes = metadata.len();
                if bytes > 1_048_576 {
                    format!("{:.1} MB", bytes as f64 / 1_048_576.0)
                } else if bytes > 1024 {
                    format!("{:.1} KB", bytes as f64 / 1024.0)
                } else {
                    format!("{} B", bytes)
                }
            };

            let modified = metadata.modified()
                .map(|t| {
                    let dur = t.duration_since(std::time::SystemTime::UNIX_EPOCH).unwrap_or_default();
                    let secs = dur.as_secs();
                    let datetime = chrono::DateTime::from_timestamp(secs as i64, 0)
                        .map(|d| d.format("%Y-%m-%d %H:%M:%S").to_string())
                        .unwrap_or_default();
                    datetime
                })
                .unwrap_or_default();

            let file_type = if metadata.is_dir() { "dir" } else if metadata.is_symlink() { "link" } else { "file" };

            output.push_str(&format!("{:<50} {:>10} {:<20} {}\n", name, size, modified, file_type));
        }

        Ok(output)
    }

    /// Search for files matching a pattern.
    pub fn search(path: &Path, pattern: &str) -> Result<String> {
        let mut results = Vec::new();

        if pattern.contains('*') || pattern.contains('?') {
            // Glob search
            let walker = walkdir(path, true);
            for entry in walker {
                if let Ok(entry) = entry {
                    let name = entry.file_name();
                    let name_str = name.to_string_lossy();
                    if glob_match(&name_str, pattern) {
                        results.push(entry.path().to_path_buf());
                    }
                }
            }
        } else {
            // Name contains search (recursive)
            let walker = walkdir(path, true);
            for entry in walker {
                if let Ok(entry) = entry {
                    let name = entry.file_name().to_string_lossy().to_lowercase();
                    if name.contains(&pattern.to_lowercase()) {
                        results.push(entry.path().to_path_buf());
                    }
                }
            }
        }

        results.sort();

        if results.is_empty() {
            return Ok(format!("No files matching '{}' found in {}", pattern, path.display()));
        }

        let mut output = format!("Found {} results for '{}':\n\n", results.len(), pattern);
        for r in &results {
            let relative = r.strip_prefix(path).unwrap_or(r);
            output.push_str(&format!("  {}\n", relative.display()));
        }

        Ok(output)
    }

    /// Delete a file.
    pub fn delete(path: &Path) -> Result<String> {
        if !path.exists() {
            anyhow::bail!("File not found: {}", path.display());
        }
        if path.is_dir() {
            anyhow::bail!("'{}' is a directory. Refusing to delete non-empty directories.", path.display());
        }
        std::fs::remove_file(path)
            .with_context(|| format!("Failed to delete file: {}", path.display()))?;
        Ok(format!("Deleted: {}", path.display()))
    }

    /// Delete all files matching a glob pattern in the given directory.
    pub fn delete_matching(dir: &Path, pattern: &str) -> Result<String> {
        if !dir.is_dir() {
            anyhow::bail!("'{}' is not a directory", dir.display());
        }
        let mut deleted = Vec::new();
        let walker = walkdir(dir, true);
        for entry in walker {
            if let Ok(entry) = entry {
                if entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                    continue;
                }
                let name = entry.file_name().to_string_lossy().to_string();
                if glob_match(&name, pattern) {
                    match std::fs::remove_file(entry.path()) {
                        Ok(_) => {
                            let path = entry.path();
                            let relative = path.strip_prefix(dir).unwrap_or(&path);
                            deleted.push(relative.display().to_string());
                        }
                        Err(e) => {
                            deleted.push(format!("{} (error: {})", entry.path().display(), e));
                        }
                    }
                }
            }
        }
        if deleted.is_empty() {
            Ok(format!("No files matching '{}' found in {}", pattern, dir.display()))
        } else {
            let mut output = format!("Deleted {} file(s) matching '{}':\n", deleted.len(), pattern);
            for f in &deleted {
                output.push_str(&format!("  - {}\n", f));
            }
            Ok(output)
        }
    }

    /// Copy a file.
    pub fn copy(src: &Path, dst: &Path) -> Result<String> {
        if let Some(parent) = dst.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create parent directories: {}", parent.display()))?;
        }
        std::fs::copy(src, dst)
            .with_context(|| format!("Failed to copy {} to {}", src.display(), dst.display()))?;
        Ok(format!("Copied {} to {}", src.display(), dst.display()))
    }

    /// Create a directory (recursive).
    pub fn create_dir(path: &Path) -> Result<String> {
        std::fs::create_dir_all(path)
            .with_context(|| format!("Failed to create directory: {}", path.display()))?;
        Ok(format!("Created directory: {}", path.display()))
    }

    /// Get file metadata.
    pub fn metadata(path: &Path) -> Result<String> {
        let meta = std::fs::metadata(path)
            .with_context(|| format!("Failed to get metadata: {}", path.display()))?;

        let file_type = if meta.is_dir() { "directory" } else if meta.is_symlink() { "symlink" } else { "file" };
        let size = meta.len();
        let size_str = if size > 1_048_576 {
            format!("{:.2} MB", size as f64 / 1_048_576.0)
        } else if size > 1024 {
            format!("{:.2} KB", size as f64 / 1024.0)
        } else {
            format!("{} B", size)
        };

        let modified = meta.modified()
            .map(|t| {
                let dur = t.duration_since(std::time::SystemTime::UNIX_EPOCH).unwrap_or_default();
                chrono::DateTime::from_timestamp(dur.as_secs() as i64, 0)
                    .map(|d| d.to_rfc3339())
                    .unwrap_or_default()
            })
            .unwrap_or_default();

        let permissions = format!("{:o}", meta.permissions().mode());
        let readonly = meta.permissions().readonly();

        Ok(format!(
            "Path: {}\nType: {}\nSize: {} ({} bytes)\nModified: {}\nPermissions: {}\nReadonly: {}",
            path.display(), file_type, size_str, size, modified, permissions, readonly
        ))
    }
}

/// Simple glob matching (supports * and ?)
fn glob_match(name: &str, pattern: &str) -> bool {
    let pattern_chars: Vec<char> = pattern.chars().collect();
    let name_chars: Vec<char> = name.chars().collect();
    glob_match_recursive(&name_chars, 0, &pattern_chars, 0)
}

fn glob_match_recursive(name: &[char], ni: usize, pattern: &[char], pi: usize) -> bool {
    if pi == pattern.len() {
        return ni == name.len();
    }

    if pattern[pi] == '*' {
        // Try matching zero or more characters
        if glob_match_recursive(name, ni, pattern, pi + 1) {
            return true;
        }
        if ni < name.len() && glob_match_recursive(name, ni + 1, pattern, pi) {
            return true;
        }
        false
    } else if pattern[pi] == '?' {
        if ni < name.len() {
            glob_match_recursive(name, ni + 1, pattern, pi + 1)
        } else {
            false
        }
    } else if ni < name.len() && pattern[pi] == name[ni] {
        glob_match_recursive(name, ni + 1, pattern, pi + 1)
    } else {
        false
    }
}

/// Simple recursive directory walker.
fn walkdir(path: &Path, recursive: bool) -> Vec<std::io::Result<std::fs::DirEntry>> {
    let mut results = Vec::new();
    if let Ok(entries) = std::fs::read_dir(path) {
        for entry in entries {
            if let Ok(ref e) = entry {
                if recursive && e.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                    results.extend(walkdir(&e.path(), recursive));
                }
            }
            results.push(entry);
        }
    }
    results
}

/// Helper to extract Unix permission bits from std::fs::Permissions (Windows compatibility).
#[cfg(windows)]
trait PermissionsExt {
    fn mode(&self) -> u32;
}

#[cfg(windows)]
impl PermissionsExt for std::fs::Permissions {
    fn mode(&self) -> u32 {
        if self.readonly() { 0o444 } else { 0o644 }
    }
}

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_glob_match_exact() {
        assert!(glob_match("main.rs", "main.rs"));
        assert!(!glob_match("main.rs", "lib.rs"));
    }

    #[test]
    fn test_glob_match_wildcard() {
        assert!(glob_match("main.rs", "*.rs"));
        assert!(glob_match("lib.rs", "*.rs"));
        assert!(!glob_match("main.rs", "*.py"));
        assert!(glob_match("data.json", "*.json"));
    }

    #[test]
    fn test_glob_match_question_mark() {
        assert!(glob_match("foo.txt", "foo.???"));
        assert!(glob_match("foo.txt", "foo.tx?"));
        assert!(!glob_match("foo.txt", "foo.????"));
        assert!(!glob_match("foo.txt", "foo.?"));
    }

    #[test]
    fn test_glob_match_star_matches_multiple() {
        assert!(glob_match("abcdef", "a*f"));
        assert!(glob_match("abcdef", "a*"));
        assert!(glob_match("abcdef", "*f"));
        assert!(glob_match("", "*"));
    }

    #[test]
    fn test_glob_match_complex() {
        assert!(glob_match("src/main.rs", "src/*.rs"));
        assert!(glob_match("src/main.rs", "src/*"));
        assert!(!glob_match("src/main.rs", "src/*.py"));
        assert!(glob_match("test_utils.rs", "test_*.rs"));
    }

    #[test]
    fn test_glob_match_case_sensitive() {
        assert!(!glob_match("Main.Rs", "*.rs"));
        assert!(!glob_match("main.RS", "*.rs"));
    }

    #[test]
    fn test_walkdir_basic() {
        let dir = std::env::temp_dir().join(format!("vo1d_walkdir_test_{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        let _ = std::fs::write(dir.join("a.txt"), "a");
        let _ = std::fs::create_dir_all(dir.join("sub"));
        let _ = std::fs::write(dir.join("sub").join("b.txt"), "b");

        let entries = walkdir(&dir, true);
        let names: Vec<String> = entries.iter()
            .filter_map(|e| e.as_ref().ok())
            .map(|e| e.file_name().to_string_lossy().to_string())
            .collect();
        assert!(names.contains(&"a.txt".to_string()));
        assert!(names.contains(&"b.txt".to_string()));
        assert!(names.contains(&"sub".to_string()));

        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn test_walkdir_non_recursive() {
        let dir = std::env::temp_dir().join(format!("vo1d_walkdir_nr_{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        let _ = std::fs::write(dir.join("top.txt"), "top");
        let _ = std::fs::create_dir_all(dir.join("sub"));
        let _ = std::fs::write(dir.join("sub").join("deep.txt"), "deep");

        let entries = walkdir(&dir, false);
        let names: Vec<String> = entries.iter()
            .filter_map(|e| e.as_ref().ok())
            .map(|e| e.file_name().to_string_lossy().to_string())
            .collect();
        assert!(names.contains(&"top.txt".to_string()));
        assert!(!names.contains(&"deep.txt".to_string()));

        let _ = std::fs::remove_dir_all(dir);
    }
}
