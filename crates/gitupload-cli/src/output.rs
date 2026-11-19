use colored::Colorize;
use serde::Serialize;

pub fn print_result<T: Serialize>(is_json: bool, data: &T, human_formatter: impl FnOnce(&T)) {
    if is_json || std::env::var("GITUPLOAD_JSON").is_ok() {
        if let Ok(json_str) = serde_json::to_string_pretty(data) {
            println!("{json_str}");
        }
    } else {
        human_formatter(data);
    }
}

pub fn render_status_human(repo_name: &str, branch: &str, staged: usize, modified: usize) {
    println!("{} {}", "Repository:".bold(), repo_name.cyan());
    println!("{} {}", "Branch:".bold(), branch.green());
    if staged > 0 {
        println!("  {} {} files ready to commit", "✔".green(), staged);
    }
    if modified > 0 {
        println!(
            "  {} {} files modified but untracked",
            "•".yellow(),
            modified
        );
    }
    if staged == 0 && modified == 0 {
        println!("  {} Working tree clean", "✔".green());
    }
}
