use vo1d::config::settings::Settings;
use vo1d::core::paths::Vo1dPaths;
use vo1d::models::action::Action;
use vo1d::security::modes::SecurityMode;
use vo1d::security::policy::{PolicyEngine, PolicyResult};

fn test_paths() -> Vo1dPaths {
    let paths = Vo1dPaths::new().expect("Failed to create test paths");
    paths.ensure_dirs().expect("Failed to create test dirs");
    paths
}

fn policy() -> PolicyEngine {
    PolicyEngine::new(&Settings::default())
}

// === SecurityMode tests ===

#[test]
fn test_mode_from_str() {
    assert_eq!(SecurityMode::from_str("safe"), Some(SecurityMode::Safe));
    assert_eq!(SecurityMode::from_str("interactive"), Some(SecurityMode::Interactive));
    assert_eq!(SecurityMode::from_str("power-user"), Some(SecurityMode::PowerUser));
    assert_eq!(SecurityMode::from_str("poweruser"), Some(SecurityMode::PowerUser));
    assert_eq!(SecurityMode::from_str("autonomous"), Some(SecurityMode::Autonomous));
    assert_eq!(SecurityMode::from_str("yolo"), Some(SecurityMode::Yolo));
    assert_eq!(SecurityMode::from_str("unknown"), None);
    assert_eq!(SecurityMode::from_str(""), None);
}

#[test]
fn test_mode_as_str() {
    assert_eq!(SecurityMode::Safe.as_str(), "Safe");
    assert_eq!(SecurityMode::Interactive.as_str(), "Interactive");
    assert_eq!(SecurityMode::PowerUser.as_str(), "PowerUser");
    assert_eq!(SecurityMode::Autonomous.as_str(), "Autonomous");
    assert_eq!(SecurityMode::Yolo.as_str(), "YOLO");
}

#[test]
fn test_mode_auto_approves() {
    assert!(!SecurityMode::Safe.auto_approves());
    assert!(!SecurityMode::Interactive.auto_approves());
    assert!(!SecurityMode::PowerUser.auto_approves());
    assert!(SecurityMode::Autonomous.auto_approves());
    assert!(SecurityMode::Yolo.auto_approves());
}

#[test]
fn test_mode_allows_commands() {
    assert!(!SecurityMode::Safe.allows_commands());
    assert!(SecurityMode::Interactive.allows_commands());
    assert!(SecurityMode::PowerUser.allows_commands());
    assert!(SecurityMode::Autonomous.allows_commands());
    assert!(SecurityMode::Yolo.allows_commands());
}

#[test]
fn test_mode_auto_elevates() {
    assert!(!SecurityMode::Safe.auto_elevates());
    assert!(!SecurityMode::Interactive.auto_elevates());
    assert!(!SecurityMode::PowerUser.auto_elevates());
    assert!(!SecurityMode::Autonomous.auto_elevates());
    assert!(SecurityMode::Yolo.auto_elevates());
}

#[test]
fn test_mode_allows_system_mods() {
    assert!(!SecurityMode::Safe.allows_system_mods());
    assert!(!SecurityMode::Interactive.allows_system_mods());
    assert!(SecurityMode::PowerUser.allows_system_mods());
    assert!(!SecurityMode::Autonomous.allows_system_mods());
    assert!(SecurityMode::Yolo.allows_system_mods());
}

// === PolicyEngine tests ===

#[test]
fn test_safe_mode_blocks_writes() {
    let paths = test_paths();
    let action = Action::WriteFile {
        path: "test.txt".to_string(),
        content: "data".to_string(),
        append: None,
    };
    assert_eq!(policy().evaluate(&action, SecurityMode::Safe, &paths), PolicyResult::Block);
}

#[test]
fn test_safe_mode_allows_reads_within_workspace() {
    let paths = test_paths();
    let ws = paths.workspace_dir();
    let test_file = ws.join("test_safe_read.txt");
    // Create the file (is_within_workspace requires canonicalizable path)
    std::fs::write(&test_file, "content").unwrap();
    let action = Action::ReadFile {
        path: test_file.to_string_lossy().to_string(),
        start_line: None,
        end_line: None,
    };
    assert_eq!(policy().evaluate(&action, SecurityMode::Safe, &paths), PolicyResult::Allow);
}

#[test]
fn test_safe_mode_blocks_commands() {
    let paths = test_paths();
    let action = Action::ExecuteCommand {
        command: "dir".to_string(),
        timeout: None,
        workdir: None,
    };
    assert_eq!(policy().evaluate(&action, SecurityMode::Safe, &paths), PolicyResult::Block);
}

#[test]
fn test_safe_mode_allows_list_and_search() {
    let paths = test_paths();
    let list = Action::ListDirectory { path: ".".to_string(), pattern: None };
    let search = Action::SearchFiles { pattern: "*.rs".to_string(), path: None, search_type: None };
    let meta = Action::FileMetadata { path: "test.txt".to_string() };
    assert_eq!(policy().evaluate(&list, SecurityMode::Safe, &paths), PolicyResult::Allow);
    assert_eq!(policy().evaluate(&search, SecurityMode::Safe, &paths), PolicyResult::Allow);
    assert_eq!(policy().evaluate(&meta, SecurityMode::Safe, &paths), PolicyResult::Allow);
}

