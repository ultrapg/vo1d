use crate::models::action::{Action, SkillStepDef};
use crate::tools::skills::{Skill, SkillStep};
use crate::AppContext;
use crate::tools::files::FileOps;
use crate::tools::shell::ShellExec;
use crate::tools::registry::ToolRegistry;
use anyhow::Result;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

/// Executes actions by dispatching to the appropriate tool.
pub struct ToolExecutor;

impl ToolExecutor {
    /// Execute an action and return the output as a string.
    pub fn execute<'a>(
        action: &'a Action,
        ctx: &'a AppContext,
        _registry: &'a Arc<ToolRegistry>,
    ) -> Pin<Box<dyn Future<Output = Result<String>> + Send + 'a>> {
        Box::pin(async move {
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
                let mut reg = ctx.skill_registry.lock().map_err(|e| anyhow::anyhow!("Lock error: {}", e))?;
                reg.create(skill)?;
                Ok(format!("Skill '{}' created with {} steps", name, steps.len()))
            }
            Action::InvokeSkill { name, params } => {
                let params = params.as_ref().cloned().unwrap_or(serde_json::Value::Object(Default::default()));
                let steps = {
                    let reg = ctx.skill_registry.lock().map_err(|e| anyhow::anyhow!("Lock error: {}", e))?;
                    reg.resolve_steps(name, &params)?
                };
                let mut outputs = Vec::new();
                for (i, (action, tool_name)) in steps.iter().enumerate() {
                    match ToolExecutor::execute(action, ctx, _registry).await {
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
                let reg = ctx.skill_registry.lock().map_err(|e| anyhow::anyhow!("Lock error: {}", e))?;
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
                let mut reg = ctx.skill_registry.lock().map_err(|e| anyhow::anyhow!("Lock error: {}", e))?;
                let existed = reg.delete(name)?;
                if existed {
                    Ok(format!("Skill '{}' deleted.", name))
                } else {
                    Ok(format!("Skill '{}' not found.", name))
                }
            }
        }
        })
    }
}
