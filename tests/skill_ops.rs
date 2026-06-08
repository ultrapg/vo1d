use std::path::PathBuf;
use tempfile::tempdir;
use vo1d::tools::skills::{Skill, SkillRegistry, SkillStep, validate_skill_name};

fn make_skill(name: &str, steps: usize) -> Skill {
    let step = SkillStep {
        tool: "read_file".into(),
        args: serde_json::json!({"path": "Cargo.toml"}),
    };
    Skill {
        name: name.to_string(),
        description: format!("Skill {} with {} steps", name, steps),
        params_schema: serde_json::Value::Object(Default::default()),
        steps: vec![step; steps],
        created_at: "2025-01-01T00:00:00Z".to_string(),
    }
}

/// Build a SkillRegistry rooted at a temp dir.
fn temp_registry() -> (SkillRegistry, PathBuf) {
    let dir = tempdir().expect("tempdir").into_path();
    std::fs::create_dir_all(&dir).unwrap();
    let reg = SkillRegistry::load_with_dir(&dir);
    (reg, dir)
}

// ── Tests ──

#[test]
fn test_create_roundtrip() {
    let (mut reg, dir) = temp_registry();
    let skill = make_skill("roundtrip-test", 2);

    reg.create(skill.clone()).unwrap();

    // Reload from disk
    let reloaded = SkillRegistry::load_with_dir(&dir);
    let stored = reloaded.get("roundtrip-test").expect("should exist");
    assert_eq!(stored.name, skill.name);
    assert_eq!(stored.description, skill.description);
    assert_eq!(stored.steps.len(), 2);
}

#[test]
fn test_list_keyword_filter() {
    let (mut reg, _dir) = temp_registry();
    reg.create(make_skill("alpha", 1)).unwrap();
    reg.create(make_skill("beta-two", 2)).unwrap();
    reg.create(make_skill("gamma", 3)).unwrap();

    let all = reg.list(None);
    assert_eq!(all.len(), 3);

    let filtered = reg.list(Some("beta"));
    assert_eq!(filtered.len(), 1);
    assert_eq!(filtered[0].name, "beta-two");

    // Case-insensitive
    let ci = reg.list(Some("ALPHA"));
    assert_eq!(ci.len(), 1);
    assert_eq!(ci[0].name, "alpha");

    // No match
    let none = reg.list(Some("zzz"));
    assert_eq!(none.len(), 0);
}

#[test]
fn test_delete_skill() {
    let (mut reg, _dir) = temp_registry();
    reg.create(make_skill("to-delete", 1)).unwrap();

    assert!(reg.delete("to-delete").unwrap());
    assert_eq!(reg.get("to-delete"), None);

    // Second delete returns false
    assert!(!reg.delete("to-delete").unwrap());
}

#[test]
fn test_invoke_resolve_steps() {
    // Test resolve_steps for a skill with multiple steps
    let skill = Skill {
        name: "multi-step".into(),
        description: "Two steps".into(),
        params_schema: serde_json::Value::Object(Default::default()),
        steps: vec![
            SkillStep {
                tool: "read_file".into(),
                args: serde_json::json!({"path": "Cargo.toml"}),
            },
            SkillStep {
                tool: "read_file".into(),
                args: serde_json::json!({"path": "README.md"}),
            },
        ],
        created_at: "2025-01-01T00:00:00Z".into(),
    };

    let mut inner = std::collections::HashMap::new();
    inner.insert(skill.name.clone(), skill);
    let reg = SkillRegistry::from_map(inner, PathBuf::from("."));

    let steps = reg.resolve_steps("multi-step", &serde_json::Value::Object(Default::default())).unwrap();
    assert_eq!(steps.len(), 2);
    assert_eq!(steps[0].1, "read_file");
    assert_eq!(steps[1].1, "read_file");
}

#[test]
fn test_invoke_missing_skill() {
    let (reg, _dir) = temp_registry();
    let result = reg.resolve_steps("nonexistent", &serde_json::Value::Object(Default::default()));
    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(err.contains("nonexistent"), "Error should mention the skill name: {}", err);
}

#[test]
fn test_invalid_name_rejected() {
    assert!(validate_skill_name("valid-name-1").is_ok());

    // Uppercase
    assert!(validate_skill_name("Invalid").is_err());
    // Spaces
    assert!(validate_skill_name("my skill").is_err());
    // Too long (65 chars)
    let long = "a".repeat(65);
    assert!(validate_skill_name(&long).is_err());
    // Empty
    assert!(validate_skill_name("").is_err());
    // Starts with digit (valid)
    assert!(validate_skill_name("1abc").is_ok());
    // Starts with hyphen (invalid)
    assert!(validate_skill_name("-abc").is_err());
}

#[test]
fn test_param_validation() {
    use serde_json::json;

    let skill = Skill {
        name: "param-skill".into(),
        description: "Needs params".into(),
        params_schema: json!({
            "type": "object",
            "properties": {
                "path": { "type": "string" }
            },
            "required": ["path"]
        }),
        steps: vec![
            SkillStep {
                tool: "read_file".into(),
                args: json!({}),
            },
        ],
        created_at: "2025-01-01T00:00:00Z".into(),
    };

    let mut inner = std::collections::HashMap::new();
    inner.insert(skill.name.clone(), skill);
    let reg = SkillRegistry::from_map(inner, PathBuf::from("."));

    // Missing required param
    let result = reg.resolve_steps("param-skill", &json!({}));
    assert!(result.is_err(), "Should reject missing required param");
    let err = result.unwrap_err().to_string();
    assert!(err.contains("requires parameter"), "Error: {}", err);

    // Valid param
    let result = reg.resolve_steps("param-skill", &json!({"path": "Cargo.toml"}));
    assert!(result.is_ok(), "Should accept valid params: {:?}", result.err());
}

#[test]
fn test_prompt_injection() {
    let (mut reg, _dir) = temp_registry();
    // Empty
    assert!(reg.as_prompt_injection().is_empty());

    reg.create(make_skill("skill-a", 2)).unwrap();
    reg.create(make_skill("skill-b", 3)).unwrap();

    let prompt = reg.as_prompt_injection();
    assert!(prompt.contains("skill-a"));
    assert!(prompt.contains("skill-b"));
    assert!(prompt.contains("Available skills"));
    assert!(prompt.contains("invoke_skill"));

    // Test truncation with many skills
    for i in 0..50 {
        reg.create(make_skill(&format!("skill-{:03}", i), 1)).unwrap();
    }
    let long_prompt = reg.as_prompt_injection();
    assert!(long_prompt.len() <= 1600, "Prompt too long: {} chars", long_prompt.len());
}
