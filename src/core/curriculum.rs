use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Criteria for evaluating whether a task was completed successfully.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvaluationCriteria {
    /// Files that must exist
    #[serde(default)]
    pub check_file_exists: Option<Vec<String>>,
    /// File content checks: "path::expected text"
    #[serde(default)]
    pub check_file_content: Option<Vec<String>>,
    /// Directories that must exist
    #[serde(default)]
    pub check_directory_exists: Option<Vec<String>>,
    /// Command output checks: "command::expected text"
    #[serde(default)]
    pub check_command_output: Option<Vec<String>>,
}

/// A single task in the curriculum.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskDefinition {
    pub id: String,
    pub description: String,
    pub expected_outcome: String,
    pub evaluation: Option<EvaluationCriteria>,
    /// Optional shell commands to set up the environment before the task
    /// (e.g., create a broken project for the model to fix)
    #[serde(default)]
    pub setup: Option<Vec<String>>,
}

/// A curriculum loaded from a JSON file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Curriculum {
    pub name: String,
    pub description: String,
    pub tasks: Vec<TaskDefinition>,
}

impl Curriculum {
    pub fn load(path: impl AsRef<Path>) -> Result<Self> {
        let content = std::fs::read_to_string(path.as_ref())?;
        let curriculum: Curriculum = serde_json::from_str(&content)?;
        Ok(curriculum)
    }

    /// Try to load from disk by name, falling back to the binary-embedded copy.
    pub fn load_from_name(ctx: &crate::AppContext, name: &str) -> Result<Self> {
        // Try disk paths
        let disk_paths = [
            ctx.paths.curriculum_dir().join(format!("{}.json", name)),
            ctx.paths.root_dir().join("curriculum").join(format!("{}.json", name)),
        ];
        for p in &disk_paths {
            if p.exists() {
                return Curriculum::load(p);
            }
        }
        // Fall back to embedded
        match crate::core::embedded_curricula::get(name) {
            Some(json) => Ok(serde_json::from_str(json)?),
            None => anyhow::bail!("Curriculum '{}' not found on disk or embedded in binary", name),
        }
    }

    pub fn task_count(&self) -> usize {
        self.tasks.len()
    }
}

/// Result of evaluating a task.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvaluationResult {
    pub task_id: String,
    pub passed: bool,
    pub details: Vec<String>,
    pub outcome: String,
}

