use crate::core::paths::Vo1dPaths;
use crate::models::action::Action;
use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A named, reusable multi-step procedure the agent can create and invoke.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Skill {
    pub name: String,
    pub description: String,
    #[serde(default = "empty_schema")]
    pub params_schema: serde_json::Value,
    pub steps: Vec<SkillStep>,
    pub created_at: String,
}

fn empty_schema() -> serde_json::Value {
    serde_json::Value::Object(serde_json::Map::new())
}

/// A single step within a skill — a tool name + its arguments.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SkillStep {
    pub tool: String,
    pub args: serde_json::Value,
}

/// Validates a skill name matches `^[a-z0-9][a-z0-9-]{0,63}$`.
pub fn validate_skill_name(name: &str) -> Result<()> {
    if name.is_empty() {
        bail!("Skill name must not be empty");
    }
    if name.len() > 64 {
        bail!("Skill name too long (max 64 chars, got {})", name.len());
    }
    let first = name.chars().next().unwrap();
    if !first.is_ascii_lowercase() && !first.is_ascii_digit() {
        bail!("Skill name must start with a lowercase letter or digit");
    }
    for c in name.chars() {
        if !c.is_ascii_lowercase() && !c.is_ascii_digit() && c != '-' {
            bail!("Skill name must only contain lowercase letters, digits, and hyphens (got '{}')", c);
        }
    }
    Ok(())
}

/// Registry of all loaded skills, backed by one JSON file per skill on disk.
pub struct SkillRegistry {
    skills: HashMap<String, Skill>,
    skills_dir: std::path::PathBuf,
}

impl SkillRegistry {
    /// Load all skills from disk. Missing directory is treated as empty.
    pub fn load(paths: &Vo1dPaths) -> Result<Self> {
        let skills_dir = paths.skills_dir();
        let mut skills = HashMap::new();

        if skills_dir.is_dir() {
            for entry in std::fs::read_dir(&skills_dir)
                .with_context(|| format!("Failed to read skills directory: {}", skills_dir.display()))?
            {
                let entry = entry?;
                let path = entry.path();
                if path.extension().is_some_and(|e| e == "json") {
                    let content = std::fs::read_to_string(&path)
                        .with_context(|| format!("Failed to read skill file: {}", path.display()))?;
                    match serde_json::from_str::<Skill>(&content) {
                        Ok(skill) => {
                            skills.insert(skill.name.clone(), skill);
                        }
                        Err(e) => {
                            tracing::warn!("Skipping invalid skill file {}: {}", path.display(), e);
                        }
                    }
                }
            }
        }

        Ok(Self { skills, skills_dir })
    }

    /// Register a new skill, persisting to disk atomically.
    pub fn create(&mut self, skill: Skill) -> Result<()> {
        validate_skill_name(&skill.name)?;

        if self.skills.contains_key(&skill.name) {
            bail!("Skill '{}' already exists", skill.name);
        }

        let path = self.skills_dir.join(format!("{}.json", skill.name));
        let tmp_path = path.with_extension("json.tmp");

        let json = serde_json::to_string_pretty(&skill)
            .with_context(|| format!("Failed to serialize skill '{}'", skill.name))?;

        std::fs::write(&tmp_path, &json)
            .with_context(|| format!("Failed to write temporary skill file: {}", tmp_path.display()))?;

        std::fs::rename(&tmp_path, &path)
            .with_context(|| format!("Failed to rename temporary skill file to: {}", path.display()))?;

        self.skills.insert(skill.name.clone(), skill);
        Ok(())
    }

    /// Remove a skill by name. Returns true if it existed.
    pub fn delete(&mut self, name: &str) -> Result<bool> {
        if !self.skills.contains_key(name) {
            return Ok(false);
        }

        let path = self.skills_dir.join(format!("{}.json", name));
        if path.exists() {
            std::fs::remove_file(&path)
                .with_context(|| format!("Failed to delete skill file: {}", path.display()))?;
        }

        self.skills.remove(name);
        Ok(true)
    }

    pub fn get(&self, name: &str) -> Option<&Skill> {
        self.skills.get(name)
    }

    /// List skills, optionally filtered by a keyword (case-insensitive substring match on name + description).
    pub fn list(&self, keyword: Option<&str>) -> Vec<&Skill> {
        match keyword {
            Some(kw) if !kw.is_empty() => {
                let kw_lower = kw.to_lowercase();
                self.skills
                    .values()
                    .filter(|s| {
                        s.name.to_lowercase().contains(&kw_lower)
                            || s.description.to_lowercase().contains(&kw_lower)
                    })
                    .collect()
            }
            _ => self.skills.values().collect(),
        }
    }

