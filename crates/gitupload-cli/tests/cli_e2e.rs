#![forbid(unsafe_code)]

use std::process::Command;
use tempfile::tempdir;

#[test]
fn test_cli_version_flag() {
    let cli_path = env!("CARGO_BIN_EXE_gitupload");
    let output = Command::new(cli_path)
        .arg("--version")
        .output()
        .expect("Failed to execute gitupload --version");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("gitupload") || stdout.contains("2."));
}

#[test]
fn test_cli_completions() {
    let cli_path = env!("CARGO_BIN_EXE_gitupload");
    let output = Command::new(cli_path)
        .args(["completions", "bash"])
        .output()
        .expect("Failed to execute gitupload completions");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("complete -F _gitupload"));
    assert!(stdout.contains("_gitupload()"));
}

#[test]
fn test_cli_status_json_in_repo() {
    let cli_path = env!("CARGO_BIN_EXE_gitupload");
    let temp_dir = tempdir().expect("Failed to create temp dir");
    let repo_path = temp_dir.path();

    // Initialize git repository using git CLI
    let init_status = Command::new("git")
        .args(["init", repo_path.to_str().unwrap()])
        .output();

    if let Ok(output) = init_status {
        if !output.status.success() {
            eprintln!("git init skipped or failed in test environment");
            return;
        }
    } else {
        eprintln!("git CLI not available, skipping repo status test");
        return;
    }

    // Write a dummy file and stage it
    let file_path = repo_path.join("file.txt");
    std::fs::write(&file_path, "hello CLI").unwrap();

    let _ = Command::new("git")
        .args(["add", "file.txt"])
        .current_dir(repo_path)
        .output();

    let output = Command::new(cli_path)
        .args(["status", "--json"])
        .current_dir(repo_path)
        .output()
        .expect("Failed to execute gitupload status --json");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let json_val: serde_json::Value =
        serde_json::from_str(&stdout).expect("Output must be valid JSON");

    assert_eq!(json_val["clean"], false);
    assert_eq!(json_val["staged_files"], 1);
}