pub fn evaluate_task(task: &TaskDefinition, sandbox_dir: &Path) -> EvaluationResult {
    let mut passed = true;
    let mut details = Vec::new();

    if let Some(ref criteria) = task.evaluation {
        if let Some(ref paths) = criteria.check_file_exists {
            for path in paths {
                let full = sandbox_dir.join(path);
                if full.exists() && full.is_file() {
                    details.push(format!("File exists: {}", path));
                } else {
                    details.push(format!("FAIL: File not found: {}", path));
                    passed = false;
                }
            }
        }

        if let Some(ref checks) = criteria.check_file_content {
            for check in checks {
                let parts: Vec<&str> = check.splitn(2, "::").collect();
                if parts.len() == 2 {
                    let full = sandbox_dir.join(parts[0]);
                    if let Ok(content) = std::fs::read_to_string(&full) {
                        if content.contains(parts[1]) {
                            details.push(format!("File '{}' contains expected content", parts[0]));
                        } else {
                            details.push(format!("FAIL: File '{}' missing expected content", parts[0]));
                            passed = false;
                        }
                    } else {
                        details.push(format!("FAIL: Cannot read file '{}'", parts[0]));
                        passed = false;
                    }
                }
            }
        }

        if let Some(ref dirs) = criteria.check_directory_exists {
            for path in dirs {
                let full = sandbox_dir.join(path);
                if full.exists() && full.is_dir() {
                    details.push(format!("Directory exists: {}", path));
                } else {
                    details.push(format!("FAIL: Directory not found: {}", path));
                    passed = false;
                }
            }
        }

        if let Some(ref checks) = criteria.check_command_output {
            for check in checks {
                let parts: Vec<&str> = check.splitn(2, "::").collect();
                if parts.len() == 2 {
                    let cmd = parts[0];
                    let expected = parts[1];
                    let shell = if cfg!(windows) { "cmd.exe" } else { "sh" };
                    let arg = if cfg!(windows) { "/C" } else { "-c" };
                    if let Ok(output) = std::process::Command::new(shell)
                        .args(&[arg, cmd])
                        .current_dir(sandbox_dir)
                        .output()
                    {
                        let stdout = String::from_utf8_lossy(&output.stdout);
                        if stdout.contains(expected) || String::from_utf8_lossy(&output.stderr).contains(expected) {
                            details.push(format!("Command '{}' produced expected output", cmd));
                        } else {
                            details.push(format!("FAIL: Command '{}' did not produce expected output", cmd));
                            passed = false;
                        }
                    } else {
                        details.push(format!("FAIL: Cannot run command '{}'", cmd));
                        passed = false;
                    }
                }
            }
        }
    }

    if details.is_empty() {
        details.push("No automated evaluation criteria".to_string());
    }

    let outcome = if passed { "passed" } else { "failed" };
    EvaluationResult {
        task_id: task.id.clone(),
        passed,
        details,
        outcome: outcome.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_curriculum_load_nonexistent() {
        let result = Curriculum::load("nonexistent.json");
        assert!(result.is_err());
    }

    #[test]
    fn test_evaluate_task_no_criteria() {
        let task = TaskDefinition {
            id: "test".to_string(),
            description: "Do something".to_string(),
            expected_outcome: "Done".to_string(),
            evaluation: None,
            setup: None,
        };
        let result = evaluate_task(&task, Path::new("."));
        assert!(result.passed);
        assert!(result.details.iter().any(|d| d.contains("No automated evaluation")));
    }

    #[test]
    fn test_evaluate_task_file_exists() {
        let dir = tempfile::tempdir().unwrap();
        let file_path = dir.path().join("test.txt");
        std::fs::write(&file_path, "hello").unwrap();

        let task = TaskDefinition {
            id: "test".to_string(),
            description: "Create test.txt".to_string(),
            expected_outcome: "File created".to_string(),
            evaluation: Some(EvaluationCriteria {
                check_file_exists: Some(vec!["test.txt".to_string()]),
                check_file_content: None,
                check_directory_exists: None,
                check_command_output: None,
            }),
            setup: None,
        };
        let result = evaluate_task(&task, dir.path());
        assert!(result.passed);
    }

    #[test]
    fn test_evaluate_task_file_missing() {
        let dir = tempfile::tempdir().unwrap();
        let task = TaskDefinition {
            id: "test".to_string(),
            description: "Create missing.txt".to_string(),
            expected_outcome: "File created".to_string(),
            evaluation: Some(EvaluationCriteria {
                check_file_exists: Some(vec!["missing.txt".to_string()]),
                check_file_content: None,
                check_directory_exists: None,
                check_command_output: None,
            }),
            setup: None,
        };
        let result = evaluate_task(&task, dir.path());
        assert!(!result.passed);
    }

    #[test]
    fn test_evaluate_task_file_content() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("hello.txt"), "Hello, World!").unwrap();

        let task = TaskDefinition {
            id: "test".to_string(),
            description: "Write hello.txt".to_string(),
            expected_outcome: "File with content".to_string(),
            evaluation: Some(EvaluationCriteria {
                check_file_exists: None,
                check_file_content: Some(vec!["hello.txt::World".to_string()]),
                check_directory_exists: None,
                check_command_output: None,
            }),
            setup: None,
        };
        let result = evaluate_task(&task, dir.path());
        assert!(result.passed);
    }

    #[test]
    fn test_evaluate_task_directory_exists() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join("subdir")).unwrap();

        let task = TaskDefinition {
            id: "test".to_string(),
            description: "Create subdir".to_string(),
            expected_outcome: "Dir created".to_string(),
            evaluation: Some(EvaluationCriteria {
                check_file_exists: None,
                check_file_content: None,
                check_directory_exists: Some(vec!["subdir".to_string()]),
                check_command_output: None,
            }),
            setup: None,
        };
        let result = evaluate_task(&task, dir.path());
        assert!(result.passed);
    }

    #[test]
    fn test_curriculum_deserialize() {
        let json = r#"{
            "name": "Test",
            "description": "A test curriculum",
            "tasks": [
                {"id": "t1", "description": "Task 1", "expected_outcome": "Done"}
            ]
        }"#;
        let c: Curriculum = serde_json::from_str(json).unwrap();
        assert_eq!(c.name, "Test");
        assert_eq!(c.tasks.len(), 1);
        assert_eq!(c.tasks[0].id, "t1");
    }
}
