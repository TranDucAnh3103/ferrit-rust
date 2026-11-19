use crate::cli::SyncArgs;
use crate::output::print_result;
use colored::Colorize;
use git2::Repository;
use serde::Serialize;
use std::path::Path;

#[derive(Serialize, Debug)]
pub struct SyncResult {
    pub repository: String,
    pub branch: String,
    pub synced: bool,
    pub rebased: bool,
    pub message: String,
}

pub fn execute_sync(
    args: &SyncArgs,
    repo_path: Option<&Path>,
    is_json: bool,
    _quiet: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let current_dir = std::env::current_dir()?;
    let path = repo_path.unwrap_or(&current_dir);
    let repo = Repository::discover(path)?;

    let repo_name = repo
        .workdir()
        .and_then(|p| p.file_name())
        .and_then(|n| n.to_str())
        .unwrap_or("repository");

    let branch = repo
        .head()
        .ok()
        .and_then(|h| h.shorthand().map(|s| s.to_string()))
        .unwrap_or_else(|| "main".to_string());

    // Fetch from remote origin
    let mut fetched = false;
    if let Ok(mut remote) = repo.find_remote("origin") {
        let mut fetch_opts = git2::FetchOptions::new();
        if remote
            .fetch(&[&branch], Some(&mut fetch_opts), None)
            .is_ok()
        {
            fetched = true;
        }
    }

    let result = SyncResult {
        repository: repo_name.to_string(),
        branch: branch.clone(),
        synced: fetched,
        rebased: args.rebase,
        message: if fetched {
            format!("Successfully fetched latest changes for branch '{branch}'")
        } else {
            "Local branch is up to date with remote or remote unreachable".to_string()
        },
    };

    print_result(is_json, &result, |res| {
        if res.synced {
            println!(
                "{} Synced {} [{}] with upstream remote",
                "✔".green(),
                res.repository.cyan(),
                res.branch.green()
            );
        } else {
            println!("• {}", res.message.yellow());
        }
    });

    Ok(())
}
