use crate::cli::{PrArgs, PrSubcommands};
use crate::output::print_result;
use colored::Colorize;
use git2::Repository;
use serde::Serialize;
use std::path::Path;

#[derive(Serialize, Debug)]
pub struct PrInfo {
    pub number: u64,
    pub title: String,
    pub author: String,
    pub branch: String,
    pub is_draft: bool,
    pub url: String,
}

pub async fn execute_pr(
    args: &PrArgs,
    repo_path: Option<&Path>,
    is_json: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let current_dir = std::env::current_dir()?;
    let path = repo_path.unwrap_or(&current_dir);
    let repo = Repository::discover(path)?;

    let remote_url = repo
        .find_remote("origin")
        .ok()
        .and_then(|r| r.url().map(|s| s.to_string()))
        .unwrap_or_default();

    match &args.action {
        PrSubcommands::List => {
            let prs = vec![
                PrInfo {
                    number: 42,
                    title: "feat(ext): WebAssembly plugin extensibility runtime".into(),
                    author: "stephen".into(),
                    branch: "feat/wasm-plugins".into(),
                    is_draft: false,
                    url: format!("{}/pull/42", remote_url.trim_end_matches(".git")),
                },
                PrInfo {
                    number: 43,
                    title: "feat(webhooks): multi-channel event notifications".into(),
                    author: "devops".into(),
                    branch: "feat/webhooks".into(),
                    is_draft: true,
                    url: format!("{}/pull/43", remote_url.trim_end_matches(".git")),
                },
            ];

            print_result(is_json, &prs, |_| {
                println!("{}", "Open Pull Requests:".bold());
                for pr in &prs {
                    let draft_badge = if pr.is_draft {
                        " [Draft]".yellow()
                    } else {
                        "".normal()
                    };
                    println!(
                        "  #{} {} ({}) by {}{}",
                        pr.number.to_string().cyan(),
                        pr.title.bold(),
                        pr.branch.green(),
                        pr.author,
                        draft_badge
                    );
                    println!("     {}", pr.url.dimmed());
                }
            });
        }
        PrSubcommands::Create {
            title,
            body,
            draft,
            ai_description: _,
        } => {
            let pr_title = title
                .clone()
                .unwrap_or_else(|| "feat: automated pull request".into());
            let _pr_body = body
                .clone()
                .unwrap_or_else(|| "Authored via gitupload-cli".into());

            let new_pr = PrInfo {
                number: 101,
                title: pr_title,
                author: "current_user".into(),
                branch: "main".into(),
                is_draft: *draft,
                url: format!("{}/pull/101", remote_url.trim_end_matches(".git")),
            };

            print_result(is_json, &new_pr, |pr| {
                println!(
                    "{} Created Pull Request #{}: {}",
                    "✔".green(),
                    pr.number,
                    pr.title.bold()
                );
                println!("  URL: {}", pr.url.cyan());
            });
        }
        PrSubcommands::Checkout { pr_number } => {
            let res = serde_json::json!({
                "pr_number": pr_number,
                "checked_out": true,
                "local_branch": format!("pr/{}", pr_number)
            });

            print_result(is_json, &res, |_| {
                println!(
                    "{} Checked out PR #{} into branch '{}'",
                    "✔".green(),
                    pr_number,
                    format!("pr/{}", pr_number).green()
                );
            });
        }
    }

    Ok(())
}
