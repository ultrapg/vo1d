use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

/// Persistent cross-session memory store.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryStore {
    path: PathBuf,
    pub task_history: Vec<TaskRecord>,
    pub preferences: HashMap<String, String>,
    pub patterns: Vec<Pattern>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskRecord {
    pub task: String,
    pub actions: Vec<String>,
    pub outcome: String,
    pub timestamp: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pattern {
    pub trigger: String,
    pub suggestion: String,
    pub confidence: f64,
}

impl MemoryStore {
    pub fn new(memory_dir: &PathBuf) -> Result<Self> {
        std::fs::create_dir_all(memory_dir)?;
        let mut store = Self {
            path: memory_dir.clone(),
            task_history: Vec::new(),
            preferences: HashMap::new(),
            patterns: Vec::new(),
        };
        let _ = store.load();
        Ok(store)
    }

    fn load(&mut self) -> Result<()> {
        self.task_history = Self::load_json(&self.path.join("task_history.json")).unwrap_or_default();
        self.preferences = Self::load_json(&self.path.join("preferences.json")).unwrap_or_default();
        self.patterns = Self::load_json(&self.path.join("patterns.json")).unwrap_or_default();
        Ok(())
    }

    fn load_json<T: serde::de::DeserializeOwned>(path: &PathBuf) -> Option<T> {
        if path.exists() {
            std::fs::read_to_string(path).ok()
                .and_then(|s| serde_json::from_str(&s).ok())
        } else {
            None
        }
    }

    pub fn save(&self) -> Result<()> {
        Self::save_json(&self.path.join("task_history.json"), &self.task_history)?;
        Self::save_json(&self.path.join("preferences.json"), &self.preferences)?;
        Self::save_json(&self.path.join("patterns.json"), &self.patterns)?;
        Ok(())
    }

    fn save_json<T: Serialize>(path: &PathBuf, value: &T) -> Result<()> {
        std::fs::write(path, serde_json::to_string_pretty(value)?)?;
        Ok(())
    }

    pub fn add_task(&mut self, task: &str, actions: Vec<String>, outcome: &str) {
        self.task_history.push(TaskRecord {
            task: task.to_string(),
            actions,
            outcome: outcome.to_string(),
            timestamp: chrono::Utc::now().to_rfc3339(),
        });
        if self.task_history.len() > 50 {
            self.task_history.remove(0);
        }
        let _ = self.save();
    }

    pub fn add_preference(&mut self, key: &str, value: &str) {
        self.preferences.insert(key.to_string(), value.to_string());
        let _ = self.save();
    }

    pub fn add_pattern(&mut self, trigger: &str, suggestion: &str) {
        let threshold = 0.7;
        if let Some(existing) = self.patterns.iter_mut().find(|p| p.trigger == trigger) {
            existing.confidence = (existing.confidence + 0.1).min(1.0);
        } else {
            self.patterns.push(Pattern {
                trigger: trigger.to_string(),
                suggestion: suggestion.to_string(),
                confidence: threshold,
            });
        }
        if self.patterns.len() > 20 {
            self.patterns.sort_by(|a, b| b.confidence.partial_cmp(&a.confidence).unwrap());
            self.patterns.truncate(20);
        }
        let _ = self.save();
    }

    pub fn to_context_string(&self) -> String {
        let mut parts = Vec::new();
        if !self.preferences.is_empty() {
            let prefs: Vec<String> = self.preferences.iter()
                .map(|(k, v)| format!("  - {}: {}", k, v))
                .collect();
            parts.push(format!("LEARNED PREFERENCES:\n{}", prefs.join("\n")));
        }
        if let Some(last) = self.task_history.last() {
            parts.push(format!(
                "LAST TASK ({}): {}\n  Outcome: {}",
                &last.timestamp[..19], last.task, last.outcome
            ));
        }
        if !self.patterns.is_empty() {
            if let Some(p) = self.patterns.iter().max_by(|a, b| a.confidence.partial_cmp(&b.confidence).unwrap()) {
                parts.push(format!("LEARNED PATTERN: \"{}\" → \"{}\"", p.trigger, p.suggestion));
            }
        }
        if parts.is_empty() {
            String::new()
        } else {
            format!("\n\n=== MEMORY ===\n{}", parts.join("\n\n"))
        }
    }
}
