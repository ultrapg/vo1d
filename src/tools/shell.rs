use anyhow::{Context, Result};
use std::path::Path;
use std::time::Duration;
use tokio::process::Command;
use tokio::io::AsyncBufReadExt;
use tracing::info;

/// Shell command execution tool.
pub struct ShellExec;

impl ShellExec {
    /// Execute a shell command with timeout and streaming output.
    pub async fn execute(command: &str, timeout_secs: u64, cwd: &Path) -> Result<String> {
        info!("Executing command: {} (timeout: {}s, cwd: {})", command, timeout_secs, cwd.display());

        let shell = detect_shell();
        let (shell_cmd, shell_arg) = match shell.as_str() {
            "powershell" => ("powershell".to_string(), "-Command".to_string()),
            "pwsh" => ("pwsh".to_string(), "-Command".to_string()),
            "cmd" => ("cmd.exe".to_string(), "/C".to_string()),
            _ => ("cmd.exe".to_string(), "/C".to_string()),
        };

        let mut child = Command::new(&shell_cmd)
            .arg(&shell_arg)
            .arg(command)
            .current_dir(cwd)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .kill_on_drop(true)
            .spawn()
            .with_context(|| format!("Failed to spawn command: {}", command))?;

        let stdout_handle = child.stdout.take()
            .context("Failed to capture stdout")?;
        let stderr_handle = child.stderr.take()
            .context("Failed to capture stderr")?;

        let mut stdout_reader = tokio::io::BufReader::new(stdout_handle).lines();
        let mut stderr_reader = tokio::io::BufReader::new(stderr_handle).lines();

        let mut output_lines = Vec::new();

        // Read stdout and stderr concurrently
        loop {
            tokio::select! {
                line = stdout_reader.next_line() => {
                    match line {
                        Ok(Some(text)) => {
                            output_lines.push(text.clone());
                            tracing::debug!("[stdout] {}", text);
                        }
                        Ok(None) => break,
                        Err(e) => {
                            output_lines.push(format!("[stdout error] {}", e));
                            break;
                        }
                    }
                }
                line = stderr_reader.next_line() => {
                    match line {
                        Ok(Some(text)) => {
                            output_lines.push(format!("[stderr] {}", text));
                            tracing::debug!("[stderr] {}", text);
                        }
                        Ok(None) => {}
                        Err(_) => break,
                    }
                }
            }
        }

        // Wait for command with timeout
        let timed_out = {
            let timeout_dur = Duration::from_secs(timeout_secs);
            tokio::time::timeout(timeout_dur, child.wait()).await
        };

        let exit_code = match timed_out {
            Ok(Ok(status)) => status.code().unwrap_or(-1),
            Ok(Err(e)) => {
                output_lines.push(format!("[error] Failed to wait for process: {}", e));
                -1
            }
            Err(_) => {
                // Kill the process on timeout
                let _ = child.kill().await;
                output_lines.push(format!("\n[timeout] Command timed out after {} seconds", timeout_secs));
                -1
            }
        };

        let output = output_lines.join("\n");

        info!("Command exited with code {}", exit_code);

        Ok(format!(
            "Exit code: {}\n\n{}",
            exit_code,
            if output.len() > 100_000 {
                format!("{}...\n[Output truncated: {} total chars]", &output[..100_000], output.len())
            } else {
                output
            }
        ))
    }
}

/// Detect the available shell on the current system.
fn detect_shell() -> String {
    #[cfg(windows)]
    {
        // Prefer cmd.exe so the model's natural %VAR% syntax works
        "cmd".to_string()
    }

    #[cfg(unix)]
    {
        if let Ok(shell) = std::env::var("SHELL") {
            return shell;
        }
        if which::which("bash").is_ok() {
            return "bash".to_string();
        }
        if which::which("sh").is_ok() {
            return "sh".to_string();
        }
        "sh".to_string()
    }

    #[cfg(not(any(windows, unix)))]
    {
        "cmd".to_string()
    }
}
