use crate::models::action::{Action, SkillStepDef};
use crate::tools::skills::{Skill, SkillStep};
use crate::AppContext;
use crate::tools::files::FileOps;
use crate::tools::shell::ShellExec;
use crate::tools::registry::ToolRegistry;
use anyhow::{Result};
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

const MAX_RECURSION_DEPTH: usize = 10;
const MAX_OUTPUT_SIZE: usize = 1_048_576; // 1MB max output per tool call

/// Executes actions by dispatching to the appropriate tool.
pub struct ToolExecutor;

impl ToolExecutor {
    /// Execute an action and return the output as a string.
    pub fn execute<'a>(
        action: &'a Action,
        ctx: &'a AppContext,
        registry: &'a Arc<ToolRegistry>,
    ) -> Pin<Box<dyn Future<Output = Result<String>> + Send + 'a>> {
        Box::pin(Self::execute_inner(action, ctx, registry, 0))
    }

    /// Execute with explicit recursion depth tracking.
    /// `depth` tracks nested skill invocations to prevent stack overflow.
    fn execute_with_depth<'a>(
        action: &'a Action,
        ctx: &'a AppContext,
        registry: &'a Arc<ToolRegistry>,
        depth: usize,
    ) -> Pin<Box<dyn Future<Output = Result<String>> + Send + 'a>> {
        Box::pin(Self::execute_inner(action, ctx, registry, depth))
    }

    async fn execute_inner<'a>(
        action: &'a Action,
        ctx: &'a AppContext,
        registry: &'a Arc<ToolRegistry>,
        depth: usize,
    ) -> Result<String> {
        let result: Result<String> = match action {
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
            Action::EditFile { path, start_line, end_line, content } => {
                let expanded = ctx.paths.resolve_workspace_path(path);
                FileOps::edit(&expanded, *start_line, *end_line, content)
            }
            Action::SearchInFiles { pattern, path, file_pattern, max_results } => {
                let search_path = path.as_ref()
                    .map(|p| ctx.paths.resolve_workspace_path(p))
                    .unwrap_or_else(|| ctx.paths.workspace_dir());
                FileOps::search_in_files(&search_path, pattern, file_pattern.as_deref(), *max_results)
            }
            Action::RagQuery { path, query, num_chunks } => {
                let expanded = ctx.paths.resolve_workspace_path(path);
                FileOps::rag_query(&expanded, query, *num_chunks)
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
            Action::PlanCreate { .. } => Ok("Plan created (handled in loop)".to_string()),
            Action::PlanStepComplete { .. } => Ok("Step completed (handled in loop)".to_string()),
            Action::PlanStepFail { .. } => Ok("Step failed (handled in loop)".to_string()),
            Action::PlanStatus { .. } => Ok("Plan status (handled in loop)".to_string()),
            Action::CreateSkill { name, description, params_schema, steps } => {
                let skill = Skill {
                    name: name.clone(),
                    description: description.clone(),
                    params_schema: params_schema.clone().unwrap_or_default(),
                    steps: steps.iter().map(|s: &SkillStepDef| SkillStep {
                        tool: s.tool.clone(),
                        args: s.args.clone(),
                    }).collect(),
                    created_at: chrono::Utc::now().to_rfc3339(),
                };
                let mut reg = ctx.skill_registry.lock().await;
                reg.create(skill)?;
                Ok(format!("Skill '{}' created with {} steps", name, steps.len()))
            }
            Action::InvokeSkill { name, params } => {
                if depth >= MAX_RECURSION_DEPTH {
                    anyhow::bail!(
                        "Skill recursion depth limit ({}) exceeded while invoking '{}'. \
                         Circular skill references are not allowed.",
                        MAX_RECURSION_DEPTH, name
                    );
                }
                let params = params.as_ref().cloned().unwrap_or(serde_json::Value::Object(Default::default()));
                let steps = {
                    let reg = ctx.skill_registry.lock().await;
                    reg.resolve_steps(name, &params)?
                };
                let mut outputs = Vec::new();
                for (i, (action, tool_name)) in steps.iter().enumerate() {
                    match ToolExecutor::execute_with_depth(action, ctx, registry, depth + 1).await {
                        Ok(output) => outputs.push(output),
                        Err(e) => {
                            let partial = outputs.join("\n---\n");
                            anyhow::bail!(
                                "Skill '{}' failed at step {} ({}): {}\nPartial output:\n{}",
                                name, i + 1, tool_name, e, partial,
                            );
                        }
                    }
                }
                Ok(outputs.join("\n---\n"))
            }
            Action::ListSkills { keyword } => {
                let reg = ctx.skill_registry.lock().await;
                let results = reg.list(keyword.as_deref());
                if results.is_empty() {
                    Ok("No skills found.".to_string())
                } else {
                    let mut out = String::new();
                    for skill in results {
                        out.push_str(&format!("- {}: {} ({} steps)\n", skill.name, skill.description, skill.steps.len()));
                    }
                    Ok(out.trim_end().to_string())
                }
            }
            Action::DeleteSkill { name } => {
                let mut reg = ctx.skill_registry.lock().await;
                let existed = reg.delete(name)?;
                if existed {
                    Ok(format!("Skill '{}' deleted.", name))
                } else {
                    Ok(format!("Skill '{}' not found.", name))
                }
            }
        };
        Ok(result.map(|content| {
            if content.len() > MAX_OUTPUT_SIZE {
                format!("{}...\n[Output truncated: {} total chars, max is {}]",
                    &content[..MAX_OUTPUT_SIZE], content.len(), MAX_OUTPUT_SIZE)
            } else {
                content
            }
        })?)
    }
}
