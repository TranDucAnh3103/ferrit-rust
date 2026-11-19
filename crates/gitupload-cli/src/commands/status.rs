use crate::cli::StatusArgs;
use crate::output::{print_result, render_status_human};
use colored::Colorize;
use git2::{Repository, StatusOptions};
use serde::Serialize;
use std::path::Path;

#[derive(Serialize, Debug)]
pub struct StatusResult {
    pub repository: String,
    pub branch: String,
    pub clean: bool,
    pub staged_files: usize,
    pub modified_files: usize,
    pub untracked_files: usize,
}

pub fn execute_status(
    args: &StatusArgs,
    repo_path: Option<&Path>,
    is_json: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let current_dir = std::env::current_dir()?;
    let path = repo_path.unwrap_or(&current_dir);

    let repo = Repository::discover(path)?;
    let repo_name = repo
        .workdir()
        .and_then(|p| p.file_name())
        .and_then(|n| n.to_str())
        .unwrap_or("unknown");

    let branch = repo
        .head()
        .ok()
        .and_then(|h| h.shorthand().map(|s| s.to_string()))
        .unwrap_or_else(|| "HEAD (detached)".to_string());

    let mut opts = StatusOptions::new();
    opts.include_untracked(true);
    let statuses = repo.statuses(Some(&mut opts))?;

    let mut staged = 0;
    let mut modified = 0;
    let mut untracked = 0;

    for entry in statuses.iter() {
        let s = entry.status();
        if s.is_index_new() || s.is_index_modified() || s.is_index_deleted() {
            staged += 1;
        }
        if s.is_wt_modified() || s.is_wt_deleted() {
            modified += 1;
        }
        if s.is_wt_new() {
            untracked += 1;
        }
    }

    let clean = staged == 0 && modified == 0 && untracked == 0;
    let result = StatusResult {
        repository: repo_name.to_string(),
        branch: branch.clone(),
        clean,
        staged_files: staged,
        modified_files: modified,
        untracked_files: untracked,
    };

    print_result(is_json, &result, |_| {
        if args.short {
            println!(
                "## {} [staged: {}, modified: {}, untracked: {}]",
                branch.green(),
                staged,
                modified,
                untracked
            );
        } else {
            render_status_human(repo_name, &branch, staged, modified + untracked);
        }
    });

    Ok(())
}
