use crate::models::plan::{Plan, PlanStep, StepStatus};
use std::path::Path;

/// Parses a markdown PLAN.md file into a structured Plan.
pub struct PlanParser;

impl PlanParser {
    /// Parse PLAN.md content into a Plan struct.
    /// Handles various markdown formats the LLM might produce.
    pub fn parse(goal: &str, content: &str) -> Plan {
        let steps = Self::extract_steps(content);
        Plan {
            goal: goal.to_string(),
            steps,
            variables: std::collections::HashMap::new(),
        }
    }

    /// Read and parse a PLAN.md file from disk.
    pub fn from_file(path: &Path, goal: &str) -> Result<Plan, String> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| format!("Cannot read PLAN.md: {}", e))?;
        Ok(Self::parse(goal, &content))
    }

    /// Check if a PLAN.md file exists and parse it.
    pub fn try_from_workspace(workspace: &Path, goal: &str) -> Option<Plan> {
        let path = workspace.join("PLAN.md");
        if path.exists() {
            Self::from_file(&path, goal).ok()
        } else {
            None
        }
    }

    fn extract_steps(content: &str) -> Vec<PlanStep> {
        let lines: Vec<&str> = content.lines().collect();
        let mut steps: Vec<PlanStep> = Vec::new();
        let mut i = 0;

        while i < lines.len() {
            let line = lines[i].trim();

            // Detect step headers: ## Step N: or ### Step N:
            if line.starts_with("## ") || line.starts_with("### ") {
                let header_text = line.trim_start_matches(|c: char| c == '#' || c == ' ')
                    .trim();

                // Gather content until next header or end
                let mut step_lines: Vec<&str> = Vec::new();
                i += 1;
                while i < lines.len() {
                    let next = lines[i].trim();
                    if next.starts_with("## ") || next.starts_with("### ") {
                        break;
                    }
                    step_lines.push(next);
                    i += 1;
                }

                if let Some(step) = Self::build_step(header_text, &step_lines, steps.len() as u32) {
                    steps.push(step);
                }
                continue;
            }

            // Fallback: numbered list items "N. description"
            if let Some(desc) = Self::match_numbered_item(line) {
                let mut step_lines: Vec<&str> = Vec::new();
                i += 1;
                // Gather subsequent non-header, non-numbered lines
                while i < lines.len() {
                    let next = lines[i].trim();
                    if next.starts_with("## ") || next.starts_with("### ")
                        || Self::match_numbered_item(next).is_some()
                    {
                        break;
                    }
                    step_lines.push(next);
                    i += 1;
                }

                steps.push(PlanStep {
                    id: steps.len() as u32,
                    description: desc.to_string(),
                    action: Self::extract_action(&step_lines).unwrap_or_else(|| "execute_command".to_string()),
                    command: Self::extract_command(&step_lines),
                    depends_on: Self::extract_deps(&step_lines),
                    status: Self::extract_status(&step_lines),
                    result: None,
                    error: None,
                    retry_count: 0,
                });
                continue;
            }

            i += 1;
        }

        // If no structured steps found, treat the whole content as one step
        if steps.is_empty() {
            steps.push(PlanStep {
                id: 0,
                description: Self::first_sentence(content),
                action: "execute_command".to_string(),
                command: None,
                depends_on: vec![],
                status: StepStatus::Pending,
                result: None,
                error: None,
                retry_count: 0,
            });
        }

        steps
    }

    fn build_step(header: &str, lines: &[&str], id: u32) -> Option<PlanStep> {
        // Extract step number and description from header like "Step 1: Read files"
        let description = header.splitn(2, ':')
            .nth(1)
            .map(|s| s.trim())
            .unwrap_or(header)
            .to_string();

        // If header doesn't look like a step, skip
        if !header.to_lowercase().contains("step") && lines.is_empty() {
            return None;
        }

        Some(PlanStep {
            id,
            description,
            action: Self::extract_action(lines).unwrap_or_else(|| "execute_command".to_string()),
            command: Self::extract_command(lines),
            depends_on: Self::extract_deps(lines),
            status: Self::extract_status(lines),
            result: None,
            error: None,
            retry_count: 0,
        })
    }

    fn match_numbered_item(line: &str) -> Option<&str> {
        // Match "1. description" or "1) description"
        let line = line.trim();
        let re = regex::Regex::new(r"^\d+[\.\)]\s+(.*)").ok()?;
        re.captures(line)
            .and_then(|c| c.get(1))
            .map(|m| m.as_str().trim())
    }

    fn extract_action(lines: &[&str]) -> Option<String> {
        for line in lines {
            let lower = line.to_lowercase();
            if lower.starts_with("action:") || lower.starts_with("- action:") || lower.starts_with("* action:") {
                let action = line.splitn(2, ':')
                    .nth(1)
                    .map(|s| s.trim().trim_matches('*').trim())
                    .unwrap_or("");
                if !action.is_empty() {
                    return Some(action.to_string());
                }
            }
            // Also match **Action:** bold markdown
            if lower.contains("**action:**") {
                let action = line.splitn(2, "**action:**")
                    .nth(1)
                    .or_else(|| line.splitn(2, "**Action:**").nth(1))
                    .map(|s| s.trim().trim_matches('*').trim())
                    .unwrap_or("");
                if !action.is_empty() {
                    return Some(action.to_string());
                }
            }
        }
        // Guess action from description keywords
        let joined = lines.join(" ").to_lowercase();
        if joined.contains("read") || joined.contains("view") || joined.contains("inspect") {
            Some("read_file".to_string())
        } else if joined.contains("edit") || joined.contains("write") || joined.contains("create")
            || joined.contains("fix") || joined.contains("update")
        {
            Some("write_file".to_string())
        } else if joined.contains("run") || joined.contains("compile") || joined.contains("build")
            || joined.contains("execute") || joined.contains("check")
        {
            Some("execute_command".to_string())
        } else if joined.contains("search") || joined.contains("find") || joined.contains("list") {
            Some("search_files".to_string())
        } else if joined.contains("delete") || joined.contains("remove") {
            Some("delete_file".to_string())
        } else if joined.contains("copy") {
            Some("copy_file".to_string())
        } else if joined.contains("mkdir") || joined.contains("create directory") {
            Some("create_directory".to_string())
        } else if joined.contains("search web") || joined.contains("look up") || joined.contains("research") {
            Some("web_search".to_string())
        } else {
            None
        }
    }

    fn extract_command(lines: &[&str]) -> Option<String> {
        for line in lines {
            let lower = line.to_lowercase();
            if lower.starts_with("command:") || lower.starts_with("- command:") || lower.starts_with("* command:") {
                return line.splitn(2, ':')
                    .nth(1)
                    .map(|s| s.trim().trim_matches('`').trim().to_string())
                    .filter(|s| !s.is_empty());
            }
        }
        None
    }

    fn extract_deps(lines: &[&str]) -> Vec<u32> {
        for line in lines {
            let lower = line.to_lowercase();
            if lower.contains("depends on") || lower.contains("depends_on") {
                let deps_str = line.splitn(2, ':')
                    .nth(1)
                    .unwrap_or("");
                return deps_str.split(|c: char| c == ',' || c == '&')
                    .filter_map(|d| {
                        let trimmed = d.trim().to_lowercase();
                        // Extract number from "step 1", "step 2", "1", etc.
                        let num: String = trimmed.chars().filter(|c| c.is_ascii_digit()).collect();
                        num.parse::<u32>().ok().map(|n| n.saturating_sub(1)) // convert to 0-indexed
                    })
                    .collect();
            }
        }
        vec![]
    }

    fn extract_status(lines: &[&str]) -> StepStatus {
        for line in lines {
            if line.contains("- [x]") || line.contains("- [X]") || line.contains("* [x]") || line.contains("* [X]") {
                return StepStatus::Completed;
            }
            if line.contains("- [ ]") || line.contains("* [ ]") {
                return StepStatus::Pending;
            }
        }
        // Check for status key
        for line in lines {
            let lower = line.to_lowercase();
            if lower.contains("status:") && (lower.contains("completed") || lower.contains("done")) {
                return StepStatus::Completed;
            }
            if lower.contains("status:") && (lower.contains("pending") || lower.contains("todo")) {
                return StepStatus::Pending;
            }
            if lower.contains("status:") && lower.contains("failed") {
                return StepStatus::Failed;
            }
        }
        StepStatus::Pending
    }

    fn first_sentence(text: &str) -> String {
        text.lines()
            .find(|l| {
                let t = l.trim();
                !t.is_empty() && !t.starts_with('#') && !t.starts_with('-')
            })
            .map(|l| {
                let t = l.trim();
                if let Some(pos) = t.find(|c: char| c == '.' || c == '!' || c == '?') {
                    t[..=pos].to_string()
                } else {
                    t.chars().take(100).collect()
                }
            })
            .unwrap_or_else(|| "Complete the task".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_empty_content() {
        let plan = PlanParser::parse("test", "");
        assert_eq!(plan.steps.len(), 1);
        assert!(plan.goal.contains("test"));
    }

    #[test]
    fn test_parse_step_headers() {
        let content = r#"# Plan: Test
## Step 1: Read files
- [ ] Read the source
**Action:** read_file

## Step 2: Fix bugs  
- [ ] Edit files
**Action:** write_file
**Depends on:** Step 1
"#;
        let plan = PlanParser::parse("test task", content);
        assert_eq!(plan.steps.len(), 2);
        assert_eq!(plan.steps[0].action, "read_file");
        assert_eq!(plan.steps[1].action, "write_file");
        assert!(plan.steps[1].depends_on.contains(&0));
    }

    #[test]
    fn test_parse_numbered_list() {
        let content = r#"1. Read the source files
   Action: read_file
2. Fix the bugs
   Action: write_file
   Depends on: step 1
"#;
        let plan = PlanParser::parse("test", content);
        assert_eq!(plan.steps.len(), 2);
    }

    #[test]
    fn test_parse_checkbox_status() {
        let content = r#"## Step 1: Done step
- [x] Already done
**Action:** read_file

## Step 2: Pending step
- [ ] Not done yet
**Action:** write_file
"#;
        let plan = PlanParser::parse("test", content);
        assert_eq!(plan.steps[0].status, StepStatus::Completed);
        assert_eq!(plan.steps[1].status, StepStatus::Pending);
    }

    #[test]
    fn test_extract_action_from_keywords() {
        let lines = ["compile the project and check for errors"];
        let action = PlanParser::extract_action(&lines);
        assert_eq!(action, Some("execute_command".to_string()));
    }

    #[test]
    fn test_extract_command() {
        let lines = ["Command: `cargo check 2>&1`"];
        let cmd = PlanParser::extract_command(&lines);
        assert!(cmd.is_some());
        assert!(cmd.unwrap().contains("cargo"));
    }

    #[test]
    fn test_no_plan_file() {
        let dir = tempfile::tempdir().unwrap();
        let plan = PlanParser::try_from_workspace(dir.path(), "test");
        assert!(plan.is_none());
    }

    #[test]
    fn test_parse_actual_plan_file() {
        let dir = tempfile::tempdir().unwrap();
        let plan_content = r#"# Plan: Test
## Step 1: Setup
- [ ] Create files
**Action:** write_file
"#;
        std::fs::write(dir.path().join("PLAN.md"), plan_content).unwrap();
        let plan = PlanParser::try_from_workspace(dir.path(), "test task");
        assert!(plan.is_some());
        assert_eq!(plan.unwrap().steps.len(), 1);
    }
}
