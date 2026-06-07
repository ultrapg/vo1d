use std::collections::HashMap;
use std::path::Path;

/// Loads and provides markdown documentation for the system prompt.
pub struct DocProvider {
    docs: HashMap<String, String>,
}

impl DocProvider {
    /// Load markdown docs from a directory. Non-existent dir = empty provider.
    pub fn load(doc_dir: &Path) -> Self {
        let mut docs = HashMap::new();
        if !doc_dir.exists() {
            return Self { docs };
        }
        if let Ok(entries) = std::fs::read_dir(doc_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().map(|e| e == "md").unwrap_or(false) {
                    if let Some(name) = path.file_stem().and_then(|s| s.to_str()) {
                        if let Ok(content) = std::fs::read_to_string(&path) {
                            docs.insert(name.to_string(), content);
                        }
                    }
                }
            }
        }
        Self { docs }
    }

    /// Get a specific doc by its filename stem (without .md).
    pub fn get(&self, name: &str) -> Option<&str> {
        self.docs.get(name).map(|s| s.as_str())
    }

    /// Get all docs concatenated into one string.
    pub fn to_context_string(&self) -> String {
        if self.docs.is_empty() {
            return String::new();
        }
        let mut out = String::from("\n\n--- REFERENCE DOCUMENTATION ---\n\n");
        // Sort for deterministic output
        let mut names: Vec<&String> = self.docs.keys().collect();
        names.sort();
        for name in names {
            if let Some(content) = self.docs.get(name) {
                out.push_str(content);
                out.push_str("\n\n---\n\n");
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_nonexistent_dir() {
        let dp = DocProvider::load(Path::new("nonexistent_dir_xyz"));
        assert!(dp.get("anything").is_none());
        assert!(dp.to_context_string().is_empty());
    }

    #[test]
    fn test_load_real_docs() {
        let dp = DocProvider::load(Path::new("docs"));
        // If docs dir exists, we should have at least the docs we created
        let file_ops = dp.get("file-ops");
        assert!(file_ops.is_some(), "file-ops.md should be loaded");
        assert!(file_ops.unwrap().contains("read_file"));
    }

    #[test]
    fn test_context_string_format() {
        let dp = DocProvider::load(Path::new("docs"));
        let ctx = dp.to_context_string();
        if dp.docs.len() > 0 {
            assert!(ctx.starts_with("\n\n--- REFERENCE DOCUMENTATION ---"));
        }
    }
}