    /// Returns a markdown block listing skill names + one-line descriptions,
    /// suitable for appending to the system prompt. Capped at ~1500 chars.
    pub fn as_prompt_injection(&self) -> String {
        if self.skills.is_empty() {
            return String::new();
        }

        let mut sorted: Vec<&Skill> = self.skills.values().collect();
        sorted.sort_by(|a, b| a.name.cmp(&b.name));

        let preamble = "\n\n# Available skills\n\
            If a skill matches your task, prefer invoking it via the invoke_skill tool over re-deriving the steps. \
            To create a new skill from a successful multi-step sequence, use create_skill.\n";

        let mut lines = Vec::new();
        for skill in &sorted {
            let line = format!("- `{}`: {} ({} steps)", skill.name, skill.description, skill.steps.len());
            lines.push(line);
        }

        let mut result = preamble.to_string();
        for line in &lines {
            if result.len() + line.len() + 1 > 1500 {
                let remaining = lines.len() - result.matches("- `").count();
                result.push_str(&format!("+ {} more skills available\n", remaining));
                break;
            }
            result.push_str(line);
            result.push('\n');
        }

        result
    }

    /// Validate params, resolve a skill into executable (Action, tool_name) pairs.
    /// Returns Err if skill not found or params invalid.
    pub fn from_map(skills: HashMap<String, Skill>, skills_dir: std::path::PathBuf) -> Self {
        Self { skills, skills_dir }
    }

    /// Load skills from an arbitrary directory (useful for testing with temp dirs).
    pub fn load_with_dir(dir: &std::path::Path) -> Self {
        let mut skills = HashMap::new();
        if dir.is_dir() {
            if let Ok(entries) = std::fs::read_dir(dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.extension().is_some_and(|e| e == "json") {
                        if let Ok(content) = std::fs::read_to_string(&path) {
                            if let Ok(skill) = serde_json::from_str::<Skill>(&content) {
                                skills.insert(skill.name.clone(), skill);
                            }
                        }
                    }
                }
            }
        }
        Self { skills, skills_dir: dir.to_path_buf() }
    }

    pub fn resolve_steps(
        &self,
        name: &str,
        params: &serde_json::Value,
    ) -> Result<Vec<(Action, String)>> {
        let skill = self
            .skills
            .get(name)
            .ok_or_else(|| anyhow::anyhow!("Skill '{}' not found", name))?;

        // Validate params against params_schema (simple required-key check for v1)
        if let Some(schema_obj) = skill.params_schema.as_object() {
            if let Some(required) = schema_obj.get("required").and_then(|r| r.as_array()) {
                if let Some(params_obj) = params.as_object() {
                    for key in required {
                        let key_str = key.as_str().unwrap_or("");
                        if !params_obj.contains_key(key_str) {
                            bail!(
                                "Skill '{}' requires parameter '{}' per its schema",
                                name,
                                key_str
                            );
                        }
                    }
                } else if !required.is_empty() {
                    bail!(
                        "Skill '{}' requires parameters (schema: {:?})",
                        name,
                        skill.params_schema
                    );
                }
            }
        }

        let mut result = Vec::with_capacity(skill.steps.len());
        for step in &skill.steps {
            let merged_args = match params.as_object() {
                Some(p) => {
                    let mut merged = p.clone();
                    if let Some(step_args) = step.args.as_object() {
                        for (k, v) in step_args {
                            merged.insert(k.clone(), v.clone());
                        }
                    }
                    serde_json::Value::Object(merged)
                }
                None => step.args.clone(),
            };
            let action = build_action_from_step(&step.tool, &merged_args)?;
            result.push((action, step.tool.clone()));
        }

        Ok(result)
    }
}

/// Convert a tool name + args JSON into an Action for execution.
/// Reuses the same serde-based deserialization the parser uses.
fn build_action_from_step(tool: &str, args: &serde_json::Value) -> Result<Action> {
    let mut map = match args {
        serde_json::Value::Object(m) => m.clone(),
        _ => serde_json::Map::new(),
    };
    map.insert("action".to_string(), serde_json::Value::String(tool.to_string()));

    let action_value = serde_json::Value::Object(map);
    let action: Action = serde_json::from_value(action_value)
        .with_context(|| format!("Failed to build action for tool '{}' with args: {}", tool, args))?;
    Ok(action)
}
