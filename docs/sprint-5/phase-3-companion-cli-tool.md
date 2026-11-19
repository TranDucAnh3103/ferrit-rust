# Sprint 5 Phase 3 — Companion Headless CLI Tool
**Document Identifier:** SPRINT5-PHASE3-SPEC  
**Target Milestone:** Release v2.6.0-extensible  
**Status:** Approved Technical Architecture  
**Parent Sprint Guide:** [Sprint 5 Master Guide](file:///F:/Build/JavaToolGitAutoUpload/rust-gitupload/docs/sprint-5/README.md)  

---

## 1. Overview & Architectural Objectives

Phase 3 delivers **`gitupload-cli`** (invoked as `gitupload`), a high-performance, standalone, headless terminal binary. Designed for terminal power users, shell scripting, automation bots, and CI/CD pipelines, the CLI shares core workspace crates with the desktop application while stripping away all GUI overhead.

### 1.1 Architectural Pillars
1. **Ultra-Low Latency Execution**: Pure Rust implementation with **sub-10ms cold start latency** and `<15 MB` peak RSS memory consumption.
2. **Unified Core Engine**: Directly leverages `gitupload-core`, `gitupload-git`, `gitupload-security`, `gitupload-ghcli`, and `gitupload-ai`, guaranteeing identical behavior between GUI and CLI.
3. **Shared State & Vault**: Reads and writes to the exact same SQLite database and OS Credential Vault (Windows Credential Manager, macOS Keychain, Linux Secret Service).
4. **Human & Machine Interfaces**: Rich terminal feedback styled with Primer color palettes and interactive spinners (`indicatif`), paired with a `--json` flag on all querying subcommands for clean piping into `jq`.

```
+------------------------------------------------------------------------------------------+
|                             `gitupload-cli` Architecture                                 |
|                                                                                          |
|   +----------------------------------------------------------------------------------+   |
|   | Terminal User / Script / CI Pipeline                                             |   |
|   | $ gitupload push -m "feat: login" --ai                                           |   |
|   +------------------------------------------+---------------------------------------+   |
|                                              | Invokes                                   |
|                                              v                                           |
|   +----------------------------------------------------------------------------------+   |
|   | crates/gitupload-cli (clap v4 Derive Parser) [<10ms Startup]                      |   |
|   +------------------------------------------+---------------------------------------+   |
|                                              | Direct Crate Reuse                        |
|                                              v                                           |
|   +----------------------------------------------------------------------------------+   |
|   | Shared Domain Crates (No Tauri / No Webview)                                     |   |
|   |                                                                                  |   |
|   |  +---------------------+  +---------------------+  +--------------------------+  |   |
|   |  | gitupload-git       |  | gitupload-security  |  | gitupload-storage        |  |   |
|   |  | (libgit2 / revwalk) |  | (OS Keyring Vault)  |  | (Shared SQLite DB)       |  |   |
|   |  +---------------------+  +---------------------+  +--------------------------+  |   |
|   |  | gitupload-ghcli     |  | gitupload-ai        |  | gitupload-hooks          |  |   |
|   |  | (gh CLI Bridge)     |  | (Cloud AI Client)   |  | (WASM / Native Hooks)    |  |   |
|   |  +----------------------------------------------------------------------------+  |   |
|   +----------------------------------------------------------------------------------+   |
+------------------------------------------------------------------------------------------+
```

---

## 2. Command Hierarchy & `clap` Specification

```rust
// crates/gitupload-cli/src/cli.rs
use clap::{Args, Parser, Subcommand, ValueEnum};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(
    name = "gitupload",
    author = "GitUpload Engineering Team",
    version = "2.6.0",
    about = "Blazing-fast GitHub-fidelity git management and AI automation CLI",
    long_about = "GitUpload CLI provides headless, scriptable git operations, AI commit message generation, multi-repo batch execution, and GitHub integration with sub-10ms startup."
)]
pub struct Cli {
    #[arg(short, long, global = true, help = "Output machine-readable JSON format")]
    pub json: bool,

    #[arg(short, long, global = true, help = "Silence non-error output")]
    pub quiet: bool,

    #[arg(short, long, global = true, help = "Repository path (defaults to current directory)")]
    pub repo: Option<PathBuf>,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    #[command(about = "Display detailed repository status, branch tracking, and staged files")]
    Status(StatusArgs),

    #[command(about = "Stage files, commit, and push to upstream remote")]
    Push(PushArgs),

    #[command(about = "Synchronize changes with upstream remote (fetch, pull/rebase)")]
    Sync(SyncArgs),

    #[command(about = "Execute operations across multiple repositories concurrently")]
    Batch(BatchArgs),

    #[command(about = "Inspect and interact with GitHub Pull Requests")]
    Pr(PrArgs),

    #[command(about = "Manage GitUpload configuration, profiles, and credentials")]
    Config(ConfigArgs),

    #[command(about = "Generate shell auto-completion scripts")]
    Completions {
        #[arg(value_enum)]
        shell: clap_complete::Shell,
    },
}

#[derive(Args, Debug)]
pub struct StatusArgs {
    #[arg(short, long, help = "Give the output in the short-format")]
    pub short: bool,
}

#[derive(Args, Debug)]
pub struct PushArgs {
    #[arg(short, long, help = "Commit message (if staged changes exist)")]
    pub message: Option<String>,

    #[arg(long, help = "Use Cloud AI to generate conventional commit message")]
    pub ai: bool,

    #[arg(short, long, help = "Target branch (defaults to current branch)")]
    pub branch: Option<String>,

    #[arg(long, help = "Bypass pre-push hooks")]
    pub no_verify: bool,

    #[arg(long, help = "Push with lease")]
    pub force_with_lease: bool,
}

#[derive(Args, Debug)]
pub struct SyncArgs {
    #[arg(long, help = "Rebase local commits instead of creating merge commit")]
    pub rebase: bool,

    #[arg(long, help = "Automatically stash and pop uncommitted changes")]
    pub autostash: bool,
}

#[derive(Args, Debug)]
pub struct BatchArgs {
    #[arg(value_enum, help = "Action to execute across repositories")]
    pub action: BatchAction,

    #[arg(long, value_delimiter = ',', help = "Comma-separated list of repository paths")]
    pub paths: Option<Vec<PathBuf>>,

    #[arg(long, help = "Target all repositories registered in GitUpload desktop")]
    pub all_tracked: bool,

    #[arg(short, long, default_value_t = 3, help = "Concurrency limit (max parallel workers)")]
    pub concurrency: usize,
}

#[derive(ValueEnum, Clone, Copy, Debug)]
pub enum BatchAction {
    Status,
    Sync,
    Push,
}

#[derive(Args, Debug)]
pub struct PrArgs {
    #[command(subcommand)]
    pub action: PrSubcommands,
}

#[derive(Subcommand, Debug)]
pub enum PrSubcommands {
    #[command(about = "List open pull requests")]
    List,
    #[command(about = "Create a new pull request on GitHub")]
    Create {
        #[arg(short, long)]
        title: Option<String>,
        #[arg(short, long)]
        body: Option<String>,
        #[arg(long)]
        draft: bool,
        #[arg(long)]
        ai_description: bool,
    },
    #[command(about = "Checkout a pull request branch locally")]
    Checkout { pr_number: u64 },
}

#[derive(Args, Debug)]
pub struct ConfigArgs {
    #[command(subcommand)]
    pub action: ConfigSubcommands,
}

#[derive(Subcommand, Debug)]
pub enum ConfigSubcommands {
    List,
    Get { key: String },
    Set { key: String, value: String },
    SwitchProfile { profile_name: String },
}
```

---

## 3. High-Performance Execution & Terminal Output

### 3.1 Sub-10ms Startup Optimization
To guarantee execution under 10 milliseconds:
1. **No Heavy Runtime Initialization**: Tokio runtime is initialized only when async operations (network fetch, push, AI generation) are requested. Synchronous local commands like `status --short` run immediately on the main thread via `gitupload-git`.
2. **Lazy Dynamic Linking**: External libraries and credentials are only loaded on demand.
3. **Optimized Release Flags**: Compiled with `opt-level = 3`, `lto = "fat"`, `codegen-units = 1`, and `panic = "abort"`.

### 3.2 Human Output vs `--json` Output

```rust
// crates/gitupload-cli/src/output.rs
use colored::*;
use serde::Serialize;

pub fn print_result<T: Serialize>(data: &T, human_formatter: impl FnOnce(&T)) {
    if std::env::var("GITUPLOAD_JSON").is_ok() {
        println!("{}", serde_json::to_string_pretty(data).unwrap());
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
        println!("  {} {} files modified but untracked", "•".yellow(), modified);
    }
    if staged == 0 && modified == 0 {
        println!("  {} Working tree clean", "✔".green());
    }
}
```

---

## 4. Work Breakdown & Granular Tasks

### TASK-P5-3-01: Crate Scaffolding & CLI Parser
- **Target Files:**  
  `rust-gitupload/crates/gitupload-cli/Cargo.toml`  
  `rust-gitupload/crates/gitupload-cli/src/main.rs`  
  `rust-gitupload/crates/gitupload-cli/src/cli.rs`
- **Actions:**
  - Create `crates/gitupload-cli` with dependencies on `clap = { version = "4.5", features = ["derive", "env"] }`, `colored = "2.1"`, `indicatif = "0.17"`, and internal crates.
  - Set up `#![forbid(unsafe_code)]`.
  - Configure release profile with LTO and binary stripping.

### TASK-P5-3-02: Status, Push, and Sync Command Implementation
- **Target Files:**  
  `rust-gitupload/crates/gitupload-cli/src/commands/status.rs`  
  `rust-gitupload/crates/gitupload-cli/src/commands/push.rs`  
  `rust-gitupload/crates/gitupload-cli/src/commands/sync.rs`
- **Actions:**
  - Connect `status` to `gitupload_git::RepositoryStatus`.
  - Wire `push --ai` to `gitupload_ai` for automated conventional commit generation.
  - Implement interactive progress spinners using `indicatif::ProgressBar`.

### TASK-P5-3-03: Multi-Repository Batch Command
- **Target Files:**  
  `rust-gitupload/crates/gitupload-cli/src/commands/batch.rs`
- **Actions:**
  - Query tracked repository list from SQLite database when `--all-tracked` is set.
  - Spawn worker tasks bounded by `tokio::sync::Semaphore` (default 3 concurrent workers).
  - Print live multi-bar progress displays for each repository.

### TASK-P5-3-04: PR & Config Command Subsystem
- **Target Files:**  
  `rust-gitupload/crates/gitupload-cli/src/commands/pr.rs`  
  `rust-gitupload/crates/gitupload-cli/src/commands/config.rs`
- **Actions:**
  - Retrieve GitHub PAT from OS Credential Vault via `gitupload_security`.
  - Query or create pull requests using `gitupload_github`.
  - Enable profile switching (`config switch-profile work`) updating the active configuration profile in SQLite.

### TASK-P5-3-05: Shell Auto-Completion Generator
- **Target Files:**  
  `rust-gitupload/crates/gitupload-cli/src/commands/completions.rs`  
  `rust-gitupload/crates/gitupload-cli/build.rs`
- **Actions:**
  - Integrate `clap_complete` to generate tab-completion scripts for `bash`, `zsh`, `fish`, and `powershell`.
  - Export scripts during `cargo build` to `target/completions/`.

---

## 5. Acceptance Criteria (Given-When-Then)

### AC-P5-3-01: Sub-10ms Cold Start Execution
- **Given:** The compiled `gitupload-cli` binary.
- **When:** A user executes `gitupload --version` or `gitupload status --short` in a clean terminal.
- **Then:** Total wall-clock execution time measures under 10 milliseconds.

### AC-P5-3-02: Terminal Progress Display & Colorized Primer Styling
- **Given:** A repository with 5 uncommitted files.
- **When:** A user executes `gitupload push -m "feat(auth): add oauth"` without the `--quiet` flag.
- **Then:** The terminal displays interactive spinners for staging, committing, and remote push with Primer green/yellow status markers.

### AC-P5-3-03: Machine-Readable JSON Output via `--json`
- **Given:** A terminal script executing `gitupload status --json`.
- **When:** The output is piped to `jq '.branch'`.
- **Then:** The CLI emits valid JSON containing keys `branch`, `staged_files`, `clean`, and exit status code 0.

### AC-P5-3-04: Concurrent Multi-Repo Batch Push via CLI
- **Given:** 5 repositories specified via `--paths repo1,repo2,repo3,repo4,repo5`.
- **When:** The user runs `gitupload batch push -c 3`.
- **Then:** The CLI throttles concurrent network uploads to at most 3 simultaneous jobs, completes all 5, and outputs a summary table of successes and durations.

### AC-P5-3-05: Shell Auto-Completion for Bash, Zsh, and PowerShell
- **Given:** The `gitupload completions powershell` command executed in PowerShell.
- **When:** The user tabs after typing `gitupload p`.
- **Then:** The shell autocompletes `push` and `pr` options accurately without syntax warnings.

---

*Document Author:* `@srs_author`  
*Reviewers & Verifiers:* `@verifier`, `@coordinator`  
*Target Release:* GitUpload v2.6.0-extensible