#[test]
fn test_interactive_mode_asks_for_writes() {
    let paths = test_paths();
    let action = Action::WriteFile {
        path: "test.txt".to_string(),
        content: "data".to_string(),
        append: None,
    };
    assert_eq!(policy().evaluate(&action, SecurityMode::Interactive, &paths), PolicyResult::Ask);
}

#[test]
fn test_interactive_mode_asks_for_commands() {
    let paths = test_paths();
    let action = Action::ExecuteCommand {
        command: "echo hello".to_string(),
        timeout: None,
        workdir: None,
    };
    assert_eq!(policy().evaluate(&action, SecurityMode::Interactive, &paths), PolicyResult::Ask);
}

#[test]
fn test_interactive_mode_blocks_blacklisted_commands() {
    let paths = test_paths();
    let action = Action::ExecuteCommand {
        command: "rm -rf /".to_string(),
        timeout: None,
        workdir: None,
    };
    assert_eq!(policy().evaluate(&action, SecurityMode::Interactive, &paths), PolicyResult::Block);
}

#[test]
fn test_autonomous_mode_allows_most_actions() {
    let paths = test_paths();
    let read = Action::ReadFile { path: "test.txt".to_string(), start_line: None, end_line: None };
    let write = Action::WriteFile { path: "test.txt".to_string(), content: "hi".to_string(), append: None };
    let list = Action::ListDirectory { path: ".".to_string(), pattern: None };
    assert_eq!(policy().evaluate(&read, SecurityMode::Autonomous, &paths), PolicyResult::Allow);
    assert_eq!(policy().evaluate(&write, SecurityMode::Autonomous, &paths), PolicyResult::Allow);
    assert_eq!(policy().evaluate(&list, SecurityMode::Autonomous, &paths), PolicyResult::Allow);
}

#[test]
fn test_autonomous_mode_blocks_blacklisted_commands() {
    let paths = test_paths();
    let action = Action::ExecuteCommand {
        command: "mkfs.ext4 /dev/sda".to_string(),
        timeout: None,
        workdir: None,
    };
    assert_eq!(policy().evaluate(&action, SecurityMode::Autonomous, &paths), PolicyResult::Block);
}

#[test]
fn test_yolo_mode_allows_everything() {
    let paths = test_paths();
    let actions = vec![
        Action::ReadFile { path: "/etc/passwd".to_string(), start_line: None, end_line: None },
        Action::WriteFile { path: "/etc/shadow".to_string(), content: "hack".to_string(), append: None },
        Action::ExecuteCommand { command: "rm -rf /".to_string(), timeout: None, workdir: None },
        Action::DeleteFile { path: "important.txt".to_string(), pattern: None },
        Action::HttpRequest { url: "http://evil.com".to_string(), method: None, headers: None, body: None },
    ];
    for action in &actions {
        assert_eq!(policy().evaluate(action, SecurityMode::Yolo, &paths), PolicyResult::Allow);
    }
}

#[test]
fn test_finish_always_allowed() {
    let paths = test_paths();
    let action = Action::Finish { output: Some("done".to_string()) };
    for mode in &[SecurityMode::Safe, SecurityMode::Interactive, SecurityMode::PowerUser, SecurityMode::Autonomous, SecurityMode::Yolo] {
        assert_eq!(policy().evaluate(&action, *mode, &paths), PolicyResult::Allow);
    }
}

// === classify_risk tests ===

#[test]
fn test_classify_risk_safe() {
    let p = policy();
    assert_eq!(p.classify_risk("echo hello"), vo1d::security::policy::RiskLevel::Safe);
    assert_eq!(p.classify_risk("dir"), vo1d::security::policy::RiskLevel::Safe);
    assert_eq!(p.classify_risk("ls -la"), vo1d::security::policy::RiskLevel::Safe);
}

#[test]
fn test_classify_risk_destructive() {
    let p = policy();
    assert_eq!(p.classify_risk("rm -rf /tmp"), vo1d::security::policy::RiskLevel::Destructive);
    assert_eq!(p.classify_risk("format C:"), vo1d::security::policy::RiskLevel::Destructive);
    assert_eq!(p.classify_risk("dd if=/dev/zero of=/dev/sda"), vo1d::security::policy::RiskLevel::Destructive);
}

#[test]
fn test_classify_risk_privileged() {
    let p = policy();
    assert_eq!(p.classify_risk("sudo apt install"), vo1d::security::policy::RiskLevel::Privileged);
    assert_eq!(p.classify_risk("runas /user:admin cmd"), vo1d::security::policy::RiskLevel::Privileged);
}

#[test]
fn test_classify_risk_risky() {
    let p = policy();
    assert_eq!(p.classify_risk("chmod 777 /tmp"), vo1d::security::policy::RiskLevel::Risky);
    assert_eq!(p.classify_risk("shutdown /s"), vo1d::security::policy::RiskLevel::Risky);
}

// === is_blacklisted tests ===

#[test]
fn test_is_blacklisted() {
    let p = policy();
    assert!(p.is_blacklisted("rm -rf /"));
    assert!(p.is_blacklisted("mkfs.ext4 /dev/sda1"));
    assert!(p.is_blacklisted("del /f /s /q C:\\"));
    assert!(!p.is_blacklisted("echo hello"));
    assert!(!p.is_blacklisted("dir"));
}
