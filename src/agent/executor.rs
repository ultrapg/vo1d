use crate::models::action::Action;
use crate::AppContext;
use crate::tools::files::FileOps;
use crate::tools::shell::ShellExec;
use crate::tools::registry::ToolRegistry;
use anyhow::Result;
use std::sync::Arc;

/// Executes actions by dispatching to the appropriate tool.
pub struct ToolExecutor;

impl ToolExecutor {
    /// Execute an action and return the output as a string.
    pub async fn execute(action: &Action, ctx: &AppContext, _registry: &Arc<ToolRegistry>) -> Result<String> {
        match action {
            Action::ReadFile { path, start_line, end_line } => {
                let expanded = ctx.paths.resolve_workspace_path(path);
                FileOps::read(&expanded, *start_line, *end_line)
            }
            Action::WriteFile { path, content, append } => {
                let expanded = ctx.paths.resolve_workspace_path(path);
                FileOps::write(&expanded, content, append.unwrap_or(false))
            }
            Action::ExecuteCommand { command, timeout, workdir } => {
                let cwd = workdir.as_ref()
                    .map(|d| ctx.paths.resolve_workspace_path(d))
                    .unwrap_or_else(|| ctx.paths.workspace_dir());
                ShellExec::execute(command.as_str(), timeout.unwrap_or(60), &cwd).await
            }
            Action::ListDirectory { path, pattern } => {
                let expanded = ctx.paths.resolve_workspace_path(path);
                FileOps::list_dir(&expanded, pattern.as_deref())
            }
            Action::SearchFiles { pattern, path, search_type: _ } => {
                let search_path = path.as_ref()
                    .map(|p| ctx.paths.resolve_workspace_path(p))
                    .unwrap_or_else(|| ctx.paths.workspace_dir());
                FileOps::search(&search_path, pattern)
            }
            Action::DeleteFile { path, pattern } => {
                let expanded = ctx.paths.resolve_workspace_path(path);
                if let Some(pat) = pattern {
                    FileOps::delete_matching(&expanded, pat)
                } else {
                    FileOps::delete(&expanded)
                }
            }
            Action::CopyFile { source, destination } => {
                let src = ctx.paths.resolve_workspace_path(source);
                let dst = ctx.paths.resolve_workspace_path(destination);
                FileOps::copy(&src, &dst)
            }
            Action::CreateDirectory { path } => {
                let expanded = ctx.paths.resolve_workspace_path(path);
                FileOps::create_dir(&expanded)
            }
            Action::FileMetadata { path } => {
                let expanded = ctx.paths.resolve_workspace_path(path);
                FileOps::metadata(&expanded)
            }
            Action::HttpRequest { url, method, headers, body } => {
                crate::tools::web::http_request(url, method.as_deref(), headers.as_ref(), body.as_deref()).await
            }
            Action::Finish { output } => {
                Ok(output.clone().unwrap_or_default())
            }
            Action::AskUser { question } => {
                println!("[VO1D ASKS] {}", question);
                let mut input = String::new();
                std::io::stdin().read_line(&mut input)?;
                Ok(format!("User response: {}", input.trim()))
            }
            Action::WebSearch { query, num_results } => {
                crate::tools::web_search::WebSearch::search(query, *num_results).await
            }
            Action::WebFetch { url, max_chars } => {
                crate::tools::web_fetch::WebFetch::fetch(url, *max_chars).await
            }
            Action::ShowChanges { path } => {
                let expanded = path.as_ref()
                    .map(|p| ctx.paths.resolve_workspace_path(p))
                    .map(|p| p.to_string_lossy().to_string());
                crate::tools::changes::Changes::show(expanded.as_deref())
            }
            Action::RestoreBackup { path } => {
                let expanded = ctx.paths.resolve_workspace_path(path);
                crate::tools::changes::Changes::restore(&expanded.to_string_lossy())
            }
        }
    }
}
