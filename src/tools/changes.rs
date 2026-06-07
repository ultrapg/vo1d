use anyhow::Result;
use std::path::Path;
use std::process::Command;

pub struct Changes;

impl Changes {
    pub fn show(path: Option<&str>) -> Result<String> {
        let cwd = std::env::current_dir()?;
        let workspace = if let Some(p) = path {
            Path::new(p).to_path_buf()
        } else {
            cwd.clone()
        };

        let mut output = String::new();

        // Try git diff
        let git_diff = Command::new("git")
            .args(["diff", "--stat"])
            .current_dir(&workspace)
            .output();
        if let Ok(out) = git_diff {
            if out.status.success() {
                let stdout = String::from_utf8_lossy(&out.stdout);
                if !stdout.trim().is_empty() {
                    output.push_str("### Changed Files (git diff --stat)\n");
                    output.push_str(&stdout);
                    output.push('\n');

                    let git_diff_full = Command::new("git")
                        .args(["diff"])
                        .current_dir(&workspace)
                        .output();
                    if let Ok(full) = git_diff_full {
                        if full.status.success() {
                            let full_out = String::from_utf8_lossy(&full.stdout);
                            if !full_out.trim().is_empty() {
                                output.push_str("### Full Diff\n");
                                output.push_str(&full_out);
                            }
                        }
                    }
                    return Ok(output);
                }
            }
        }

        // No git or no changes — list recently modified files
        output.push_str("### Recently Modified Files\n");
        let entries = match std::fs::read_dir(&workspace) {
            Ok(e) => e,
            Err(_) => return Ok("No changes found.".to_string()),
        };

        let mut files: Vec<_> = Vec::new();
        for entry in entries.flatten() {
            if let Ok(meta) = entry.metadata() {
                if meta.is_file() {
                    if let Ok(modified) = meta.modified() {
                        if let Ok(elapsed) = modified.elapsed() {
                            if elapsed.as_secs() < 3600 {
                                files.push(entry.path().display().to_string());
                            }
                        }
                    }
                }
            }
        }

        if files.is_empty() {
            output.push_str("No recently modified files found.\n");
        } else {
            for f in &files {
                output.push_str(&format!("  {}\n", f));
            }
        }

        Ok(output)
    }

    pub fn restore(path: &str) -> Result<String> {
        let workspace = std::env::current_dir()?;
        let target = Path::new(path);

        // Try git restore first
        let git_restore = Command::new("git")
            .args(["checkout", "--", path])
            .current_dir(&workspace)
            .output();
        if let Ok(out) = git_restore {
            if out.status.success() {
                return Ok(format!("Restored '{}' from git.", path));
            }
            let stderr = String::from_utf8_lossy(&out.stderr);
            if !stderr.trim().is_empty() {
                // Try git restore as alternative
                let git_restore2 = Command::new("git")
                    .args(["restore", path])
                    .current_dir(&workspace)
                    .output();
                if let Ok(out2) = git_restore2 {
                    if out2.status.success() {
                        return Ok(format!("Restored '{}' from git.", path));
                    }
                }
                if target.exists() {
                    return Err(anyhow::anyhow!("Could not restore '{}': {}. File exists but is not tracked by git.", path, stderr.trim()));
                }
                return Err(anyhow::anyhow!("Could not restore '{}': {}", path, stderr.trim()));
            }
        }

        if target.exists() {
            Err(anyhow::anyhow!("File '{}' exists but no git repository found. Cannot restore.", path))
        } else {
            Err(anyhow::anyhow!("File '{}' does not exist and no git repository found.", path))
        }
    }
}
