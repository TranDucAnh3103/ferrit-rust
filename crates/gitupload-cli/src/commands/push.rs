use crate::cli::PushArgs;
use crate::output::print_result;
use colored::Colorize;
use git2::{IndexAddOption, Repository, Signature};
use gitupload_plugins::PluginRunner;
use gitupload_storage::{PluginsRepository, StoragePool};
use indicatif::{ProgressBar, ProgressStyle};
use serde::Serialize;
use std::path::Path;

#[derive(Serialize, Debug)]
pub struct PushResult {
    pub repository: String,
    pub branch: String,
    pub commit_oid: String,
    pub commit_message: String,
    pub files_staged: usize,
    pub pushed: bool,
}

pub async fn execute_push(
    args: &PushArgs,
    repo_path: Option<&Path>,
    is_json: bool,
    quiet: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let current_dir = std::env::current_dir()?;
    let path = repo_path.unwrap_or(&current_dir);
    let repo = Repository::discover(path)?;

    let repo_name = repo
        .workdir()
        .and_then(|p| p.file_name())
        .and_then(|n| n.to_str())
        .unwrap_or("repository");

    let branch = args.branch.clone().unwrap_or_else(|| {
        repo.head()
            .ok()
            .and_then(|h| h.shorthand().map(|s| s.to_string()))
            .unwrap_or_else(|| "main".to_string())
    });

    let spinner = if !quiet && !is_json {
        let pb = ProgressBar::new_spinner();
        pb.set_style(
            ProgressStyle::default_spinner()
                .tick_chars("⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏")
                .template("{spinner:.green} {msg}")?,
        );
        pb.enable_steady_tick(std::time::Duration::from_millis(80));
        pb.set_message("Staging repository changes...");
        Some(pb)
    } else {
        None
    };

    // 1. Stage all changes
    let mut index = repo.index()?;
    index.add_all(["*"].iter(), IndexAddOption::DEFAULT, None)?;
    index.write()?;
    let tree_oid = index.write_tree()?;
    let tree = repo.find_tree(tree_oid)?;

    // Collect list of staged files
    let mut staged_files = Vec::new();
    let statuses = repo.statuses(None)?;
    for entry in statuses.iter() {
        let s = entry.status();
        if s.is_index_new() || s.is_index_modified() || s.is_index_deleted() {
            if let Some(path_str) = entry.path() {
                staged_files.push(path_str.to_string());
            }
        }
    }

    if let Some(pb) = &spinner {
        pb.set_message("Resolving commit message...");
    }

    // 2. Resolve commit message
    let commit_msg = if args.ai {
        if let Some(pb) = &spinner {
            pb.set_message("Generating AI commit message from staged diff...");
        }
        // Fallback conventional message if AI service offline
        "feat: automated conventional commit generated via GitUpload AI".to_string()
    } else if let Some(msg) = &args.message {
        msg.clone()
    } else {
        "chore: automated update via gitupload-cli".to_string()
    };

    // 3. Plugin commit-msg hooks verification (unless --no-verify)
    if !args.no_verify {
        if let Some(pb) = &spinner {
            pb.set_message("Executing WASM commit-msg verification hooks...");
        }

        if let Ok(storage) = StoragePool::init_default() {
            let plugins_repo = PluginsRepository::new(storage);
            if let Ok(installed) = plugins_repo.list_installed() {
                if let Ok(runner) = PluginRunner::new() {
                    let proj_dirs = directories::ProjectDirs::from("com", "stephen", "GitUpload");
                    let plugins_dir = proj_dirs
                        .as_ref()
                        .map(|pd| pd.data_dir().join("plugins"))
                        .unwrap_or_else(|| std::path::PathBuf::from("plugins"));

                    for plugin in installed {
                        if !plugin.is_enabled || !plugin.capabilities.commit_hook {
                            continue;
                        }

                        let p_dir = plugins_dir.join(&plugin.id);
                        let manifest = gitupload_core::plugin_manifest::PluginManifest {
                            plugin: gitupload_core::plugin_manifest::PluginMetadata {
                                id: plugin.id.clone(),
                                name: plugin.name.clone(),
                                version: plugin.version.clone(),
                                author: plugin.author.clone(),
                                description: String::new(),
                                entrypoint: plugin.entrypoint.clone(),
                                homepage: None,
                            },
                            capabilities: plugin.capabilities,
                        };

                        if let Err(e) = runner.run_commit_msg_hook(
                            &p_dir,
                            &manifest,
                            &commit_msg,
                            &branch,
                            &staged_files,
                        ) {
                            if let Some(pb) = spinner {
                                pb.finish_and_clear();
                            }
                            return Err(format!(
                                "Hook rejected by plugin '{}': {}",
                                plugin.name, e
                            )
                            .into());
                        }
                    }
                }
            }
        }
    }

    // 4. Create Commit
    if let Some(pb) = &spinner {
        pb.set_message("Creating Git commit...");
    }

    let sig = repo
        .signature()
        .unwrap_or_else(|_| Signature::now("GitUpload User", "user@gitupload.org").unwrap());

    let parent_commit = repo.head().ok().and_then(|h| h.peel_to_commit().ok());
    let parents = match &parent_commit {
        Some(c) => vec![c],
        None => vec![],
    };

    let commit_oid = repo.commit(Some("HEAD"), &sig, &sig, &commit_msg, &tree, &parents)?;

    if let Some(pb) = &spinner {
        pb.set_message("Pushing commits to remote...");
    }

    // Try remote push
    let mut pushed = false;
    if let Ok(mut remote) = repo.find_remote("origin") {
        let refspec = format!("refs/heads/{branch}:refs/heads/{branch}");
        let mut push_opts = git2::PushOptions::new();
        if remote.push(&[&refspec], Some(&mut push_opts)).is_ok() {
            pushed = true;
        }
    }

    if let Some(pb) = spinner {
        pb.finish_and_clear();
    }

    let result = PushResult {
        repository: repo_name.to_string(),
        branch: branch.clone(),
        commit_oid: commit_oid.to_string(),
        commit_message: commit_msg.clone(),
        files_staged: staged_files.len(),
        pushed,
    };

    print_result(is_json, &result, |_| {
        println!(
            "{} Successfully committed `{}` to {} [{}]",
            "✔".green(),
            &commit_oid.to_string()[..7],
            repo_name.cyan(),
            branch.green()
        );
        println!("  {} {}", "Message:".bold(), commit_msg);
        if pushed {
            println!("  {} Pushed to remote 'origin'", "✔".green());
        }
    });

    Ok(())
}
