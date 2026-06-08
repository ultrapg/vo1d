use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

/// Persistent cross-session memory store with learning and recall.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryStore {
    path: PathBuf,
    pub task_history: Vec<TaskRecord>,
    pub preferences: HashMap<String, String>,
    pub patterns: Vec<Pattern>,
    pub solutions: Vec<SolutionRecord>,
    pub mistakes: Vec<MistakeRecord>,
    pub notes: Vec<Note>,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SolutionRecord {
    pub id: String,
    pub task_description: String,
    pub tags: Vec<String>,
    pub solution: String,
    pub outcome: String,
    pub timestamp: String,
    pub success_count: u32,
    /// Structured action sequence that led to success
    #[serde(default)]
    pub actions: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MistakeRecord {
    pub id: String,
    pub task_description: String,
    pub tags: Vec<String>,
    pub mistake: String,
    pub lesson: String,
    pub how_to_avoid: String,
    pub timestamp: String,
    pub frequency: u32,
    /// Action sequence that led to the mistake
    #[serde(default)]
    pub actions: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Note {
    pub id: String,
    pub content: String,
    pub tags: Vec<String>,
    pub timestamp: String,
}

#[derive(Debug, Clone)]
pub struct SimilarMemories {
    pub solutions: Vec<SolutionRecord>,
    pub mistakes: Vec<MistakeRecord>,
    pub patterns: Vec<Pattern>,
    pub notes: Vec<Note>,
}

impl MemoryStore {
    pub fn new(memory_dir: &PathBuf) -> Result<Self> {
        std::fs::create_dir_all(memory_dir)?;
        let mut store = Self {
            path: memory_dir.clone(),
            task_history: Vec::new(),
            preferences: HashMap::new(),
            patterns: Vec::new(),
            solutions: Vec::new(),
            mistakes: Vec::new(),
            notes: Vec::new(),
        };
        let _ = store.load();
        Ok(store)
    }

    fn load(&mut self) -> Result<()> {
        self.task_history = Self::load_json(&self.path.join("task_history.json")).unwrap_or_default();
        self.preferences = Self::load_json(&self.path.join("preferences.json")).unwrap_or_default();
        self.patterns = Self::load_json(&self.path.join("patterns.json")).unwrap_or_default();
        self.solutions = Self::load_json(&self.path.join("solutions.json")).unwrap_or_default();
        self.mistakes = Self::load_json(&self.path.join("mistakes.json")).unwrap_or_default();
        self.notes = Self::load_json(&self.path.join("notes.json")).unwrap_or_default();
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

    const MAX_TOTAL_BYTES: u64 = 50 * 1024 * 1024; // 50MB total memory limit

    pub fn save(&self) -> Result<()> {
        Self::save_json(&self.path.join("task_history.json"), &self.task_history)?;
        Self::save_json(&self.path.join("preferences.json"), &self.preferences)?;
        Self::save_json(&self.path.join("patterns.json"), &self.patterns)?;
        Self::save_json(&self.path.join("solutions.json"), &self.solutions)?;
        Self::save_json(&self.path.join("mistakes.json"), &self.mistakes)?;
        Self::save_json(&self.path.join("notes.json"), &self.notes)?;
        self.check_total_size()?;
        Ok(())
    }

    /// Check that total memory directory size stays under the limit.
    fn check_total_size(&self) -> Result<()> {
        let mut total: u64 = 0;
        let files = [
            "task_history.json",
            "preferences.json",
            "patterns.json",
            "solutions.json",
            "mistakes.json",
            "notes.json",
        ];
        for fname in &files {
            let path = self.path.join(fname);
            if let Ok(meta) = std::fs::metadata(&path) {
                total += meta.len();
            }
        }
        if total > Self::MAX_TOTAL_BYTES {
            tracing::warn!(
                "Memory store exceeds size limit: {:.2} MB / {:.2} MB. Consider clearing old data.",
                total as f64 / 1_048_576.0,
                Self::MAX_TOTAL_BYTES as f64 / 1_048_576.0,
            );
        }
        Ok(())
    }

    fn save_json<T: Serialize>(path: &PathBuf, value: &T) -> Result<()> {
        std::fs::write(path, serde_json::to_string_pretty(value)?)?;
        Ok(())
    }

    fn next_id(prefix: &str) -> String {
        let ts = chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0);
        format!("{}_{}", prefix, ts)
    }

    pub fn add_task(&mut self, task: &str, actions: Vec<String>, outcome: &str) {
        self.task_history.push(TaskRecord {
            task: task.to_string(),
            actions,
            outcome: outcome.to_string(),
            timestamp: chrono::Utc::now().to_rfc3339(),
        });
        if self.task_history.len() > 100 {
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
        if self.patterns.len() > 50 {
            self.patterns.sort_by(|a, b| b.confidence.partial_cmp(&a.confidence).unwrap());
            self.patterns.truncate(50);
        }
        let _ = self.save();
    }

    /// Store a successful solution for future recall.
    /// `actions_taken` records the sequence of actions that led to success.
    /// Extracts tags from the task description for similarity matching.
    pub fn add_solution(&mut self, task_description: &str, solution: &str, outcome: &str, actions_taken: &[String]) {
        let tags = Self::extract_tags(task_description);
        let id = Self::next_id("sol");

        // Deduplicate: if same task exists, update it
        if let Some(existing) = self.solutions.iter_mut().find(|s| s.task_description == task_description) {
            existing.success_count += 1;
            existing.solution = solution.to_string();
            existing.outcome = outcome.to_string();
            existing.timestamp = chrono::Utc::now().to_rfc3339();
            if !actions_taken.is_empty() {
                existing.actions = actions_taken.to_vec();
            }
        } else {
            self.solutions.push(SolutionRecord {
                id,
                task_description: task_description.to_string(),
                tags,
                solution: solution.to_string(),
                outcome: outcome.to_string(),
                timestamp: chrono::Utc::now().to_rfc3339(),
                success_count: 1,
                actions: actions_taken.to_vec(),
            });
        }
        if self.solutions.len() > 100 {
            self.solutions.remove(0);
        }
        let _ = self.save();
    }

    /// Store a mistake with what was learned and how to avoid it.
    /// `actions_taken` records the action sequence that led to the mistake.
    pub fn add_mistake(&mut self, task_description: &str, mistake: &str, lesson: &str, how_to_avoid: &str, actions_taken: &[String]) {
        let tags = Self::extract_tags(task_description);
        let id = Self::next_id("mist");

        if let Some(existing) = self.mistakes.iter_mut().find(|m| m.mistake == mistake) {
            existing.frequency += 1;
            existing.timestamp = chrono::Utc::now().to_rfc3339();
            if !actions_taken.is_empty() {
                existing.lesson = lesson.to_string();
                existing.actions = actions_taken.to_vec();
            }
        } else {
            self.mistakes.push(MistakeRecord {
                id,
                task_description: task_description.to_string(),
                tags,
                mistake: mistake.to_string(),
                lesson: lesson.to_string(),
                how_to_avoid: how_to_avoid.to_string(),
                timestamp: chrono::Utc::now().to_rfc3339(),
                frequency: 1,
                actions: actions_taken.to_vec(),
            });
        }
        if self.mistakes.len() > 100 {
            self.mistakes.sort_by(|a, b| b.frequency.cmp(&a.frequency));
            self.mistakes.truncate(100);
        }
        let _ = self.save();
    }

    /// Add a user-curated note.
    pub fn add_note(&mut self, content: &str, tags: Vec<String>) -> String {
        let id = Self::next_id("note");
        self.notes.push(Note {
            id: id.clone(),
            content: content.to_string(),
            tags,
            timestamp: chrono::Utc::now().to_rfc3339(),
        });
        if self.notes.len() > 50 {
            self.notes.remove(0);
        }
        let _ = self.save();
        id
    }

    /// Delete a memory by its type prefix and id.
    pub fn delete(&mut self, id: &str) -> bool {
        if id.starts_with("sol_") {
            let before = self.solutions.len();
            self.solutions.retain(|s| s.id != id);
            let _ = self.save();
            self.solutions.len() != before
        } else if id.starts_with("mist_") {
            let before = self.mistakes.len();
            self.mistakes.retain(|m| m.id != id);
            let _ = self.save();
            self.mistakes.len() != before
        } else if id.starts_with("note_") {
            let before = self.notes.len();
            self.notes.retain(|n| n.id != id);
            let _ = self.save();
            self.notes.len() != before
        } else {
            false
        }
    }

    /// Clear all memories (or a subset by type).
    pub fn clear(&mut self, memory_type: Option<&str>) {
        match memory_type {
            Some("solutions") => self.solutions.clear(),
            Some("mistakes") => self.mistakes.clear(),
            Some("notes") => self.notes.clear(),
            Some("patterns") => self.patterns.clear(),
            Some("history") => self.task_history.clear(),
            Some("all") | None => {
                self.solutions.clear();
                self.mistakes.clear();
                self.notes.clear();
                self.patterns.clear();
                self.task_history.clear();
                self.preferences.clear();
            }
            _ => {}
        }
        let _ = self.save();
    }

    /// Find memories similar to a query string by keyword overlap.
    pub fn find_similar(&self, query: &str, max_results: usize) -> SimilarMemories {
        let query_lower = query.to_lowercase();
        let query_words: Vec<&str> = query_lower.split_whitespace()
            .filter(|w| w.len() > 2)
            .collect();

        if query_words.is_empty() {
            return SimilarMemories {
                solutions: Vec::new(),
                mistakes: Vec::new(),
                patterns: Vec::new(),
                notes: Vec::new(),
            };
        }

        let score = |text: &str| -> f64 {
            let lower = text.to_lowercase();
            let matches = query_words.iter()
                .filter(|w| lower.contains(*w))
                .count();
            matches as f64 / query_words.len() as f64
        };

        let mut scored_solutions: Vec<(f64, &SolutionRecord)> = self.solutions.iter()
            .map(|s| {
                let s_score = score(&s.task_description) * 0.6 + score(&s.solution) * 0.4;
                (s_score, s)
            })
            .filter(|(s, _)| *s > 0.25)
            .collect();
        scored_solutions.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap());

        let mut scored_mistakes: Vec<(f64, &MistakeRecord)> = self.mistakes.iter()
            .map(|m| {
                let m_score = score(&m.task_description) * 0.5 + score(&m.mistake) * 0.3 + score(&m.lesson) * 0.2;
                (m_score, m)
            })
            .filter(|(s, _)| *s > 0.25)
            .collect();
        scored_mistakes.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap());

        let mut scored_patterns: Vec<(f64, &Pattern)> = self.patterns.iter()
            .map(|p| (score(&p.trigger) * 0.7 + p.confidence * 0.3, p))
            .filter(|(s, _)| *s > 0.3)
            .collect();
        scored_patterns.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap());

        let mut scored_notes: Vec<(f64, &Note)> = self.notes.iter()
            .map(|n| {
                let tag_score = n.tags.iter().any(|t| query_lower.contains(t.as_str())) as i32 as f64 * 0.5;
                (score(&n.content).max(tag_score), n)
            })
            .filter(|(s, _)| *s > 0.25)
            .collect();
        scored_notes.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap());

        SimilarMemories {
            solutions: scored_solutions.into_iter().take(max_results).map(|(_, s)| s.clone()).collect(),
            mistakes: scored_mistakes.into_iter().take(max_results).map(|(_, m)| m.clone()).collect(),
            patterns: scored_patterns.into_iter().take(max_results).map(|(_, p)| p.clone()).collect(),
            notes: scored_notes.into_iter().take(max_results).map(|(_, n)| n.clone()).collect(),
        }
    }

    /// Build a rich context string incorporating learned patterns, similar memories, and notes.
    /// Optionally pass a task description to get relevant recall.
    pub fn to_context_string(&self) -> String {
        let mut parts = Vec::new();

        if !self.preferences.is_empty() {
            let prefs: Vec<String> = self.preferences.iter()
                .filter(|(k, _)| !k.starts_with('_'))
                .map(|(k, v)| format!("  - {}: {}", k, v))
                .collect();
            if !prefs.is_empty() {
                parts.push(format!("LEARNED PREFERENCES:\n{}", prefs.join("\n")));
            }
        }

        if let Some(last) = self.task_history.last() {
            parts.push(format!(
                "LAST TASK ({}): {}\n  Outcome: {}",
                &last.timestamp[..19], last.task, last.outcome
            ));
        }

        if !self.patterns.is_empty() {
            let top_patterns: Vec<String> = self.patterns.iter()
                .filter(|p| p.confidence > 0.6)
                .take(3)
                .map(|p| format!("  [conf:{:.0}%] \"{}\" → \"{}\"", p.confidence * 100.0, p.trigger, p.suggestion))
                .collect();
            if !top_patterns.is_empty() {
                parts.push(format!("LEARNED PATTERNS:\n{}", top_patterns.join("\n")));
            }
        }

        if !self.mistakes.is_empty() {
            let recent_lessons: Vec<String> = self.mistakes.iter()
                .filter(|m| m.frequency > 1)
                .take(3)
                .map(|m| format!("  [freq:{}] {} → Lesson: {} (Avoid: {})",
                    m.frequency, m.mistake, m.lesson, m.how_to_avoid))
                .collect();
            if !recent_lessons.is_empty() {
                parts.push(format!("MISTAKES TO AVOID:\n{}", recent_lessons.join("\n")));
            }
        }

        if !self.solutions.is_empty() {
            let top_solutions: Vec<String> = self.solutions.iter()
                .rev()
                .take(3)
                .map(|s| format!("  {} → Outcome: {}", s.task_description, s.outcome))
                .collect();
            if !top_solutions.is_empty() {
                parts.push(format!("RECENT SOLUTIONS:\n{}", top_solutions.join("\n")));
            }
        }

        if parts.is_empty() {
            String::new()
        } else {
            format!("\n\n=== MEMORY ===\n{}", parts.join("\n\n"))
        }
    }

    /// Get context with task-specific recall. Includes similar past solutions and mistakes.
    pub fn to_context_string_with_recall(&self, current_task: &str) -> String {
        let base = self.to_context_string();

        let similar = self.find_similar(current_task, 3);

        let mut recall_parts = Vec::new();

        if !similar.solutions.is_empty() {
            let sol_texts: Vec<String> = similar.solutions.iter()
                .map(|s| {
                    let actions_note = if s.actions.is_empty() {
                        String::new()
                    } else {
                        format!("\n    Actions taken: {}", s.actions.join(" → "))
                    };
                    format!("  [past solution] {} → {}{}", s.task_description, s.solution, actions_note)
                })
                .collect();
            recall_parts.push(format!("SIMILAR PAST SOLUTIONS:\n{}", sol_texts.join("\n")));
        }

        if !similar.mistakes.is_empty() {
            let mist_texts: Vec<String> = similar.mistakes.iter()
                .map(|m| {
                    let actions_note = if m.actions.is_empty() {
                        String::new()
                    } else {
                        format!("\n    Actions that led to mistake: {}", m.actions.join(" → "))
                    };
                    format!("  [past mistake] {} → Lesson: {} | Avoid: {}{}", m.mistake, m.lesson, m.how_to_avoid, actions_note)
                })
                .collect();
            recall_parts.push(format!("RELEVANT MISTAKES FROM THE PAST:\n{}", mist_texts.join("\n")));
        }

        if !similar.notes.is_empty() {
            let note_texts: Vec<String> = similar.notes.iter()
                .map(|n| format!("  [note] {} (tags: {})", n.content, n.tags.join(", ")))
                .collect();
            recall_parts.push(format!("RELEVANT NOTES:\n{}", note_texts.join("\n")));
        }

        // Plan template matching: find past solutions with PLAN.md-like step structures
        // Score by keyword overlap with current task
        let task_lower = current_task.to_lowercase();
        let task_words: Vec<&str> = task_lower.split_whitespace().filter(|w| w.len() > 2).collect();
        let score_solution = |s: &SolutionRecord| -> f64 {
            if !s.solution.to_lowercase().contains("## step") && !s.solution.to_lowercase().contains("## plan") {
                return 0.0;
            }
            let combined = format!("{} {}", s.task_description, s.solution).to_lowercase();
            let matches = task_words.iter().filter(|w| combined.contains(*w)).count();
            let overlap = if task_words.is_empty() { 0.0 } else { matches as f64 / task_words.len() as f64 };
            // Bonus if it was in similar results
            let sim_bonus = if similar.solutions.iter().any(|sim| sim.id == s.id) { 0.3 } else { 0.0 };
            overlap + sim_bonus
        };

        let mut scored_plans: Vec<(f64, &SolutionRecord)> = self.solutions.iter()
            .map(|s| (score_solution(s), s))
            .filter(|(score, _)| *score > 0.0)
            .collect();
        scored_plans.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap());

        let plan_templates: Vec<String> = scored_plans.iter()
            .take(2)
            .map(|(_, s)| {
                let plan_section = if s.solution.contains("## Step") || s.solution.contains("## Plan") || s.solution.contains("PLAN.md") {
                    s.solution.clone()
                } else {
                    String::new()
                };
                if plan_section.is_empty() {
                    format!("  [{}] {} — review memory for plan details", s.id, s.task_description)
                } else {
                    let truncated = if plan_section.len() > 600 {
                        format!("{}...", &plan_section[..597])
                    } else {
                        plan_section
                    };
                    format!("  [{}] {} — plan template:\n{}", s.id, s.task_description, truncated)
                }
            })
            .collect();

        if !plan_templates.is_empty() {
            recall_parts.push(format!("PLAN TEMPLATES FROM PAST TASKS:\n{}", plan_templates.join("\n\n")));
        }

        if recall_parts.is_empty() {
            base
        } else {
            format!("{}{}\n\n=== SIMILAR PAST EXPERIENCES ===\n{}", base, "\n", recall_parts.join("\n\n"))
        }
    }

    /// Extract simple tags from a task description (capitalized words, common keywords).
    fn extract_tags(text: &str) -> Vec<String> {
        let mut tags: Vec<String> = Vec::new();
        let lowercase = text.to_lowercase();

        let keywords = ["file", "write", "read", "create", "delete", "copy", "move",
                        "directory", "folder", "search", "command", "shell",
                        "web", "http", "download", "network", "install",
                        "config", "code", "script", "test", "build", "deploy",
                        "python", "rust", "javascript", "json", "yaml", "toml",
                        "git", "docker", "database", "api", "server",
                        "hello world", "greeting",
                        "refactor", "debug", "fix", "module", "dependency",
                        "compile", "syntax", "error", "lint", "format",
                        "documentation", "readme", "markdown",
                        "scaffold", "template", "init", "setup",
                        "pipeline", "workflow", "automation",
                        "sort", "filter", "transform", "parse", "validate",
                        "backup", "revert", "rollback", "restore"];

        for kw in &keywords {
            if lowercase.contains(kw) {
                tags.push(kw.to_string());
            }
        }

        tags.dedup();
        tags
    }

    pub fn stats(&self) -> MemoryStats {
        MemoryStats {
            total_tasks: self.task_history.len() as u64,
            total_patterns: self.patterns.len() as u64,
            total_solutions: self.solutions.len() as u64,
            total_mistakes: self.mistakes.len() as u64,
            total_notes: self.notes.len() as u64,
            total_preferences: self.preferences.len() as u64,
        }
    }
}

#[derive(Debug, Clone)]
pub struct MemoryStats {
    pub total_tasks: u64,
    pub total_patterns: u64,
    pub total_solutions: u64,
    pub total_mistakes: u64,
    pub total_notes: u64,
    pub total_preferences: u64,
}

impl std::fmt::Display for MemoryStats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f,
            "Tasks: {} | Patterns: {} | Solutions: {} | Mistakes: {} | Notes: {} | Preferences: {}",
            self.total_tasks, self.total_patterns, self.total_solutions,
            self.total_mistakes, self.total_notes, self.total_preferences
        )
    }
}
